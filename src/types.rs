use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchType {
    Web,
    News,
    Images,
    Videos,
}

impl SearchType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::News => "news",
            Self::Images => "images",
            Self::Videos => "videos",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BraveSectionName {
    Web,
    Discussions,
    Videos,
    News,
    Images,
    Infobox,
}

impl BraveSectionName {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Discussions => "discussions",
            Self::Videos => "videos",
            Self::News => "news",
            Self::Images => "images",
            Self::Infobox => "infobox",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebResultFilter {
    Web,
    Discussions,
    Videos,
    News,
    Infobox,
}

impl WebResultFilter {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Discussions => "discussions",
            Self::Videos => "videos",
            Self::News => "news",
            Self::Infobox => "infobox",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BraveWebSearchArgs {
    pub query: String,
    pub search_type: Option<String>,
    pub result_filter: Option<Vec<String>>,
    pub max_results: Option<usize>,
    pub offset: Option<usize>,
    pub country: Option<String>,
    pub search_language: Option<String>,
    pub ui_language: Option<String>,
    pub safe_search: Option<String>,
    pub units: Option<String>,
    pub freshness: Option<String>,
    pub spellcheck: Option<bool>,
    pub extra_snippets: Option<bool>,
    pub text_decorations: Option<bool>,
    pub max_lines: Option<usize>,
    pub max_bytes: Option<usize>,
    pub debug: Option<bool>,
    pub include_raw_payload: Option<bool>,
    pub disable_cache: Option<bool>,
    pub disable_throttle: Option<bool>,
    pub include_request_url: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelpArgs {
    pub topic: Option<HelpTopic>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusArgs {
    pub probe_connectivity: Option<bool>,
    pub verbose: Option<bool>,
    pub include_limits: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct NormalizedSearchRequest {
    pub query: String,
    pub search_type: SearchType,
    pub result_filter_values: Vec<WebResultFilter>,
    pub requested: usize,
    pub offset: usize,
    pub country: Option<String>,
    pub search_language: Option<String>,
    pub ui_language: Option<String>,
    pub safe_search: Option<String>,
    pub units: Option<String>,
    pub freshness: Option<String>,
    pub spellcheck: bool,
    pub extra_snippets: bool,
    pub text_decorations: bool,
    pub max_lines: usize,
    pub max_bytes: usize,
    pub debug: bool,
    pub include_raw_payload: bool,
    pub disable_cache: bool,
    pub disable_throttle: bool,
    pub include_request_url: bool,
    pub warnings: Vec<WarningEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarningEntry {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolErrorEnvelope {
    pub api_version: String,
    pub error: ToolErrorInfo,
    pub meta: ErrorMeta,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolErrorInfo {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorMeta {
    pub provider: String,
    pub server_version: String,
    pub trace_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResponse {
    pub api_version: String,
    pub summary: String,
    pub sections: Vec<SearchSection>,
    pub meta: SearchMeta,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<WarningEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_data: Option<DebugData>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchSection {
    pub key: BraveSectionName,
    pub label: String,
    pub provider: String,
    pub results: Vec<SearchResultItem>,
    pub section_limit_reached: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResultItem {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub extra_snippets: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata_lines: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_live: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchMeta {
    pub query: String,
    pub search_type: SearchType,
    pub requested: usize,
    pub returned: usize,
    pub offset: usize,
    pub has_more: bool,
    pub provider: String,
    pub duration_ms: u128,
    pub warnings_count: usize,
    pub server_version: String,
    pub trace_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HelpTopic {
    Params,
    Examples,
    Limits,
    Errors,
    All,
}

impl HelpTopic {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Params => "params",
            Self::Examples => "examples",
            Self::Limits => "limits",
            Self::Errors => "errors",
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_payload: Option<serde_json::Value>,
    pub raw_payload_truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_payload_original_bytes: Option<usize>,
    pub cache_bypassed: bool,
    pub throttle_bypassed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct HelpResponse {
    pub api_version: String,
    pub topic: String,
    pub summary: String,
    pub sections: HelpSections,
    pub examples_markdown: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HelpSections {
    pub parameters: serde_json::Value,
    pub limits: serde_json::Value,
    pub errors: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusResponse {
    pub api_version: String,
    pub status: String,
    pub server_version: String,
    pub provider: String,
    pub key_config: KeyConfigStatus,
    pub settings: RuntimeSettingsStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probe: Option<ProbeStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KeyConfigStatus {
    pub has_key: bool,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeSettingsStatus {
    pub cache_ttl_secs: u64,
    pub throttle_rate_per_sec: u32,
    pub throttle_burst: u32,
    pub retry_count: usize,
    pub retry_base_delay_ms: u64,
    pub retry_max_delay_ms: u64,
    pub per_attempt_timeout_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limits: Option<OutputLimitSettings>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputLimitSettings {
    pub default_max_lines: usize,
    pub default_max_bytes: usize,
    pub min_max_lines: usize,
    pub min_max_bytes: usize,
    pub max_max_lines: usize,
    pub max_max_bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProbeStatus {
    pub query: String,
    pub degraded: bool,
    pub endpoints: Vec<EndpointProbeResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EndpointProbeResult {
    pub search_type: SearchType,
    pub endpoint: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub duration_ms: u128,
}

#[derive(Debug, Clone)]
pub struct NormalizedResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub extra_snippets: Vec<String>,
    pub source: Option<String>,
    pub age: Option<String>,
    pub published: Option<String>,
    pub item_type: Option<String>,
    pub subtype: Option<String>,
    pub duration: Option<String>,
    pub creator: Option<String>,
    pub location: Option<String>,
    pub is_live: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedSection {
    pub key: BraveSectionName,
    pub label: String,
    pub provider: String,
    pub results: Vec<NormalizedResult>,
    pub section_limit_reached: bool,
}

#[derive(Debug, Clone)]
pub struct ParseSectionsResult {
    pub sections: Vec<ParsedSection>,
    pub has_more: bool,
    pub warnings: Vec<WarningEntry>,
}

#[derive(Debug, Clone)]
pub struct FetchSearchParams {
    pub count: usize,
    pub offset: usize,
    pub country: Option<String>,
    pub search_language: Option<String>,
    pub ui_language: Option<String>,
    pub safe_search: Option<String>,
    pub freshness: Option<String>,
    pub result_filter_values: Vec<WebResultFilter>,
    pub units: Option<String>,
    pub spellcheck: bool,
    pub extra_snippets: bool,
    pub text_decorations: bool,
}

#[derive(Debug, Clone)]
pub struct FetchSearchResult {
    pub sections: Vec<ParsedSection>,
    pub has_more: bool,
    pub warnings: Vec<WarningEntry>,
    pub query_echo: String,
    pub request_url: String,
    pub raw_payload: serde_json::Value,
    pub raw_payload_bytes: usize,
}
