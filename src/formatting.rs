use crate::constants::WARNING_OUTPUT_TRUNCATED;
use crate::types::{NormalizedResult, SearchResponse, SearchResultItem, WarningEntry};

#[derive(Debug, Clone, Copy)]
struct TruncationContext {
    initial_lines: usize,
    initial_bytes: usize,
    max_lines: usize,
    max_bytes: usize,
    removed_results: usize,
    omitted_debug_data: bool,
    collapsed_warnings: bool,
    condensed_summary: bool,
    condensed_query: bool,
}

#[must_use]
pub fn build_summary(
    query: &str,
    total_results: usize,
    search_type: crate::types::SearchType,
    offset: usize,
    requested: usize,
    has_more: bool,
) -> String {
    let mut summary = format!(
        "Found {total_results} result{} for \"{query}\" in {} (offset {offset}, requested {requested}).",
        if total_results == 1 { "" } else { "s" },
        search_type.as_str(),
    );
    if has_more {
        summary.push_str(" More results may be available.");
    }
    summary
}

#[must_use]
pub fn to_result_item(result: NormalizedResult) -> SearchResultItem {
    let mut metadata_lines = Vec::<String>::new();

    if let Some(source) = &result.source {
        metadata_lines.push(format!("Source: {source}"));
    }
    if let Some(age) = &result.age {
        metadata_lines.push(format!("Age: {age}"));
    }
    if let Some(published) = &result.published {
        metadata_lines.push(format!("Published: {published}"));
    }
    if let Some(item_type) = &result.item_type {
        metadata_lines.push(format!("Type: {item_type}"));
    }
    if let Some(subtype) = &result.subtype {
        metadata_lines.push(format!("Subtype: {subtype}"));
    }
    if let Some(duration) = &result.duration {
        metadata_lines.push(format!("Duration: {duration}"));
    }
    if let Some(creator) = &result.creator {
        metadata_lines.push(format!("Creator: {creator}"));
    }
    if let Some(location) = &result.location {
        metadata_lines.push(format!("Location: {location}"));
    }
    if result.is_live {
        metadata_lines.push("Live".to_string());
    }

    SearchResultItem {
        title: result.title,
        url: result.url,
        snippet: result.snippet,
        extra_snippets: result.extra_snippets,
        metadata_lines,
        source: result.source,
        age: result.age,
        published: result.published,
        item_type: result.item_type,
        subtype: result.subtype,
        duration: result.duration,
        creator: result.creator,
        location: result.location,
        is_live: result.is_live.then_some(true),
    }
}

pub fn enforce_output_limits(response: &mut SearchResponse, max_lines: usize, max_bytes: usize) {
    let (initial_lines, initial_bytes) = serialized_shape(response);

    if initial_lines <= max_lines && initial_bytes <= max_bytes {
        return;
    }

    let mut removed_results = 0usize;
    while !within_limits(response, max_lines, max_bytes) {
        let mut removed_any = false;
        for section in response.sections.iter_mut().rev() {
            if section.results.pop().is_some() {
                removed_results += 1;
                removed_any = true;
                break;
            }
        }

        if !removed_any {
            break;
        }
    }

    let mut omitted_debug_data = false;
    if !within_limits(response, max_lines, max_bytes) && response.debug_data.take().is_some() {
        omitted_debug_data = true;
    }

    let mut collapsed_warnings = false;
    if !within_limits(response, max_lines, max_bytes) && !response.warnings.is_empty() {
        response.warnings.clear();
        collapsed_warnings = true;
    }

    let mut condensed_summary = false;
    if !within_limits(response, max_lines, max_bytes) {
        response.summary = "Output truncated by configured limits.".to_string();
        condensed_summary = true;
    }

    let mut condensed_query = false;
    if !within_limits(response, max_lines, max_bytes) {
        if !response.meta.query.is_empty() {
            condensed_query = true;
        }

        while !within_limits(response, max_lines, max_bytes) && !response.meta.query.is_empty() {
            let len = response.meta.query.chars().count();
            let next_len = if len > 8 {
                len / 2
            } else {
                len.saturating_sub(1)
            };
            response.meta.query = response.meta.query.chars().take(next_len).collect();
        }
    }

    if !within_limits(response, max_lines, max_bytes) && !response.sections.is_empty() {
        response.sections.clear();
    }

    if !within_limits(response, max_lines, max_bytes) && !response.summary.is_empty() {
        response.summary.clear();
    }

    response.meta.returned = response
        .sections
        .iter()
        .map(|section| section.results.len())
        .sum::<usize>();
    if removed_results > 0 {
        response.meta.has_more = true;
    }

    response
        .warnings
        .push(build_truncation_warning(TruncationContext {
            initial_lines,
            initial_bytes,
            max_lines,
            max_bytes,
            removed_results,
            omitted_debug_data,
            collapsed_warnings,
            condensed_summary,
            condensed_query,
        }));

    if !within_limits(response, max_lines, max_bytes) {
        response.warnings.pop();

        response.warnings.push(WarningEntry {
            code: WARNING_OUTPUT_TRUNCATED.to_string(),
            message: "Output truncated by configured limits.".to_string(),
        });
    }

    if !within_limits(response, max_lines, max_bytes) {
        response.warnings.clear();
    }
}

fn serialized_shape(response: &SearchResponse) -> (usize, usize) {
    let serialized = serde_json::to_string_pretty(response).unwrap_or_else(|_| "{}".to_string());
    (serialized.lines().count(), serialized.len())
}

fn within_limits(response: &SearchResponse, max_lines: usize, max_bytes: usize) -> bool {
    let (line_count, byte_count) = serialized_shape(response);
    line_count <= max_lines && byte_count <= max_bytes
}

fn build_truncation_warning(context: TruncationContext) -> WarningEntry {
    let mut notes = Vec::<&str>::new();
    if context.removed_results > 0 {
        notes.push("results");
    }
    if context.omitted_debug_data {
        notes.push("debug_data");
    }
    if context.collapsed_warnings {
        notes.push("warnings");
    }
    if context.condensed_summary {
        notes.push("summary");
    }
    if context.condensed_query {
        notes.push("meta.query");
    }

    let details = if notes.is_empty() {
        String::new()
    } else {
        format!(" Modified: {}.", notes.join(", "))
    };

    WarningEntry {
        code: WARNING_OUTPUT_TRUNCATED.to_string(),
        message: format!(
            "Output truncated by configured limits ({} -> <= {} lines, {} -> <= {} bytes, removed {} results).{details}",
            context.initial_lines,
            context.max_lines,
            context.initial_bytes,
            context.max_bytes,
            context.removed_results,
        ),
    }
}
