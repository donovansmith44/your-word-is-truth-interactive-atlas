//! An unknown translation code is an error here, never a silent fall back to the default.

use std::collections::HashMap;

use crate::CoreError;

/// Lowercase, matching the key every curated witness's `translations` map is built with.
pub const DEFAULT_TRANSLATION: &str = "kjv";

/// Case-sensitive: curated data and callers share the lowercase convention, so
/// normalising case here would hide a curator's typo instead of surfacing it.
pub fn resolve<'a>(translations: &'a HashMap<String, Vec<String>>, code: &str) -> Result<&'a [String], CoreError> {
    translations.get(code).map(|v| v.as_slice()).ok_or_else(|| CoreError::UnknownTranslation(code.to_string()))
}

/// Case-sensitive for the same reason as [`resolve`].
pub fn resolve_name<'a>(names: &'a HashMap<String, String>, code: &str) -> Result<&'a str, CoreError> {
    names.get(code).map(|s| s.as_str()).ok_or_else(|| CoreError::UnknownTranslation(code.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_with_kjv() -> HashMap<String, Vec<String>> {
        HashMap::from([(DEFAULT_TRANSLATION.to_string(), vec!["MAT.27.35".to_string(), "MAT.27.36".to_string()])])
    }

    #[test]
    fn resolve_kjv_returns_the_verse_set() {
        let translations = map_with_kjv();
        let verses = resolve(&translations, "kjv").expect("kjv is populated");
        assert_eq!(verses, &["MAT.27.35".to_string(), "MAT.27.36".to_string()]);
    }

    #[test]
    fn resolve_unknown_translation_fails_loud() {
        let translations = map_with_kjv();
        let err = resolve(&translations, "esv").expect_err("esv is not compiled by this atlas");
        assert!(matches!(err, CoreError::UnknownTranslation(code) if code == "esv"));
    }

    #[test]
    fn resolve_is_case_sensitive_not_a_silent_normalize() {
        let translations = map_with_kjv();
        assert!(resolve(&translations, "KJV").is_err());
    }

    #[test]
    fn resolve_against_an_empty_map_fails_loud_not_panics() {
        let translations: HashMap<String, Vec<String>> = HashMap::new();
        assert!(resolve(&translations, "kjv").is_err());
    }

    fn name_map_with_kjv() -> HashMap<String, String> {
        HashMap::from([(DEFAULT_TRANSLATION.to_string(), "Ethiopia".to_string())])
    }

    #[test]
    fn resolve_name_kjv_returns_the_display_name() {
        let names = name_map_with_kjv();
        assert_eq!(resolve_name(&names, "kjv").expect("kjv is populated"), "Ethiopia");
    }

    #[test]
    fn resolve_name_unknown_translation_fails_loud() {
        let names = name_map_with_kjv();
        let err = resolve_name(&names, "esv").expect_err("esv is not compiled by this atlas");
        assert!(matches!(err, CoreError::UnknownTranslation(code) if code == "esv"));
    }

    #[test]
    fn resolve_name_is_case_sensitive_not_a_silent_normalize() {
        let names = name_map_with_kjv();
        assert!(resolve_name(&names, "KJV").is_err());
    }

    #[test]
    fn resolve_name_against_an_empty_map_fails_loud_not_panics() {
        let names: HashMap<String, String> = HashMap::new();
        assert!(resolve_name(&names, "kjv").is_err());
    }
}
