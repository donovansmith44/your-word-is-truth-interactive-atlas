//! Parses Theographic people-group records. Only 2 of the 23 carry a `verses` field, and `members` and the
//! event ids are not imported. The source ships no lookup field, so a compiled id is a kebab-case slug of
//! the group name: curated rows reference these ids by hand, where an opaque record id would not read.

use std::collections::HashMap;

use anyhow::{Context, Result};
use atlas_core::data::PeopleGroup;
use serde::Deserialize;

use crate::osis;

#[derive(Deserialize)]
struct Record<F> {
    id: String,
    fields: F,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PeopleGroupFields {
    #[serde(default)]
    group_name: Option<String>,
    /// Present and non-empty on exactly 2 of the 23 real records.
    #[serde(default)]
    verses: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct VerseFields {
    #[serde(default)]
    osis_ref: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PeopleGroupStats {
    pub total: usize,
    /// Records with no usable, non-empty name: dropped rather than fatal, and counted.
    pub no_name: usize,
    /// PG-1B rider: groups carrying >=1 resolved verse link (2 of 23 in
    /// the real committed data: Tribe of Judah, Nation of Israel).
    pub with_verses: usize,
    /// Total raw `verses` foreign-key entries seen across all records, before resolution.
    pub verse_refs_total: usize,
    /// Raw verse refs that failed to resolve -- a dangling foreign key, or an unparseable reference --
    /// dropped rather than fatal, and counted.
    pub verse_refs_unresolved: usize,
}

/// Kebab-case slug from a display name: lowercased, every run of non-alphanumeric characters collapsed to
/// one hyphen, no leading or trailing hyphen. `"Tribe of Levi"` becomes `"tribe-of-levi"`.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut pending_hyphen = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if pending_hyphen && !out.is_empty() {
                out.push('-');
            }
            pending_hyphen = false;
            out.push(c.to_ascii_lowercase());
        } else {
            pending_hyphen = true;
        }
    }
    out
}

