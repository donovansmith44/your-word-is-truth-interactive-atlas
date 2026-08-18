//! Hard validation of the fully-merged `AtlasData` before it's written to
//! `data/compiled/`. Collects every violation into one error (never stops
//! at the first) so a curator fixing `data/curated/` sees the whole list in
//! one ETL run.
//!
//! Checks required by the brief: unknown place ids referenced by events,
//! dangling narrative legs, non-canon verse ids (in `Event.verses` AND
//! `Place.verse_links` — both are the "etl-validated" trust class
//! `atlas_core::scene` relies on via `VerseId::parse_canonical(..).expect(..)`),
//! era gaps/overlaps/zero-years/coverage of `[-4004,100]`, duplicate event
//! ids, non-chronological narrative legs.
//!
//! Two extra checks beyond the brief's literal list, added to protect
//! invariants `atlas_core::scene::build_arrows` trusts without re-checking
//! (it indexes `event.places[0]` directly for every narrative leg it keeps):
//! duplicate place ids (mirrors the duplicate-event-id check) and a
//! narrative leg whose event has zero places (would panic scene
//! composition, not just misbehave).
//!
//! Cross-reference validity (ruling: every target parses, no self-refs,
//! first verse exists in the compiled KJV text) is enforced by
//! `xrefs::parse` + `xrefs::filter_missing_first_verse` instead — those
//! rows are dropped-and-counted during ETL, so by the time `run` sees
//! `AtlasData` they're already clean and don't need re-checking here.

use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};
use atlas_core::data::{AtlasData, Era, Event};
use atlas_core::refs::VerseId;
use atlas_core::time::next_year;

const ATLAS_START_YEAR: i32 = -4004;
const ATLAS_END_YEAR: i32 = 100;

pub fn run(data: &AtlasData) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    check_duplicate_ids(data.places.iter().map(|p| p.id.as_str()), "place", &mut errors);
    check_duplicate_ids(data.events.iter().map(|e| e.id.as_str()), "event", &mut errors);

    let place_ids: HashSet<&str> = data.places.iter().map(|p| p.id.as_str()).collect();
    let event_by_id: HashMap<&str, &Event> = data.events.iter().map(|e| (e.id.as_str(), e)).collect();

    for e in &data.events {
        for pid in &e.places {
            if !place_ids.contains(pid.as_str()) {
                errors.push(format!("event '{}' references unknown place id '{}'", e.id, pid));
            }
        }
        for v in &e.verses {
            if let Err(err) = VerseId::parse_canonical(v) {
                errors.push(format!("event '{}' has a non-canonical verse id '{}': {}", e.id, v, err));
            }
        }
    }

    for p in &data.places {
        for v in &p.verse_links {
            if let Err(err) = VerseId::parse_canonical(v) {
                errors.push(format!("place '{}' has a non-canonical verse id in verse_links '{}': {}", p.id, v, err));
            }
        }
    }

    for n in &data.narratives {
        let mut resolved: Vec<&Event> = Vec::new();
        for leg in &n.legs {
            match event_by_id.get(leg.as_str()) {
                None => errors.push(format!("narrative '{}' has a dangling leg: event id '{}' does not exist", n.id, leg)),
                Some(ev) if ev.places.is_empty() => errors.push(format!(
                    "narrative '{}' leg event '{}' has no places (required as the arrow anchor)",
                    n.id, leg
                )),
                Some(ev) => resolved.push(ev),
            }
        }
        for pair in resolved.windows(2) {
            if pair[1].when.from_year < pair[0].when.from_year {
                errors.push(format!(
                    "narrative '{}' has non-chronological legs: '{}' (from_year={}) precedes '{}' (from_year={})",
                    n.id, pair[0].id, pair[0].when.from_year, pair[1].id, pair[1].when.from_year
                ));
            }
        }
    }

    check_eras(&data.eras, &mut errors);

    if errors.is_empty() {
        return Ok(());
    }
    let joined = errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("validation failed with {} error(s):\n{}", errors.len(), joined);
}

fn check_duplicate_ids<'a>(ids: impl Iterator<Item = &'a str>, kind: &str, errors: &mut Vec<String>) {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut dupes: Vec<&str> = Vec::new();
    for id in ids {
        if !seen.insert(id) && !dupes.contains(&id) {
            dupes.push(id);
        }
    }
    for d in dupes {
        errors.push(format!("duplicate {kind} id '{d}'"));
    }
}

fn check_eras(eras: &[Era], errors: &mut Vec<String>) {
    for e in eras {
        if e.from_year == 0 || e.to_year == 0 {
            errors.push(format!(
                "era '{}' has zero year (from_year={}, to_year={}); year cannot be zero",
                e.id, e.from_year, e.to_year
            ));
        }
        if e.from_year > e.to_year {
            errors.push(format!("era '{}' has an inverted range (from_year={} > to_year={})", e.id, e.from_year, e.to_year));
        }
    }

    if eras.is_empty() {
        errors.push(format!("no eras defined; eras must cover [{ATLAS_START_YEAR},{ATLAS_END_YEAR}]"));
        return;
    }

    let mut sorted: Vec<&Era> = eras.iter().collect();
    sorted.sort_by_key(|e| e.from_year);

    let first = sorted[0];
    if first.from_year != ATLAS_START_YEAR {
        errors.push(format!(
            "era coverage: first era '{}' starts at {} but eras must cover [{ATLAS_START_YEAR},{ATLAS_END_YEAR}] (expected start {ATLAS_START_YEAR})",
            first.id, first.from_year
        ));
    }
    let last = sorted[sorted.len() - 1];
    if last.to_year != ATLAS_END_YEAR {
        errors.push(format!(
            "era coverage: last era '{}' ends at {} but eras must cover [{ATLAS_START_YEAR},{ATLAS_END_YEAR}] (expected end {ATLAS_END_YEAR})",
            last.id, last.to_year
        ));
    }

    for pair in sorted.windows(2) {
        let expected = next_year(pair[0].to_year);
        if pair[1].from_year != expected {
            let kind = if pair[1].from_year > expected { "gap" } else { "overlap" };
            errors.push(format!(
                "era {kind} between '{}' (ends {}) and '{}' (starts {}): expected '{}' to start at {}",
                pair[0].id, pair[0].to_year, pair[1].id, pair[1].from_year, pair[1].id, expected
            ));
        }
    }
}
