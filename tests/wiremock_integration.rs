use codex_brave_web_search::config::RuntimeConfig;
use codex_brave_web_search::service::SearchService;
use codex_brave_web_search::types::BraveWebSearchArgs;
use serial_test::serial;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn mock_payload(title: &str, url: &str) -> serde_json::Value {
    serde_json::json!({
        "query": {"original": "openai", "more_results_available": false},
        "web": {
            "results": [
                {
                    "title": title,
                    "url": url,
                    "description": "desc"
                }
            ]
        }
    })
}

fn base_args() -> BraveWebSearchArgs {
    BraveWebSearchArgs {
        query: "openai".to_string(),
        search_type: Some("web".to_string()),
        result_filter: None,
        max_results: Some(5),
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

fn configure_for_mock_server(server: &MockServer) -> RuntimeConfig {
    let mut config = RuntimeConfig::from_env();
    config.endpoints.web = format!("{}/web", server.uri());
    config.endpoints.news = format!("{}/news", server.uri());
    config.endpoints.images = format!("{}/images", server.uri());
    config.endpoints.videos = format!("{}/videos", server.uri());
    config.retry_count = 2;
    config.retry_base_delay_ms = 10;
    config.retry_max_delay_ms = 50;
    config.per_attempt_timeout_ms = 150;
    config
}

#[tokio::test]
#[serial]
async fn retries_on_transient_error_then_succeeds() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/web"))
        .respond_with(
            ResponseTemplate::new(500).set_body_json(serde_json::json!({"type": "server_error"})),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/web"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(mock_payload("A", "https://example.com/a")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let service = temp_env::with_var("BRAVE_SEARCH_API_KEY", Some("test-key"), || {
        SearchService::new(configure_for_mock_server(&server)).expect("service init")
    });

    let response = service
        .execute_web_search(base_args(), "trace-retry", || false)
        .await
        .expect("search should eventually succeed");

    assert_eq!(response.meta.returned, 1);
    assert_eq!(response.sections.len(), 1);
}

#[tokio::test]
#[serial]
async fn errors_when_response_body_exceeds_size_limit() {
    let server = MockServer::start().await;

    let big_body = "x".repeat(16 * 1024);
    Mock::given(method("GET"))
        .and(path("/web"))
        .respond_with(ResponseTemplate::new(200).set_body_string(big_body))
        .expect(3)
        .mount(&server)
        .await;

    let mut config = configure_for_mock_server(&server);
    config.max_response_bytes = 128;
    config.retry_count = 2;
    let service = temp_env::with_var("BRAVE_SEARCH_API_KEY", Some("test-key"), || {
        SearchService::new(config).expect("service init")
    });

    let err = service
        .execute_web_search(base_args(), "trace-big", || false)
        .await
        .expect_err("expected oversize failure");

    assert!(
        err.to_string()
            .contains("Response body exceeded 128 byte limit")
    );
}

#[tokio::test]
#[serial]
async fn uses_correct_endpoint_for_each_search_type() {
    let server = MockServer::start().await;

    for route in ["/web", "/news", "/images", "/videos"] {
        Mock::given(method("GET"))
            .and(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "query": {"original": "openai", "more_results_available": false},
                "results": [{"title": route, "url": format!("https://example.com{}", route)}]
            })))
            .expect(1)
            .mount(&server)
            .await;
    }

    let service = temp_env::with_var("BRAVE_SEARCH_API_KEY", Some("test-key"), || {
        SearchService::new(configure_for_mock_server(&server)).expect("service init")
    });

    for search_type in ["web", "news", "images", "videos"] {
        let mut args = base_args();
        args.search_type = Some(search_type.to_string());
        let response = service
            .execute_web_search(args, &format!("trace-{search_type}"), || false)
            .await
            .expect("search should work");

        assert_eq!(response.meta.search_type.as_str(), search_type);
    }
}

#[tokio::test]
#[serial]
async fn times_out_slow_endpoint_after_retries() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/web"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_millis(300))
                .set_body_json(mock_payload("A", "https://example.com/a")),
        )
        .expect(3)
        .mount(&server)
        .await;

    let mut config = configure_for_mock_server(&server);
    config.per_attempt_timeout_ms = 100;
    config.retry_count = 2;
    let service = temp_env::with_var("BRAVE_SEARCH_API_KEY", Some("test-key"), || {
        SearchService::new(config).expect("service init")
    });

    let err = service
        .execute_web_search(base_args(), "trace-timeout", || false)
        .await
        .expect_err("search should timeout after retries");

    assert!(err.to_string().contains("timeout"));
}