/// A record with no usable name is dropped rather than fatal, and counted. The verse refs are joined through
/// the verses file, deduped and canon-sorted: the source's own list order is not trusted.
pub fn parse_people_groups(people_groups_json: &str, verses_json: &str) -> Result<(Vec<PeopleGroup>, PeopleGroupStats)> {
    let records: Vec<Record<PeopleGroupFields>> =
        serde_json::from_str(people_groups_json).context("theographic peopleGroups.json is not valid JSON")?;
    let verses: Vec<Record<VerseFields>> =
        serde_json::from_str(verses_json).context("theographic verses.json is not valid JSON")?;
    let verse_osis_by_id: HashMap<&str, &str> =
        verses.iter().filter_map(|r| r.fields.osis_ref.as_deref().map(|o| (r.id.as_str(), o))).collect();

    let mut out = Vec::with_capacity(records.len());
    let mut stats = PeopleGroupStats::default();
    for rec in &records {
        stats.total += 1;
        let Some(label) = rec.fields.group_name.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
            stats.no_name += 1;
            continue;
        };

        let mut resolved: Vec<(u8, u16, u16, String)> = Vec::new();
        for vref in &rec.fields.verses {
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

        out.push(PeopleGroup { id: slugify(label), label: label.to_string(), verse_links });
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
    fn slugify_lowercases_and_hyphenates() {
        assert_eq!(slugify("Tribe of Levi"), "tribe-of-levi");
        assert_eq!(slugify("Nation of Israel"), "nation-of-israel");
    }

    #[test]
    fn slugify_collapses_punctuation_and_trims_edges() {
        assert_eq!(slugify("Apostles (The Eleven)"), "apostles-the-eleven");
        assert_eq!(slugify("Apostles (Post-Ascension)"), "apostles-post-ascension");
        assert_eq!(slugify("  Chief Priests!  "), "chief-priests");
    }

    #[test]
    fn parses_group_name_into_label_and_derives_the_id() {
        let json = r#"[
            {"id": "recuYvXjZsXumRLPL", "fields": {"groupName": "Tribe of Levi", "members": ["recA"], "events_dev": ["recB"]}},
            {"id": "recsTcOXoP1DEM5lL", "fields": {"groupName": "Nation of Israel"}}
        ]"#;
        let (groups, stats) = parse_people_groups(json, VERSES_FIXTURE).unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.no_name, 0);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].id, "tribe-of-levi");
        assert_eq!(groups[0].label, "Tribe of Levi");
        assert_eq!(groups[1].id, "nation-of-israel");
        assert_eq!(groups[1].label, "Nation of Israel");
    }

    #[test]
    fn a_record_with_no_group_name_is_dropped_not_panicked_on() {
        let json = r#"[{"id": "recX", "fields": {}}]"#;
        let (groups, stats) = parse_people_groups(json, VERSES_FIXTURE).unwrap();
        assert_eq!(groups.len(), 0);
        assert_eq!(stats.total, 1);
        assert_eq!(stats.no_name, 1);
    }

    #[test]
    fn members_events_dev_and_part_of_are_read_by_nothing_here() {
        let json = r#"[{"id": "recX", "fields": {"groupName": "Tribe of Gad", "members": ["recA","recB"], "partOf": ["recC"], "events_dev": ["recD"]}}]"#;
        let (groups, _) = parse_people_groups(json, VERSES_FIXTURE).unwrap();
        assert_eq!(groups, vec![atlas_core::data::PeopleGroup { id: "tribe-of-gad".into(), label: "Tribe of Gad".into(), verse_links: vec![] }]);
    }

    #[test]
    fn resolves_and_canon_sorts_a_groups_own_verses_field() {
        let json = r#"[{"id": "recX", "fields": {"groupName": "Nation of Israel", "verses": ["v3", "v1", "v2"]}}]"#;
        let (groups, stats) = parse_people_groups(json, VERSES_FIXTURE).unwrap();
        assert_eq!(groups[0].verse_links, vec!["GEN.1.1", "GEN.1.2", "EXO.2.1"]);
        assert_eq!(stats.verse_refs_total, 3);
        assert_eq!(stats.verse_refs_unresolved, 0);
        assert_eq!(stats.with_verses, 1);
    }

    #[test]
    fn a_group_with_no_verses_field_at_all_gets_an_empty_verse_links_not_an_error() {
        let json = r#"[{"id": "recX", "fields": {"groupName": "Tribe of Gad"}}]"#;
        let (groups, stats) = parse_people_groups(json, VERSES_FIXTURE).unwrap();
        assert_eq!(groups[0].verse_links, Vec::<String>::new());
        assert_eq!(stats.with_verses, 0);
        assert_eq!(stats.verse_refs_total, 0);
    }

    #[test]
    fn an_unresolvable_group_verse_ref_is_dropped_not_panicked_on() {
        let json = r#"[{"id": "recX", "fields": {"groupName": "Nation of Israel", "verses": ["dangling-id"]}}]"#;
        let (groups, stats) = parse_people_groups(json, VERSES_FIXTURE).unwrap();
        assert_eq!(groups[0].verse_links.len(), 0);
        assert_eq!(stats.verse_refs_unresolved, 1);
        assert_eq!(stats.with_verses, 0);
    }

    #[test]
    fn a_group_verse_ref_repeated_in_the_source_is_deduped() {
        let json = r#"[{"id": "recX", "fields": {"groupName": "Nation of Israel", "verses": ["v1", "v1"]}}]"#;
        let (groups, _) = parse_people_groups(json, VERSES_FIXTURE).unwrap();
        assert_eq!(groups[0].verse_links, vec!["GEN.1.1"]);
    }

    #[test]
    fn real_committed_data_yields_exactly_23_groups_with_collision_free_slugs() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw/theographic/theographic-bible-metadata-master/json");
        let json = std::fs::read_to_string(dir.join("peopleGroups.json")).expect("data/raw/theographic/.../peopleGroups.json must exist");
        let verses_json = std::fs::read_to_string(dir.join("verses.json")).expect("data/raw/theographic/.../verses.json must exist");
        let (groups, stats) = parse_people_groups(&json, &verses_json).expect("the real committed file must parse");
        assert_eq!(stats.total, 23, "the real committed peopleGroups.json must carry exactly 23 records (PG-1a batch report's own count)");
        assert_eq!(groups.len(), 23);
        let mut ids: Vec<&str> = groups.iter().map(|g| g.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 23, "every derived slug must be collision-free over the real committed group names");
    }

    #[test]
    fn real_committed_data_resolves_exactly_the_two_verse_bearing_groups() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw/theographic/theographic-bible-metadata-master/json");
        let json = std::fs::read_to_string(dir.join("peopleGroups.json")).expect("peopleGroups.json must exist");
        let verses_json = std::fs::read_to_string(dir.join("verses.json")).expect("verses.json must exist");
        let (groups, stats) = parse_people_groups(&json, &verses_json).expect("the real committed file must parse");

        assert_eq!(stats.with_verses, 2, "exactly 2 of 23 real records carry a non-empty verses field");
        assert_eq!(stats.verse_refs_unresolved, 0, "every raw verse ref in the real committed data must resolve");

        let judah = groups.iter().find(|g| g.id == "tribe-of-judah").expect("Tribe of Judah must exist");
        assert_eq!(judah.verse_links, vec!["PRO.25.1"], "Tribe of Judah's own one real verse -- NOT JDG.1.2, the owner's own suspected motivating example");

        let israel = groups.iter().find(|g| g.id == "nation-of-israel").expect("Nation of Israel must exist");
        assert_eq!(
            israel.verse_links,
            vec!["PSA.14.7", "PSA.53.6", "PSA.76.1", "PSA.78.21", "PSA.78.31", "PSA.78.41", "PSA.81.8", "PSA.81.11", "PSA.81.13", "PSA.89.18", "PSA.105.10", "PSA.147.19"],
            "Nation of Israel's own real 12 verses, canon-sorted"
        );
        assert_eq!(judah.verse_links.len() + israel.verse_links.len(), 13, "13 total loci across the two verse-bearing groups");

        for g in &groups {
            if g.id != "tribe-of-judah" && g.id != "nation-of-israel" {
                assert!(g.verse_links.is_empty(), "'{}' must carry no verse_links -- only Tribe of Judah/Nation of Israel do in the real data", g.id);
            }
        }
    }
}
