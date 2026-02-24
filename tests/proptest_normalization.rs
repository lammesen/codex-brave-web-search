use codex_brave_web_search::normalization::{
    clean_text, normalize_freshness, normalize_url_for_dedup, sanitize_param_for_warning,
};
use proptest::prelude::*;

proptest! {
    #[test]
    fn sanitize_param_output_is_capped_and_control_free(input in ".{0,500}") {
        let output = sanitize_param_for_warning(&input);
        prop_assert!(output.chars().count() <= 100);
        let has_control = output
            .chars()
            .any(|ch| {
                let code = ch as u32;
                (0x00..=0x08).contains(&code)
                    || code == 0x0B
                    || code == 0x0C
                    || (0x0E..=0x1F).contains(&code)
                    || (0x7F..=0x9F).contains(&code)
            });
        prop_assert!(!has_control);
    }

    #[test]
    fn clean_text_output_has_no_ansi_sequences(input in ".{0,200}") {
        let output = clean_text(&input, false);
        prop_assert!(!output.contains('\x1B'));
    }

    #[test]
    fn normalize_url_for_dedup_never_panics_and_trims(input in ".{0,200}") {
        let output = normalize_url_for_dedup(&input);
        prop_assert_eq!(output.as_str(), output.trim());
    }

    #[test]
    fn normalize_freshness_accepts_only_expected_pattern(days in 0u32..20_000) {
        let token = format!("{}d", days);
        let normalized = normalize_freshness(Some(&token));
        if days == 0 || days > 9_999 {
            prop_assert!(normalized.is_none());
        } else {
            prop_assert_eq!(normalized, Some(token.to_lowercase()));
        }
    }
}
