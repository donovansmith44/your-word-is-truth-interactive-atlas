//! ENT-1a ("Easton's descriptions: the data half"): parses Theographic
//! Bible Metadata's `easton.json` (6,519 entries -- Easton's Bible
//! Dictionary, 1897, public domain) into compiled [`EastonEntry`]s -- a new
//! sibling fact file to `people.rs`/`theographic.rs`/`geo.rs`, the SAME
//! "adapter reads one source's bytes, emits typed data, no filesystem/
//! network I/O" discipline this crate's own module doc comment names
//! (`lib.rs`).
//!
//! WHY THIS MODULE EXISTS SEPARATELY FROM `people.rs` (even though
//! `Person::dict_text` -- ENT-1a's own tier (a) source -- rides on the
//! SAME `people.json` this module never reads): `easton.json` is a
//! standalone 6,519-entry file, independent of any one person/place record,
//! and this module's own OUTPUT (tier (b)/(c) candidates) is consumed by
//! EVERY node kind the graph's `description_adapter.rs` fills, not just
//! Person -- a parallel sibling file, not an extension of `people.rs`.
//!
//! PLACE-NAME RESOLUTION, disclosed (batch-ent1a-brief.md's own tier (b),
//! "Theographic's own attested matching; e.g. 'Ammonite' -> ammon_58,
//! matchType 'place'"): a real, load-bearing finding from this batch's own
//! data scouting -- a compiled `PlaceId` is GEO-DERIVED (kebab-case of
//! OpenBible's own `friendly_id`, e.g. `"ammon"`), a COMPLETELY DIFFERENT
//! id space than Theographic's own internal place `slug` field (e.g.
//! `"ammon_58"`, the SAME `"name_NNN"` convention `people.json`'s own
//! `personLookup` already uses). `matchSlugs` on a `"place"`-typed easton
//! entry names THAT slug, not a compiled id -- so this module independently
//! re-parses `places.json`'s own `slug`/`displayTitle`/`kjvName` fields
//! (a second, minimal copy -- NOT `theographic::parse_events`'s own private
//! `PlaceFields`, which is `pub(crate)`-invisible outside that module and
//! serves a different join; the SAME "two independently-built copies... not
//! shared code" precedent `people.rs`'s own module doc comment already
//! establishes for its own `verse_osis_by_id`) to resolve the slug to a
//! NAME, which `description_adapter.rs` then joins against a compiled
//! Place's own `canonical` field by exact case-insensitive equality -- a
//! NAME join, deliberately, never an id join, because the id spaces never
//! agree.
//!
//! AMBIGUITY, disclosed: `matchType == "multi"` (155 of 6,519 real entries)
//! means Theographic itself attests this ONE term to MORE than one entity
//! -- exactly the "multi-candidate" case batch-ent1a-brief.md's own tier
//! (c) rule names ("no multi-candidate guessing -- ambiguity means None").
//! This module does not even attempt to unpack `"multi"`'s own `matchSlugs`
//! (verified: a stringified list-of-slugs, e.g.
//! `"['abdon_3', 'abdon_11']"`, not a clean delimited value) --
//! `person_slug`/`place_name` both stay `None` for `"multi"` (and
//! `"unmatched"`) entries; the entry's OWN `dict_lookup` remains eligible
//! for `description_adapter.rs`'s OWN tier (c) (a literal name fallback,
//! independent of Theographic's match-type judgment) regardless.

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
    /// A plain string here (UNLIKE `people.json`'s own `dictText`, which is
    /// a one-element array -- verified against both real files; each source
    /// file's own shape is honored as found, never assumed uniform across
    /// files just because the key name matches).
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
    /// Entries with no usable (non-empty, after trim) `dictText` -- dropped,
    /// not fatal (159 real entries; module doc comment has no further note
    /// since this is a plain, expected data gap, not a finding).
    pub no_text: usize,
    pub person_matches: usize,
    pub place_matches: usize,
    /// A `"place"`-typed entry whose `matchSlugs` names no record in
    /// `places.json` at all -- kept (its own `dict_lookup` still stays tier
    /// (c)-eligible), just not a tier (b) place hit. Zero in the real
    /// committed data (verified), surfaced here rather than assumed so a
    /// future data refresh regression would be visible, not silent.
    pub place_slug_unresolved: usize,
    pub multi: usize,
    pub unmatched: usize,
}

/// Parses `easton_json` (+ `places_json`, read ONLY for its own
/// `slug`/`displayTitle`/`kjvName` fields -- this module's own "PLACE-NAME
/// RESOLUTION" doc comment above has the full reasoning) into
/// `Vec<EastonEntry>`. An entry with no usable `dictText` is dropped (not
/// fatal -- `EastonStats::no_text` surfaces the count), matching every
/// sibling adapter's own "skip, don't panic, but count it" discipline for a
/// plain, expected raw-data gap.
pub fn parse_easton(easton_json: &str, places_json: &str) -> Result<(Vec<EastonEntry>, EastonStats)> {
    let entries: Vec<Record<EastonFields>> = serde_json::from_str(easton_json).context("theographic easton.json is not valid JSON")?;
    let places: Vec<Record<PlaceSlugFields>> = serde_json::from_str(places_json).context("theographic places.json is not valid JSON")?;

    // Theographic place slug -> its own display name, `displayTitle`
    // preferred then `kjvName` -- the SAME preference order
    // `atlas_etl::theographic::parse_events` already uses for its own
    // place-name resolution (module doc comment above).
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

        // batch-polish1-brief.md ENT1A-m2 (trim inconsistency, disclosed,
        // not silently unified): `str::trim` here means the STORED
        // `dict_text` is the TRIMMED slice, not the source's own raw
        // bytes -- a real inconsistency against `people.rs`'s own sibling
        // `dict_text` resolution (see that module's own doc comment at its
        // trim call site), which keeps leading/trailing whitespace
        // verbatim (proven by its own
        // `dict_text_falls_back_to_dictionary_text_when_the_array_field_
        // is_absent` test, over the real Judas record). NOT unified this
        // batch: direct inspection of the real committed data shows 536 of
        // 6,519 `easton.json` dictText entries carry edge whitespace (a
        // leading "\n" is the common shape) -- switching THIS module to
        // "keep verbatim" would change 536 real compiled descriptions'
        // own bytes; switching `people.rs` to "trim" instead would change
        // 1 (its own single dictionaryText-fallback record, Judas).
        // EITHER direction moves description content already baked into
        // the committed `graph.bin`, which batch-polish1-brief.md's own
        // law forbids this batch from doing (no version-root move this
        // batch). Shipped as-is, pinned by this module's own
        // `dict_text_with_edge_whitespace_is_trimmed_before_storage` test
        // below -- a future batch free to move the version root can pick
        // the honest rule (verbatim-from-source is the natural default,
        // per batch-polish1-brief.md item 1's own framing) and reconcile
        // both files in one commit.
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
        // The brief's own worked example: "Ammonite" -> ammon_58, matchType "place".
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
        // dict_lookup/dict_text still ride through, verbatim, for tier (c):
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
        // "the matcher never concatenates/synthesizes" -- exact byte
        // identity with the source string, not a derived/rebuilt one.
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
        // batch-polish1-brief.md ENT1A-m2: pins CURRENT behavior (this
        // module trims; `people.rs`'s own sibling resolution does NOT --
        // see the doc comment at this module's own trim call site above
        // for the full disclosure of why the inconsistency ships as-is
        // this batch). Real easton.json has 536 dictText entries carrying
        // exactly this shape (a leading "\n", from direct inspection).
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
