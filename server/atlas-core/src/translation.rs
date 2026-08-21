//! Batch T ("events as the narrative nodes"): the translation-indirection
//! seam the owner's own internal representation asks for, verbatim
//! (batch-t-brief.md): "this set of passages with their titles maps to a
//! mapping of translation to a set of verses (so that we can expand into
//! different translations and keep mappings the same)."
//!
//! [`data::EventWitness::translations`] is a real `HashMap<String,
//! Vec<String>>` keyed by translation code, not a single flat `Vec<String>`
//! with an implicit "it's KJV" comment -- `resolve` below is a genuine,
//! fail-loud lookup exercised by the server (`handlers::event`) on every
//! request, not decorative scaffolding. KJV (`DEFAULT_TRANSLATION`) is the
//! only translation this app compiles today (every curated witness carries
//! exactly one key); asking for anything else -- a future ESV/NASB import
//! that never lands, or a client typo -- returns
//! [`crate::CoreError::UnknownTranslation`], never a silent fallback to KJV
//! and never a panic. This is the ENTIRE indirection this batch ships: the
//! shape exists so a future translation can be added under the SAME witness
//! identity (book + ref span) without restructuring anything, per the
//! brief's own "ships NOW so future translations keep passage identities."

use std::collections::HashMap;

use crate::CoreError;

/// The one translation this atlas compiles verse text for today. Lowercase,
/// matching the key every curated witness's `translations` map is built
/// with (`atlas_etl::curated::parse_event_witnesses`).
pub const DEFAULT_TRANSLATION: &str = "kjv";

/// Resolves `code` against `translations` (a `PASSAGE` witness's own
/// translation -> verse-set mapping) -- `Ok` with that translation's own
/// flat, canonical verse-id list on a hit, `Err(CoreError::UnknownTranslation)`
/// on a miss. Case-sensitive by design (curated data and callers both use
/// the same lowercase convention throughout this codebase -- e.g.
/// `CatechismQuestion::source`, `PolityDelta::event` -- so normalizing case
/// here would hide a real curator typo rather than surface it).
pub fn resolve<'a>(translations: &'a HashMap<String, Vec<String>>, code: &str) -> Result<&'a [String], CoreError> {
    translations.get(code).map(|v| v.as_slice()).ok_or_else(|| CoreError::UnknownTranslation(code.to_string()))
}

/// Batch E3 (KJV display-name alias layer): sibling to [`resolve`] above,
/// same fail-loud lookup, same case-sensitive-by-design reasoning -- but for
/// a `PlaceNameAlias::translations`-shaped map (translation code -> ONE
/// display name), not `EventWitness::translations`'s translation -> verse
/// SET shape. A place's curated KJV alias travels exactly this same
/// indirection ("kjv is the only key today; identity survives future
/// translations" -- batch-e3-brief.md requirement 1) so a future ESV/NASB
/// alias can be added under the SAME place identity without restructuring.
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
        // "KJV" (uppercase) must NOT silently match "kjv" -- a curator/caller
        // typo in translation-code CASE is a real bug worth surfacing, not
        // smoothing over, matching this codebase's own convention for every
        // other lowercase provenance tag (CatechismQuestion::source, etc.).
        assert!(resolve(&translations, "KJV").is_err());
    }

    #[test]
    fn resolve_against_an_empty_map_fails_loud_not_panics() {
        let translations: HashMap<String, Vec<String>> = HashMap::new();
        assert!(resolve(&translations, "kjv").is_err());
    }

    // --- Batch E3: resolve_name (place-alias sibling of resolve) -----------

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
