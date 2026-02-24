use codex_brave_web_search::client::compute_retry_delay_ms;
use codex_brave_web_search::formatting::{build_summary, to_result_item};
use codex_brave_web_search::normalization::{
    clamp_offset, clean_text, is_valid_search_type_input, normalize_country, normalize_freshness,
    normalize_safe_search, normalize_search_type, normalize_ui_language, normalize_units,
    normalize_url_for_dedup, parse_result_filter_values, pick_locale_language,
    sanitize_param_for_warning, strip_html_tags, to_limited_count,
};
use codex_brave_web_search::parsing::{parse_brave_error_message, parse_sections};
use codex_brave_web_search::types::{
    BraveSectionName, NormalizedResult, SearchType, WebResultFilter,
};

#[test]
fn normalize_search_type_and_validator() {
    assert_eq!(normalize_search_type(Some("web")), SearchType::Web);
    assert_eq!(normalize_search_type(Some("VIDEOS")), SearchType::Videos);
    assert_eq!(normalize_search_type(Some("nope")), SearchType::Web);

    assert!(is_valid_search_type_input(Some("news")));
    assert!(!is_valid_search_type_input(Some("NoPe")));
    assert!(!is_valid_search_type_input(None));
}

#[test]
fn parse_result_filter_values_accepts_supported_tokens_and_rejects_unsupported() {
    let (accepted, rejected) = parse_result_filter_values(Some(&[
        "web".to_string(),
        "discussions".to_string(),
        "faq".to_string(),
        "web".to_string(),
        "news".to_string(),
    ]));

    assert_eq!(
        accepted,
        vec![
            WebResultFilter::Web,
            WebResultFilter::Discussions,
            WebResultFilter::News
        ]
    );
    assert_eq!(rejected, vec!["faq"]);
}

#[test]
fn locale_country_normalization() {
    assert_eq!(
        pick_locale_language(Some("en-gb")),
        Some("en-gb".to_string())
    );
    assert_eq!(pick_locale_language(Some("en_US")), Some("en".to_string()));
    assert_eq!(pick_locale_language(Some("en-ZZ")), Some("en".to_string()));
    assert_eq!(pick_locale_language(Some("ja-JP")), Some("jp".to_string()));
    assert_eq!(pick_locale_language(Some("ja")), Some("jp".to_string()));
    assert_eq!(pick_locale_language(Some("zz")), None);

    assert_eq!(
        normalize_ui_language(Some("en-us")),
        Some("en-US".to_string())
    );
    assert_eq!(normalize_ui_language(Some("zz-ZZ")), None);

    assert_eq!(normalize_country(Some("us")), Some("US".to_string()));
    assert_eq!(normalize_country(Some("zz")), None);
}

#[test]
fn freshness_normalization() {
    assert_eq!(normalize_freshness(Some("pw")), Some("pw".to_string()));
    assert_eq!(normalize_freshness(Some("1d")), Some("1d".to_string()));
    assert_eq!(normalize_freshness(Some("2w")), Some("2w".to_string()));
    assert_eq!(normalize_freshness(Some("bad-value")), None);
    assert_eq!(
        normalize_freshness(Some("9999d")),
        Some("9999d".to_string())
    );
    assert_eq!(normalize_freshness(Some("10000d")), None);
}

#[test]
fn count_and_offset_clamping() {
    assert_eq!(to_limited_count(None), 5);
    assert_eq!(to_limited_count(Some(0)), 1);
    assert_eq!(to_limited_count(Some(999)), 20);

    assert_eq!(clamp_offset(None, SearchType::Web), 0);
    assert_eq!(clamp_offset(Some(999), SearchType::Web), 9);
    assert_eq!(clamp_offset(Some(999), SearchType::Images), 50);
}

#[test]
fn html_stripping_and_comments() {
    assert_eq!(strip_html_tags("1 < 2 and 3 > 2"), "1 < 2 and 3 > 2");
    assert_eq!(strip_html_tags("<a title=\"a>b\">x</a>"), "x");
    assert_eq!(
        strip_html_tags("before<!-- comment -->after"),
        "beforeafter"
    );
    assert_eq!(strip_html_tags("before<!-- a > b -->after"), "beforeafter");
    assert_eq!(strip_html_tags("before<!-- unclosed"), "before");
}

