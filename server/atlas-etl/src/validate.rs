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
//!
//! Batch HOTFIX-2 fix-round-1 (`run_place_merges`, review findings I-1/I-3):
//! same separate-entry-point pattern as `run_place_history`/`run_landmarks`
//! above — `atlas_core::merge::MERGE_PAIRS` is curated Rust data, not a
//! `data/curated/*.toml` file, but it deserves the exact same fail-loud
//! treatment: every `survivor`/`absorbed` id must exist in the real,
//! pre-merge compiled place set, and every pair's REAL distance must be
//! within `atlas_core::merge::SAME_PLACE_THRESHOLD_KM`. See this function's
//! own doc comment below for why both checks matter and what they replace.

use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};
use atlas_core::data::{AtlasData, BookNarrationWindow, CatechismPart, ChronologyAnchor, Era, Event, Landmark, LandMaskRegion, Place, PlaceHistory, PlaceNameAlias, Polity};
use atlas_core::event_merge::{
    cross_book_duplicate_candidate, is_layer0, title_jaccard, verse_jaccard, EventDistinct, EventMerge,
    DUPLICATE_JACCARD_THRESHOLD, TITLE_JACCARD_THRESHOLD,
};
use atlas_core::history::strip_disambiguation_suffix;
use atlas_core::merge::{great_circle_km, PlaceMerge, SAME_PLACE_THRESHOLD_KM};
use atlas_core::refs::VerseId;
use atlas_core::time::{next_year, TimeRange};
use atlas_core::translation::DEFAULT_TRANSLATION;

use crate::polities::{ring_is_simple, Bbox};

const ATLAS_START_YEAR: i32 = -4004;
const ATLAS_END_YEAR: i32 = 100;

/// The curated `kind` values `landmarks.toml` may use (design-direction.md's
/// Atlas plate detail: water names styled italic/lapis, mountain/region
/// names styled letterspaced small caps).
const ALLOWED_LANDMARK_KINDS: [&str; 3] = ["water", "mountain", "region"];

