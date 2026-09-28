//! Parses Easton's Bible Dictionary entries. A `"place"` match names Theographic's own slug, which is a
//! different id space from a compiled place id, so the slug is resolved to a NAME here and joined by name,
//! never by id. A `"multi"` or `"unmatched"` match resolves to nothing: ambiguity means no match.

use std::collections::HashMap;

use anyhow::{Context, Result};
use atlas_core::data::EastonEntry;
use serde::Deserialize;

#[derive(Deserialize)]
struct Record<F> {
    fields: F,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct EastonFields {
    #[serde(default)]
    dict_lookup: Option<String>,
    /// A plain string here, UNLIKE the people file's own `dictText`, which is a one-element array: each
    /// source file's shape is honored as found rather than assumed uniform because the key name matches.
    #[serde(default)]
    dict_text: Option<String>,
    #[serde(default)]
    match_type: Option<String>,
    #[serde(default)]
    match_slugs: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PlaceSlugFields {
    #[serde(default)]
    display_title: Option<String>,
    #[serde(default)]
    kjv_name: Option<String>,
    #[serde(default)]
    slug: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EastonStats {
    pub total: usize,
    /// Entries with no usable, non-empty `dictText`: dropped rather than fatal, and counted.
    pub no_text: usize,
    pub person_matches: usize,
    pub place_matches: usize,
    /// A `"place"` entry whose match names no place record at all. Kept, since its own lookup term stays
    /// eligible for the literal fallback, and counted so a future data refresh cannot regress silently.
    pub place_slug_unresolved: usize,
    pub multi: usize,
    pub unmatched: usize,
}

/// The places file is read ONLY for its slug and name fields. An entry with no usable text is dropped
/// rather than fatal, and counted.
pub fn parse_easton(easton_json: &str, places_json: &str) -> Result<(Vec<EastonEntry>, EastonStats)> {
    let entries: Vec<Record<EastonFields>> = serde_json::from_str(easton_json).context("theographic easton.json is not valid JSON")?;
    let places: Vec<Record<PlaceSlugFields>> = serde_json::from_str(places_json).context("theographic places.json is not valid JSON")?;

    // Theographic place slug -> its display name, preferring `displayTitle` then `kjvName`: the same
    // preference order the events join already uses.
    let mut name_by_slug: HashMap<&str, &str> = HashMap::new();
    for rec in &places {
        let Some(slug) = rec.fields.slug.as_deref() else { continue };
        let name = rec.fields.display_title.as_deref().or(rec.fields.kjv_name.as_deref());
        if let Some(name) = name {
            name_by_slug.entry(slug).or_insert(name);
        }
    }

    let mut out = Vec::with_capacity(entries.len());
    let mut stats = EastonStats::default();

    for rec in &entries {
        stats.total += 1;
        let f = &rec.fields;

        // `trim` here means the STORED text is the trimmed slice, an inconsistency with the sibling person
        // resolution, which keeps edge whitespace verbatim: 536 real entries carry a leading newline, so
        // either direction would move description bytes already baked into the compiled artifact.
        let Some(dict_text) = f.dict_text.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
            stats.no_text += 1;
            continue;
        };

        let match_type = f.match_type.clone().unwrap_or_default();
        let match_slugs = f.match_slugs.clone().unwrap_or_default();

        let (person_slug, place_name) = match match_type.as_str() {
            "person" => {
                stats.person_matches += 1;
                (Some(match_slugs.clone()), None)
            }
            "place" => match name_by_slug.get(match_slugs.as_str()) {
                Some(name) => {
                    stats.place_matches += 1;
                    (None, Some(name.to_lowercase()))
                }
                None => {
                    stats.place_slug_unresolved += 1;
                    (None, None)
                }
            },
            "multi" => {
                stats.multi += 1;
                (None, None)
            }
            _ => {
                stats.unmatched += 1;
                (None, None)
            }
        };

        out.push(EastonEntry {
            dict_lookup: f.dict_lookup.clone().unwrap_or_default(),
            dict_text: dict_text.to_string(),
            match_type,
            match_slugs,
            person_slug,
            place_name,
        });
    }

    Ok((out, stats))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLACES_FIXTURE: &str = r#"[
        {"id": "recA", "fields": {"slug": "ammon_58", "displayTitle": "Ammon"}},
        {"id": "recB", "fields": {"slug": "abarim_2", "kjvName": "Abarim (KJV)"}}
    ]"#;

    #[test]
    fn person_match_type_resolves_directly_to_the_theographic_person_slug() {
        let easton_json = r#"[
            {"id": "rec1", "fields": {"dictLookup": "Aaron", "dictText": "The eldest son...", "matchType": "person", "matchSlugs": "aaron_1"}}
        ]"#;
        let (entries, stats) = parse_easton(easton_json, PLACES_FIXTURE).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].person_slug.as_deref(), Some("aaron_1"));
        assert_eq!(entries[0].place_name, None);
        assert_eq!(stats.person_matches, 1);
    }

    #[test]
    fn place_match_type_resolves_through_places_json_slug_to_a_lowercased_display_name() {
        let easton_json = r#"[
            {"id": "rec1", "fields": {"dictLookup": "Ammonite", "dictText": "The usual name...", "matchType": "place", "matchSlugs": "ammon_58"}}
        ]"#;
        let (entries, stats) = parse_easton(easton_json, PLACES_FIXTURE).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].person_slug, None);
        assert_eq!(entries[0].place_name.as_deref(), Some("ammon"));
        assert_eq!(stats.place_matches, 1);
    }

    #[test]
    fn place_match_type_falls_back_to_kjv_name_when_display_title_is_absent() {
        let easton_json = r#"[
            {"id": "rec1", "fields": {"dictLookup": "Abarim", "dictText": "A mountain range...", "matchType": "place", "matchSlugs": "abarim_2"}}
        ]"#;
        let (entries, _) = parse_easton(easton_json, PLACES_FIXTURE).unwrap();
        assert_eq!(entries[0].place_name.as_deref(), Some("abarim (kjv)"));
    }

    #[test]
    fn a_place_slug_absent_from_places_json_resolves_to_no_place_name_but_is_still_kept() {
        let easton_json = r#"[
            {"id": "rec1", "fields": {"dictLookup": "Nowhere", "dictText": "Some text.", "matchType": "place", "matchSlugs": "does-not-exist_1"}}
        ]"#;
        let (entries, stats) = parse_easton(easton_json, PLACES_FIXTURE).unwrap();
        assert_eq!(entries.len(), 1, "the entry itself is kept -- its dict_lookup stays tier-(c) eligible");
        assert_eq!(entries[0].place_name, None);
        assert_eq!(stats.place_slug_unresolved, 1);
        assert_eq!(stats.place_matches, 0);
    }

    #[test]
    fn multi_and_unmatched_match_types_resolve_to_neither_person_nor_place() {
        let easton_json = r#"[
            {"id": "rec1", "fields": {"dictLookup": "Abdon", "dictText": "Text A.", "matchType": "multi", "matchSlugs": "['abdon_3', 'abdon_11']"}},
            {"id": "rec2", "fields": {"dictLookup": "A", "dictText": "Alpha, the first letter.", "matchType": "unmatched", "matchSlugs": "unmatched"}}
        ]"#;
        let (entries, stats) = parse_easton(easton_json, PLACES_FIXTURE).unwrap();
        assert_eq!(entries.len(), 2);
        for e in &entries {
            assert_eq!(e.person_slug, None);
            assert_eq!(e.place_name, None);
        }
        assert_eq!(stats.multi, 1);
        assert_eq!(stats.unmatched, 1);
        assert_eq!(entries[0].dict_lookup, "Abdon");
        assert_eq!(entries[1].dict_text, "Alpha, the first letter.");
    }

    #[test]
    fn an_entry_with_no_dict_text_is_dropped_not_panicked_on() {
        let easton_json = r#"[
            {"id": "rec1", "fields": {"dictLookup": "Empty", "matchType": "unmatched", "matchSlugs": "unmatched"}},
            {"id": "rec2", "fields": {"dictLookup": "Blank", "dictText": "   ", "matchType": "unmatched", "matchSlugs": "unmatched"}}
        ]"#;
        let (entries, stats) = parse_easton(easton_json, PLACES_FIXTURE).unwrap();
        assert_eq!(entries.len(), 0);
        assert_eq!(stats.no_text, 2);
        assert_eq!(stats.total, 2);
    }

    #[test]
    fn dict_text_and_dict_lookup_ride_through_verbatim_never_transformed() {
        let src_text = "The eldest son of Amram and Jochebed ([Ex. 6:20](/exod#Exod.6.20)).";
        let easton_json = format!(
            r#"[{{"id": "rec1", "fields": {{"dictLookup": "Aaron", "dictText": {:?}, "matchType": "person", "matchSlugs": "aaron_1"}}}}]"#,
            src_text
        );
        let (entries, _) = parse_easton(&easton_json, PLACES_FIXTURE).unwrap();
        assert_eq!(entries[0].dict_text, src_text);
        assert_eq!(entries[0].dict_lookup, "Aaron");
    }

    #[test]
    fn dict_text_with_edge_whitespace_is_trimmed_before_storage() {
        let easton_json = r#"[
            {"id": "rec1", "fields": {"dictLookup": "Ben", "dictText": "\nA son. ", "matchType": "unmatched", "matchSlugs": "unmatched"}}
        ]"#;
        let (entries, _) = parse_easton(easton_json, PLACES_FIXTURE).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].dict_text, "A son.",
            "leading/trailing whitespace is trimmed before storage -- CURRENT behavior, pinned (not necessarily the honest rule; see the doc comment at this module's own trim call site)"
        );
    }
}
