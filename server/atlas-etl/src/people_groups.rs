//! PG-1a ("People groups & eponymy: the data half" -- batch-pg1a-brief.md
//! controller decision 1a): parses Theographic Bible Metadata's
//! `peopleGroups.json` (23 records: 12 tribes + Nation of Israel + a
//! handful of NT collectives) into compiled [`PeopleGroup`]s -- a new
//! sibling fact file to `people.rs`/`easton.rs`, the SAME "adapter reads
//! one source's bytes, emits typed data, no filesystem/network I/O"
//! discipline this crate's own module doc comment names (`lib.rs`).
//!
//! NODES ONLY, disclosed (the corrected data-scouting finding this
//! batch's own brief ledgers, "PG-1/ENT-1 data scouting: ledgered premise
//! CORRECTED"): the ledgered PG-1 proposal originally claimed
//! `peopleGroups.json` ships per-group verse arrays -- FALSE. The real
//! shape is `groupName` + `members` (person-record ids) + `events_dev`
//! (event ids): membership/event-participation data, not per-locus
//! mentions. This module reads `members`/`events_dev`/`partOf` from NO
//! record (decision 1a: "the members data is NOT imported this batch --
//! a member-of relation is a noted owner option, unordered") -- `id`
//! (derived, see below) and `label` (`groupName` verbatim) are the only
//! two facts the graph adapter (`atlas_graph::peoples_adapter`) needs.
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

use anyhow::{Context, Result};
use atlas_core::data::PeopleGroup;
use serde::Deserialize;

#[derive(Deserialize)]
struct Record<F> {
    fields: F,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PeopleGroupFields {
    #[serde(default)]
    group_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PeopleGroupStats {
    pub total: usize,
    /// Records with no usable (non-empty, after trim) `groupName` --
    /// dropped, not fatal (zero in the real committed data, verified;
    /// surfaced anyway per this crate's own "skip, don't panic, but count
    /// it" discipline for a plain, expected raw-data gap).
    pub no_name: usize,
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

/// Parses `people_groups_json` (Theographic's own `peopleGroups.json`)
/// into `Vec<PeopleGroup>`. A record with no usable `groupName` is dropped
/// (not fatal -- `PeopleGroupStats::no_name` surfaces the count), matching
/// every sibling adapter's own "skip, don't panic, but count it"
/// discipline for a plain, expected raw-data gap.
pub fn parse_people_groups(people_groups_json: &str) -> Result<(Vec<PeopleGroup>, PeopleGroupStats)> {
    let records: Vec<Record<PeopleGroupFields>> =
        serde_json::from_str(people_groups_json).context("theographic peopleGroups.json is not valid JSON")?;

    let mut out = Vec::with_capacity(records.len());
    let mut stats = PeopleGroupStats::default();
    for rec in &records {
        stats.total += 1;
        let Some(label) = rec.fields.group_name.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
            stats.no_name += 1;
            continue;
        };
        out.push(PeopleGroup { id: slugify(label), label: label.to_string() });
    }
    Ok((out, stats))
}

#[cfg(test)]
mod tests {
    use super::*;

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
            {"id": "recsTcOXoP1DEM5lL", "fields": {"groupName": "Nation of Israel", "verses": ["recC"]}}
        ]"#;
        let (groups, stats) = parse_people_groups(json).unwrap();
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
        let (groups, stats) = parse_people_groups(json).unwrap();
        assert_eq!(groups.len(), 0);
        assert_eq!(stats.total, 1);
        assert_eq!(stats.no_name, 1);
    }

    #[test]
    fn members_events_dev_and_part_of_are_read_by_nothing_here() {
        // Decision 1a: "the members data is NOT imported this batch" --
        // proven by feeding a record that ALSO carries partOf (a real
        // shape some Theographic tribe records have) and confirming the
        // parsed PeopleGroup carries only id/label, nothing else to leak.
        let json = r#"[{"id": "recX", "fields": {"groupName": "Tribe of Gad", "members": ["recA","recB"], "partOf": ["recC"], "events_dev": ["recD"]}}]"#;
        let (groups, _) = parse_people_groups(json).unwrap();
        assert_eq!(groups, vec![atlas_core::data::PeopleGroup { id: "tribe-of-gad".into(), label: "Tribe of Gad".into() }]);
    }

    #[test]
    fn real_committed_data_yields_exactly_23_groups_with_collision_free_slugs() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw/theographic/theographic-bible-metadata-master/json/peopleGroups.json");
        let json = std::fs::read_to_string(&path).expect("data/raw/theographic/.../peopleGroups.json must exist");
        let (groups, stats) = parse_people_groups(&json).expect("the real committed file must parse");
        assert_eq!(stats.total, 23, "the real committed peopleGroups.json must carry exactly 23 records (PG-1a batch report's own count)");
        assert_eq!(groups.len(), 23);
        let mut ids: Vec<&str> = groups.iter().map(|g| g.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 23, "every derived slug must be collision-free over the real committed group names");
    }
}
