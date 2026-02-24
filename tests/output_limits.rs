use codex_brave_web_search::formatting::enforce_output_limits;
use codex_brave_web_search::types::{
    BraveSectionName, DebugData, SearchMeta, SearchResponse, SearchResultItem, SearchSection,
    SearchType, WarningEntry,
};

fn build_result(index: usize) -> SearchResultItem {
    SearchResultItem {
        title: format!("Result {index}"),
        url: format!("https://example.com/{index}"),
        snippet: "snippet ".repeat(30),
        extra_snippets: vec!["extra snippet ".repeat(10)],
        metadata_lines: vec!["metadata ".repeat(8)],
        source: Some("Example Source".to_string()),
        age: Some("1h".to_string()),
        published: Some("2026-01-01".to_string()),
        item_type: Some("article".to_string()),
        subtype: Some("blog".to_string()),
        duration: None,
        creator: None,
        location: None,
        is_live: None,
    }
}

fn oversized_response() -> SearchResponse {
    SearchResponse {
        api_version: "v1".to_string(),
        summary: "Very long summary ".repeat(40),
        sections: vec![SearchSection {
            key: BraveSectionName::Web,
            label: "Web results".to_string(),
            provider: "web".to_string(),
            results: vec![build_result(1), build_result(2)],
            section_limit_reached: false,
        }],
        meta: SearchMeta {
            query: "openai ".repeat(120),
            search_type: SearchType::Web,
            requested: 2,
            returned: 2,
            offset: 0,
            has_more: false,
            provider: "brave".to_string(),
            duration_ms: 12,
            warnings_count: 2,
            server_version: "0.1.0".to_string(),
            trace_id: "trace-id-1234".to_string(),
        },
        warnings: vec![
            WarningEntry {
                code: "A".to_string(),
                message: "warning ".repeat(80),
            },
            WarningEntry {
                code: "B".to_string(),
                message: "warning ".repeat(80),
            },
        ],
        debug_data: Some(DebugData {
            request_url: Some("https://example.com/search?q=openai".to_string()),
            raw_payload: Some(serde_json::json!({"payload": "x".repeat(6_000)})),
            raw_payload_truncated: false,
            raw_payload_original_bytes: Some(6_500),
            cache_bypassed: false,
            throttle_bypassed: false,
        }),
    }
}

#[test]
fn enforces_limits_when_only_debug_and_warnings_are_large() {
    let mut response = oversized_response();
    response.sections.clear();
    response.meta.returned = 0;

    enforce_output_limits(&mut response, 20, 1024);

    let serialized = serde_json::to_string_pretty(&response).expect("serialize response");
    assert!(serialized.lines().count() <= 20);
    assert!(serialized.len() <= 1024);
    assert!(response.debug_data.is_none());
    assert!(!response.meta.has_more);
    assert!(
        response.warnings.is_empty()
            || response
                .warnings
                .iter()
                .any(|warning| warning.code == "OUTPUT_TRUNCATED")
    );
}

#[test]
fn enforces_limits_by_removing_results_and_marking_has_more() {
    let mut response = oversized_response();

    enforce_output_limits(&mut response, 36, 1800);

    let serialized = serde_json::to_string_pretty(&response).expect("serialize response");
    assert!(serialized.lines().count() <= 36);
    assert!(serialized.len() <= 1800);
    assert!(response.meta.returned < 2);
    assert!(response.meta.has_more);
    assert!(
        response
            .warnings
            .iter()
            .any(|warning| warning.code == "OUTPUT_TRUNCATED")
    );
}

#[test]
fn tiny_limits_can_drop_warning_but_remain_bounded() {
    let mut response = oversized_response();

    enforce_output_limits(&mut response, 20, 640);

    let serialized = serde_json::to_string_pretty(&response).expect("serialize response");
    assert!(serialized.lines().count() <= 20);
    assert!(serialized.len() <= 640);
    assert!(
        response.warnings.is_empty()
            || response
                .warnings
                .iter()
                .any(|warning| warning.code == "OUTPUT_TRUNCATED")
    );
}
