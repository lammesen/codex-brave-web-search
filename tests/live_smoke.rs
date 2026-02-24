use codex_brave_web_search::config::RuntimeConfig;
use codex_brave_web_search::service::SearchService;
use codex_brave_web_search::types::BraveWebSearchArgs;
use serial_test::serial;

fn require_live_key() {
    let has_primary = std::env::var("BRAVE_SEARCH_API_KEY")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());
    let has_fallback = std::env::var("BRAVE_API_KEY")
        .ok()
        .is_some_and(|v| !v.trim().is_empty());

    assert!(
        has_primary || has_fallback,
        "Live tests require BRAVE_SEARCH_API_KEY (preferred) or BRAVE_API_KEY"
    );
}

fn args_for(search_type: &str) -> BraveWebSearchArgs {
    BraveWebSearchArgs {
        query: "openai".to_string(),
        search_type: Some(search_type.to_string()),
        result_filter: None,
        max_results: Some(2),
        offset: Some(0),
        country: None,
        search_language: Some("en".to_string()),
        ui_language: Some("en-US".to_string()),
        safe_search: None,
        units: None,
        freshness: None,
        spellcheck: Some(true),
        extra_snippets: Some(false),
        text_decorations: None,
        max_lines: Some(120),
        max_bytes: Some(32 * 1024),
        debug: Some(false),
        include_raw_payload: None,
        disable_cache: None,
        disable_throttle: None,
        include_request_url: None,
    }
}

#[tokio::test]
#[serial]
async fn live_smoke_web() {
    require_live_key();
    let service = SearchService::new(RuntimeConfig::from_env()).expect("service init");
    let response = service
        .execute_web_search(args_for("web"), "live-web", || false)
        .await
        .expect("live web request should succeed");
    assert_eq!(response.meta.search_type.as_str(), "web");
}

#[tokio::test]
#[serial]
async fn live_smoke_news() {
    require_live_key();
    let service = SearchService::new(RuntimeConfig::from_env()).expect("service init");
    let response = service
        .execute_web_search(args_for("news"), "live-news", || false)
        .await
        .expect("live news request should succeed");
    assert_eq!(response.meta.search_type.as_str(), "news");
}

#[tokio::test]
#[serial]
async fn live_smoke_images() {
    require_live_key();
    let service = SearchService::new(RuntimeConfig::from_env()).expect("service init");
    let response = service
        .execute_web_search(args_for("images"), "live-images", || false)
        .await
        .expect("live images request should succeed");
    assert_eq!(response.meta.search_type.as_str(), "images");
}

#[tokio::test]
#[serial]
async fn live_smoke_videos() {
    require_live_key();
    let service = SearchService::new(RuntimeConfig::from_env()).expect("service init");
    let response = service
        .execute_web_search(args_for("videos"), "live-videos", || false)
        .await
        .expect("live videos request should succeed");
    assert_eq!(response.meta.search_type.as_str(), "videos");
}
