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
//!
//! Batch E (`run_place_history`, same separate-entry-point pattern as
//! `run_landmarks` — place histories are `#[serde(skip)]` "extra" data on
//! `AtlasData`, not part of the core eight fields `run` above validates):
//! unknown place id, a cited verse that doesn't parse as canonical OR
//! doesn't exist in the compiled KJV text (a STRICTER bar than
//! `run`'s own event-verse/verse_links check above, deliberate — every verse
//! here is hand-typed, and this batch's standing citation-integrity rule
//! means a typo'd verse ref must fail loudly, not silently pass a
//! format-only check), a year of zero or outside `[ATLAS_START_YEAR,
//! ATLAS_END_YEAR]` (zero/inverted are already impossible by the time this
//! runs — `TimeRange::new` enforces both at parse time, see
//! `curated::parse_place_history` — so only the atlas-span bound is left to
//! check here), overlapping name ranges within one place, and — within one
//! place's own blurbs — two ranges of the SAME breadth overlapping (a
//! "broad" range is expected to overlap every "era" range it summarizes, so
//! only same-breadth overlaps are an error). Duplicate place ids within
//! place-history.toml itself are an extra check beyond the brief's literal
//! list, same spirit as `run`'s own duplicate-place/duplicate-event checks.

use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};
use atlas_core::data::{AtlasData, CatechismPart, Era, Event, Landmark, LandMaskRegion, PlaceHistory, Polity};
use atlas_core::refs::VerseId;
use atlas_core::time::{next_year, TimeRange};

use crate::polities::{ring_is_simple, Bbox};

const ATLAS_START_YEAR: i32 = -4004;
const ATLAS_END_YEAR: i32 = 100;

/// The curated `kind` values `landmarks.toml` may use (design-direction.md's
/// Atlas plate detail: water names styled italic/lapis, mountain/region
/// names styled letterspaced small caps).
const ALLOWED_LANDMARK_KINDS: [&str; 3] = ["water", "mountain", "region"];

/// Batch C2: the curated `size` hint `landmarks.toml` may optionally set
/// (`None` is always valid too — only a PRESENT value is checked against
/// this enum). Mirrors BorderLayer's own polity-label size tiers
/// (map.js's `_sizeTier`); see `atlas_core::data::Landmark::size`'s own doc
/// comment for what each value does.
const ALLOWED_LANDMARK_SIZES: [&str; 3] = ["sm", "md", "lg"];

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

/// Validates curated landmarks (`data/curated/landmarks.toml`, parsed
/// separately by `curated::parse_landmarks` — see that function's doc
/// comment for why this is a distinct pipeline step): every `kind` must be
/// one of [`ALLOWED_LANDMARK_KINDS`], every PRESENT `size` must be one of
/// [`ALLOWED_LANDMARK_SIZES`] (absent is always fine), and every
/// `(lat, lon)` must fall inside `bbox` (a landmark the map is locked away
/// from ever showing is a curation bug, not a fact worth silently keeping).
/// Collects every violation before failing, same aggregate-don't-fail-fast
/// policy as [`run`].
pub fn run_landmarks(landmarks: &[Landmark], bbox: &Bbox) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    for l in landmarks {
        if !ALLOWED_LANDMARK_KINDS.contains(&l.kind.as_str()) {
            errors.push(format!(
                "landmark '{}' has invalid kind '{}' (expected one of {:?})",
                l.name, l.kind, ALLOWED_LANDMARK_KINDS
            ));
        }
        if let Some(size) = &l.size {
            if !ALLOWED_LANDMARK_SIZES.contains(&size.as_str()) {
                errors.push(format!(
                    "landmark '{}' has invalid size '{}' (expected one of {:?})",
                    l.name, size, ALLOWED_LANDMARK_SIZES
                ));
            }
        }
        if !bbox.contains(l.lat, l.lon) {
            errors.push(format!("landmark '{}' at (lat={}, lon={}) is outside the clip bbox", l.name, l.lat, l.lon));
        }
    }

    if errors.is_empty() {
        return Ok(());
    }
    let joined = errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("landmark validation failed with {} error(s):\n{}", errors.len(), joined);
}

