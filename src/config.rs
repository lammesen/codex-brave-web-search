use crate::constants::{
    BRAVE_ENDPOINT_IMAGES, BRAVE_ENDPOINT_NEWS, BRAVE_ENDPOINT_VIDEOS, BRAVE_ENDPOINT_WEB,
    DEFAULT_CACHE_TTL_SECS, DEFAULT_MAX_BYTES, DEFAULT_MAX_LINES, DEFAULT_MAX_MAX_BYTES,
    DEFAULT_MAX_MAX_LINES, DEFAULT_MAX_RESPONSE_BYTES, DEFAULT_MAX_RETRY_DELAY_MS,
    DEFAULT_MIN_MAX_BYTES, DEFAULT_MIN_MAX_LINES, DEFAULT_PER_ATTEMPT_TIMEOUT_MS,
    DEFAULT_RAW_PAYLOAD_CAP_BYTES, DEFAULT_RETRY_BASE_DELAY_MS, DEFAULT_RETRY_COUNT,
    DEFAULT_THROTTLE_BURST, DEFAULT_THROTTLE_RATE_PER_SEC, ENV_BRAVE_API_KEY,
    ENV_BRAVE_SEARCH_API_KEY, ENV_CACHE_TTL_SECS, ENV_DEFAULT_MAX_BYTES, ENV_DEFAULT_MAX_LINES,
    ENV_ENDPOINT_IMAGES, ENV_ENDPOINT_NEWS, ENV_ENDPOINT_VIDEOS, ENV_ENDPOINT_WEB, ENV_LOG,
    ENV_MAX_MAX_BYTES, ENV_MAX_MAX_LINES, ENV_MAX_QUERY_LENGTH, ENV_MAX_RESPONSE_BYTES,
    ENV_MIN_MAX_BYTES, ENV_MIN_MAX_LINES, ENV_PER_ATTEMPT_TIMEOUT_MS, ENV_RAW_PAYLOAD_CAP_BYTES,
    ENV_RETRY_BASE_DELAY_MS, ENV_RETRY_COUNT, ENV_RETRY_MAX_DELAY_MS, ENV_THROTTLE_BURST,
    ENV_THROTTLE_RATE, MAX_QUERY_LENGTH,
};
use crate::types::OutputLimitSettings;

#[derive(Debug, Clone)]
pub struct BraveEndpoints {
    pub web: String,
    pub news: String,
    pub images: String,
    pub videos: String,
}

impl BraveEndpoints {
    #[must_use]
    pub fn endpoint_for(&self, search_type: crate::types::SearchType) -> &str {
        match search_type {
            crate::types::SearchType::Web => &self.web,
            crate::types::SearchType::News => &self.news,
            crate::types::SearchType::Images => &self.images,
            crate::types::SearchType::Videos => &self.videos,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub output_limits: OutputLimitSettings,
    pub cache_ttl_secs: u64,
    pub throttle_rate_per_sec: u32,
    pub throttle_burst: u32,
    pub retry_count: usize,
    pub retry_base_delay_ms: u64,
    pub retry_max_delay_ms: u64,
    pub per_attempt_timeout_ms: u64,
    pub max_response_bytes: usize,
    pub raw_payload_cap_bytes: usize,
    pub max_query_length: usize,
    pub endpoints: BraveEndpoints,
    pub log_filter: String,
}

#[derive(Debug, Clone)]
pub struct ApiKeyConfig {
    pub key: Option<String>,
    pub source: Option<String>,
}

impl ApiKeyConfig {
    #[must_use]
    pub fn from_env() -> Self {
        if let Ok(value) = std::env::var(ENV_BRAVE_SEARCH_API_KEY) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Self {
                    key: Some(trimmed.to_string()),
                    source: Some(ENV_BRAVE_SEARCH_API_KEY.to_string()),
                };
            }
        }
        if let Ok(value) = std::env::var(ENV_BRAVE_API_KEY) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Self {
                    key: Some(trimmed.to_string()),
                    source: Some(ENV_BRAVE_API_KEY.to_string()),
                };
            }
        }
        Self {
            key: None,
            source: None,
        }
    }

    #[must_use]
    pub fn has_key(&self) -> bool {
        self.key.is_some()
    }
}

