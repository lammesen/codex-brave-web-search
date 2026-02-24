use crate::types::{BraveSectionName, SearchType, WebResultFilter};

pub const API_VERSION: &str = "v1";
pub const PROVIDER_NAME: &str = "brave";

pub const TOOL_BRAVE_WEB_SEARCH: &str = "brave_web_search";
pub const TOOL_BRAVE_WEB_SEARCH_HELP: &str = "brave_web_search_help";
pub const TOOL_BRAVE_WEB_SEARCH_STATUS: &str = "brave_web_search_status";

pub const DEFAULT_SEARCH_TYPE: SearchType = SearchType::Web;
pub const DEFAULT_RESULTS: usize = 5;
pub const MAX_RESULTS: usize = 20;
pub const MAX_EXTRA_SNIPPETS: usize = 2;
pub const MAX_QUERY_LENGTH: usize = 2_000;

pub const DEFAULT_MIN_MAX_LINES: usize = 20;
pub const DEFAULT_MIN_MAX_BYTES: usize = 4 * 1_024;
pub const DEFAULT_MAX_MAX_LINES: usize = 300;
pub const DEFAULT_MAX_MAX_BYTES: usize = 96 * 1_024;
pub const DEFAULT_MAX_LINES: usize = 120;
pub const DEFAULT_MAX_BYTES: usize = 32 * 1_024;

pub const DEFAULT_CACHE_TTL_SECS: u64 = 300;
pub const DEFAULT_THROTTLE_RATE_PER_SEC: u32 = 2;
pub const DEFAULT_THROTTLE_BURST: u32 = 4;

pub const DEFAULT_RETRY_COUNT: usize = 3;
pub const DEFAULT_RETRY_BASE_DELAY_MS: u64 = 250;
pub const DEFAULT_MAX_RETRY_DELAY_MS: u64 = 5_000;
pub const DEFAULT_PER_ATTEMPT_TIMEOUT_MS: u64 = 15_000;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2_097_152;
pub const DEFAULT_RAW_PAYLOAD_CAP_BYTES: usize = 64 * 1_024;

pub const BRAVE_ENDPOINT_WEB: &str = "https://api.search.brave.com/res/v1/web/search";
pub const BRAVE_ENDPOINT_NEWS: &str = "https://api.search.brave.com/res/v1/news/search";
pub const BRAVE_ENDPOINT_IMAGES: &str = "https://api.search.brave.com/res/v1/images/search";
pub const BRAVE_ENDPOINT_VIDEOS: &str = "https://api.search.brave.com/res/v1/videos/search";

pub const RETRYABLE_HTTP_STATUS: &[u16] = &[429, 500, 502, 503, 504];

pub const FRESHNESS_SHORTCUT_OPTIONS: &[&str] = &["pd", "pw", "pm", "py"];

pub const SEARCH_TYPES: &[SearchType] = &[
    SearchType::Web,
    SearchType::News,
    SearchType::Images,
    SearchType::Videos,
];

pub const ALLOWED_RESULT_FILTERS: &[WebResultFilter] = &[
    WebResultFilter::Web,
    WebResultFilter::Discussions,
    WebResultFilter::Videos,
    WebResultFilter::News,
    WebResultFilter::Infobox,
];

pub const SAFE_SEARCH_OPTIONS: &[&str] = &["off", "moderate", "strict"];
pub const UNIT_OPTIONS: &[&str] = &["metric", "imperial"];

pub const SEARCH_LANGUAGE_OPTIONS: &[&str] = &[
    "ar", "eu", "bn", "bg", "ca", "zh-hans", "zh-hant", "hr", "cs", "da", "en", "en-gb", "et",
    "fi", "fr", "gl", "de", "el", "gu", "he", "hi", "hu", "is", "it", "jp", "kn", "ko", "lv", "lt",
    "ms", "ml", "mr", "nb", "pl", "pt-br", "pt-pt", "pa", "ro", "ru", "sr", "sk", "sl", "es", "sv",
    "ta", "te", "th", "tr", "uk", "vi",
];

pub const UI_LANGUAGE_OPTIONS: &[&str] = &[
    "es-AR", "en-AU", "de-AT", "nl-BE", "fr-BE", "pt-BR", "en-CA", "fr-CA", "es-CL", "da-DK",
    "fi-FI", "fr-FR", "de-DE", "el-GR", "zh-HK", "en-IN", "en-ID", "it-IT", "ja-JP", "ko-KR",
    "en-MY", "es-MX", "nl-NL", "en-NZ", "no-NO", "zh-CN", "pl-PL", "en-PH", "ru-RU", "en-ZA",
    "es-ES", "sv-SE", "fr-CH", "de-CH", "zh-TW", "tr-TR", "en-GB", "en-US", "es-US",
];

pub const COUNTRY_OPTIONS: &[&str] = &[
    "AR", "AU", "AT", "BE", "BR", "CA", "CL", "DK", "FI", "FR", "DE", "GR", "HK", "IN", "ID", "IT",
    "JP", "KR", "MY", "MX", "NL", "NZ", "NO", "CN", "PL", "PT", "PH", "RU", "SA", "ZA", "ES", "SE",
    "CH", "TW", "TR", "GB", "US", "ALL",
];

pub const MAX_OFFSET_WEB_NEWS_VIDEOS: usize = 9;
pub const MAX_OFFSET_IMAGES: usize = 50;

pub const SECTION_WEB_RESULTS: &str = "Web results";
pub const SECTION_DISCUSSIONS: &str = "Discussions";
pub const SECTION_VIDEOS: &str = "Videos";
pub const SECTION_NEWS: &str = "News";
pub const SECTION_IMAGES: &str = "Images";
pub const SECTION_INFOBOX: &str = "Infobox";

