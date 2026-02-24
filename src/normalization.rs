use crate::constants::{
    ALLOWED_RESULT_FILTERS, COUNTRY_OPTIONS, DEFAULT_RESULTS, DEFAULT_SEARCH_TYPE,
    FRESHNESS_SHORTCUT_OPTIONS, MAX_OFFSET_IMAGES, MAX_OFFSET_WEB_NEWS_VIDEOS, MAX_RESULTS,
    SAFE_SEARCH_OPTIONS, SEARCH_LANGUAGE_OPTIONS, SEARCH_TYPES, UI_LANGUAGE_OPTIONS, UNIT_OPTIONS,
};
use crate::types::{SearchType, WebResultFilter};
use once_cell::sync::Lazy;
use regex::Regex;
use std::borrow::Cow;

static HTML_ENTITY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"&(#x[0-9a-fA-F]+|#[0-9]+|[a-zA-Z]+);").expect("valid entity regex"));
static ANSI_CSI_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new("\\x1b\\[[0-9;]*[A-Za-z]").expect("valid ansi csi regex"));
static ANSI_OSC_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new("\\x1b\\].*?(?:\\x07|\\x1b\\\\)").expect("valid ansi osc regex"));
static ANSI_OTHER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new("\\x1b[^\\[\\]]").expect("valid ansi other regex"));
static CONTROL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\x00-\x08\x0B\x0C\x0E-\x1F\x7F-\x9F]").expect("valid control regex")
});
static WHITESPACE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s+").expect("valid whitespace regex"));

fn named_entity(entity: &str) -> Option<&'static str> {
    match entity {
        "lt" => Some("<"),
        "gt" => Some(">"),
        "amp" => Some("&"),
        "quot" => Some("\""),
        "apos" => Some("'"),
        "nbsp" => Some(" "),
        "mdash" => Some("\u{2014}"),
        "ndash" => Some("\u{2013}"),
        "hellip" => Some("\u{2026}"),
        "lsquo" => Some("\u{2018}"),
        "rsquo" => Some("\u{2019}"),
        "ldquo" => Some("\u{201C}"),
        "rdquo" => Some("\u{201D}"),
        "middot" => Some("\u{00B7}"),
        "copy" => Some("\u{00A9}"),
        "reg" => Some("\u{00AE}"),
        "trade" => Some("\u{2122}"),
        "euro" => Some("\u{20AC}"),
        _ => None,
    }
}

fn is_valid_codepoint(code_point: u32) -> bool {
    (1..=0x10FFFF).contains(&code_point) && !(0xD800..=0xDFFF).contains(&code_point)
}

fn decode_html_entity(entity: &str) -> Cow<'_, str> {
    if let Some(stripped) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        if let Ok(code_point) = u32::from_str_radix(stripped, 16)
            && is_valid_codepoint(code_point)
            && let Some(ch) = char::from_u32(code_point)
        {
            return Cow::Owned(ch.to_string());
        }
        return Cow::Owned(format!("&{entity};"));
    }

    if let Some(stripped) = entity.strip_prefix('#') {
        if let Ok(code_point) = stripped.parse::<u32>()
            && is_valid_codepoint(code_point)
            && let Some(ch) = char::from_u32(code_point)
        {
            return Cow::Owned(ch.to_string());
        }
        return Cow::Owned(format!("&{entity};"));
    }

    if let Some(named) = named_entity(entity) {
        return Cow::Borrowed(named);
    }

    Cow::Owned(format!("&{entity};"))
}

