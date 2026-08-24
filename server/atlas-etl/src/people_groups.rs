//! PG-1a ("People groups & eponymy: the data half" -- batch-pg1a-brief.md
//! controller decision 1a): parses Theographic Bible Metadata's
//! `peopleGroups.json` (23 records: 12 tribes + Nation of Israel + a
//! handful of NT collectives) into compiled [`PeopleGroup`]s -- a new
//! sibling fact file to `people.rs`/`easton.rs`, the SAME "adapter reads
//! one source's bytes, emits typed data, no filesystem/network I/O"
//! discipline this crate's own module doc comment names (`lib.rs`).
//!
//! MOSTLY NODES ONLY, CORRECTED (PG-1B rider, batch-edge1a-brief.md
//! controller decision 0): the PG-1a doc comment this replaces claimed
//! `peopleGroups.json` ships NO per-group verse arrays at all -- FALSE for
//! 2 of the 23 records. Tribe of Judah and Nation of Israel DO carry a
//! real `verses` field (Airtable record ids into `verses.json`, the SAME
//! foreign-key shape `people.rs`'s own `PersonFields.verses` already
//! resolves) -- reciprocally back-referenced by each of those 13 verse
//! records' own `peopleGroups` field (the Sin-guard's "source-attested,
//! not string-guessed" bar). This module now resolves them the SAME way
//! `people::parse_people` resolves `Person.verse_links`: join through
//! `verses.json`'s own `osisRef`, dedup, canon-sort. The OTHER 21 records
//! genuinely carry no `verses` field, so `verse_links` stays honestly
//! empty for them. `members` (person-record ids) and `events_dev` (event
//! ids) remain UNIMPORTED (decision 1a: "the members data is NOT imported
//! this batch -- a member-of relation is a noted owner option, unordered")
//! -- `id` (derived, see below), `label` (`groupName` verbatim), and now
//! `verse_links` are the three facts the graph adapter
//! (`atlas_graph::peoples_adapter`) needs.
//!
//! NO ID-LOOKUP FIELD, disclosed: unlike `people.json`'s own
//! `personLookup`, `peopleGroups.json` ships no analogous slug/lookup
//! field for its own 23 records -- every record's ONLY name-bearing field
//! is `groupName` (`"Tribe of Levi"`, `"Nation of Israel"`, ...). This
//! module derives a kebab-case slug FROM `groupName` (see [`slugify`]) as
//! the compiled `PeopleGroup.id`, rather than falling back to the raw
//! Airtable record id the way `people.rs`'s own no-lookup-field fallback
//! does (`f.person_lookup.clone().unwrap_or_else(|| rec.id.clone())`) --
//! a DELIBERATE divergence from that precedent, not an oversight: curated
//! data (`data/curated/people-groups.toml`'s own `[[named_after]]` rows)
//! needs to REFERENCE these ids BY HAND, and an opaque Airtable id (e.g.
//! `"reciI2noa29XOlF3E"`) would make that curated authoring illegible,
//! where a friendly kebab-case slug (`"tribe-of-judah"`) does not -- the
//! SAME "friendly, human-legible slug" shape `PlaceId` already uses
//! (geo-derived kebab-case, per `easton.rs`'s own "PLACE-NAME RESOLUTION"
//! doc comment). Verified collision-free over the real committed data (23
//! distinct `groupName` values -> 23 distinct slugs; `parse_people_groups`'s
//! own tests re-prove this against the real file).

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
    /// PG-1B rider: present (non-empty) on exactly 2 of the 23 real
    /// records -- Tribe of Judah, Nation of Israel. Module doc comment
    /// above has the full correction story.
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
    /// Records with no usable (non-empty, after trim) `groupName` --
    /// dropped, not fatal (zero in the real committed data, verified;
    /// surfaced anyway per this crate's own "skip, don't panic, but count
    /// it" discipline for a plain, expected raw-data gap).
    pub no_name: usize,
    /// PG-1B rider: groups carrying >=1 resolved verse link (2 of 23 in
    /// the real committed data: Tribe of Judah, Nation of Israel).
    pub with_verses: usize,
    /// PG-1B rider: total raw `verses` foreign-key entries seen across all
    /// records, before resolution -- mirrors `people::PeopleStats::
    /// verse_refs_total`.
    pub verse_refs_total: usize,
    /// PG-1B rider: raw verse refs that failed to resolve (dangling
    /// `verses.json` foreign key, or an unparseable `osisRef`) -- dropped,
    /// not fatal, same "skip, don't panic, but count it" discipline as
    /// `no_name` above. Zero in the real committed data (verified).
    pub verse_refs_unresolved: usize,
}

