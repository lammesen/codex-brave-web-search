use codex_brave_web_search::config::RuntimeConfig;
use codex_brave_web_search::error::AppError;
use codex_brave_web_search::service::SearchService;
use codex_brave_web_search::types::BraveWebSearchArgs;

fn minimal_args() -> BraveWebSearchArgs {
    BraveWebSearchArgs {
        query: "openai".to_string(),
        search_type: Some("web".to_string()),
        result_filter: None,
        max_results: Some(1),
        offset: Some(0),
        country: None,
        search_language: None,
        ui_language: None,
        safe_search: None,
        units: None,
        freshness: None,
        spellcheck: None,
        extra_snippets: None,
        text_decorations: None,
        max_lines: None,
        max_bytes: None,
        debug: None,
        include_raw_payload: None,
        disable_cache: None,
        disable_throttle: None,
        include_request_url: None,
    }
}

#[tokio::test]
async fn cancellation_is_respected_before_network_call() {
    let mut config = RuntimeConfig::from_env();
    config.throttle_rate_per_sec = 1;
    config.throttle_burst = 1;

    let service = SearchService::new(config).expect("service init");

    let err = service
        .execute_web_search(minimal_args(), "trace-cancelled", || true)
        .await
        .expect_err("request should be cancelled before fetch");

    assert!(matches!(err, AppError::Cancelled));
}
