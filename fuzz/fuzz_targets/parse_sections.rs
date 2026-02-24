#![no_main]

use codex_brave_web_search::parsing::parse_sections;
use codex_brave_web_search::types::{SearchType, WebResultFilter};
use libfuzzer_sys::fuzz_target;

fn pick_search_type(byte: u8) -> SearchType {
    match byte % 4 {
        0 => SearchType::Web,
        1 => SearchType::News,
        2 => SearchType::Images,
        _ => SearchType::Videos,
    }
}

fn pick_filter(byte: u8) -> WebResultFilter {
    match byte % 5 {
        0 => WebResultFilter::Web,
        1 => WebResultFilter::Discussions,
        2 => WebResultFilter::Videos,
        3 => WebResultFilter::News,
        _ => WebResultFilter::Infobox,
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(data) else {
        return;
    };

    let search_type = pick_search_type(*data.first().unwrap_or(&0));
    let requested = usize::from(*data.get(1).unwrap_or(&5)).clamp(1, 20);
    let preserve_decorations = data.get(2).is_some_and(|b| b % 2 == 1);

    let filters = if search_type == SearchType::Web {
        let mut picked = Vec::new();
        for byte in data.iter().skip(3).take(4) {
            let filter = pick_filter(*byte);
            if !picked.contains(&filter) {
                picked.push(filter);
            }
        }
        picked
    } else {
        Vec::new()
    };

    let _ = parse_sections(
        &value,
        search_type,
        &filters,
        requested,
        preserve_decorations,
    );
});
