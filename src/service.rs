use crate::cache::SearchCache;
use crate::client::{BraveClient, maybe_cap_debug_raw_payload};
use crate::config::RuntimeConfig;
use crate::constants::{
    API_VERSION, DEFAULT_SEARCH_TYPE, PROVIDER_NAME, WARNING_INVALID_COUNTRY,
    WARNING_INVALID_FRESHNESS, WARNING_INVALID_RESULT_FILTER, WARNING_INVALID_SAFE_SEARCH,
    WARNING_INVALID_SEARCH_LANGUAGE, WARNING_INVALID_UI_LANGUAGE, WARNING_INVALID_UNITS,
    WARNING_OFFSET_CAPPED, WARNING_QUERY_TRUNCATED, WARNING_RESULT_FILTER_IGNORED,
};
use crate::error::AppError;
use crate::formatting::{build_summary, enforce_output_limits, to_result_item};
use crate::normalization::{
    clamp_offset, is_valid_search_type_input, normalize_country, normalize_freshness,
    normalize_safe_search, normalize_search_type, normalize_ui_language, normalize_units,
    parse_result_filter_values, pick_locale_language, sanitize_param_for_warning,
    search_type_from_str, to_limited_count,
};
use crate::throttle::RequestThrottle;
use crate::types::{
    BraveWebSearchArgs, DebugData, EndpointProbeResult, FetchSearchParams, HelpResponse,
    HelpSections, HelpTopic, KeyConfigStatus, NormalizedSearchRequest, OutputLimitSettings,
    ProbeStatus, SearchMeta, SearchResponse, SearchSection, SearchType, StatusArgs, StatusResponse,
    WarningEntry,
};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct SearchService {
    client: BraveClient,
    config: RuntimeConfig,
    cache: SearchCache<crate::types::FetchSearchResult>,
    throttle: RequestThrottle,
    server_version: String,
}

