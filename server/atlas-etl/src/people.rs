//! Batch P (the extensibility proof): parses Theographic Bible Metadata's
//! `people.json` (+ `verses.json` for locus resolution) into compiled
//! `Person`s -- a new sibling fact file to `theographic.rs` (events) and
//! `geo.rs` (places), the SAME "adapter reads one source's bytes, emits
//! typed data, no filesystem/network I/O" discipline this crate's own
//! module doc comment names (`lib.rs`). Record cross-references are the
//! SAME 14-char `rec...` Airtable ids `theographic::parse_events` already
//! joins through; this module's own `verse_osis_by_id` map is a second,
//! independently-built copy of that same join table (not shared code --
//! the two parses run at different points in `compile.rs`'s own pipeline,
//! and there is no existing caller both could thread one map through
//! without a larger refactor this batch does not need).
//!
//! CANON ORDER, disclosed (a real finding, not assumed): each person's own
//! `verses` field is Theographic's own list order (Airtable insertion
//! order) -- spot-checked against the real committed data and found MOSTLY
//! already ascending, but never proven so for every person, and genuinely
//! unordered for at least one real record. Every resolved verse ref is
//! explicitly SORTED here into true canon order (book index, then chapter,
//! then verse -- the same `(book, chapter, verse)` key `atlas_core::refs::
//! VerseId` carries, which does not itself derive `Ord`) rather than
//! trusted as upstream happenstance, so the graph's own `mentioned-in`
//! frontier (batch-p-brief.md: "canon order") is correct BY CONSTRUCTION,
//! never by luck -- the same discipline `geo.rs`'s own KJV-filtered, deduped
//! `verse_links` already models for places.
//!
//! STATUS FIELD, disclosed (a real finding this batch's own scoping made,
//! not an oversight): Theographic tags each person `status: "wip"` (2,781
//! of 3,067 real records) or `"publish"` (286) -- verified this is THEIR
//! OWN authoring-workflow flag (has a prose biography been finished?), not
//! a notability or correctness signal: filtering to `"publish"` only would
//! silently drop Saul, Elijah, Jeremiah, Daniel, Job, Absalom, Ahab,
//! Elisha, Nebuchadnezzar, and dozens of other unmistakably major,
//! well-attested Biblical figures (confirmed against the real committed
//! data by verse-mention count). This field is read and deliberately NOT
//! used as a filter -- every person record ships, matching the place
//! adapter's own "import the source as-is, no editorial re-curation"
//! precedent. A further, disclosed consequence: Theographic's own People
//! table is not exclusively individuated humans -- it also carries "God",
//! "Holy Spirit", eponyms doubling as tribe/nation names ("Judah": four
//! separate, genuinely bare records, `name == displayTitle == "Judah"`,
//! no disambiguation -- CORRECTED, fix round 1, R-P4: an earlier draft of
//! this note cited "Israel" here too, but the only Israel-named record,
//! `israel_682`, has `displayTitle: "Jacob (Israel)"` -- already
//! individuated and disambiguated in the exact label this adapter ships,
//! `people.rs`'s own `display_title`-first resolution below, so it was an
//! imprecise example, not a real gap), and at least one false god
//! ("Baal"), among the 3,067. This batch does not attempt to editorially
//! re-classify any of it (doctrine: "imported confidence class,"
//! `Confidence::Imported` -- carried, not curated); flagged here plainly
//! for the batch report rather than silently shipped.
//!
//! RELATIONAL DATA, ALSO PRESENT, ALSO NOT MODELED (fix round 1, R-P2 --
//! a real gap the first draft of this disclosure missed): each real
//! person record ALSO carries `father`/`mother`/`children`/`siblings`/
//! `partners` (arrays of foreign-key record ids resolving to OTHER real
//! Person records in this SAME file -- the identical "array of
//! foreign-key ids" shape `verses` uses for mentions) plus
//! `birthPlace`/`deathPlace` (resolving to place-like records) and
//! `memberOf`/`timeline`. This parser reads none of it. Not an oversight
//! this round -- turning it into typed edges (a genealogy/family
//! relation; a Person-to-Place `birthplace`/`deathplace` edge mirroring
//! `located-at`) is a NEW RELATION KIND, a types-first + owner-approved
//! decision (design types doc §3's own relation-manifest law), correctly
//! outside this batch's own card+mentions scope -- ledgered for the
//! owner, since the data needs zero new sourcing, only an adapter.