#[must_use]
pub fn strip_html_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '<' {
            if i + 3 < chars.len()
                && chars[i + 1] == '!'
                && chars[i + 2] == '-'
                && chars[i + 3] == '-'
            {
                let mut j = i + 4;
                let mut found = false;
                while j + 2 < chars.len() {
                    if chars[j] == '-' && chars[j + 1] == '-' && chars[j + 2] == '>' {
                        i = j + 3;
                        found = true;
                        break;
                    }
                    j += 1;
                }
                if found {
                    continue;
                }
                break;
            }

            let next = chars.get(i + 1).copied().unwrap_or_default();
            if next.is_ascii_alphabetic() || matches!(next, '!' | '/' | '?') {
                i += 2;
                let mut quote_char: Option<char> = None;
                while i < chars.len() {
                    let tc = chars[i];
                    if let Some(active_quote) = quote_char {
                        if tc == active_quote {
                            quote_char = None;
                        }
                        i += 1;
                        continue;
                    }

                    if tc == '"' || tc == '\'' {
                        quote_char = Some(tc);
                        i += 1;
                        continue;
                    }

                    if tc == '>' {
                        i += 1;
                        break;
                    }

                    i += 1;
                }
                continue;
            }
        }

        output.push(ch);
        i += 1;
    }

    output
}

fn decode_html_entities(text: &str) -> String {
    HTML_ENTITY_RE
        .replace_all(text, |caps: &regex::Captures<'_>| {
            let entity = caps.get(1).map_or("", |m| m.as_str());
            decode_html_entity(entity).into_owned()
        })
        .into_owned()
}

fn strip_control_chars(text: &str) -> String {
    let no_csi = ANSI_CSI_RE.replace_all(text, "");
    let no_osc = ANSI_OSC_RE.replace_all(&no_csi, "");
    let no_other = ANSI_OTHER_RE.replace_all(&no_osc, "");
    CONTROL_RE.replace_all(&no_other, "").into_owned()
}

#[must_use]
pub fn clean_text(text: &str, preserve_decorations: bool) -> String {
    let normalized = if preserve_decorations {
        decode_html_entities(text)
    } else {
        decode_html_entities(&strip_html_tags(text))
    };

    WHITESPACE_RE
        .replace_all(&strip_control_chars(&normalized), " ")
        .trim()
        .to_string()
}

#[must_use]
pub fn normalize_search_type(input: Option<&str>) -> SearchType {
    let Some(raw) = input else {
        return DEFAULT_SEARCH_TYPE;
    };
    let lower = raw.trim().to_lowercase();
    search_type_from_str(&lower).unwrap_or(DEFAULT_SEARCH_TYPE)
}

#[must_use]
pub fn is_valid_search_type_input(input: Option<&str>) -> bool {
    let Some(raw) = input else {
        return false;
    };
    search_type_from_str(&raw.trim().to_lowercase()).is_some()
}

#[must_use]
pub fn search_type_from_str(value: &str) -> Option<SearchType> {
    SEARCH_TYPES
        .iter()
        .copied()
        .find(|candidate| candidate.as_str() == value)
}

#[must_use]
pub fn web_result_filter_from_str(value: &str) -> Option<WebResultFilter> {
    ALLOWED_RESULT_FILTERS
        .iter()
        .copied()
        .find(|candidate| candidate.as_str() == value)
}

#[must_use]
pub fn parse_result_filter_values(input: Option<&[String]>) -> (Vec<WebResultFilter>, Vec<String>) {
    let Some(values) = input else {
        return (Vec::new(), Vec::new());
    };

    let mut accepted = Vec::<WebResultFilter>::new();
    let mut rejected = Vec::<String>::new();

    for token in values {
        let normalized = token.trim().to_lowercase();
        if normalized.is_empty() {
            continue;
        }

        if let Some(filter) = web_result_filter_from_str(&normalized) {
            if !accepted.contains(&filter) {
                accepted.push(filter);
            }
        } else if !rejected.contains(&normalized) {
            rejected.push(normalized);
        }
    }

    (accepted, rejected)
}

