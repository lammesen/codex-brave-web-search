use crate::constants::{
    MAX_EXTRA_SNIPPETS, WARNING_DEDUPLICATED, WARNING_NO_RECOGNIZED_SECTIONS, section_specs_for,
};
use crate::normalization::{clean_text, normalize_url_for_dedup};
use crate::types::{
    BraveSectionName, NormalizedResult, ParseSectionsResult, ParsedSection, SearchType,
    WarningEntry, WebResultFilter,
};
use serde_json::{Map, Value};
use std::collections::HashSet;

const MAX_ERROR_DETAIL_LENGTH: usize = 500;

fn truncate_error_detail(text: &str) -> String {
    if text.chars().count() <= MAX_ERROR_DETAIL_LENGTH {
        return text.to_string();
    }
    let mut truncated: String = text.chars().take(MAX_ERROR_DETAIL_LENGTH).collect();
    truncated.push('\u{2026}');
    truncated
}

#[must_use]
pub fn parse_brave_error_message(payload_text: &str, fallback: &str) -> String {
    let Ok(payload) = serde_json::from_str::<Value>(payload_text) else {
        return fallback.to_string();
    };

    if let Some(detail) = payload
        .get("error")
        .and_then(|error| error.get("detail"))
        .and_then(Value::as_str)
    {
        let mut message = truncate_error_detail(detail);
        if let Some(errors) = payload
            .get("error")
            .and_then(|error| error.get("meta"))
            .and_then(|meta| meta.get("errors"))
            .and_then(Value::as_array)
        {
            let expected = errors
                .iter()
                .filter_map(|entry| {
                    entry
                        .get("msg")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .or_else(|| {
                            entry
                                .get("ctx")
                                .and_then(|ctx| ctx.get("expected"))
                                .map(|expected| expected.to_string())
                        })
                })
                .collect::<Vec<String>>()
                .join("; ");
            if !expected.is_empty() {
                message.push_str(" (");
                message.push_str(&truncate_error_detail(&expected));
                message.push(')');
            }
        }
        return message;
    }

    if let Some(kind) = payload.get("type").and_then(Value::as_str) {
        return truncate_error_detail(kind);
    }

    fallback.to_string()
}

fn to_objects(value: Option<&Value>) -> Vec<&Map<String, Value>> {
    value
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |array| {
            array.iter().filter_map(Value::as_object).collect()
        })
}

fn collect_raw_results(payload: &Value, section: BraveSectionName) -> Vec<&Map<String, Value>> {
    match section {
        BraveSectionName::Web => to_objects(payload.get("web").and_then(|v| v.get("results"))),
        BraveSectionName::Discussions => {
            to_objects(payload.get("discussions").and_then(|v| v.get("results")))
        }
        BraveSectionName::Infobox => {
            to_objects(payload.get("infobox").and_then(|v| v.get("results")))
        }
        BraveSectionName::Videos => {
            let nested = to_objects(payload.get("videos").and_then(|v| v.get("results")));
            if !nested.is_empty() {
                return nested;
            }
            if payload.get("type").and_then(Value::as_str) == Some("videos") {
                return to_objects(payload.get("results"));
            }
            Vec::new()
        }
        BraveSectionName::News => {
            let nested = to_objects(payload.get("news").and_then(|v| v.get("results")));
            if !nested.is_empty() {
                return nested;
            }
            if payload.get("type").and_then(Value::as_str) == Some("news") {
                return to_objects(payload.get("results"));
            }
            Vec::new()
        }
        BraveSectionName::Images => {
            let nested = to_objects(payload.get("images").and_then(|v| v.get("results")));
            if !nested.is_empty() {
                return nested;
            }
            if payload.get("type").and_then(Value::as_str) == Some("images") {
                return to_objects(payload.get("results"));
            }
            Vec::new()
        }
    }
}

fn to_clean_string(value: Option<&Value>) -> Option<String> {
    value.and_then(|v| match v {
        Value::String(text) => {
            let cleaned = clean_text(text, false);
            (!cleaned.is_empty()).then_some(cleaned)
        }
        Value::Number(number) => {
            let cleaned = clean_text(&number.to_string(), false);
            (!cleaned.is_empty()).then_some(cleaned)
        }
        _ => None,
    })
}