use std::collections::HashMap;

use anyhow::{Context, Result};
use atlas_core::data::Person;
use serde::Deserialize;

use crate::osis;
use crate::theographic::parse_theo_year;

#[derive(Deserialize)]
struct Record<F> {
    id: String,
    fields: F,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct VerseFields {
    #[serde(default)]
    osis_ref: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PersonFields {
    #[serde(default)]
    person_lookup: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    display_title: Option<String>,
    #[serde(default)]
    gender: Option<String>,
    #[serde(default)]
    birth_year: Option<String>,
    #[serde(default)]
    death_year: Option<String>,
    #[serde(default)]
    also_called: Option<String>,
    #[serde(default)]
    verses: Vec<String>,
    /// ENT-1a: Theographic's newer, markdown-linked Easton's extraction --
    /// always either absent or a ONE-element array in the real committed
    /// data (verified: 1,816 records carry it, every one length exactly 1;
    /// zero records carry 2+) -- never trusted as a fixed-shape tuple
    /// though, `#[serde(default)]` + `Vec` reads either shape honestly.
    #[serde(default)]
    dict_text: Vec<String>,
    /// ENT-1a: the OLDER plain-text sibling of `dict_text` above (no
    /// markdown links) -- only 1 of 3,067 real records carries this without
    /// also carrying `dict_text` (Judas -> "the Graecized form of Judah.");
    /// see `parse_people`'s own resolution below.
    #[serde(default)]
    dictionary_text: Option<String>,
    /// D5: kinship as Airtable record-id lists (verified 2026-09-18 over the
    /// real 3,067 records: father 1,584 / mother 200 / children 963 /
    /// partners 173 carry one; `siblings` (944) is NOT read -- derived).
    #[serde(default)]
    father: Vec<String>,
    #[serde(default)]
    mother: Vec<String>,
    #[serde(default)]
    children: Vec<String>,
    #[serde(default)]
    partners: Vec<String>,
    /// D5: every record carries both -- the corpus-mention span, not a life.
    #[serde(default)]
    min_year: Option<i64>,
    #[serde(default)]
    max_year: Option<i64>,
    /// D5: event record ids (268 records carry one).
    #[serde(default)]
    timeline: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PeopleStats {
    pub total: usize,
    /// D5: kinship record links seen / dropped (no person behind the record, or a self-link).
    pub kin_refs_total: usize,
    pub kin_refs_unresolved: usize,
    /// D5: `timeline` record links seen / dropped (no event behind the record).
    pub timeline_refs_total: usize,
    pub timeline_refs_unresolved: usize,
    pub with_verses: usize,
    pub verse_refs_total: usize,
    pub verse_refs_unresolved: usize,
}

/// Joins people -> verses -> canonical, canon-sorted verse ids (module doc
/// comment above has the full reasoning for the sort). An unresolvable
/// verse record id (dangling foreign key) or an unparseable `osisRef` is
/// dropped, not fatal -- `verse_refs_unresolved` surfaces the count so a
/// real regression is still visible, matching `theographic::parse_events`'s
/// own "skip, don't panic, but count it" discipline for the identical class
/// of raw-data gap.
pub fn parse_people(people_json: &str, verses_json: &str) -> Result<(Vec<Person>, PeopleStats)> {
    parse_people_full(people_json, verses_json, None)
}

/// D5: the full parse -- kinship resolved record-id -> person id through
/// the file's own `personLookup`s, `timeline` resolved through
/// `events_json` (`theographic::event_ids_by_record`) when supplied. A
/// link whose target record has no person (or, for `timeline`, no event)
/// is DROPPED and counted, never carried as a dangling record id.
pub fn parse_people_full(people_json: &str, verses_json: &str, events_json: Option<&str>) -> Result<(Vec<Person>, PeopleStats)> {
    let people: Vec<Record<PersonFields>> =
        serde_json::from_str(people_json).context("theographic people.json is not valid JSON")?;
    let person_id_by_record: HashMap<&str, String> =
        people.iter().map(|r| (r.id.as_str(), r.fields.person_lookup.clone().unwrap_or_else(|| r.id.clone()))).collect();
    let event_id_by_record: HashMap<String, String> = match events_json {
        Some(json) => crate::theographic::event_ids_by_record(json)?,
        None => HashMap::new(),
    };
    let verses: Vec<Record<VerseFields>> =
        serde_json::from_str(verses_json).context("theographic verses.json is not valid JSON")?;
    let verse_osis_by_id: HashMap<&str, &str> =
        verses.iter().filter_map(|r| r.fields.osis_ref.as_deref().map(|o| (r.id.as_str(), o))).collect();

    let mut out = Vec::with_capacity(people.len());
    let mut stats = PeopleStats::default();

    for rec in &people {
        stats.total += 1;
        let f = &rec.fields;

        let id = f.person_lookup.clone().unwrap_or_else(|| rec.id.clone());
        let name = f.display_title.clone().or_else(|| f.name.clone()).unwrap_or_else(|| id.clone());

        let also_called: Vec<String> = f
            .also_called
            .as_deref()
            .map(|s| s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect())
            .unwrap_or_default();

        // ENT-1a: `dict_text` wins over its older `dictionary_text` sibling
        // when both are present (module doc comment above has the real
        // counts) -- resolved ONCE here, the same "one ambiguity, resolved
        // at parse time" discipline `name` itself already uses just above.
        // Empty-string entries never occur in the real data (verified), but
        // `.filter(|s| !s.trim().is_empty())` keeps this honest regardless
        // of upstream happenstance: an empty string is not a source-attested
        // description any more than a missing field is.
        // batch-polish1-brief.md ENT1A-m2 (trim inconsistency, disclosed,
        // not silently unified): `.filter(|s| !s.trim().is_empty())` below
        // tests emptiness AFTER trimming, but the STORED value is the
        // ORIGINAL `s`, untrimmed -- unlike `easton.rs`'s own sibling
        // `dict_text` resolution (see that module's own doc comment at its
        // trim call site), which stores the TRIMMED slice. NOT unified
        // this batch: doing so would change compiled description bytes
        // for real data either direction (easton.rs's own doc comment has
        // the full count disclosure: 536 easton.json / 1 people.json real
        // records affected) -- forbidden by batch-polish1-brief.md's own
        // version-root law. Pinned as-is by
        // `dict_text_falls_back_to_dictionary_text_when_the_array_field_
        // is_absent` below (the real Judas record, its own leading space
        // kept) and `dict_text_with_edge_whitespace_is_never_trimmed_on_
        // the_primary_array_path_either` (synthetic, over the common
        // path -- the real array field never carries edge whitespace).
        let dict_text: Option<String> = f
            .dict_text
            .first()
            .cloned()
            .or_else(|| f.dictionary_text.clone())
            .filter(|s| !s.trim().is_empty());

        // Resolve, dedup, and canon-sort -- see this module's own "CANON
        // ORDER" doc comment for why the sort is never skipped.
        let mut resolved: Vec<(u8, u16, u16, String)> = Vec::new();
        for vref in &f.verses {
            stats.verse_refs_total += 1;
            let Some(osis_ref) = verse_osis_by_id.get(vref.as_str()) else {
                stats.verse_refs_unresolved += 1;
                continue;
            };
            let Some(vid) = osis::parse_verse(osis_ref) else {
                stats.verse_refs_unresolved += 1;
                continue;
            };
            let canon = osis::canonical(&vid);
            if !resolved.iter().any(|(_, _, _, c)| c == &canon) {
                resolved.push((vid.book.0, vid.chapter, vid.verse, canon));
            }
        }
        resolved.sort_by_key(|(book, chapter, verse, _)| (*book, *chapter, *verse));
        let verse_links: Vec<String> = resolved.into_iter().map(|(_, _, _, canon)| canon).collect();
        if !verse_links.is_empty() {
            stats.with_verses += 1;
        }

        let mut kin = |records: &[String]| -> Vec<String> {
            let mut ids: Vec<String> = Vec::new();
            for rec_id in records {
                stats.kin_refs_total += 1;
                match person_id_by_record.get(rec_id.as_str()) {
                    Some(pid) if *pid != id => {
                        if !ids.contains(pid) {
                            ids.push(pid.clone());
                        }
                    }
                    _ => stats.kin_refs_unresolved += 1,
                }
            }
            ids
        };
        let father = kin(&f.father);
        let mother = kin(&f.mother);
        let children = kin(&f.children);
        let partners = kin(&f.partners);
        let mut timeline: Vec<String> = Vec::new();
        for rec_id in &f.timeline {
            stats.timeline_refs_total += 1;
            match event_id_by_record.get(rec_id.as_str()) {
                Some(eid) => {
                    if !timeline.contains(eid) {
                        timeline.push(eid.clone());
                    }
                }
                None => stats.timeline_refs_unresolved += 1,
            }
        }

        out.push(Person {
            id,
            name,
            gender: f.gender.clone(),
            birth_year: f.birth_year.as_deref().and_then(parse_theo_year),
            death_year: f.death_year.as_deref().and_then(parse_theo_year),
            also_called,
            verse_links,
            dict_text,
            father,
            mother,
            children,
            partners,
            first_year: f.min_year.and_then(|y| i32::try_from(y).ok()),
            last_year: f.max_year.and_then(|y| i32::try_from(y).ok()),
            timeline,
            eternal: false,
            eternal_grounds: Vec::new(),
        });
    }

    Ok((out, stats))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERSES_FIXTURE: &str = r#"[
        {"id": "v1", "fields": {"osisRef": "Gen.1.1"}},
        {"id": "v2", "fields": {"osisRef": "Gen.1.2"}},
        {"id": "v3", "fields": {"osisRef": "Exod.2.1"}}
    ]"#;

    #[test]
    fn resolves_and_canon_sorts_verse_links_regardless_of_source_order() {
        // Deliberately OUT of canon order in the source list (Exodus 2:1
        // before Genesis 1:1/1:2) -- proves the explicit sort, not upstream
        // happenstance.
        let people_json = r#"[
            {"id": "p1", "fields": {"personLookup": "test_1", "name": "Test", "gender": "Male", "verses": ["v3", "v1", "v2"]}}
        ]"#;
        let (people, stats) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(people.len(), 1);
        assert_eq!(people[0].verse_links, vec!["GEN.1.1", "GEN.1.2", "EXO.2.1"]);
        assert_eq!(stats.verse_refs_total, 3);
        assert_eq!(stats.verse_refs_unresolved, 0);
        assert_eq!(stats.with_verses, 1);
    }

    #[test]
    fn also_called_splits_on_comma_and_trims() {
        let people_json = r#"[
            {"id": "p1", "fields": {"personLookup": "cephas_1", "name": "Simon Peter", "alsoCalled": "Cephas, Simon , Peter"}}
        ]"#;
        let (people, _) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(people[0].also_called, vec!["Cephas", "Simon", "Peter"]);
    }