#[test]
fn clean_text_honors_decorations_and_entities() {
    let sample = "Hello <strong>world</strong> &amp; team";
    assert_eq!(clean_text(sample, false), "Hello world & team");
    assert_eq!(
        clean_text(sample, true),
        "Hello <strong>world</strong> & team"
    );

    assert_eq!(
        clean_text("literal &lt;script&gt;safe&lt;/script&gt; text", false),
        "literal <script>safe</script> text"
    );
    assert_eq!(clean_text("pi &#x3C; agent", false), "pi < agent");
    assert_eq!(
        clean_text("unknown &bogus; entity", false),
        "unknown &bogus; entity"
    );

    assert_eq!(clean_text("&#xD800;", false), "&#xD800;");
    assert_eq!(clean_text("&#55296;", false), "&#55296;");
}

#[test]
fn clean_text_strips_ansi_and_control_sequences() {
    assert_eq!(
        clean_text("Hello \x1b[31mRED\x1b[0m world", false),
        "Hello RED world"
    );
    assert_eq!(
        clean_text(
            "click \x1b]8;;https://evil.com\x07here\x1b]8;;\x07 please",
            false
        ),
        "click here please"
    );
    assert_eq!(
        clean_text("hello\x00world\u{009F}test", false),
        "helloworldtest"
    );
}

#[test]
fn sanitize_param_for_warning_strips_control_and_limits_length() {
    assert_eq!(sanitize_param_for_warning("normal"), "normal");
    assert_eq!(
        sanitize_param_for_warning("has\x1b[31mANSI\x1b[0m"),
        "hasANSI"
    );
    assert_eq!(
        sanitize_param_for_warning("has\x00null\x1fctrl"),
        "hasnullctrl"
    );
    assert_eq!(sanitize_param_for_warning(&"a".repeat(200)).len(), 100);
}

#[test]
fn normalize_url_for_dedup_behavior() {
    let upper = normalize_url_for_dedup("HTTPS://EXAMPLE.com/Foo?Bar=1");
    let lower = normalize_url_for_dedup("https://example.com/foo?bar=1");
    assert_eq!(upper, "https://example.com/Foo?Bar=1");
    assert_eq!(lower, "https://example.com/foo?bar=1");
    assert_ne!(upper, lower);

    assert_eq!(
        normalize_url_for_dedup("https://example.com/path///"),
        "https://example.com/path"
    );
    assert_eq!(normalize_url_for_dedup(" not a url "), "not a url");
    assert_eq!(
        normalize_url_for_dedup("https://example.com/page#section"),
        normalize_url_for_dedup("https://example.com/page")
    );
}

#[test]
fn parse_sections_dedupes_and_has_more() {
    let payload = serde_json::json!({
        "query": { "more_results_available": true },
        "web": {
            "results": [
                { "title": "A", "url": "https://example.com/a", "description": "primary" },
                { "title": "B", "url": "https://example.com/b", "description": "secondary" }
            ]
        },
        "discussions": {
            "results": [
                { "title": "A dup", "url": "https://example.com/a", "description": "dup" },
                { "title": "C", "url": "https://example.com/c", "description": "unique" }
            ]
        }
    });

    let parsed = parse_sections(
        &payload,
        SearchType::Web,
        &[WebResultFilter::Web, WebResultFilter::Discussions],
        2,
        false,
    );

    assert_eq!(parsed.sections.len(), 2);
    assert_eq!(parsed.sections[0].results.len(), 2);
    assert_eq!(parsed.sections[1].results.len(), 1);
    assert!(parsed.has_more);
    assert!(
        parsed
            .warnings
            .iter()
            .any(|warning| warning.message.contains("Deduplicated"))
    );
}

#[test]
fn parse_sections_fallback_shapes_for_images_videos_news() {
    let videos_payload = serde_json::json!({
        "type": "videos",
        "results": [{ "title": "Video", "url": "https://example.com/v" }]
    });
    let parsed_videos = parse_sections(&videos_payload, SearchType::Videos, &[], 10, false);
    assert_eq!(parsed_videos.sections[0].results.len(), 1);

    let images_payload = serde_json::json!({
        "type": "images",
        "results": [{ "title": "Image", "url": "https://example.com/i" }]
    });
    let parsed_images = parse_sections(&images_payload, SearchType::Images, &[], 10, false);
    assert_eq!(parsed_images.sections[0].results.len(), 1);

    let news_payload = serde_json::json!({
        "type": "news",
        "results": [{ "title": "News", "url": "https://example.com/n" }]
    });
    let parsed_news = parse_sections(&news_payload, SearchType::News, &[], 10, false);
    assert_eq!(parsed_news.sections[0].results.len(), 1);
}

