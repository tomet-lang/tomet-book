//! Internationalization dictionaries and helpers for tmtbook.

pub mod en;
pub mod ja;

use crate::config::HeroChipConfig;
use std::collections::HashMap;

/// Returns true if the given language code represents English (e.g. "en", "en-US").
pub fn is_english(lang: &str) -> bool {
    lang.trim().to_ascii_lowercase().starts_with("en")
}

/// Look up default UI strings for a given book language code.
pub fn default_ui_strings_for(lang: &str) -> HashMap<String, String> {
    if is_english(lang) {
        en::ui_strings()
    } else {
        ja::ui_strings()
    }
}

/// Look up default infobox labels for a given book language code.
pub fn default_key_labels_for(lang: &str) -> HashMap<String, String> {
    if is_english(lang) {
        en::key_labels()
    } else {
        ja::key_labels()
    }
}

/// Look up default hero chips for a given book language code.
pub fn default_hero_chips_for(lang: &str) -> Vec<HeroChipConfig> {
    if is_english(lang) {
        en::hero_chips()
    } else {
        ja::hero_chips()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_and_japanese_dictionaries_have_identical_keys() {
        let ja_strings = ja::ui_strings();
        let en_strings = en::ui_strings();
        assert_eq!(ja_strings.len(), en_strings.len());
        for key in ja_strings.keys() {
            assert!(
                en_strings.contains_key(key),
                "Missing English UI string key: {key}"
            );
        }

        let ja_labels = ja::key_labels();
        let en_labels = en::key_labels();
        assert_eq!(ja_labels.len(), en_labels.len());
        for key in ja_labels.keys() {
            assert!(
                en_labels.contains_key(key),
                "Missing English infobox label key: {key}"
            );
        }

        let ja_chips = ja::hero_chips();
        let en_chips = en::hero_chips();
        assert_eq!(ja_chips.len(), en_chips.len());
        for i in 0..ja_chips.len() {
            assert_eq!(ja_chips[i].key, en_chips[i].key);
        }
    }
}