#[must_use]
pub fn pick_locale_language(raw: Option<&str>) -> Option<String> {
    let normalized = raw?.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }

    let normalize_alias = |value: &str| {
        if value == "ja" {
            "jp".to_string()
        } else {
            value.to_string()
        }
    };

    let full_candidate = normalize_alias(&normalized);
    if SEARCH_LANGUAGE_OPTIONS.contains(&full_candidate.as_str()) {
        return Some(full_candidate);
    }

    let short = normalized
        .split(['-', '_'])
        .next()
        .map_or(String::new(), ToString::to_string);
    if short.is_empty() {
        return None;
    }

    let short_candidate = normalize_alias(&short);
    if SEARCH_LANGUAGE_OPTIONS.contains(&short_candidate.as_str()) {
        return Some(short_candidate);
    }

    None
}

#[must_use]
pub fn normalize_safe_search(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim().to_lowercase();
    SAFE_SEARCH_OPTIONS
        .contains(&value.as_str())
        .then_some(value)
}

#[must_use]
pub fn normalize_units(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim().to_lowercase();
    UNIT_OPTIONS.contains(&value.as_str()).then_some(value)
}

#[must_use]
pub fn normalize_freshness(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim().to_lowercase();
    if value.is_empty() {
        return None;
    }
    if FRESHNESS_SHORTCUT_OPTIONS.contains(&value.as_str()) {
        return Some(value);
    }
    static FRESHNESS_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^\d{1,4}[dwmy]$").expect("valid freshness regex"));
    FRESHNESS_RE.is_match(&value).then_some(value)
}

#[must_use]
pub fn clamp_offset(raw_offset: Option<usize>, search_type: SearchType) -> usize {
    let value = raw_offset.unwrap_or(0);
    let max_offset = match search_type {
        SearchType::Images => MAX_OFFSET_IMAGES,
        SearchType::Web | SearchType::News | SearchType::Videos => MAX_OFFSET_WEB_NEWS_VIDEOS,
    };
    value.min(max_offset)
}

#[must_use]
pub fn to_limited_count(raw_count: Option<usize>) -> usize {
    raw_count.unwrap_or(DEFAULT_RESULTS).clamp(1, MAX_RESULTS)
}

#[must_use]
pub fn normalize_ui_language(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim();
    if value.is_empty() {
        return None;
    }

    let normalized = value.replace('_', "-");
    let parts: Vec<&str> = normalized.split('-').collect();
    let candidate = if parts.len() == 2 {
        format!("{}-{}", parts[0].to_lowercase(), parts[1].to_uppercase())
    } else {
        normalized
    };

    UI_LANGUAGE_OPTIONS
        .contains(&candidate.as_str())
        .then_some(candidate)
}

#[must_use]
pub fn normalize_country(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim().to_uppercase();
    COUNTRY_OPTIONS.contains(&value.as_str()).then_some(value)
}

#[must_use]
pub fn sanitize_param_for_warning(value: &str) -> String {
    let no_csi = ANSI_CSI_RE.replace_all(value, "");
    let no_osc = ANSI_OSC_RE.replace_all(&no_csi, "");
    let no_other = ANSI_OTHER_RE.replace_all(&no_osc, "");
    CONTROL_RE
        .replace_all(&no_other, "")
        .chars()
        .take(100)
        .collect()
}

#[must_use]
pub fn normalize_url_for_dedup(url: &str) -> String {
    let trimmed = url.trim();
    match url::Url::parse(trimmed) {
        Ok(parsed) => {
            let protocol = parsed.scheme().to_lowercase();
            let host = parsed
                .host_str()
                .map_or_else(String::new, str::to_lowercase);
            let port = parsed.port().map_or_else(String::new, |p| format!(":{p}"));
            let mut path = parsed.path().to_string();
            while path.ends_with('/') && path.len() > 1 {
                path.pop();
            }
            let query = parsed.query().map_or_else(String::new, |q| format!("?{q}"));
            format!("{protocol}://{host}{port}{path}{query}")
        }
        Err(_) => trimmed.to_string(),
    }
}