impl SearchService {
    pub fn new(config: RuntimeConfig) -> Result<Self, AppError> {
        let cache = SearchCache::new(Duration::from_secs(config.cache_ttl_secs));
        let throttle = RequestThrottle::new(config.throttle_rate_per_sec, config.throttle_burst);
        let client = BraveClient::new(config.clone())?;

        Ok(Self {
            client,
            config,
            cache,
            throttle,
            server_version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    #[must_use]
    pub fn server_version(&self) -> &str {
        &self.server_version
    }

    pub async fn execute_web_search<F>(
        &self,
        args: BraveWebSearchArgs,
        trace_id: &str,
        is_cancelled: F,
    ) -> Result<SearchResponse, AppError>
    where
        F: Fn() -> bool,
    {
        let mut normalized = self.normalize_request(args)?;
        let started = Instant::now();

        let fetch_params = FetchSearchParams {
            count: normalized.requested,
            offset: normalized.offset,
            country: normalized.country.clone(),
            search_language: normalized.search_language.clone(),
            ui_language: normalized.ui_language.clone(),
            safe_search: normalized.safe_search.clone(),
            freshness: normalized.freshness.clone(),
            result_filter_values: normalized.result_filter_values.clone(),
            units: normalized.units.clone(),
            spellcheck: normalized.spellcheck,
            extra_snippets: normalized.extra_snippets,
            text_decorations: normalized.text_decorations,
        };

        let cache_key = self.cache_key(&normalized, &fetch_params);
        let cache_bypass = normalized.disable_cache || normalized.freshness.is_some();

        let fetch_result = if !cache_bypass {
            self.cache.get(&cache_key).await
        } else {
            None
        };

        let fetch_result = if let Some(result) = fetch_result {
            result
        } else {
            if !normalized.disable_throttle {
                self.throttle
                    .acquire_cancellable(&is_cancelled)
                    .await
                    .map_err(|_| AppError::Cancelled)?;
            }

            let result = self
                .client
                .fetch_search(
                    &normalized.query,
                    normalized.search_type,
                    &fetch_params,
                    &is_cancelled,
                )
                .await?;

            if !cache_bypass {
                self.cache.insert(cache_key.clone(), result.clone()).await;
            }

            result
        };

        normalized.warnings.extend(fetch_result.warnings.clone());

        let mut sections = fetch_result
            .sections
            .clone()
            .into_iter()
            .map(|section| SearchSection {
                key: section.key,
                label: section.label,
                provider: section.provider,
                results: section.results.into_iter().map(to_result_item).collect(),
                section_limit_reached: section.section_limit_reached,
            })
            .collect::<Vec<SearchSection>>();

        let returned = sections
            .iter()
            .map(|section| section.results.len())
            .sum::<usize>();

        let has_more = fetch_result.has_more;

        let summary = build_summary(
            &fetch_result.query_echo,
            returned,
            normalized.search_type,
            normalized.offset,
            normalized.requested,
            has_more,
        );

        let mut response = SearchResponse {
            api_version: API_VERSION.to_string(),
            summary,
            sections: std::mem::take(&mut sections),
            meta: SearchMeta {
                query: fetch_result.query_echo,
                search_type: normalized.search_type,
                requested: normalized.requested,
                returned,
                offset: normalized.offset,
                has_more,
                provider: PROVIDER_NAME.to_string(),
                duration_ms: started.elapsed().as_millis(),
                warnings_count: 0,
                server_version: self.server_version.clone(),
                trace_id: trace_id.to_string(),
            },
            warnings: normalized.warnings,
            debug_data: None,
        };

        if normalized.debug {
            let request_url = normalized
                .include_request_url
                .then_some(fetch_result.request_url.clone());

            let (raw_payload, raw_payload_truncated, raw_payload_original_bytes) =
                if normalized.include_raw_payload {
                    maybe_cap_debug_raw_payload(
                        &fetch_result.raw_payload,
                        fetch_result.raw_payload_bytes,
                        self.config.raw_payload_cap_bytes,
                        &mut response.warnings,
                    )
                } else {
                    (None, false, None)
                };

            response.debug_data = Some(DebugData {
                request_url,
                raw_payload,
                raw_payload_truncated,
                raw_payload_original_bytes,
                cache_bypassed: cache_bypass,
                throttle_bypassed: normalized.disable_throttle,
            });
        }

        enforce_output_limits(&mut response, normalized.max_lines, normalized.max_bytes);
        response.meta.warnings_count = response.warnings.len();
        response.meta.duration_ms = started.elapsed().as_millis();
        Ok(response)
    }

    pub fn help(&self, topic: Option<HelpTopic>) -> HelpResponse {
        let resolved_topic = topic.unwrap_or(HelpTopic::All);

        let parameters = serde_json::json!({
            "query": "string (required)",
            "search_type": ["web", "news", "images", "videos"],
            "result_filter": ["web", "discussions", "videos", "news", "infobox"],
            "max_results": "integer 1..20 per section (default 5; web multi-section queries may return more total results)",
            "offset": "integer >= 0 (web/news/videos capped at 9; images capped at 50)",
            "country": "country code (e.g. US, DE, ALL)",
            "search_language": "language code (e.g. en, en-gb, de, pt-br)",
            "ui_language": "UI language code (e.g. en-US, de-DE)",
            "safe_search": ["off", "moderate", "strict"],
            "units": ["metric", "imperial"],
            "freshness": ["pd", "pw", "pm", "py", "1d", "1w", "1m", "1y"],
            "spellcheck": "boolean",
            "extra_snippets": "boolean (adaptive default enabled only when max_results <= 3)",
            "text_decorations": "boolean (auto: true for news, false otherwise)",
            "max_lines": "integer override with bounds",
            "max_bytes": "integer override with bounds",
            "debug": "boolean",
            "include_raw_payload": "boolean (requires debug=true)",
            "disable_cache": "boolean (requires debug=true)",
            "disable_throttle": "boolean (requires debug=true)",
            "include_request_url": "boolean (requires debug=true)"
        });

        let limits = serde_json::json!({
            "default_max_lines": self.config.output_limits.default_max_lines,
            "default_max_bytes": self.config.output_limits.default_max_bytes,
            "min_max_lines": self.config.output_limits.min_max_lines,
            "min_max_bytes": self.config.output_limits.min_max_bytes,
            "max_max_lines": self.config.output_limits.max_max_lines,
            "max_max_bytes": self.config.output_limits.max_max_bytes,
            "cache_ttl_secs": self.config.cache_ttl_secs,
            "throttle": {
                "rate_per_sec": self.config.throttle_rate_per_sec,
                "burst": self.config.throttle_burst
            },
            "retry": {
                "count": self.config.retry_count,
                "base_delay_ms": self.config.retry_base_delay_ms,
                "max_delay_ms": self.config.retry_max_delay_ms,
                "per_attempt_timeout_ms": self.config.per_attempt_timeout_ms,
            }
        });

        let errors = serde_json::json!({
            "INVALID_ARGUMENT": "Input schema/validation failure",
            "MISSING_API_KEY": "Missing BRAVE_SEARCH_API_KEY or BRAVE_API_KEY",
            "CANCELLED": "Tool request cancelled",
            "UPSTREAM_ERROR": "Brave API/network error",
            "PARSE_ERROR": "Unexpected provider payload shape",
            "INTERNAL_ERROR": "Unexpected server failure"
        });

        let examples = r#"### Examples

```json
{ "query": "TypeScript generics" }
```

```json
{ "query": "OpenAI", "search_type": "news", "max_results": 3 }
```

```json
{ "query": "Rust", "search_type": "images", "max_results": 5, "offset": 10 }
```

```json
{ "query": "site:github.com mcpkit", "result_filter": ["web", "discussions"] }
```

```json
{ "query": "Kubernetes", "country": "US", "search_language": "en", "ui_language": "en-US" }
```

```json
{ "query": "AI regulation", "freshness": "1w", "safe_search": "moderate" }
```

```json
{ "query": "websocket server", "debug": true, "include_request_url": true, "include_raw_payload": true }
```
"#;

        let (parameters_section, limits_section, errors_section) = match resolved_topic {
            HelpTopic::Params => (parameters, serde_json::json!({}), serde_json::json!({})),
            HelpTopic::Limits => (serde_json::json!({}), limits, serde_json::json!({})),
            HelpTopic::Errors => (serde_json::json!({}), serde_json::json!({}), errors),
            HelpTopic::Examples => (
                serde_json::json!({}),
                serde_json::json!({}),
                serde_json::json!({}),
            ),
            HelpTopic::All => (parameters, limits, errors),
        };

        HelpResponse {
            api_version: API_VERSION.to_string(),
            topic: resolved_topic.as_str().to_string(),
            summary: format!(
                "{tool} supports Brave web/news/images/videos search with structured JSON output.",
                tool = crate::constants::TOOL_BRAVE_WEB_SEARCH
            ),
            sections: HelpSections {
                parameters: parameters_section,
                limits: limits_section,
                errors: errors_section,
            },
            examples_markdown: examples.to_string(),
        }
    }

    pub async fn status<F>(&self, args: StatusArgs, is_cancelled: F) -> StatusResponse
    where
        F: Fn() -> bool,
    {
        let verbose = args.verbose.unwrap_or(false);
        let include_limits = args.include_limits.unwrap_or(false) || verbose;
        let probe_connectivity = args.probe_connectivity.unwrap_or(false);

        let key_config = self.client.key_config();
        let mut status = if key_config.has_key() {
            "ok".to_string()
        } else {
            "degraded".to_string()
        };

        let settings = crate::types::RuntimeSettingsStatus {
            cache_ttl_secs: self.config.cache_ttl_secs,
            throttle_rate_per_sec: self.config.throttle_rate_per_sec,
            throttle_burst: self.config.throttle_burst,
            retry_count: self.config.retry_count,
            retry_base_delay_ms: self.config.retry_base_delay_ms,
            retry_max_delay_ms: self.config.retry_max_delay_ms,
            per_attempt_timeout_ms: self.config.per_attempt_timeout_ms,
            limits: include_limits.then_some(OutputLimitSettings {
                default_max_lines: self.config.output_limits.default_max_lines,
                default_max_bytes: self.config.output_limits.default_max_bytes,
                min_max_lines: self.config.output_limits.min_max_lines,
                min_max_bytes: self.config.output_limits.min_max_bytes,
                max_max_lines: self.config.output_limits.max_max_lines,
                max_max_bytes: self.config.output_limits.max_max_bytes,
            }),
        };

        let probe = if probe_connectivity && key_config.has_key() {
            let mut endpoints = Vec::<EndpointProbeResult>::new();

            for search_type in [
                SearchType::Web,
                SearchType::News,
                SearchType::Images,
                SearchType::Videos,
            ] {
                let endpoint = self.config.endpoints.endpoint_for(search_type).to_string();
                let started = Instant::now();
                let probe_result = self.client.probe_endpoint(search_type, &is_cancelled).await;
                let duration_ms = started.elapsed().as_millis();

                match probe_result {
                    Ok(()) => endpoints.push(EndpointProbeResult {
                        search_type,
                        endpoint,
                        ok: true,
                        message: None,
                        duration_ms,
                    }),
                    Err(error) => endpoints.push(EndpointProbeResult {
                        search_type,
                        endpoint,
                        ok: false,
                        message: Some(error.to_string()),
                        duration_ms,
                    }),
                }
            }

            let degraded = endpoints.iter().any(|entry| !entry.ok);
            if degraded {
                status = "degraded".to_string();
            }

            Some(ProbeStatus {
                query: "mcp healthcheck".to_string(),
                degraded,
                endpoints,
            })
        } else {
            None
        };

        StatusResponse {
            api_version: API_VERSION.to_string(),
            status,
            server_version: self.server_version.clone(),
            provider: PROVIDER_NAME.to_string(),
            key_config: KeyConfigStatus {
                has_key: key_config.has_key(),
                source: key_config.source.clone(),
            },
            settings,
            probe,
        }
    }

    fn normalize_request(
        &self,
        args: BraveWebSearchArgs,
    ) -> Result<NormalizedSearchRequest, AppError> {
        let trimmed = args.query.trim();
        if trimmed.is_empty() {
            return Err(AppError::invalid_argument_with_details(
                "query must not be empty",
                serde_json::json!({"field": "query"}),
            ));
        }

        let mut warnings = Vec::<WarningEntry>::new();

        let mut query = trimmed.to_string();
        if query.chars().count() > self.config.max_query_length {
            let truncated: String = query.chars().take(self.config.max_query_length).collect();
            warnings.push(WarningEntry {
                code: WARNING_QUERY_TRUNCATED.to_string(),
                message: format!(
                    "Query truncated to {} characters (original length {}).",
                    self.config.max_query_length,
                    query.chars().count()
                ),
            });
            query = truncated;
        }

        let search_type = if let Some(raw_search_type) = args.search_type.as_deref() {
            if !is_valid_search_type_input(Some(raw_search_type)) {
                return Err(AppError::invalid_argument_with_details(
                    format!(
                        "search_type '{}' is invalid",
                        sanitize_param_for_warning(raw_search_type)
                    ),
                    serde_json::json!({"field": "search_type", "value": raw_search_type}),
                ));
            }
            search_type_from_str(&raw_search_type.trim().to_lowercase())
                .unwrap_or(DEFAULT_SEARCH_TYPE)
        } else {
            normalize_search_type(None)
        };

        let requested = to_limited_count(args.max_results);
        let offset = clamp_offset(args.offset, search_type);
        if offset != args.offset.unwrap_or(0) {
            warnings.push(WarningEntry {
                code: WARNING_OFFSET_CAPPED.to_string(),
                message: format!(
                    "offset capped to {offset} for {} search.",
                    search_type.as_str()
                ),
            });
        }

        let (result_filter_values, rejected_result_filters) =
            parse_result_filter_values(args.result_filter.as_deref());

        if search_type != SearchType::Web && args.result_filter.is_some() {
            warnings.push(WarningEntry {
                code: WARNING_RESULT_FILTER_IGNORED.to_string(),
                message: "result_filter is only supported for search_type='web' and was ignored."
                    .to_string(),
            });
        }

        if search_type == SearchType::Web && !rejected_result_filters.is_empty() {
            if result_filter_values.is_empty() {
                return Err(AppError::invalid_argument_with_details(
                    format!(
                        "result_filter contains no valid values: {}",
                        rejected_result_filters.join(", ")
                    ),
                    serde_json::json!({
                        "field": "result_filter",
                        "invalid_values": rejected_result_filters,
                    }),
                ));
            }

            warnings.push(WarningEntry {
                code: WARNING_INVALID_RESULT_FILTER.to_string(),
                message: format!(
                    "Unsupported result_filter values ignored: {}.",
                    rejected_result_filters.join(", ")
                ),
            });
        }

        let search_language = pick_locale_language(args.search_language.as_deref());
        if args.search_language.is_some() && search_language.is_none() {
            warnings.push(WarningEntry {
                code: WARNING_INVALID_SEARCH_LANGUAGE.to_string(),
                message: format!(
                    "search_language '{}' is invalid and was ignored.",
                    sanitize_param_for_warning(args.search_language.as_deref().unwrap_or_default())
                ),
            });
        }

        let ui_language = normalize_ui_language(args.ui_language.as_deref());
        if args.ui_language.is_some() && ui_language.is_none() {
            warnings.push(WarningEntry {
                code: WARNING_INVALID_UI_LANGUAGE.to_string(),
                message: format!(
                    "ui_language '{}' is invalid and was ignored.",
                    sanitize_param_for_warning(args.ui_language.as_deref().unwrap_or_default())
                ),
            });
        }

        let country = normalize_country(args.country.as_deref());
        if args.country.is_some() && country.is_none() {
            warnings.push(WarningEntry {
                code: WARNING_INVALID_COUNTRY.to_string(),
                message: format!(
                    "country '{}' is invalid and was ignored.",
                    sanitize_param_for_warning(args.country.as_deref().unwrap_or_default())
                ),
            });
        }

        let safe_search = normalize_safe_search(args.safe_search.as_deref());
        if args.safe_search.is_some() && safe_search.is_none() {
            warnings.push(WarningEntry {
                code: WARNING_INVALID_SAFE_SEARCH.to_string(),
                message: format!(
                    "safe_search '{}' is invalid and was ignored.",
                    sanitize_param_for_warning(args.safe_search.as_deref().unwrap_or_default())
                ),
            });
        }

        let units = normalize_units(args.units.as_deref());
        if args.units.is_some() && units.is_none() {
            warnings.push(WarningEntry {
                code: WARNING_INVALID_UNITS.to_string(),
                message: format!(
                    "units '{}' is invalid and was ignored.",
                    sanitize_param_for_warning(args.units.as_deref().unwrap_or_default())
                ),
            });
        }

        let freshness = normalize_freshness(args.freshness.as_deref());
        if args.freshness.is_some() && freshness.is_none() {
            warnings.push(WarningEntry {
                code: WARNING_INVALID_FRESHNESS.to_string(),
                message: format!(
                    "freshness '{}' is invalid and was ignored.",
                    sanitize_param_for_warning(args.freshness.as_deref().unwrap_or_default())
                ),
            });
        }

        let spellcheck = args.spellcheck.unwrap_or(true);
        let extra_snippets = args.extra_snippets.unwrap_or(requested <= 3);
        let text_decorations = args
            .text_decorations
            .unwrap_or(search_type == SearchType::News);

        let (max_lines, max_bytes) = self
            .config
            .clamp_output_limits(args.max_lines, args.max_bytes);

        let debug = args.debug.unwrap_or(false);
        let include_raw_payload = debug && args.include_raw_payload.unwrap_or(false);
        let disable_cache = debug && args.disable_cache.unwrap_or(false);
        let disable_throttle = debug && args.disable_throttle.unwrap_or(false);
        let include_request_url = debug && args.include_request_url.unwrap_or(false);

        Ok(NormalizedSearchRequest {
            query,
            search_type,
            result_filter_values: if search_type == SearchType::Web {
                result_filter_values
            } else {
                Vec::new()
            },
            requested,
            offset,
            country,
            search_language,
            ui_language,
            safe_search,
            units,
            freshness,
            spellcheck,
            extra_snippets,
            text_decorations,
            max_lines,
            max_bytes,
            debug,
            include_raw_payload,
            disable_cache,
            disable_throttle,
            include_request_url,
            warnings,
        })
    }

    fn cache_key(&self, request: &NormalizedSearchRequest, params: &FetchSearchParams) -> String {
        let material = serde_json::json!({
            "query": request.query,
            "search_type": request.search_type.as_str(),
            "count": params.count,
            "offset": params.offset,
            "country": params.country,
            "search_language": params.search_language,
            "ui_language": params.ui_language,
            "safe_search": params.safe_search,
            "freshness": params.freshness,
            "result_filter_values": params
                .result_filter_values
                .iter()
                .map(|v| v.as_str())
                .collect::<Vec<&str>>(),
            "units": params.units,
            "spellcheck": params.spellcheck,
            "extra_snippets": params.extra_snippets,
            "text_decorations": params.text_decorations,
        });

        let bytes = serde_json::to_vec(&material).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }
}