/// Batch T requirement 1, real data since Batch T2: the curated
/// `Event::kind` values -- "event" (real date/place) or "general"
/// (dateless/placeless titled container -- see `atlas_core::data::
/// Event::kind`'s own doc comment for the shape each implies, and
/// `atlas_etl::curated::parse_events_extra` for the fabrication guard
/// that keeps a general-kind row from ever carrying a curator-typed
/// date/place).
const ALLOWED_EVENT_KINDS: [&str; 2] = ["event", "general"];

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

    // Batch T requirement 1: pure verse-format check for hand-typed witness
    // refs -- both parses canonically AND actually exists in the compiled
    // KJV text (the STRICTER bar `run_place_history`/`run_polities` already
    // apply to every other hand-typed curated citation in this app, since a
    // typo there must fail loudly rather than silently pass a format-only
    // check -- unlike `e.verses` below, whose OWN Theographic-imported
    // majority is ETL-derived, not hand-typed, so that check stays
    // format-only, its own long-standing behavior, unchanged by this batch).
    let check_witness_verse = |v: &str, ctx: &str, errors: &mut Vec<String>| match VerseId::parse_canonical(v) {
        Err(err) => errors.push(format!("{ctx}: verse '{v}' is not a canonical single-verse ref: {err}")),
        Ok(_) if !data.verses.contains_key(v) => {
            errors.push(format!("{ctx}: verse '{v}' parses but does not exist in the compiled KJV text"))
        }
        Ok(_) => {}
    };

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

        // Batch T requirement 1 ("date outside span"): every event's own
        // `when` must fall inside the atlas's own curated span -- previously
        // unchecked for events specifically (unlike eras/polities/place-
        // history, which each already enforce this bound; `TimeRange::new`
        // itself only ever rejects zero/inverted, never an out-of-span year).
        if e.when.from_year < ATLAS_START_YEAR || e.when.from_year > ATLAS_END_YEAR {
            errors.push(format!(
                "event '{}': from_year {} is outside [{ATLAS_START_YEAR},{ATLAS_END_YEAR}]",
                e.id, e.when.from_year
            ));
        }
        if e.when.to_year < ATLAS_START_YEAR || e.when.to_year > ATLAS_END_YEAR {
            errors.push(format!("event '{}': to_year {} is outside [{ATLAS_START_YEAR},{ATLAS_END_YEAR}]", e.id, e.when.to_year));
        }

        // Batch T requirement 1: `kind` enum (see `ALLOWED_EVENT_KINDS`'s
        // own doc comment for why every real record is "event" today).
        if !ALLOWED_EVENT_KINDS.contains(&e.kind.as_str()) {
            errors.push(format!("event '{}' has invalid kind '{}' (expected one of {:?})", e.id, e.kind, ALLOWED_EVENT_KINDS));
        }

        // Batch T requirement 1: PARALLEL WITNESSES -- every witness's own
        // book must be a real canon code, every witness must cite >=1 verse
        // (an authored-but-empty witness is a curator mistake, not a valid
        // "no evidence" state -- an event that genuinely has no parallel
        // account simply carries no `[[witness]]` row at all, relying on
        // the single-implicit-witness fallback, `scene::witnesses_for`'s
        // own doc comment), and every cited verse both parses AND exists.
        // Overlapping witness ranges: within ONE event, two witnesses of
        // the SAME book whose own verse sets intersect are a curator
        // mistake (a real account is split across two witness rows, or a
        // copy-paste duplicate) -- checked pairwise per (event, book) group.
        let mut by_book: HashMap<&str, Vec<&atlas_core::data::EventWitness>> = HashMap::new();
        for w in &e.witnesses {
            let ctx = format!("event '{}' witness ({})", e.id, w.book);
            if atlas_core::canon::resolve_alias(&w.book).is_none() {
                errors.push(format!("{ctx}: '{}' is not a real canonical book code", w.book));
            }
            match atlas_core::translation::resolve(&w.translations, atlas_core::translation::DEFAULT_TRANSLATION) {
                Ok(verses) if verses.is_empty() => {
                    errors.push(format!("{ctx}: has zero verses -- every witness must cite at least one"))
                }
                Ok(verses) => {
                    for v in verses {
                        check_witness_verse(v, &ctx, &mut errors);
                    }
                }
                Err(_) => errors.push(format!(
                    "{ctx}: carries no '{}' translation entry -- every curated witness must",
                    atlas_core::translation::DEFAULT_TRANSLATION
                )),
            }
            by_book.entry(w.book.as_str()).or_default().push(w);
        }
        for (book, group) in &by_book {
            for a in 0..group.len() {
                for b in (a + 1)..group.len() {
                    let va: HashSet<&str> = group[a]
                        .translations
                        .get(atlas_core::translation::DEFAULT_TRANSLATION)
                        .map(|v| v.iter().map(String::as_str).collect())
                        .unwrap_or_default();
                    let vb: HashSet<&str> = group[b]
                        .translations
                        .get(atlas_core::translation::DEFAULT_TRANSLATION)
                        .map(|v| v.iter().map(String::as_str).collect())
                        .unwrap_or_default();
                    if !va.is_disjoint(&vb) {
                        errors.push(format!(
                            "event '{}': two witnesses for book '{}' have overlapping verse ranges",
                            e.id, book
                        ));
                    }
                }
            }
        }

        // Batch T2 fix-round-1 (review finding I-1): the exact invariant
        // whose violation caused the real 72-event silent-heading-drop bug
        // (data-only patched by commit 9679583). `heading_anchors_for`
        // (atlas-core/src/data.rs) uses ONLY `e.witnesses` once ANY witness
        // row exists, never falling back to `e.verses` -- so a top-level
        // book with no matching witness row silently loses its own reader
        // heading and PARALLEL ACCOUNTS entry, even though its verse
        // content still shows up everywhere else. Deliberately NOT fixed by
        // adding a fallback there (the owner's own ruling: a silent
        // fallback would mask the curation gap instead of surfacing it,
        // against this file's own house fail-loud pattern) -- this is the
        // permanent guard instead. Only applies once `e.witnesses` is
        // non-empty (an event relying purely on the single-implicit-witness
        // fallback has nothing to check against here, and stays legal,
        // unchanged); an event with no top-level `verses` at all likewise
        // has nothing to check. Resolves both sides through the SAME
        // `canon::resolve_alias` the unknown-book-code check above already
        // uses, so an aliased witness book (e.g. an OSIS code) still counts
        // -- an already-invalid book code gets its own separate error above
        // and simply contributes nothing here.
        if !e.witnesses.is_empty() {
            let witness_books: HashSet<&str> =
                e.witnesses.iter().filter_map(|w| atlas_core::canon::resolve_alias(&w.book)).map(|b| b.code()).collect();
            let mut missing_books: Vec<&str> = Vec::new();
            for v in &e.verses {
                if let Ok(vid) = VerseId::parse_canonical(v) {
                    let code = vid.book.code();
                    if !witness_books.contains(code) && !missing_books.contains(&code) {
                        missing_books.push(code);
                    }
                }
            }
            for book in missing_books {
                errors.push(format!(
                    "event '{}': top-level verses touch book '{book}', but no witness row covers '{book}' -- once ANY witness exists, only witness rows anchor a reader heading (heading_anchors_for), so '{book}' would silently lose its own heading and PARALLEL ACCOUNTS entry; add an explicit witness row for '{book}' (the exact bug class fixed in commit 9679583)",
                    e.id
                ));
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
        // Batch T requirement 2: same check, extended with `order_key` as an
        // explicit SUB-YEAR tiebreak (never a fake year offset, per the
        // brief's own instruction) -- `(from_year, order_key)` must be
        // non-decreasing along the leg chain, not just `from_year` alone.
        // Every pre-Batch-T event defaults `order_key` to 0, so this is a
        // strict extension: any narrative that already passed the
        // year-only check keeps passing (0 >= 0, always) UNLESS this batch
        // (or a future curator) explicitly assigns order_keys that
        // contradict the curated leg order -- exactly the class of mistake
        // this check exists to catch (Passion Week's own 8-9 legs, all
        // dated to the identical traditional year, had NO ordering
        // enforcement at all before this field existed).
        for pair in resolved.windows(2) {
            let a_key = (pair[0].when.from_year, pair[0].order_key);
            let b_key = (pair[1].when.from_year, pair[1].order_key);
            if b_key < a_key {
                errors.push(format!(
                    "narrative '{}' has non-chronological legs: '{}' (from_year={}, order_key={}) precedes '{}' (from_year={}, order_key={})",
                    n.id, pair[0].id, pair[0].when.from_year, pair[0].order_key, pair[1].id, pair[1].when.from_year, pair[1].order_key
                ));
            }
        }
    }

    // Batch T2 (owner's own ruling: "Robertson sections within one Gospel
    // should partition, not collide with each other -- a within-layer
    // anchor collision is a curation error your validation must catch").
    // `AtlasData::finish()` already derives every such collision in ONE
    // pass alongside `verse_heading` itself (see `heading_anchor_
    // collisions`'s own doc comment, `atlas-core/src/data.rs`) -- this is
    // purely the fail-loud reporting half, same split every other
    // AtlasData-derived check in this function follows (era gaps/overlaps
    // via `check_eras`, chronological legs above).
    for (anchor, a, b) in data.heading_anchor_collisions() {
        errors.push(format!(
            "verse '{anchor}' is anchored by two real curated containers, '{a}' and '{b}' -- curated sections (Robertson or otherwise) must partition, never share an anchor verse (within-layer anchor collision)"
        ));
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
pub fn run_polities(polities: &[Polity], bbox: &Bbox, verses: &HashMap<String, String>) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    check_duplicate_ids(polities.iter().map(|p| p.id.as_str()), "polity", &mut errors);

    // Batch M requirement 1: same verse-check shape `run_place_history`
    // already established (parses canonically AND actually exists in the
    // compiled KJV text -- every delta verse is hand-typed, so a typo must
    // fail loud, not silently pass a format-only check).
    let check_verse = |v: &str, ctx: &str, errors: &mut Vec<String>| match VerseId::parse_canonical(v) {
        Err(err) => errors.push(format!("{ctx}: verse '{v}' is not a canonical single-verse ref: {err}")),
        Ok(_) if !verses.contains_key(v) => {
            errors.push(format!("{ctx}: verse '{v}' parses but does not exist in the compiled KJV text"))
        }
        Ok(_) => {}
    };
    // Fix round 1 (I1): `era_from` is the era ACTUALLY hosting this delta,
    // per TOML's own attachment (whatever `[[era]]` block it's nested
    // under) -- cross-checked against the delta's own curator-authored
    // `for_era_from` echo. This is the structural safety net the original
    // mis-attachment bug had none of: TOML's array-of-tables rule attaches
    // a nested `[era.transition]`/`[era.fall]` table to whichever `[[era]]`
    // was MOST RECENTLY OPENED, not whichever era a curator's own
    // surrounding comment describes -- a mismatch here means the block
    // landed on the wrong era, exactly the failure mode that shipped once
    // (7 of 22 deltas, this batch's own self-review) and had nothing
    // catching it before this field/check existed.
    let check_delta = |delta: &atlas_core::data::PolityDelta, era_from: i32, ctx: &str, errors: &mut Vec<String>| {
        if delta.event.trim().is_empty() {
            errors.push(format!("{ctx}: event is empty"));
        }
        if delta.ref_note.trim().is_empty() {
            errors.push(format!("{ctx}: ref_note is empty (citation-integrity rule -- name the source actually consulted)"));
        }
        for v in &delta.verses {
            check_verse(v, ctx, errors);
        }
        if delta.for_era_from != era_from {
            errors.push(format!(
                "{ctx}: for_era_from={} does not match this era's own from={era_from} -- TOML's array-of-tables rule likely attached this block to the WRONG era (it attaches to whichever [[era]] was MOST RECENTLY OPENED, not whichever era a surrounding comment describes); move the block to sit immediately after the [[era]] whose own from is {}",
                delta.for_era_from, delta.for_era_from
            ));
        }
    };

    for p in polities {
        if p.eras.is_empty() {
            errors.push(format!("polity '{}' has no eras", p.id));
            continue;
        }

        // Batch M requirement 1: `fall` is only meaningful on the polity's
        // own chronologically FINAL era -- determined by the era with the
        // greatest `to` year, never assumed from file/array order (a
        // curator could, in principle, list eras out of order; the overlap
        // check just below doesn't depend on file order either). A `fall`
        // authored on any OTHER era is a curator mistake (it describes an
        // end that isn't actually this polity's end), caught here rather
        // than silently accepted.
        if let Some(final_era) = p.eras.iter().max_by_key(|e| e.to) {
            let final_key = (final_era.from, final_era.to);
            let final_name = final_era.name.clone();
            for era in &p.eras {
                if era.fall.is_some() && (era.from, era.to) != final_key {
                    errors.push(format!(
                        "polity '{}' era '{}' ({}..{}) carries [era.fall] but is not this polity's chronologically final era ('{final_name}', ending {})",
                        p.id, era.name, era.from, era.to, final_key.1
                    ));
                }
            }
        }

        for era in &p.eras {
            let delta_ctx = format!("polity '{}' era '{}' ({}..{})", p.id, era.name, era.from, era.to);
            if let Some(t) = &era.transition {
                check_delta(t, era.from, &format!("{delta_ctx} [era.transition]"), &mut errors);
            }
            if let Some(f) = &era.fall {
                check_delta(f, era.from, &format!("{delta_ctx} [era.fall]"), &mut errors);
            }
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

/// Batch E3 (KJV display-name alias layer): validates the curated
/// `place-names-kjv.toml` (`atlas_core::data::PlaceNameAlias`, parsed by
/// `curated::parse_place_names_kjv`) against the compiled place set and KJV
/// text -- same aggregate-every-error-then-bail shape `run_place_history`
/// above uses. `places` (the FULL compiled place list, not just a
/// `HashSet<&str>` of ids like `run_place_history` takes) is needed here
/// specifically because the noise check below needs each alias's OWN
/// place's canonical name, not just its existence.
///
/// Checks (requirement 1's own named validation list): every alias id
/// resolves to a real compiled place; no duplicate alias ids; an alias
/// equal to its own place's canonical name is rejected as noise (compared
/// against the SAME stripped form `resolve_display_name`'s own fallback
/// produces -- `strip_disambiguation_suffix`, reused directly rather than
/// re-derived, so "Cush" is correctly caught as noise for `cush-2` even
/// though its raw compiled name is "Cush 2"); every cited verse parses as a
/// canonical single-verse ref AND exists in the compiled KJV text (same
/// two-part check `run_place_history`'s own `check_verse` closure applies);
/// a curated row missing its own "kjv" translation entry (defensive --
/// `parse_place_names_kjv` always populates it today, but this function
/// stays honest for a future translation-keyed row shape that might not).
pub fn run_place_names_kjv(aliases: &[PlaceNameAlias], places: &[Place], verses: &HashMap<String, String>) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    check_duplicate_ids(aliases.iter().map(|a| a.id.as_str()), "place-names-kjv alias", &mut errors);

    let places_by_id: HashMap<&str, &Place> = places.iter().map(|p| (p.id.as_str(), p)).collect();

    for a in aliases {
        let ctx = format!("place-names-kjv alias '{}'", a.id);
        let Some(place) = places_by_id.get(a.id.as_str()) else {
            errors.push(format!("{ctx}: unknown place id (not in compiled places.json)"));
            continue;
        };

        match a.translations.get(DEFAULT_TRANSLATION) {
            None => errors.push(format!("{ctx}: missing a '{DEFAULT_TRANSLATION}' translation entry")),
            Some(kjv_name) => {
                let canonical = strip_disambiguation_suffix(&place.name);
                if kjv_name == canonical {
                    errors.push(format!("{ctx}: kjv name '{kjv_name}' is equal to '{}' own canonical name -- noise, not a genuine mismatch", a.id));
                }
            }
        }

        for v in &a.verses {
            match VerseId::parse_canonical(v) {
                Err(err) => errors.push(format!("{ctx}: verse '{v}' is not a canonical single-verse ref: {err}")),
                Ok(_) if !verses.contains_key(v) => {
                    errors.push(format!("{ctx}: verse '{v}' parses but does not exist in the compiled KJV text"))
                }
                Ok(_) => {}
            }
        }
    }

    if errors.is_empty() {
        return Ok(());
    }
    let joined = errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("place-names-kjv validation failed with {} error(s):\n{}", errors.len(), joined);
}

/// Batch F ("the small catechism"), extended Batch F2 (question-level
/// citations): validates the curated catechism (`data/curated/
/// catechism.toml`, MERGED with the brain-fuel/catechism mapping and the
/// Deut5 supplement by the time this runs -- see `main.rs`'s own Batch F2
/// section; parsed separately by `curated::parse_catechism` -- same
/// distinct-entry-point pattern as `run_place_history`/`run_landmarks`:
/// this needs the compiled KJV text, which the pure parse step doesn't
/// own). `verses` is the compiled KJV text map, same stricter-than-`run`'s-
/// own-check bar `run_place_history` already applies to hand-typed verse
/// refs (must both parse canonically AND exist). Checks, all aggregated
/// (never fail-fast) same as every other check in this file: duplicate
/// part ids, duplicate item ids (GLOBAL across every part, since item
/// lookup by id -- `AtlasData::catechism_item_by_id` --  is itself global,
/// not scoped to a part), every part has >=1 item, every item's `verses`
/// both parse canonically and exist in the compiled KJV text, and (Batch
/// F2) every one of every item's own `questions[].verses` too -- the exact
/// same per-verse bar, so a bad ref from either citation source (Luther's
/// own embedded citations OR the repo mapping/Deut5 supplement) fails the
/// SAME way, loudly, never silently dropped.
pub fn run_catechism(parts: &[CatechismPart], verses: &HashMap<String, String>) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    check_duplicate_ids(parts.iter().map(|p| p.id.as_str()), "catechism part", &mut errors);
    check_duplicate_ids(parts.iter().flat_map(|p| p.items.iter()).map(|i| i.id.as_str()), "catechism item", &mut errors);

    let check_verse = |v: &str, ctx: &str, errors: &mut Vec<String>| match VerseId::parse_canonical(v) {
        Err(err) => errors.push(format!("{ctx}: verse '{v}' is not a canonical single-verse ref: {err}")),
        Ok(_) if !verses.contains_key(v) => {
            errors.push(format!("{ctx}: verse '{v}' parses but does not exist in the compiled KJV text"))
        }
        Ok(_) => {}
    };

    for part in parts {
        if part.items.is_empty() {
            errors.push(format!("catechism part '{}' has no items", part.id));
        }
        for item in &part.items {
            let ctx = format!("catechism item '{}' ({})", item.id, item.name);
            for v in &item.verses {
                check_verse(v, &ctx, &mut errors);
            }
            for q in &item.questions {
                let qctx = format!("{ctx} question '{}' (source: {})", q.title, q.source);
                if q.verses.is_empty() {
                    errors.push(format!("{qctx}: has zero verses -- every question must cite at least one"));
                }
                for v in &q.verses {
                    check_verse(v, &qctx, &mut errors);
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

/// Batch HOTFIX-2 fix-round-1 (review findings I-1, I-3): validates
/// `atlas_core::merge::MERGE_PAIRS` — the small, hand-curated table of
/// same-place merges `AtlasData::finish()` applies via
/// `atlas_core::merge::apply_place_merges` — against `places`, the REAL
/// pre-merge compiled place set (`main.rs` calls this with `all_places`,
/// BEFORE `AtlasData::new(...).finish()` ever runs the merge). Two checks,
/// both aggregated, never fail-fast, same policy as every other `run_*` in
/// this file:
///
/// - **I-1 (fail loud on a bad table entry)**: every `survivor`/`absorbed`
///   id must actually exist in `places`. `apply_place_merges` itself treats
///   a missing id as a silent no-op — it has to, since that same function
///   also has to tolerate its own idempotent second call (after the first
///   already dropped `absorbed`), and cannot tell "already merged" apart
///   from "never existed" from inside that function. Called here, BEFORE any
///   merge has ever applied, there is no such ambiguity: a missing id at
///   this point is unambiguously a curation mistake (a typo, or an id that
///   existed when the pair was added but was later renamed/removed
///   upstream), so it hard-fails the ETL build — mirroring
///   `run_place_history`'s own "unknown place id" treatment of curated ids.
/// - **I-3 (a safety net that actually fires)**: every pair's REAL distance
///   (`great_circle_km`, using the ACTUAL coordinates from `places` — not a
///   hand-copied snapshot, unlike `merge.rs`'s own
///   `every_curated_pair_is_within_the_same_place_threshold` unit test) must
///   be within `SAME_PLACE_THRESHOLD_KM`. This is the same bound
///   `apply_place_merges`'s own `debug_assert!` checks, but that check
///   compiles to nothing in a release build, and nothing in the test suite
///   ever calls it against live data (every test builds an in-memory
///   fixture, never `AtlasData::load()`). This check runs in EVERY build
///   profile (`bail!`, not `debug_assert!`) against the real
///   `data/compiled/places.json` contents on every single ETL run, so
///   upstream data drift that pushes a curated pair's real coordinates past
///   the threshold is caught at build time, not silently shipped.
pub fn run_place_merges(pairs: &[PlaceMerge], places: &[Place]) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();
    let by_id: HashMap<&str, &Place> = places.iter().map(|p| (p.id.as_str(), p)).collect();

    for pair in pairs {
        let survivor = by_id.get(pair.survivor);
        let absorbed = by_id.get(pair.absorbed);
        if survivor.is_none() {
            errors.push(format!(
                "merge pair (survivor '{}', absorbed '{}'): survivor id '{}' does not exist in the compiled place set",
                pair.survivor, pair.absorbed, pair.survivor
            ));
        }
        if absorbed.is_none() {
            errors.push(format!(
                "merge pair (survivor '{}', absorbed '{}'): absorbed id '{}' does not exist in the compiled place set",
                pair.survivor, pair.absorbed, pair.absorbed
            ));
        }
        if let (Some(s), Some(a)) = (survivor, absorbed) {
            let d = great_circle_km(s.lat, s.lon, a.lat, a.lon);
            if d > SAME_PLACE_THRESHOLD_KM {
                errors.push(format!(
                    "merge pair (survivor '{}', absorbed '{}') is {d:.3}km apart, over the {SAME_PLACE_THRESHOLD_KM}km same-place threshold",
                    pair.survivor, pair.absorbed
                ));
            }
        }
    }

    if errors.is_empty() {
        return Ok(());
    }
    let joined = errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("place-merge validation failed with {} error(s):\n{}", errors.len(), joined);
}

/// Batch HOTFIX-4 (duplicate-identity rectification, `atlas_core::event_merge`'s
/// own module doc comment has the full root-cause chain): runs on the RAW,
/// PRE-`AtlasData::finish()` event set (called from `main.rs`, same call
/// position/timing as `run_place_merges` above -- BEFORE
/// `atlas_core::event_merge::apply_event_merges` ever runs, so the
/// duplicates this sweep is looking for are still actually there to find;
/// running it post-merge would trivially always pass). Two checks:
///
/// 1. Every `EVENT_MERGE_PAIRS`/`EVENT_DISTINCT_PAIRS` entry names two real
///    ids in the compiled event set (a stale/typo'd curated entry is a
///    curation error, same class as `run_place_merges`'s own dangling-id
///    check above).
/// 2. THE FAIL-LOUD SWEEP ITSELF: every (LAYER-0, LAYER-1) event pair in
///    the WHOLE compiled set at verse-set jaccard >= `DUPLICATE_JACCARD_THRESHOLD`
///    must be accounted for -- either merged (`EVENT_MERGE_PAIRS`) or
///    explicitly documented as genuinely distinct despite the overlap
///    (`EVENT_DISTINCT_PAIRS`). An unlisted pair fails loud, naming both
///    ids, both labels, and the jaccard score, so a future curator adding a
///    new event that happens to duplicate an existing one is caught
///    immediately, not silently shipped as a second "straight up lie" the
///    way `theo-267`/`jm_jordan` was.
pub fn run_event_merges(merge_pairs: &[EventMerge], distinct_pairs: &[EventDistinct], events: &[Event]) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();
    let by_id: HashMap<&str, &Event> = events.iter().map(|e| (e.id.as_str(), e)).collect();

    for pair in merge_pairs {
        if !by_id.contains_key(pair.survivor) {
            errors.push(format!(
                "event-merge pair (survivor '{}', absorbed '{}'): survivor id does not exist in the compiled event set",
                pair.survivor, pair.absorbed
            ));
        }
        if !by_id.contains_key(pair.absorbed) {
            errors.push(format!(
                "event-merge pair (survivor '{}', absorbed '{}'): absorbed id does not exist in the compiled event set",
                pair.survivor, pair.absorbed
            ));
        }
    }
    for pair in distinct_pairs {
        if !by_id.contains_key(pair.a) {
            errors.push(format!("event-distinct pair ('{}', '{}'): id '{}' does not exist in the compiled event set", pair.a, pair.b, pair.a));
        }
        if !by_id.contains_key(pair.b) {
            errors.push(format!("event-distinct pair ('{}', '{}'): id '{}' does not exist in the compiled event set", pair.a, pair.b, pair.b));
        }
    }

    // The sweep: every listed (merge OR distinct) pair, keyed BOTH
    // directions -- a curator lists a merge pair as (survivor, absorbed)
    // and a distinct pair in whatever order reads naturally, so the lookup
    // below must not care which side is which.
    let mut listed: HashSet<(&str, &str)> = HashSet::new();
    for pair in merge_pairs {
        listed.insert((pair.survivor, pair.absorbed));
        listed.insert((pair.absorbed, pair.survivor));
    }
    for pair in distinct_pairs {
        listed.insert((pair.a, pair.b));
        listed.insert((pair.b, pair.a));
    }

    let layer0: Vec<&Event> = events.iter().filter(|e| is_layer0(e)).collect();
    let layer1: Vec<&Event> = events.iter().filter(|e| !is_layer0(e)).collect();
    let mut unlisted: Vec<String> = Vec::new();
    for a in &layer0 {
        for b in &layer1 {
            let j = verse_jaccard(a, b);
            if j < DUPLICATE_JACCARD_THRESHOLD {
                continue;
            }
            if listed.contains(&(a.id.as_str(), b.id.as_str())) {
                continue;
            }
            unlisted.push(format!(
                "'{}' ({:?}) <-> '{}' ({:?}): jaccard {:.3} >= {DUPLICATE_JACCARD_THRESHOLD} and NEITHER merged (EVENT_MERGE_PAIRS) nor explicitly documented as distinct (EVENT_DISTINCT_PAIRS)",
                a.id, a.label, b.id, b.label, j
            ));
        }
    }
    unlisted.sort();
    errors.extend(unlisted);

    if errors.is_empty() {
        return Ok(());
    }
    let joined = errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("event-merge validation failed with {} error(s):\n{}", errors.len(), joined);
}

/// Batch W4 fix round 1 (batch-w4-review.md Critical-1's own SYSTEMIC
/// GUARD): the SECOND duplicate-identity sweep. `atlas_core::event_merge`'s
/// own doc comment above `cross_book_duplicate_candidate` has the full "why
/// a second detector" rationale -- `verse_jaccard`/`run_event_merges` above
/// are structurally blind to a same-occurrence pair that cites disjoint
/// verse sets (different books, or a small subset fully contained in a much
/// larger sibling). Same call timing as `run_event_merges` (pre-merge event
/// set, called from `main.rs` right after it) and the SAME two-part shape:
///
/// 1. Dangling-id checking is NOT repeated here: this function is handed the
///    identical `merge_pairs`/`distinct_pairs`/`events` `run_event_merges`
///    already validates for stale/typo'd ids -- re-checking would only ever
///    duplicate that function's own errors verbatim.
/// 2. THE FAIL-LOUD SWEEP ITSELF: every DISTINCT pair of `kind == "event"`
///    entries in the whole compiled set that `cross_book_duplicate_candidate`
///    flags must be accounted for -- either merged (`EVENT_MERGE_PAIRS`) or
///    explicitly documented as genuinely distinct (`EVENT_DISTINCT_PAIRS`) --
///    checked against the SAME shared exemption tables `run_event_merges`
///    uses (that module's own updated doc comment: "this list is now
///    consulted by BOTH... sweeps"). An unlisted pair fails loud, naming
///    both ids, both labels, and the title-jaccard score, so a future
///    curator authoring a fresh dated event that happens to duplicate an
///    existing one's real-world occurrence is caught immediately -- the
///    exact gap that let `jer_the_fall_of_jerusalem_retold`/`exl_jerusalem`
///    and `jer_jeremiah_stays_with_gedaliah`+`jer_the_assassination_of_gedaliah`
///    /`exl_mizpah` ship live undetected in the first place.
///
/// O(n^2) over every `kind == "event"` entry (not LAYER-0-vs-LAYER-1 like
/// `run_event_merges` -- this shape can strike two LAYER-1 events, e.g.
/// `exl_mizpah`'s own `atlas_section` provenance against
/// `jer_jeremiah_stays_with_gedaliah`'s own curated-but-not-`kjv_superscription`
/// row), but `cross_book_duplicate_candidate`'s own cheap early checks
/// (kind, year-overlap, place-overlap) reject the overwhelming majority of
/// pairs before the title-jaccard word-set computation ever runs, and the
/// compiled dated-event count (~900) keeps this sweep well under a second
/// in practice.
pub fn run_cross_book_duplicates(merge_pairs: &[EventMerge], distinct_pairs: &[EventDistinct], events: &[Event]) -> Result<()> {
    let mut listed: HashSet<(&str, &str)> = HashSet::new();
    for pair in merge_pairs {
        listed.insert((pair.survivor, pair.absorbed));
        listed.insert((pair.absorbed, pair.survivor));
    }
    for pair in distinct_pairs {
        listed.insert((pair.a, pair.b));
        listed.insert((pair.b, pair.a));
    }

    let dated: Vec<&Event> = events.iter().filter(|e| e.kind == "event").collect();
    let mut unlisted: Vec<String> = Vec::new();
    for (i, a) in dated.iter().enumerate() {
        for b in dated[i + 1..].iter() {
            if !cross_book_duplicate_candidate(a, b) {
                continue;
            }
            if listed.contains(&(a.id.as_str(), b.id.as_str())) {
                continue;
            }
            let j = title_jaccard(&a.label, &b.label);
            unlisted.push(format!(
                "'{}' ({:?}) <-> '{}' ({:?}): title jaccard {:.3} >= {TITLE_JACCARD_THRESHOLD}, same-year-range and sharing a place, and NEITHER merged (EVENT_MERGE_PAIRS) nor explicitly documented as distinct (EVENT_DISTINCT_PAIRS)",
                a.id, a.label, b.id, b.label, j
            ));
        }
    }
    unlisted.sort();

    if unlisted.is_empty() {
        return Ok(());
    }
    let joined = unlisted.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("cross-book event-duplicate validation failed with {} error(s):\n{}", unlisted.len(), joined);
}

/// Batch HOTFIX-6 (graph-wide chronology audit): `data/curated/
/// chronology-anchors.toml`'s own structural validity -- every `event_id`
/// (when present) must name a real compiled event, and every `era_boundary`
/// row MUST carry one (the E4 property test needs a real timeline position
/// to gate on; an unbound era-boundary row is a curation error, not a
/// legal "disclosed adjacency" gap the way an ordinary unbound anchor is).
/// Runs against `data.events`, the FINAL (post-merge, post-calibration,
/// post-override) compiled set -- same post-`finish()` orientation as
/// `run` above, since an anchor binding an id a merge/override step could
/// still touch should be checked against what actually ships.
pub fn run_chronology_anchors(anchors: &[ChronologyAnchor], events: &[Event]) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();
    let by_id: HashSet<&str> = events.iter().map(|e| e.id.as_str()).collect();

    check_duplicate_ids(anchors.iter().map(|a| a.id.as_str()), "chronology anchor", &mut errors);

    for a in anchors {
        if let Some(eid) = &a.event_id {
            if !by_id.contains(eid.as_str()) {
                errors.push(format!("chronology anchor '{}': event_id '{eid}' does not exist in the compiled event set", a.id));
            }
        }
        if a.era_boundary && a.event_id.is_none() {
            errors.push(format!(
                "chronology anchor '{}': era_boundary = true but event_id is unset -- an era-boundary anchor must bind to a real event (the E4 property test needs a timeline position); if this anchor carries a disclosed-adjacency tension, use the primary-row/structural-row split (see chronology-anchors.toml's own header)",
                a.id
            ));
        }
    }

    if errors.is_empty() {
        return Ok(());
    }
    let joined = errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("chronology-anchor validation failed with {} error(s):\n{}", errors.len(), joined);
}

/// Batch HOTFIX-6: the ERA-WINDOW VALIDATOR itself -- the permanent,
/// fail-loud guard the owner's own "i'm sure these errors are graph-wide"
/// assertion earns. `atlas_core::chronology`'s own module doc comment has
/// the full root-cause/design story; this is purely the "turn a finding
/// into an ETL failure" half, same split every other validator in this file
/// follows.
///
/// Two independent failure modes, both real curation errors, both reported
/// together (never one masking the other):
/// 1. `missing_windows`: a book some dated event actually cites (net of
///    recounting citations) has no curated `BookNarrationWindow` row at
///    all -- a curation gap, not silently un-checked.
/// 2. `window_violations`: a dated event's own year falls outside a witness
///    book's own curated window, and is not individually exempted
///    (`chronology::WINDOW_EXEMPTIONS`) -- the df_ramah-class bug this
///    validator exists to catch.
pub fn run_chronology_windows(events: &[Event], windows: &[BookNarrationWindow]) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    for book in atlas_core::chronology::missing_windows(events, windows) {
        errors.push(format!("book '{book}' is cited by >=1 dated event (net of recounting citations) but has no BookNarrationWindow row in book-narration-windows.toml"));
    }

    for v in atlas_core::chronology::window_violations(events, windows) {
        errors.push(format!(
            "event '{}' ({:?}): year {}..{} falls outside '{}''s own narration window {}..{} -- FIX the date (with anchor-table justification) or add a WINDOW_EXEMPTIONS row with a stated reason",
            v.event_id, v.label, v.year.0, v.year.1, v.book, v.window.0, v.window.1
        ));
    }

    if errors.is_empty() {
        return Ok(());
    }
    errors.sort();
    let joined = errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("chronology era-window validation failed with {} error(s):\n{}", errors.len(), joined);
}

/// Batch HOTFIX-6: the E4-shaped structural guard, generalizing HOTFIX-4's
/// own single NT era-boundary gate (Passion cluster vs. every ACT-witnessed
/// event) to EVERY `era_boundary` anchor in the curated table. Needs the
/// full `AtlasData` (unlike the two checks above) because it reasons about
/// GLOBAL TIMELINE POSITION, not raw years -- `atlas_core::chronology::
/// era_boundary_violations`'s own doc comment has the full design.
pub fn run_era_boundaries(data: &AtlasData) -> Result<()> {
    let violations = atlas_core::chronology::era_boundary_violations(data);
    if violations.is_empty() {
        return Ok(());
    }
    let mut lines: Vec<String> = violations
        .iter()
        .map(|v| {
            format!(
                "event '{}' ({:?}) sorts on the WRONG side of era-boundary anchor '{}' (year {}) -- expected it to sort {} that boundary on the global timeline",
                v.event_id, v.label, v.boundary_id, v.boundary_year, v.side
            )
        })
        .collect();
    lines.sort();
    let joined = lines.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("chronology era-boundary validation failed with {} error(s):\n{}", violations.len(), joined);
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