pub fn section_specs_for(search_type: SearchType) -> &'static [(&'static str, BraveSectionName)] {
    match search_type {
        SearchType::Web => &[
            (SECTION_WEB_RESULTS, BraveSectionName::Web),
            (SECTION_DISCUSSIONS, BraveSectionName::Discussions),
            (SECTION_VIDEOS, BraveSectionName::Videos),
            (SECTION_NEWS, BraveSectionName::News),
            (SECTION_INFOBOX, BraveSectionName::Infobox),
        ],
        SearchType::News => &[(SECTION_NEWS, BraveSectionName::News)],
        SearchType::Images => &[(SECTION_IMAGES, BraveSectionName::Images)],
        SearchType::Videos => &[(SECTION_VIDEOS, BraveSectionName::Videos)],
    }
}

pub const WARNING_QUERY_TRUNCATED: &str = "QUERY_TRUNCATED";
pub const WARNING_INVALID_SEARCH_TYPE: &str = "INVALID_SEARCH_TYPE";
pub const WARNING_INVALID_RESULT_FILTER: &str = "INVALID_RESULT_FILTER";
pub const WARNING_RESULT_FILTER_IGNORED: &str = "RESULT_FILTER_IGNORED";
pub const WARNING_INVALID_SEARCH_LANGUAGE: &str = "INVALID_SEARCH_LANGUAGE";
pub const WARNING_INVALID_UI_LANGUAGE: &str = "INVALID_UI_LANGUAGE";
pub const WARNING_INVALID_COUNTRY: &str = "INVALID_COUNTRY";
pub const WARNING_INVALID_SAFE_SEARCH: &str = "INVALID_SAFE_SEARCH";
pub const WARNING_INVALID_UNITS: &str = "INVALID_UNITS";
pub const WARNING_INVALID_FRESHNESS: &str = "INVALID_FRESHNESS";
pub const WARNING_OFFSET_CAPPED: &str = "OFFSET_CAPPED";
pub const WARNING_DEDUPLICATED: &str = "DEDUPLICATED";
pub const WARNING_NO_RECOGNIZED_SECTIONS: &str = "NO_RECOGNIZED_SECTIONS";
pub const WARNING_OUTPUT_TRUNCATED: &str = "OUTPUT_TRUNCATED";
pub const WARNING_RAW_PAYLOAD_TRUNCATED: &str = "RAW_PAYLOAD_TRUNCATED";

pub const ERROR_INVALID_ARGUMENT: &str = "INVALID_ARGUMENT";
pub const ERROR_MISSING_API_KEY: &str = "MISSING_API_KEY";
pub const ERROR_CANCELLED: &str = "CANCELLED";
pub const ERROR_UPSTREAM: &str = "UPSTREAM_ERROR";
pub const ERROR_PARSE: &str = "PARSE_ERROR";
pub const ERROR_INTERNAL: &str = "INTERNAL_ERROR";

pub const ENV_BRAVE_SEARCH_API_KEY: &str = "BRAVE_SEARCH_API_KEY";
pub const ENV_BRAVE_API_KEY: &str = "BRAVE_API_KEY";

pub const ENV_DEFAULT_MAX_LINES: &str = "CODEX_BRAVE_DEFAULT_MAX_LINES";
pub const ENV_DEFAULT_MAX_BYTES: &str = "CODEX_BRAVE_DEFAULT_MAX_BYTES";
pub const ENV_MIN_MAX_LINES: &str = "CODEX_BRAVE_MIN_MAX_LINES";
pub const ENV_MIN_MAX_BYTES: &str = "CODEX_BRAVE_MIN_MAX_BYTES";
pub const ENV_MAX_MAX_LINES: &str = "CODEX_BRAVE_MAX_MAX_LINES";
pub const ENV_MAX_MAX_BYTES: &str = "CODEX_BRAVE_MAX_MAX_BYTES";
pub const ENV_CACHE_TTL_SECS: &str = "CODEX_BRAVE_CACHE_TTL_SECS";
pub const ENV_THROTTLE_RATE: &str = "CODEX_BRAVE_THROTTLE_RATE_PER_SEC";
pub const ENV_THROTTLE_BURST: &str = "CODEX_BRAVE_THROTTLE_BURST";
pub const ENV_RETRY_COUNT: &str = "CODEX_BRAVE_RETRY_COUNT";
pub const ENV_RETRY_BASE_DELAY_MS: &str = "CODEX_BRAVE_RETRY_BASE_DELAY_MS";
pub const ENV_RETRY_MAX_DELAY_MS: &str = "CODEX_BRAVE_RETRY_MAX_DELAY_MS";
pub const ENV_PER_ATTEMPT_TIMEOUT_MS: &str = "CODEX_BRAVE_PER_ATTEMPT_TIMEOUT_MS";
pub const ENV_MAX_RESPONSE_BYTES: &str = "CODEX_BRAVE_MAX_RESPONSE_BYTES";
pub const ENV_RAW_PAYLOAD_CAP_BYTES: &str = "CODEX_BRAVE_RAW_PAYLOAD_CAP_BYTES";
pub const ENV_MAX_QUERY_LENGTH: &str = "CODEX_BRAVE_MAX_QUERY_LENGTH";
pub const ENV_LOG: &str = "CODEX_BRAVE_LOG";
pub const ENV_ENDPOINT_WEB: &str = "CODEX_BRAVE_ENDPOINT_WEB";
pub const ENV_ENDPOINT_NEWS: &str = "CODEX_BRAVE_ENDPOINT_NEWS";
pub const ENV_ENDPOINT_IMAGES: &str = "CODEX_BRAVE_ENDPOINT_IMAGES";
pub const ENV_ENDPOINT_VIDEOS: &str = "CODEX_BRAVE_ENDPOINT_VIDEOS";
