#![no_main]

use codex_brave_web_search::parsing::parse_brave_error_message;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = parse_brave_error_message(text, "fallback");
    }
});
