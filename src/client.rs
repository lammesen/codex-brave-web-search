use crate::config::{ApiKeyConfig, RuntimeConfig};
use crate::constants::{ERROR_CANCELLED, RETRYABLE_HTTP_STATUS, WARNING_RAW_PAYLOAD_TRUNCATED};
use crate::error::AppError;
use crate::parsing::{parse_brave_error_message, parse_sections, query_echo_or_original};
use crate::types::{FetchSearchParams, FetchSearchResult, SearchType, WarningEntry};
use futures_util::StreamExt;
use rand::Rng;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue};
use serde_json::Value;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct BraveClient {
    http: reqwest::Client,
    config: RuntimeConfig,
    api_key: ApiKeyConfig,
}

impl BraveClient {
    pub fn new(config: RuntimeConfig) -> Result<Self, AppError> {
        let http = reqwest::Client::builder()
            .user_agent(format!(
                "codex-brave-web-search/{}",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|error| {
                AppError::Internal(format!("Failed to create HTTP client: {error}"))
            })?;

        Ok(Self {
            http,
            config,
            api_key: ApiKeyConfig::from_env(),
        })
    }

    #[must_use]
    pub fn key_config(&self) -> &ApiKeyConfig {
        &self.api_key
    }

    #[must_use]
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    pub async fn fetch_search<F>(
        &self,
        query: &str,
        search_type: SearchType,
        params: &FetchSearchParams,
        is_cancelled: F,
    ) -> Result<FetchSearchResult, AppError>
    where
        F: Fn() -> bool,
    {
        let api_key = self.api_key.key.as_deref().ok_or(AppError::MissingApiKey)?;

        let request_url = self.build_request_url(query, search_type, params)?;

        let mut last_error: Option<AppError> = None;
        let mut last_status: Option<u16> = None;
        let mut last_body = String::new();

        for attempt in 0..=self.config.retry_count {
            if is_cancelled() {
                return Err(AppError::Cancelled);
            }

            let mut headers = HeaderMap::new();
            headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
            let subscription = HeaderValue::from_str(api_key)
                .map_err(|error| AppError::Internal(format!("Invalid API key header: {error}")))?;
            headers.insert("X-Subscription-Token", subscription);

            let send_result = tokio::time::timeout(
                Duration::from_millis(self.config.per_attempt_timeout_ms),
                self.http.get(request_url.clone()).headers(headers).send(),
            )
            .await;

            let response = match send_result {
                Ok(Ok(response)) => response,
                Ok(Err(error)) => {
                    last_error = Some(AppError::Upstream(format!(
                        "Failed to call Brave API: {error}"
                    )));
                    if attempt < self.config.retry_count {
                        self.wait_for_retry(None, attempt, &is_cancelled).await?;
                        continue;
                    }
                    break;
                }
                Err(_) => {
                    last_error = Some(AppError::Upstream(
                        "Per-attempt timeout waiting for Brave API response".to_string(),
                    ));
                    if attempt < self.config.retry_count {
                        self.wait_for_retry(None, attempt, &is_cancelled).await?;
                        continue;
                    }
                    break;
                }
            };

            let status = response.status().as_u16();
            let retry_after_header = response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);

            let read_body = tokio::time::timeout(
                Duration::from_millis(self.config.per_attempt_timeout_ms),
                self.read_response_body(response, &is_cancelled),
            )
            .await;

            let raw_body = match read_body {
                Ok(Ok(body)) => body,
                Ok(Err(error)) => {
                    last_error = Some(error);
                    if attempt < self.config.retry_count {
                        self.wait_for_retry(None, attempt, &is_cancelled).await?;
                        continue;
                    }
                    break;
                }
                Err(_) => {
                    last_error = Some(AppError::Upstream(
                        "Per-attempt timeout reading Brave API response".to_string(),
                    ));
                    if attempt < self.config.retry_count {
                        self.wait_for_retry(None, attempt, &is_cancelled).await?;
                        continue;
                    }
                    break;
                }
            };

            last_status = Some(status);
            last_body = raw_body.clone();

            if (200..300).contains(&status) {
                let parsed_payload = serde_json::from_str::<Value>(&raw_body)
                    .map_err(|error| AppError::Parse(format!("Invalid JSON response: {error}")))?;

                let parsed_sections = parse_sections(
                    &parsed_payload,
                    search_type,
                    &params.result_filter_values,
                    params.count,
                    params.text_decorations,
                );

                return Ok(FetchSearchResult {
                    sections: parsed_sections.sections,
                    has_more: parsed_sections.has_more,
                    warnings: parsed_sections.warnings,
                    query_echo: query_echo_or_original(&parsed_payload, query),
                    request_url,
                    raw_payload: parsed_payload,
                    raw_payload_bytes: raw_body.len(),
                });
            }

            if RETRYABLE_HTTP_STATUS.contains(&status) && attempt < self.config.retry_count {
                self.wait_for_retry(retry_after_header.as_deref(), attempt, &is_cancelled)
                    .await?;
                continue;
            }

            let fallback = format!("Request failed ({status}).");
            let detail = parse_brave_error_message(&raw_body, &fallback);
            return Err(AppError::Upstream(format!(
                "Brave Search API returned HTTP {status}: {detail}"
            )));
        }

        if let Some(error) = last_error {
            return Err(error);
        }

        if let Some(status) = last_status {
            let fallback = format!("Request failed ({status}).");
            let detail = parse_brave_error_message(&last_body, &fallback);
            return Err(AppError::Upstream(format!(
                "Brave Search API returned HTTP {status}: {detail}"
            )));
        }

        Err(AppError::Internal(
            "Brave request loop exited without a result".to_string(),
        ))
    }