fn normalize_result(
    item: &Map<String, Value>,
    source: BraveSectionName,
    preserve_decorations: bool,
) -> Option<NormalizedResult> {
    let title = clean_text(
        item.get("title")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        preserve_decorations,
    );
    let url = item
        .get("url")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string();

    if title.is_empty() || url.is_empty() {
        return None;
    }

    let primary_snippet = item
        .get("description")
        .and_then(Value::as_str)
        .or_else(|| item.get("snippet").and_then(Value::as_str))
        .unwrap_or_default();

    let mut extra_snippets = Vec::new();
    if let Some(snippets) = item.get("extra_snippets").and_then(Value::as_array) {
        for snippet in snippets.iter().take(MAX_EXTRA_SNIPPETS) {
            if let Some(text) = snippet.as_str() {
                let cleaned = clean_text(text, preserve_decorations);
                if !cleaned.is_empty() {
                    extra_snippets.push(cleaned);
                }
            }
        }
    }

    let snippet = clean_text(primary_snippet, preserve_decorations);

    let source_name = item
        .get("profile")
        .and_then(Value::as_object)
        .and_then(|profile| {
            to_clean_string(profile.get("name"))
                .or_else(|| to_clean_string(profile.get("long_name")))
        })
        .or_else(|| to_clean_string(item.get("source")))
        .or_else(|| to_clean_string(item.get("source_name")));

    let age = to_clean_string(item.get("age"));
    let published = to_clean_string(item.get("page_age"));
    let item_type = to_clean_string(item.get("type")).filter(|value| value != "search_result");
    let subtype = to_clean_string(item.get("subtype"));

    let (duration, creator) = if source == BraveSectionName::Videos {
        let video_obj = item.get("video").and_then(Value::as_object);
        (
            video_obj.and_then(|video| to_clean_string(video.get("duration"))),
            video_obj.and_then(|video| to_clean_string(video.get("creator"))),
        )
    } else {
        (None, None)
    };

    let location = to_clean_string(item.get("location"));
    let is_live = item
        .get("is_live")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Some(NormalizedResult {
        title,
        url,
        snippet,
        extra_snippets,
        source: source_name,
        age,
        published,
        item_type,
        subtype,
        duration,
        creator,
        location,
        is_live,
    })
}

fn parse_query_original(payload: &Value) -> Option<String> {
    payload
        .get("query")
        .and_then(Value::as_object)
        .and_then(|query| query.get("original"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn parse_more_results_available(payload: &Value) -> bool {
    payload
        .get("query")
        .and_then(Value::as_object)
        .and_then(|query| query.get("more_results_available"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

#[must_use]
pub fn parse_sections(
    payload: &Value,
    search_type: SearchType,
    result_filter_values: &[WebResultFilter],
    requested: usize,
    preserve_decorations: bool,
) -> ParseSectionsResult {
    let normalized_filters = if result_filter_values.is_empty() {
        vec![WebResultFilter::Web]
    } else {
        result_filter_values.to_vec()
    };

    let configured = section_specs_for(search_type);
    let allowed_sections: Vec<BraveSectionName> = if search_type == SearchType::Web {
        normalized_filters
            .iter()
            .map(|filter| match filter {
                WebResultFilter::Web => BraveSectionName::Web,
                WebResultFilter::Discussions => BraveSectionName::Discussions,
                WebResultFilter::Videos => BraveSectionName::Videos,
                WebResultFilter::News => BraveSectionName::News,
                WebResultFilter::Infobox => BraveSectionName::Infobox,
            })
            .collect()
    } else {
        vec![configured[0].1]
    };

    let mut warnings = Vec::<WarningEntry>::new();
    let mut sections = Vec::<ParsedSection>::new();
    let mut seen_url_keys = HashSet::<String>::new();
    let mut duplicate_count = 0usize;

    for section_name in allowed_sections {
        let Some(section_spec) = configured
            .iter()
            .find(|(_, configured_name)| *configured_name == section_name)
        else {
            continue;
        };

        let raw = collect_raw_results(payload, section_name);
        let parsed: Vec<NormalizedResult> = raw
            .into_iter()
            .filter_map(|entry| normalize_result(entry, section_name, preserve_decorations))
            .collect();

        let mut unique = Vec::<NormalizedResult>::new();
        for result in parsed {
            let dedup_key = normalize_url_for_dedup(&result.url);
            if seen_url_keys.contains(&dedup_key) {
                duplicate_count += 1;
                continue;
            }
            seen_url_keys.insert(dedup_key);
            unique.push(result);
        }

        let limited = unique
            .into_iter()
            .take(requested)
            .collect::<Vec<NormalizedResult>>();
        let more_available = parse_more_results_available(payload);
        let section_limit_reached = limited.len() == requested && more_available;

        sections.push(ParsedSection {
            key: section_name,
            label: section_spec.0.to_string(),
            provider: section_name.as_str().to_string(),
            results: limited,
            section_limit_reached,
        });
    }

    if sections.is_empty() {
        warnings.push(WarningEntry {
            code: WARNING_NO_RECOGNIZED_SECTIONS.to_string(),
            message: format!(
                "No recognized result sections for search_type '{}'.",
                search_type.as_str()
            ),
        });
    }

    if duplicate_count > 0 {
        warnings.push(WarningEntry {
            code: WARNING_DEDUPLICATED.to_string(),
            message: format!(
                "Deduplicated {duplicate_count} duplicate result{} across sections by URL.",
                if duplicate_count == 1 { "" } else { "s" }
            ),
        });
    }

    let has_renderable_results = sections.iter().any(|section| !section.results.is_empty());
    let has_more = has_renderable_results
        && (parse_more_results_available(payload)
            || sections.iter().any(|section| {
                section.section_limit_reached && section.results.len() == requested
            }));

    ParseSectionsResult {
        sections,
        has_more,
        warnings,
    }
}

#[must_use]
pub fn query_echo_or_original(payload: &Value, fallback_query: &str) -> String {
    parse_query_original(payload).unwrap_or_else(|| fallback_query.to_string())
}