/// Batch B2 ("borders v2"): validates curated polities
/// (`data/curated/polities/{id}.toml`, one `Polity` per file, parsed
/// separately by `curated::parse_polity` -- same distinct-entry-point
/// pattern as `run_landmarks`/`run_place_history` above: this needs the
/// clip bbox, which the pure parse step doesn't own). Checks required by
/// the batch brief, all aggregated (never fail-fast) same as every other
/// check in this file:
/// - duplicate polity ids across the curated set
/// - every era's `from`/`to`: non-zero, non-inverted, and inside
///   `[ATLAS_START_YEAR, ATLAS_END_YEAR]`
/// - within ONE polity, no two eras' `[from,to]` windows intersect (reuses
///   `TimeRange::intersects`, the exact "overlapping name/blurb ranges"
///   check `run_place_history` already applies to a place's own curated
///   ranges -- same shape of invariant, same reused check)
/// - every ring: closed (first point repeats as the last, >=4 points) AND
///   simple (`ring_is_simple` -- the reused Batch L segment-crossing test,
///   see `crate::polities`'s own module doc comment)
/// - every ring point falls inside `bbox` (same "a curator's typo shouldn't
///   silently draw off in the ocean somewhere this app never renders" reason
///   `run_landmarks` already checks `lat`/`lon` against it)
pub fn run_polities(polities: &[Polity], bbox: &Bbox) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    check_duplicate_ids(polities.iter().map(|p| p.id.as_str()), "polity", &mut errors);

    for p in polities {
        if p.eras.is_empty() {
            errors.push(format!("polity '{}' has no eras", p.id));
            continue;
        }

        // Only eras with a structurally sound (non-zero, non-inverted) range
        // are collected here for the cross-era overlap check below -- a
        // malformed era already gets its own specific error from the loop
        // just below and would otherwise make `TimeRange::intersects`
        // meaningless to ask about it.
        let mut sound_ranges: Vec<(usize, TimeRange)> = Vec::new();

        for (i, era) in p.eras.iter().enumerate() {
            let ctx = format!("polity '{}' era '{}' ({}..{})", p.id, era.name, era.from, era.to);

            if era.from == 0 || era.to == 0 {
                errors.push(format!("{ctx}: year cannot be zero"));
            } else if era.from > era.to {
                errors.push(format!("{ctx}: inverted range (from={} > to={})", era.from, era.to));
            } else {
                if era.from < ATLAS_START_YEAR || era.from > ATLAS_END_YEAR {
                    errors.push(format!("{ctx}: from {} is outside [{ATLAS_START_YEAR},{ATLAS_END_YEAR}]", era.from));
                }
                if era.to < ATLAS_START_YEAR || era.to > ATLAS_END_YEAR {
                    errors.push(format!("{ctx}: to {} is outside [{ATLAS_START_YEAR},{ATLAS_END_YEAR}]", era.to));
                }
                sound_ranges.push((i, TimeRange { from_year: era.from, to_year: era.to }));
            }

            if era.rings.is_empty() {
                errors.push(format!("{ctx}: has no rings"));
            }
            for (ri, ring) in era.rings.iter().enumerate() {
                let ring_ctx = format!("{ctx} ring {ri}");
                if ring.len() < 4 || ring.first() != ring.last() {
                    errors.push(format!(
                        "{ring_ctx}: not a closed ring ({} points; the first point must repeat as the last, >=4 points total)",
                        ring.len()
                    ));
                    continue; // a not-closed ring isn't a meaningful shape to run the simplicity/bbox checks against either
                }
                if !ring_is_simple(ring) {
                    errors.push(format!("{ring_ctx}: self-intersects (not a simple polygon)"));
                }
                for &(lat, lon) in ring {
                    if !bbox.contains(lat, lon) {
                        errors.push(format!("{ring_ctx}: point (lat={lat}, lon={lon}) is outside the clip bbox"));
                    }
                }
            }
        }

        for a in 0..sound_ranges.len() {
            for b in (a + 1)..sound_ranges.len() {
                let (ia, ra) = sound_ranges[a];
                let (ib, rb) = sound_ranges[b];
                if ra.intersects(&rb) {
                    errors.push(format!(
                        "polity '{}': era '{}' ({}..{}) overlaps era '{}' ({}..{})",
                        p.id, p.eras[ia].name, ra.from_year, ra.to_year, p.eras[ib].name, rb.from_year, rb.to_year
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        return Ok(());
    }
    let joined = errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("polity validation failed with {} error(s):\n{}", errors.len(), joined);
}

/// Batch R requirement 1 ("borders become part of the plate"): validates the
/// curated land mask (`data/curated/land-mask.toml`, parsed separately by
/// `curated::parse_land_mask` -- same distinct-entry-point pattern as
/// `run_polities`/`run_landmarks`: this needs the clip bbox, which the pure
/// parse step doesn't own). Checks, all aggregated (never fail-fast) same as
/// every other check in this file: at least one region, every region has at
/// least one ring, every ring closed (first point repeats as the last, >=4
/// points) AND simple (reuses `ring_is_simple`, the SAME segment-crossing
/// test `run_polities` already applies to polity rings -- "no fancy geometry
/// math anywhere," one algorithm for every hand-authored ring in this app),
/// and every ring point falls inside `bbox` (same "a curator's typo
/// shouldn't silently draw off in the ocean somewhere this app never
/// renders" reason `run_polities`/`run_landmarks` already check their own
/// coordinates against it).
pub fn run_land_mask(regions: &[LandMaskRegion], bbox: &Bbox) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    if regions.is_empty() {
        errors.push("land-mask.toml defines no regions".to_string());
    }

    for region in regions {
        if region.rings.is_empty() {
            errors.push(format!("land-mask region '{}' has no rings", region.name));
        }
        for (ri, ring) in region.rings.iter().enumerate() {
            let ring_ctx = format!("land-mask region '{}' ring {ri}", region.name);
            if ring.len() < 4 || ring.first() != ring.last() {
                errors.push(format!(
                    "{ring_ctx}: not a closed ring ({} points; the first point must repeat as the last, >=4 points total)",
                    ring.len()
                ));
                continue;
            }
            if !ring_is_simple(ring) {
                errors.push(format!("{ring_ctx}: self-intersects (not a simple polygon)"));
            }
            for &(lat, lon) in ring {
                if !bbox.contains(lat, lon) {
                    errors.push(format!("{ring_ctx}: point (lat={lat}, lon={lon}) is outside the clip bbox"));
                }
            }
        }
    }

    if errors.is_empty() {
        return Ok(());
    }
    let joined = errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("land-mask validation failed with {} error(s):\n{}", errors.len(), joined);
}

/// Batch E: validates curated place histories (`data/curated/place-history.toml`,
/// parsed separately by `curated::parse_place_history` — see that
/// function's doc comment for why this is a distinct pipeline step, same
/// reason `run_landmarks` above is separate from `run`). `place_ids` is the
/// FULL compiled place-id set (so an unknown id is caught regardless of
/// whether ANY event references it) and `verses` is the compiled KJV text
/// map (so a cited verse must both parse canonically and actually exist).
/// Same aggregate-don't-fail-fast policy as every other check in this file.
pub fn run_place_history(history: &[PlaceHistory], place_ids: &HashSet<&str>, verses: &HashMap<String, String>) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    check_duplicate_ids(history.iter().map(|h| h.id.as_str()), "place-history", &mut errors);

    let check_verse = |v: &str, ctx: &str, errors: &mut Vec<String>| match VerseId::parse_canonical(v) {
        Err(err) => errors.push(format!("{ctx}: verse '{v}' is not a canonical single-verse ref: {err}")),
        Ok(_) if !verses.contains_key(v) => {
            errors.push(format!("{ctx}: verse '{v}' parses but does not exist in the compiled KJV text"))
        }
        Ok(_) => {}
    };
    let check_bounds = |from_year: i32, to_year: i32, ctx: &str, errors: &mut Vec<String>| {
        if from_year < ATLAS_START_YEAR || from_year > ATLAS_END_YEAR {
            errors.push(format!("{ctx}: from_year {from_year} is outside [{ATLAS_START_YEAR},{ATLAS_END_YEAR}]"));
        }
        if to_year < ATLAS_START_YEAR || to_year > ATLAS_END_YEAR {
            errors.push(format!("{ctx}: to_year {to_year} is outside [{ATLAS_START_YEAR},{ATLAS_END_YEAR}]"));
        }
    };

    for h in history {
        if !place_ids.contains(h.id.as_str()) {
            errors.push(format!("place-history '{}': unknown place id (not in compiled places.json)", h.id));
        }

        let mut name_ranges: Vec<(&str, atlas_core::time::TimeRange)> = Vec::new();
        for n in &h.names {
            let ctx = format!("place-history '{}' name '{}'", h.id, n.name);
            check_bounds(n.when.from_year, n.when.to_year, &ctx, &mut errors);
            for v in &n.verses {
                check_verse(v, &ctx, &mut errors);
            }
            name_ranges.push((n.name.as_str(), n.when));
        }
        for i in 0..name_ranges.len() {
            for j in (i + 1)..name_ranges.len() {
                if name_ranges[i].1.intersects(&name_ranges[j].1) {
                    errors.push(format!(
                        "place-history '{}': name ranges '{}' and '{}' overlap",
                        h.id, name_ranges[i].0, name_ranges[j].0
                    ));
                }
            }
        }

        let mut blurb_ranges_by_breadth: HashMap<&str, Vec<atlas_core::time::TimeRange>> = HashMap::new();
        for b in &h.blurbs {
            let ctx = format!("place-history '{}' blurb ({})", h.id, b.breadth);
            check_bounds(b.when.from_year, b.when.to_year, &ctx, &mut errors);
            if b.breadth != "era" && b.breadth != "broad" {
                errors.push(format!("{ctx}: invalid breadth '{}' (expected 'era' or 'broad')", b.breadth));
            }
            blurb_ranges_by_breadth.entry(b.breadth.as_str()).or_default().push(b.when);
        }
        for (breadth, ranges) in &blurb_ranges_by_breadth {
            for i in 0..ranges.len() {
                for j in (i + 1)..ranges.len() {
                    if ranges[i].intersects(&ranges[j]) {
                        errors.push(format!("place-history '{}': two '{breadth}' blurb ranges overlap", h.id));
                    }
                }
            }
        }

        for (label, claim) in [("established", &h.established), ("destroyed", &h.destroyed)] {
            let Some(claim) = claim else { continue };
            let ctx = format!("place-history '{}' {label}", h.id);
            check_bounds(claim.when.from_year, claim.when.to_year, &ctx, &mut errors);
            for v in &claim.verses {
                check_verse(v, &ctx, &mut errors);
            }
        }
    }

    if errors.is_empty() {
        return Ok(());
    }
    let joined = errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("place-history validation failed with {} error(s):\n{}", errors.len(), joined);
}

/// Batch F ("the small catechism"): validates the curated catechism
/// (`data/curated/catechism.toml`, parsed separately by
/// `curated::parse_catechism` -- same distinct-entry-point pattern as
/// `run_place_history`/`run_landmarks`: this needs the compiled KJV text,
/// which the pure parse step doesn't own). `verses` is the compiled KJV text
/// map, same stricter-than-`run`'s-own-check bar `run_place_history` already
/// applies to hand-typed verse refs (must both parse canonically AND exist).
/// Checks, all aggregated (never fail-fast) same as every other check in
/// this file: duplicate part ids, duplicate item ids (GLOBAL across every
/// part, since item lookup by id -- `AtlasData::catechism_item_by_id` --  is
/// itself global, not scoped to a part), every part has >=1 item, and every
/// item's `verses` both parse canonically and exist in the compiled KJV
/// text.
pub fn run_catechism(parts: &[CatechismPart], verses: &HashMap<String, String>) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    check_duplicate_ids(parts.iter().map(|p| p.id.as_str()), "catechism part", &mut errors);
    check_duplicate_ids(parts.iter().flat_map(|p| p.items.iter()).map(|i| i.id.as_str()), "catechism item", &mut errors);

    for part in parts {
        if part.items.is_empty() {
            errors.push(format!("catechism part '{}' has no items", part.id));
        }
        for item in &part.items {
            let ctx = format!("catechism item '{}' ({})", item.id, item.name);
            for v in &item.verses {
                match VerseId::parse_canonical(v) {
                    Err(err) => errors.push(format!("{ctx}: verse '{v}' is not a canonical single-verse ref: {err}")),
                    Ok(_) if !verses.contains_key(v) => {
                        errors.push(format!("{ctx}: verse '{v}' parses but does not exist in the compiled KJV text"))
                    }
                    Ok(_) => {}
                }
            }
        }
    }

    if errors.is_empty() {
        return Ok(());
    }
    let joined = errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("catechism validation failed with {} error(s):\n{}", errors.len(), joined);
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
