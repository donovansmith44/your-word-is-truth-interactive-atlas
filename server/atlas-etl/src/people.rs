//! Parses Theographic person records into compiled persons. Each record's `verses` is the source's own
//! insertion order rather than canon order -- genuinely unordered for at least one real record -- so every
//! resolved ref is sorted here. The `status` field is an upstream authoring flag, so nothing filters on it.

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
    /// Always absent or a ONE-element array in the real data, but read as a `Vec` so either shape is honest.
    #[serde(default)]
    dict_text: Vec<String>,
    /// The OLDER plain-text sibling of `dict_text`, with no markdown links: exactly one real record carries
    /// this without also carrying that one.
    #[serde(default)]
    dictionary_text: Option<String>,
    /// Kinship as record-id lists. `siblings` is deliberately not read: it is derivable from the rest.
    #[serde(default)]
    father: Vec<String>,
    #[serde(default)]
    mother: Vec<String>,
    #[serde(default)]
    children: Vec<String>,
    #[serde(default)]
    partners: Vec<String>,
    /// Every record carries both, and they bound the corpus mentions, never a life.
    #[serde(default)]
    min_year: Option<i64>,
    #[serde(default)]
    max_year: Option<i64>,
    /// Event record ids.
    #[serde(default)]
    timeline: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PeopleStats {
    pub total: usize,
    /// Kinship record links seen and dropped: no person behind the record, or a self-link.
    pub kin_refs_total: usize,
    pub kin_refs_unresolved: usize,
    /// `timeline` record links seen and dropped: no event behind the record.
    pub timeline_refs_total: usize,
    pub timeline_refs_unresolved: usize,
    pub with_verses: usize,
    pub verse_refs_total: usize,
    pub verse_refs_unresolved: usize,
}

/// An unresolvable verse record id or an unparseable reference is dropped rather than fatal, and counted,
/// so a real regression still shows.
pub fn parse_people(people_json: &str, verses_json: &str) -> Result<(Vec<Person>, PeopleStats)> {
    parse_people_full(people_json, verses_json, None)
}

/// Kinship resolves record id -> person id through the file's own lookups, and `timeline` through the events
/// file when one is supplied. A link whose target has no person, or no event, is DROPPED and counted rather
/// than carried as a dangling record id.
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

        // `dict_text` wins over its older sibling where both are present, resolved once here. The stored
        // value is the ORIGINAL string, untrimmed, unlike the sibling dictionary module, which stores the
        // trimmed slice: unifying either way would move description bytes already baked into the artifact.
        let dict_text: Option<String> = f
            .dict_text
            .first()
            .cloned()
            .or_else(|| f.dictionary_text.clone())
            .filter(|s| !s.trim().is_empty());

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
        let (people, stats) = parse_people(people_json, VERSES_FIXTURE).unwrap();
        assert!(people.iter().all(|p| p.timeline.is_empty()));
        assert_eq!(stats.timeline_refs_unresolved, 2);
    }
}