    async fn wait_for_retry<F>(
        &self,
        retry_after_header: Option<&str>,
        attempt: usize,
        is_cancelled: &F,
    ) -> Result<(), AppError>
    where
        F: Fn() -> bool,
    {
        let delay_ms = compute_retry_delay_ms(
            attempt,
            retry_after_header,
            self.config.retry_base_delay_ms,
            self.config.retry_max_delay_ms,
        );

        let total_wait = Duration::from_millis(delay_ms);
        let step = Duration::from_millis(100);
        let start = std::time::Instant::now();

        while start.elapsed() < total_wait {
            if is_cancelled() {
                return Err(AppError::Cancelled);
            }
            let remaining = total_wait.saturating_sub(start.elapsed());
            tokio::time::sleep(remaining.min(step)).await;
        }

        Ok(())
    }

    async fn read_response_body<F>(
        &self,
        response: reqwest::Response,
        is_cancelled: &F,
    ) -> Result<String, AppError>
    where
        F: Fn() -> bool,
    {
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::<u8>::new();

        while let Some(chunk_result) = stream.next().await {
            if is_cancelled() {
                return Err(AppError::Cancelled);
            }

            let chunk = chunk_result.map_err(|error| {
                AppError::Upstream(format!("Failed while reading response body: {error}"))
            })?;

            if bytes.len() + chunk.len() > self.config.max_response_bytes {
                let max_bytes = self.config.max_response_bytes;
                let max_mebibytes = max_bytes as f64 / 1_048_576.0;
                return Err(AppError::Upstream(format!(
                    "Response body exceeded {max_bytes} byte limit ({max_mebibytes:.2} MiB)",
                )));
            }

            bytes.extend_from_slice(&chunk);
        }

        String::from_utf8(bytes)
            .map_err(|error| AppError::Parse(format!("Response body was not valid UTF-8: {error}")))
    }