/// Kebab-case slug from a display name: lowercase; every run of
/// non-ASCII-alphanumeric characters collapses to one hyphen; leading/
/// trailing hyphens trimmed. Module doc comment above has the full "why"
/// (no source lookup field exists to reuse). E.g. `"Tribe of Levi"` ->
/// `"tribe-of-levi"`; `"Apostles (The Eleven)"` -> `"apostles-the-eleven"`.
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

/// Parses `people_groups_json` (Theographic's own `peopleGroups.json`) +
/// `verses_json` (for locus resolution, PG-1B rider) into
/// `Vec<PeopleGroup>`. A record with no usable `groupName` is dropped (not
/// fatal -- `PeopleGroupStats::no_name` surfaces the count), matching
/// every sibling adapter's own "skip, don't panic, but count it"
/// discipline for a plain, expected raw-data gap. `verses` resolution
/// mirrors `people::parse_people` exactly: join through `verses.json`'s
/// own `osisRef`, dedup, canon-sort (module doc comment's own "CANON
/// ORDER" reasoning applies here too -- Theographic's own list order is
/// not trusted as upstream happenstance).
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
        // Decision 1a: "the members data is NOT imported this batch" --
        // proven by feeding a record that ALSO carries partOf (a real
        // shape some Theographic tribe records have) and confirming the
        // parsed PeopleGroup carries only id/label/verse_links, nothing
        // else to leak.
        let json = r#"[{"id": "recX", "fields": {"groupName": "Tribe of Gad", "members": ["recA","recB"], "partOf": ["recC"], "events_dev": ["recD"]}}]"#;
        let (groups, _) = parse_people_groups(json, VERSES_FIXTURE).unwrap();
        assert_eq!(groups, vec![atlas_core::data::PeopleGroup { id: "tribe-of-gad".into(), label: "Tribe of Gad".into(), verse_links: vec![] }]);
    }

    // --- PG-1B rider: `verses` resolution -----------------------------------

    #[test]
    fn resolves_and_canon_sorts_a_groups_own_verses_field() {
        // Deliberately OUT of canon order in the source list -- proves the
        // explicit sort, not upstream happenstance (mirrors people.rs's own
        // identical test for Person.verse_links).
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

    /// PG-1B rider (batch-edge1a-brief.md decision 0): "REPORT WHICH 13
    /// LOCI these are verbatim (if Tribe of Judah's one verse is JDG 1:2,
    /// say so loudly -- it is the owner's own motivating example)." Read
    /// against the REAL committed data: it is NOT JDG 1:2 -- Tribe of
    /// Judah's one real, reciprocally-linked verse is PRO.25.1 ("These are
    /// also proverbs of Solomon, which the men of Hezekiah king of Judah
    /// copied out"). Nation of Israel carries the other 12, all in Psalms.
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

        // Every OTHER group must stay honestly empty -- these two are not
        // representative of the whole 23-record set.
        for g in &groups {
            if g.id != "tribe-of-judah" && g.id != "nation-of-israel" {
                assert!(g.verse_links.is_empty(), "'{}' must carry no verse_links -- only Tribe of Judah/Nation of Israel do in the real data", g.id);
            }
        }
    }
}