impl RuntimeConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let min_max_lines = get_env_usize(ENV_MIN_MAX_LINES).unwrap_or(DEFAULT_MIN_MAX_LINES);
        let min_max_bytes = get_env_usize(ENV_MIN_MAX_BYTES).unwrap_or(DEFAULT_MIN_MAX_BYTES);
        let max_max_lines = get_env_usize(ENV_MAX_MAX_LINES).unwrap_or(DEFAULT_MAX_MAX_LINES);
        let max_max_bytes = get_env_usize(ENV_MAX_MAX_BYTES).unwrap_or(DEFAULT_MAX_MAX_BYTES);

        let clamped_min_lines = min_max_lines.min(max_max_lines);
        let clamped_min_bytes = min_max_bytes.min(max_max_bytes);

        let default_max_lines = clamp_usize(
            get_env_usize(ENV_DEFAULT_MAX_LINES).unwrap_or(DEFAULT_MAX_LINES),
            clamped_min_lines,
            max_max_lines,
        );
        let default_max_bytes = clamp_usize(
            get_env_usize(ENV_DEFAULT_MAX_BYTES).unwrap_or(DEFAULT_MAX_BYTES),
            clamped_min_bytes,
            max_max_bytes,
        );

        let cache_ttl_secs = get_env_u64(ENV_CACHE_TTL_SECS).unwrap_or(DEFAULT_CACHE_TTL_SECS);
        let throttle_rate_per_sec = get_env_u32(ENV_THROTTLE_RATE)
            .unwrap_or(DEFAULT_THROTTLE_RATE_PER_SEC)
            .max(1);
        let throttle_burst = get_env_u32(ENV_THROTTLE_BURST)
            .unwrap_or(DEFAULT_THROTTLE_BURST)
            .max(throttle_rate_per_sec)
            .max(1);

        let retry_count = get_env_usize(ENV_RETRY_COUNT)
            .unwrap_or(DEFAULT_RETRY_COUNT)
            .clamp(0, 10);
        let retry_base_delay_ms = get_env_u64(ENV_RETRY_BASE_DELAY_MS)
            .unwrap_or(DEFAULT_RETRY_BASE_DELAY_MS)
            .max(1);
        let retry_max_delay_ms = get_env_u64(ENV_RETRY_MAX_DELAY_MS)
            .unwrap_or(DEFAULT_MAX_RETRY_DELAY_MS)
            .max(retry_base_delay_ms);
        let per_attempt_timeout_ms = get_env_u64(ENV_PER_ATTEMPT_TIMEOUT_MS)
            .unwrap_or(DEFAULT_PER_ATTEMPT_TIMEOUT_MS)
            .max(100);

        let max_response_bytes = get_env_usize(ENV_MAX_RESPONSE_BYTES)
            .unwrap_or(DEFAULT_MAX_RESPONSE_BYTES)
            .max(1_024);
        let raw_payload_cap_bytes = get_env_usize(ENV_RAW_PAYLOAD_CAP_BYTES)
            .unwrap_or(DEFAULT_RAW_PAYLOAD_CAP_BYTES)
            .max(1_024);
        let max_query_length = get_env_usize(ENV_MAX_QUERY_LENGTH)
            .unwrap_or(MAX_QUERY_LENGTH)
            .clamp(256, 10_000);

        let endpoints = BraveEndpoints {
            web: std::env::var(ENV_ENDPOINT_WEB).unwrap_or_else(|_| BRAVE_ENDPOINT_WEB.to_string()),
            news: std::env::var(ENV_ENDPOINT_NEWS)
                .unwrap_or_else(|_| BRAVE_ENDPOINT_NEWS.to_string()),
            images: std::env::var(ENV_ENDPOINT_IMAGES)
                .unwrap_or_else(|_| BRAVE_ENDPOINT_IMAGES.to_string()),
            videos: std::env::var(ENV_ENDPOINT_VIDEOS)
                .unwrap_or_else(|_| BRAVE_ENDPOINT_VIDEOS.to_string()),
        };

        let log_filter = std::env::var(ENV_LOG)
            .unwrap_or_else(|_| "warn,codex_brave_web_search=warn".to_string());

        Self {
            output_limits: OutputLimitSettings {
                default_max_lines,
                default_max_bytes,
                min_max_lines: clamped_min_lines,
                min_max_bytes: clamped_min_bytes,
                max_max_lines,
                max_max_bytes,
            },
            cache_ttl_secs,
            throttle_rate_per_sec,
            throttle_burst,
            retry_count,
            retry_base_delay_ms,
            retry_max_delay_ms,
            per_attempt_timeout_ms,
            max_response_bytes,
            raw_payload_cap_bytes,
            max_query_length,
            endpoints,
            log_filter,
        }
    }

    #[must_use]
    pub fn clamp_output_limits(
        &self,
        max_lines: Option<usize>,
        max_bytes: Option<usize>,
    ) -> (usize, usize) {
        let lines = clamp_usize(
            max_lines.unwrap_or(self.output_limits.default_max_lines),
            self.output_limits.min_max_lines,
            self.output_limits.max_max_lines,
        );
        let bytes = clamp_usize(
            max_bytes.unwrap_or(self.output_limits.default_max_bytes),
            self.output_limits.min_max_bytes,
            self.output_limits.max_max_bytes,
        );
        (lines, bytes)
    }
}

fn clamp_usize(value: usize, min: usize, max: usize) -> usize {
    value.clamp(min, max)
}

fn get_env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
}

fn get_env_u64(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|v| v.parse::<u64>().ok())
}

fn get_env_u32(name: &str) -> Option<u32> {
    std::env::var(name).ok().and_then(|v| v.parse::<u32>().ok())
}