    fn build_request_url(
        &self,
        query: &str,
        search_type: SearchType,
        params: &FetchSearchParams,
    ) -> Result<String, AppError> {
        let endpoint = self.config.endpoints.endpoint_for(search_type);
        let mut url = url::Url::parse(endpoint).map_err(|error| {
            AppError::Internal(format!("Invalid endpoint URL '{endpoint}': {error}"))
        })?;

        {
            let mut search_params = url.query_pairs_mut();
            search_params.append_pair("q", query);
            search_params.append_pair("count", &params.count.to_string());
            search_params.append_pair("text_decorations", &params.text_decorations.to_string());
            search_params.append_pair("extra_snippets", &params.extra_snippets.to_string());

            if params.offset > 0 {
                search_params.append_pair("offset", &params.offset.to_string());
            }
            if let Some(country) = &params.country {
                search_params.append_pair("country", country);
            }
            if let Some(search_language) = &params.search_language {
                search_params.append_pair("search_lang", search_language);
            }
            if let Some(ui_language) = &params.ui_language {
                search_params.append_pair("ui_lang", ui_language);
            }
            if let Some(units) = &params.units {
                search_params.append_pair("units", units);
            }
            if let Some(safe_search) = &params.safe_search {
                search_params.append_pair("safesearch", safe_search);
            }
            if let Some(freshness) = &params.freshness {
                search_params.append_pair("freshness", freshness);
            }
            search_params.append_pair(
                "spellcheck",
                if params.spellcheck { "true" } else { "false" },
            );

            if search_type == SearchType::Web && !params.result_filter_values.is_empty() {
                let filter = params
                    .result_filter_values
                    .iter()
                    .map(|value| value.as_str())
                    .collect::<Vec<&str>>()
                    .join(",");
                search_params.append_pair("result_filter", &filter);
            }
        }

        Ok(url.to_string())
    }

    pub async fn probe_endpoint<F>(
        &self,
        search_type: SearchType,
        is_cancelled: F,
    ) -> Result<(), AppError>
    where
        F: Fn() -> bool,
    {
        let params = FetchSearchParams {
            count: 1,
            offset: 0,
            country: None,
            search_language: None,
            ui_language: None,
            safe_search: None,
            freshness: None,
            result_filter_values: Vec::new(),
            units: None,
            spellcheck: true,
            extra_snippets: false,
            text_decorations: matches!(search_type, SearchType::News),
        };

        self.fetch_search("mcp healthcheck", search_type, &params, is_cancelled)
            .await
            .map(|_| ())
    }
}

#[must_use]
pub fn compute_retry_delay_ms(
    attempt: usize,
    retry_after_header: Option<&str>,
    base_delay_ms: u64,
    max_delay_ms: u64,
) -> u64 {
    let mut delay_ms = retry_after_header
        .and_then(parse_retry_after_delay_ms)
        .unwrap_or_else(|| {
            let exp = 2_u64.saturating_pow(attempt as u32);
            base_delay_ms.saturating_mul(exp)
        })
        .min(max_delay_ms);

    let jitter = rand::rng().random_range(0.8_f64..=1.2_f64);
    delay_ms = ((delay_ms as f64) * jitter).round() as u64;
    delay_ms.clamp(1, max_delay_ms)
}

fn parse_retry_after_delay_ms(retry_after_header: &str) -> Option<u64> {
    if let Ok(seconds) = retry_after_header.trim().parse::<u64>()
        && seconds > 0
    {
        return Some(seconds.saturating_mul(1_000));
    }

    let retry_time = httpdate::parse_http_date(retry_after_header).ok()?;
    let now = SystemTime::now();
    let diff = retry_time.duration_since(now).ok()?;
    Some(diff.as_millis().min(u128::from(u64::MAX)) as u64)
}

pub fn maybe_cap_debug_raw_payload(
    payload: &Value,
    original_size: usize,
    cap_bytes: usize,
    warnings: &mut Vec<WarningEntry>,
) -> (Option<Value>, bool, Option<usize>) {
    let serialized = serde_json::to_vec(payload).unwrap_or_default();
    if serialized.len() <= cap_bytes {
        return (Some(payload.clone()), false, Some(original_size));
    }

    warnings.push(WarningEntry {
        code: WARNING_RAW_PAYLOAD_TRUNCATED.to_string(),
        message: format!(
            "Raw payload exceeded debug cap ({} bytes > {} bytes); returning truncated preview object.",
            serialized.len(), cap_bytes
        ),
    });

    let preview =
        String::from_utf8_lossy(&serialized[..cap_bytes.min(serialized.len())]).to_string();
    let truncated = serde_json::json!({
        "truncated": true,
        "original_size_bytes": serialized.len(),
        "preview": preview,
    });
    (Some(truncated), true, Some(original_size))
}

#[must_use]
pub fn is_cancelled_error(error: &AppError) -> bool {
    matches!(error, AppError::Cancelled)
}

#[must_use]
pub fn cancelled_code() -> &'static str {
    ERROR_CANCELLED
}