    #[test]
    fn missing_also_called_is_an_empty_list_not_a_one_element_list() {
        let people_json = r#"[{"id": "p1", "fields": {"personLookup": "x_1", "name": "X"}}]"#;
        let (people, _) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(people[0].also_called, Vec::<String>::new());
    }

    #[test]
    fn an_unresolvable_verse_ref_is_dropped_not_panicked_on() {
        let people_json = r#"[
            {"id": "p1", "fields": {"personLookup": "x_1", "name": "X", "verses": ["dangling-id"]}}
        ]"#;
        let (people, stats) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(people[0].verse_links.len(), 0);
        assert_eq!(stats.verse_refs_unresolved, 1);
        assert_eq!(stats.with_verses, 0);
    }

    #[test]
    fn a_verse_ref_repeated_in_the_source_is_deduped() {
        let people_json = r#"[
            {"id": "p1", "fields": {"personLookup": "x_1", "name": "X", "verses": ["v1", "v1"]}}
        ]"#;
        let (people, _) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(people[0].verse_links, vec!["GEN.1.1"]);
    }

    #[test]
    fn birth_and_death_years_use_the_same_astronomical_conversion_as_events() {
        // parse_theo_year's own convention: astronomical "-1574" -> historical
        // -1575 (theographic.rs's own doc comment has the full worked example).
        let people_json = r#"[
            {"id": "p1", "fields": {"personLookup": "aaron_1", "name": "Aaron", "birthYear": "-1574", "deathYear": "-1451"}}
        ]"#;
        let (people, _) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(people[0].birth_year, Some(-1575));
        assert_eq!(people[0].death_year, Some(-1452));
    }

    #[test]
    fn missing_birth_and_death_years_stay_none_not_zero() {
        let people_json = r#"[{"id": "p1", "fields": {"personLookup": "x_1", "name": "X"}}]"#;
        let (people, _) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(people[0].birth_year, None);
        assert_eq!(people[0].death_year, None);
    }

    // ENT-1a: dict_text resolution (module doc comment above has the exact
    // real-data field-shape verification this test set mirrors).

    #[test]
    fn dict_text_prefers_the_newer_array_field_over_the_legacy_string_field() {
        let people_json = r#"[
            {"id": "p1", "fields": {"personLookup": "aaron_1", "name": "Aaron",
              "dictText": ["The eldest son of Amram."],
              "dictionaryText": " the eldest son of Amram (old form)."}}
        ]"#;
        let (people, _) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(people[0].dict_text.as_deref(), Some("The eldest son of Amram."));
    }

    #[test]
    fn dict_text_falls_back_to_dictionary_text_when_the_array_field_is_absent() {
        // The real committed data's own one exception (Judas -> "the
        // Graecized form of Judah.") -- see module doc comment. ALSO the
        // batch-polish1-brief.md ENT1A-m2 whitespace-edge pin: this real
        // record's own leading space survives untrimmed below (see the
        // doc comment at the `dict_text` resolution site above for the
        // full disclosure).
        let people_json = r#"[
            {"id": "p1", "fields": {"personLookup": "judas_1757", "name": "Judas",
              "dictionaryText": " the Graecized form of Judah."}}
        ]"#;
        let (people, _) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(
            people[0].dict_text.as_deref(), Some(" the Graecized form of Judah."),
            "leading whitespace must survive untrimmed -- CURRENT behavior, pinned (batch-polish1-brief.md ENT1A-m2)"
        );
    }

    #[test]
    fn dict_text_with_edge_whitespace_is_never_trimmed_on_the_primary_array_path_either() {
        // batch-polish1-brief.md ENT1A-m2: the sibling to the fallback
        // pin above, over the PRIMARY `dictText` array field (the real
        // data never carries edge whitespace there -- verified by direct
        // inspection, 0 of 3,067 -- so this shape is synthetic, unlike the
        // fallback test's real Judas record above).
        let people_json = r#"[
            {"id": "p1", "fields": {"personLookup": "x_1", "name": "X",
              "dictText": ["\nA son. "]}}
        ]"#;
        let (people, _) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(
            people[0].dict_text.as_deref(), Some("\nA son. "),
            "leading/trailing whitespace must survive untrimmed -- CURRENT behavior, pinned (batch-polish1-brief.md ENT1A-m2)"
        );
    }

    #[test]
    fn dict_text_stays_none_when_both_fields_are_absent() {
        let people_json = r#"[{"id": "p1", "fields": {"personLookup": "x_1", "name": "X"}}]"#;
        let (people, _) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(people[0].dict_text, None);
    }

    #[test]
    fn dict_text_stays_none_rather_than_some_empty_string() {
        let people_json = r#"[
            {"id": "p1", "fields": {"personLookup": "x_1", "name": "X",
              "dictText": [""], "dictionaryText": "   "}}
        ]"#;
        let (people, _) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(people[0].dict_text, None);
    }

    #[test]
    fn name_falls_back_to_display_title_then_bare_id_when_name_is_absent() {
        let people_json = r#"[
            {"id": "recXYZ", "fields": {"personLookup": "unnamed_1", "displayTitle": "The Unnamed One"}}
        ]"#;
        let (people, _) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(people[0].name, "The Unnamed One");

        let people_json_2 = r#"[{"id": "recXYZ", "fields": {"personLookup": "bare_1"}}]"#;
        let (people2, _) = parse_people(people_json_2, VERSES_FIXTURE).unwrap();
        assert_eq!(people2[0].name, "bare_1");
    }

    #[test]
    fn every_real_person_record_gets_a_distinct_id() {
        let people_json = r#"[
            {"id": "r1", "fields": {"personLookup": "a_1", "name": "A"}},
            {"id": "r2", "fields": {"personLookup": "b_1", "name": "B"}}
        ]"#;
        let (people, stats) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert_eq!(stats.total, 2);
        let ids: std::collections::BTreeSet<_> = people.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids.len(), 2);
    }

    /// D5: kinship and timeline resolve through record ids to PERSON and
    /// EVENT ids; a dangling record is dropped and counted; a self-link is
    /// dropped; minYear/maxYear ride as first/last year.
    #[test]
    fn kinship_and_timeline_resolve_to_person_and_event_ids() {
        let people_json = r#"[
            {"id": "recA", "fields": {"personLookup": "abraham_1", "name": "Abraham", "children": ["recI", "recGhost", "recA"], "partners": ["recS"], "minYear": -1997, "maxYear": -1821, "timeline": ["recE1", "recEGhost"]}},
            {"id": "recI", "fields": {"personLookup": "isaac_1", "name": "Isaac", "father": ["recA"], "mother": ["recS"]}},
            {"id": "recS", "fields": {"personLookup": "sarah_1", "name": "Sarah", "partners": ["recA"], "children": ["recI"]}}
        ]"#;
        let events_json = r#"[
            {"id": "recE1", "fields": {"title": "Abraham called", "eventID": 12}}
        ]"#;
        let (people, stats) = parse_people_full(people_json, VERSES_FIXTURE, Some(events_json)).unwrap();
        let abraham = people.iter().find(|p| p.id == "abraham_1").unwrap();
        assert_eq!(abraham.children, vec!["isaac_1"]);
        assert_eq!(abraham.partners, vec!["sarah_1"]);
        assert_eq!((abraham.first_year, abraham.last_year), (Some(-1997), Some(-1821)));
        assert_eq!(abraham.timeline, vec!["theo-12"]);
        assert!(!abraham.eternal && abraham.eternal_grounds.is_empty(), "eternity is curated, never parsed");
        let isaac = people.iter().find(|p| p.id == "isaac_1").unwrap();
        assert_eq!(isaac.father, vec!["abraham_1"]);
        assert_eq!(isaac.mother, vec!["sarah_1"]);
        assert_eq!(stats.kin_refs_total, 8, "3 + 1 (Abraham) + 2 (Isaac) + 2 (Sarah)");
        assert_eq!(stats.kin_refs_unresolved, 2, "the ghost record and the self-link");
        assert_eq!(stats.timeline_refs_total, 2);
        assert_eq!(stats.timeline_refs_unresolved, 1);
        // without events_json, timeline is honestly empty (and counted)
        let (people, stats) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert!(people.iter().all(|p| p.timeline.is_empty()));
        assert_eq!(stats.timeline_refs_unresolved, 2);
    }
}