#[test]
fn parse_sections_rejects_cross_contamination_fallback() {
    let payload = serde_json::json!({
        "type": "web",
        "results": [{ "title": "Web", "url": "https://example.com/web" }]
    });

    let parsed_images = parse_sections(&payload, SearchType::Images, &[], 10, false);
    assert_eq!(parsed_images.sections[0].results.len(), 0);

    let parsed_news = parse_sections(&payload, SearchType::News, &[], 10, false);
    assert_eq!(parsed_news.sections[0].results.len(), 0);
}

#[test]
fn parse_sections_warns_when_none_selected() {
    let payload = serde_json::json!({
        "query": { "more_results_available": false },
        "web": { "results": [] }
    });
    let parsed = parse_sections(
        &payload,
        SearchType::Web,
        &[WebResultFilter::News],
        3,
        false,
    );

    // News selection with empty results still yields section with zero results
    assert_eq!(parsed.sections.len(), 1);
    assert_eq!(parsed.sections[0].key, BraveSectionName::News);
}

#[test]
fn parse_brave_error_message_extracts_detail_and_expected_hints() {
    let message = parse_brave_error_message(
        &serde_json::json!({
            "error": {
                "detail": "Invalid request",
                "meta": { "errors": [{ "msg": "bad field" }, { "ctx": { "expected": "string" } }] }
            }
        })
        .to_string(),
        "fallback",
    );

    assert!(message.contains("Invalid request"));
    assert!(message.contains("bad field"));
    assert!(message.contains("string"));
}

#[test]
fn parse_brave_error_message_handles_non_json_and_type_fallback() {
    assert_eq!(
        parse_brave_error_message("not json", "fallback"),
        "fallback"
    );
    assert_eq!(
        parse_brave_error_message(
            &serde_json::json!({"type": "rate_limited"}).to_string(),
            "fallback"
        ),
        "rate_limited"
    );
}

#[test]
fn compute_retry_delay_respects_retry_after_and_caps_with_jitter() {
    let delay = compute_retry_delay_ms(0, Some("2"), 250, 5_000);
    assert!((1_600..=2_400).contains(&delay));

    let capped = compute_retry_delay_ms(0, Some("999"), 250, 5_000);
    assert!((4_000..=5_000).contains(&capped));
}

#[test]
fn compute_retry_delay_fallback_exponential_with_jitter() {
    let d0 = compute_retry_delay_ms(0, None, 250, 5_000);
    let d1 = compute_retry_delay_ms(1, None, 250, 5_000);
    let d2 = compute_retry_delay_ms(2, None, 250, 5_000);
    assert!((200..=300).contains(&d0));
    assert!((400..=600).contains(&d1));
    assert!((800..=1200).contains(&d2));
}

#[test]
fn summary_and_result_item_mapping() {
    let summary = build_summary("TypeScript", 3, SearchType::Web, 0, 5, true);
    assert!(summary.contains("TypeScript"));
    assert!(summary.contains("More results"));

    let result_item = to_result_item(NormalizedResult {
        title: "Title".to_string(),
        url: "https://example.com".to_string(),
        snippet: "Snippet".to_string(),
        extra_snippets: vec!["Extra".to_string()],
        source: Some("Source".to_string()),
        age: Some("1h".to_string()),
        published: Some("2026-01-01".to_string()),
        item_type: Some("article".to_string()),
        subtype: Some("blog".to_string()),
        duration: Some("5:00".to_string()),
        creator: Some("Creator".to_string()),
        location: Some("US".to_string()),
        is_live: true,
    });

    assert_eq!(result_item.metadata_lines.len(), 9);
    assert_eq!(result_item.extra_snippets.len(), 1);
    assert_eq!(result_item.is_live, Some(true));
}

#[test]
fn normalize_safe_search_and_units() {
    assert_eq!(normalize_safe_search(Some("OFF")), Some("off".to_string()));
    assert_eq!(
        normalize_safe_search(Some("Moderate")),
        Some("moderate".to_string())
    );
    assert_eq!(
        normalize_safe_search(Some("STRICT")),
        Some("strict".to_string())
    );
    assert_eq!(normalize_safe_search(Some("other")), None);

    assert_eq!(normalize_units(Some("METRIC")), Some("metric".to_string()));
    assert_eq!(
        normalize_units(Some("Imperial")),
        Some("imperial".to_string())
    );
    assert_eq!(normalize_units(Some("other")), None);
}
