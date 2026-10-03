//! Hard validation of the fully-merged `AtlasData`. Every violation is collected into one error rather than
//! stopping at the first, so a curator sees the whole list in one run. A HAND-TYPED curated verse must both
//! parse and EXIST in the compiled text; an imported one is checked for format only, being ETL-derived.

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

/// Aggregates, so a curator sees every mistake at once. A word outside a closed
/// vocabulary is NOT among them: the curated file's own parse refuses it, and a parse
/// stops at the first one.
pub fn run(data: &AtlasData) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    check_duplicate_ids(data.places.iter().map(|p| p.id.as_str()), "place", &mut errors);
    check_duplicate_ids(data.events.iter().map(|e| e.id.as_str()), "event", &mut errors);

    let place_ids: HashSet<&str> = data.places.iter().map(|p| p.id.as_str()).collect();
    let event_by_id: HashMap<&str, &Event> = data.events.iter().map(|e| (e.id.as_str(), e)).collect();

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

        // Every event's own span must fall inside the atlas's: `TimeRange::new` only ever rejects a zero or
        // inverted range, never an out-of-span year.
        if e.when.from_year < ATLAS_START_YEAR || e.when.from_year > ATLAS_END_YEAR {
            errors.push(format!(
                "event '{}': from_year {} is outside [{ATLAS_START_YEAR},{ATLAS_END_YEAR}]",
                e.id, e.when.from_year
            ));
        }
        if e.when.to_year < ATLAS_START_YEAR || e.when.to_year > ATLAS_END_YEAR {
            errors.push(format!("event '{}': to_year {} is outside [{ATLAS_START_YEAR},{ATLAS_END_YEAR}]", e.id, e.when.to_year));
        }

        // Every witness must name a real canon book and cite at least one verse: an authored-but-empty witness is a
        // curator mistake, not a valid "no evidence" state -- an event with no parallel account carries no witness
        // row at all. Two witnesses of the SAME book whose verse sets intersect are likewise a mistake.
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

        // The heading anchors use ONLY the witness rows once any exist, never falling back to the top-level verses,
        // so a top-level book with no matching witness row silently loses its reader heading -- the real bug that
        // once dropped 72 events' headings. A fallback there would mask the curation gap, so this guard is the fix.
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
        // `(from_year, order_key)` must be non-decreasing along a leg chain, not just the year: `order_key` is
        // the explicit SUB-YEAR tiebreak, so a narrative whose legs all share one traditional year still has
        // its order enforced.
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

    // The collisions are already derived in one pass alongside the heading index: this is purely the fail-loud
    // reporting half.
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

/// Every coordinate must fall inside `bbox`: a landmark the map can never show is a curation bug, not a fact
/// worth silently keeping. The kind and the size need no check -- each is a closed vocabulary, refused by the
/// curated file's own parse.
pub fn run_landmarks(landmarks: &[Landmark], bbox: &Bbox) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    for l in landmarks {
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

/// Every era's years must be non-zero, non-inverted and inside the atlas span; no two eras of ONE polity may
/// intersect; every ring must be closed, with its first point repeated last, and simple; and every ring point
/// must fall inside `bbox`, so a transposed coordinate cannot silently draw somewhere this app never renders.
pub fn run_polities(polities: &[Polity], bbox: &Bbox, verses: &HashMap<String, String>) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    check_duplicate_ids(polities.iter().map(|p| p.id.as_str()), "polity", &mut errors);

    let check_verse = |v: &str, ctx: &str, errors: &mut Vec<String>| match VerseId::parse_canonical(v) {
        Err(err) => errors.push(format!("{ctx}: verse '{v}' is not a canonical single-verse ref: {err}")),
        Ok(_) if !verses.contains_key(v) => {
            errors.push(format!("{ctx}: verse '{v}' parses but does not exist in the compiled KJV text"))
        }
        Ok(_) => {}
    };
    // The hosting era is whichever one TOML attached this block to -- the most recently opened one, not
    // whichever era a curator's surrounding comment describes -- cross-checked against the curator's own echo
    // field. A mismatch means the block landed on the wrong era, the failure mode that once shipped live.
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

        // A `fall` is only meaningful on the polity's chronologically FINAL era, found by the greatest `to` year
        // rather than by file order, since a curator may list eras in any order. On any other era it describes
        // an end that is not this polity's end.
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

        // Only eras with a structurally sound range are collected for the overlap check: an intersection test
        // against a malformed range would be meaningless, and it already has its own error.
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

/// At least one region, every region at least one ring, every ring closed and simple by the same test the polity
/// rings use, and every point inside `bbox` for the same reason.
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

/// `place_ids` is the FULL compiled place-id set, so an unknown id is caught whether or not any event references
/// it, and `verses` is the compiled text, so a cited verse must both parse and exist.
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
            errors.push(format!("place-history '{}': unknown place id (not compiled -- no matching place)", h.id));
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

/// Takes the full compiled places rather than just their ids, because the noise check needs each alias's own
/// place's canonical name: an alias equal to that name is rejected, compared against the SAME stripped form the
/// display-name fallback produces, so a disambiguated name still catches its bare alias.
pub fn run_place_names_kjv(aliases: &[PlaceNameAlias], places: &[Place], verses: &HashMap<String, String>) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    // A place may legitimately carry more than one curated alias row -- several distinct verbatim wordings of one
    // idiom -- so a repeated id is not itself an error. The same (id, name) pair twice still is.
    {
        let mut seen: HashSet<(&str, &str)> = HashSet::new();
        let mut dupes: Vec<(&str, &str)> = Vec::new();
        for a in aliases {
            if let Some(kjv_name) = a.translations.get(DEFAULT_TRANSLATION) {
                let key = (a.id.as_str(), kjv_name.as_str());
                if !seen.insert(key) && !dupes.contains(&key) {
                    dupes.push(key);
                }
            }
        }
        for (id, name) in dupes {
            errors.push(format!("duplicate place-names-kjv alias: id '{id}' names '{name}' more than once"));
        }
    }

    let places_by_id: HashMap<&str, &Place> = places.iter().map(|p| (p.id.as_str(), p)).collect();

    for a in aliases {
        let ctx = format!("place-names-kjv alias '{}'", a.id);
        let Some(place) = places_by_id.get(a.id.as_str()) else {
            errors.push(format!("{ctx}: unknown place id (not compiled -- no matching place)"));
            continue;
        };

        match a.translations.get(DEFAULT_TRANSLATION) {
            None => errors.push(format!("{ctx}: missing a '{DEFAULT_TRANSLATION}' translation entry")),
            Some(kjv_name) => {
                let canonical = strip_disambiguation_suffix(&place.name);
                if kjv_name == canonical {
                    errors.push(format!("{ctx}: kjv name '{kjv_name}' is equal to '{}' own canonical name -- noise, not a genuine mismatch", a.id));
                }

                // Every curated alias must be a case-sensitive verbatim substring of the text of EVERY verse listed
                // for it: a wording appearing in no listed verse is an authoring mistake. Dashes are normalized
                // first, because this text typesets compound names with an en dash while aliases use a hyphen.
                let normalized_name = normalize_dashes(kjv_name);
                for v in &a.verses {
                    // An unresolvable verse is already reported by the parse loop below, not duplicated here.
                    if let Some(text) = verses.get(v) {
                        let normalized_text = normalize_dashes(text);
                        if !normalized_text.contains(&normalized_name) {
                            errors.push(format!(
                                "{ctx}: kjv name '{kjv_name}' is not a verbatim substring of verse '{v}''s own KJV text: \"{text}\""
                            ));
                        }
                    }
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

/// Normalizes en and em dashes to a plain ASCII hyphen, for the verbatim-substring comparison.
fn normalize_dashes(s: &str) -> String {
    s.replace('\u{2013}', "-").replace('\u{2014}', "-")
}

/// Duplicate item ids are checked GLOBALLY across every part, because item lookup by id is itself global rather
/// than scoped to a part. Every verse of an item, and of every one of its questions, must both parse and exist,
/// so a bad ref from either citation source fails the same way.
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

/// Validates the curated same-place merge table against the REAL pre-merge place set, before any merge has applied:
/// the merge itself must treat a missing id as a no-op to stay idempotent, but here a missing id is unambiguously a
/// curation mistake. Every pair's REAL distance is checked too, in every build profile and against live coordinates.
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

/// Runs on the RAW, pre-merge event set, where the duplicates being looked for still exist to be found; running
/// it after the merge would trivially pass. Every table entry must name two real ids, and every pair above the
/// duplicate threshold must be accounted for -- merged, or explicitly documented as genuinely distinct.
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

    // Keyed in BOTH directions: a curator lists a merge pair as survivor-then-absorbed and a distinct pair in
    // whatever order reads naturally, so the lookup must not care which side is which.
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
    // The sweep also compares layer-0 against layer-0, the shape a verse-overlap or title-similarity pass alone
    // misses: with neither side carrying the richer provenance, there is no obvious survivor to pick. Same
    // threshold, same exemptions, same unordered keying.
    for i in 0..layer0.len() {
        for j_idx in (i + 1)..layer0.len() {
            let (a, b) = (layer0[i], layer0[j_idx]);
            let j = verse_jaccard(a, b);
            if j < DUPLICATE_JACCARD_THRESHOLD {
                continue;
            }
            if listed.contains(&(a.id.as_str(), b.id.as_str())) {
                continue;
            }
            unlisted.push(format!(
                "'{}' ({:?}) <-> '{}' ({:?}): jaccard {:.3} >= {DUPLICATE_JACCARD_THRESHOLD} (LAYER0-LAYER0) and NEITHER merged (EVENT_MERGE_PAIRS) nor explicitly documented as distinct (EVENT_DISTINCT_PAIRS)",
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

/// The SECOND duplicate-identity sweep: a verse-overlap detector is structurally blind to a same-occurrence pair
/// citing disjoint verse sets -- different books, or a small subset inside a much larger sibling -- so this one
/// flags by title similarity instead. Dangling ids are not re-checked here; the first sweep already did.
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

    let dated: Vec<&Event> = events.iter().filter(|e| e.kind == atlas_core::data::EventKind::Event).collect();
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

/// The law itself, on the event set a READER sees: post-merge, post-calibration and post-override, no two
/// surviving events at heavy verse overlap -- the same real-world episode -- may carry independent placements.
/// A pair listed as genuinely distinct is exempt by definition, since two placements is what distinct means.
pub fn run_no_two_opinions(distinct_pairs: &[EventDistinct], events: &[Event]) -> Result<()> {
    let exempt: HashSet<(&str, &str)> = distinct_pairs.iter().flat_map(|p| [(p.a, p.b), (p.b, p.a)]).collect();

    let dated: Vec<&Event> = events.iter().filter(|e| e.kind == atlas_core::data::EventKind::Event).collect();

    // book code -> the indices of every event touching it, so a pair is only ever compared once they share a
    // book.
    let mut by_book: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, e) in dated.iter().enumerate() {
        let mut seen_books: HashSet<&str> = HashSet::new();
        for v in e.verses.iter().chain(e.witnesses.iter().filter_map(|w| w.translations.get(DEFAULT_TRANSLATION)).flatten()) {
            if let Some(book) = v.split('.').next() {
                if seen_books.insert(book) {
                    by_book.entry(book).or_default().push(i);
                }
            }
        }
    }

    let mut candidate_pairs: HashSet<(usize, usize)> = HashSet::new();
    for indices in by_book.values() {
        for a in 0..indices.len() {
            for b in (a + 1)..indices.len() {
                let (i, j) = (indices[a], indices[b]);
                candidate_pairs.insert(if i < j { (i, j) } else { (j, i) });
            }
        }
    }

    let mut violations: Vec<String> = Vec::new();
    for (i, j) in candidate_pairs {
        let (a, b) = (dated[i], dated[j]);
        let j_score = verse_jaccard(a, b);
        if j_score < DUPLICATE_JACCARD_THRESHOLD {
            continue;
        }
        if exempt.contains(&(a.id.as_str(), b.id.as_str())) {
            continue;
        }
        if a.when.from_year == b.when.from_year && a.when.to_year == b.when.to_year && a.order_key == b.order_key {
            continue;
        }
        violations.push(format!(
            "'{}' ({:?}, placed {}..{} order_key {}) <-> '{}' ({:?}, placed {}..{} order_key {}): jaccard {j_score:.3} >= {DUPLICATE_JACCARD_THRESHOLD} but the two placements DISAGREE, and neither is documented as genuinely distinct (EVENT_DISTINCT_PAIRS)",
            a.id, a.label, a.when.from_year, a.when.to_year, a.order_key,
            b.id, b.label, b.when.from_year, b.when.to_year, b.order_key,
        ));
    }
    violations.sort();

    if violations.is_empty() {
        return Ok(());
    }
    let joined = violations.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("no-two-opinions validation failed with {} violation(s) (THE CHRONOLOGY AUTHORITY LAW):\n{}", violations.len(), joined);
}

/// Every anchor's `event_id`, where present, must name a real compiled event, and an `era_boundary` row MUST
/// carry one -- an unbound boundary row is a curation error, unlike an ordinary unbound anchor. Runs against the
/// FINAL event set, since an anchor binding an id a merge or override could still touch must be checked as shipped.
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

/// The fail-loud ETL-time twin of the anchor-equality property test, calling the SAME predicate so the two layers
/// cannot drift into two hand-written copies. A stale deferral -- one whose recorded value no longer matches its
/// event's date -- fails loud exactly like a real violation: a deferral is a time-bounded gap, not a licence to drift.
pub fn run_chronology_anchor_equality(anchors: &[ChronologyAnchor], events: &[Event]) -> Result<()> {
    let (violations, _deferred) = atlas_core::chronology::anchor_equality_check(anchors, events);
    if violations.is_empty() {
        return Ok(());
    }
    let mut lines: Vec<String> = violations
        .iter()
        .map(|v| {
            if v.is_stale_deferral {
                format!(
                    "anchor '{}': STALE DEFERRAL -- ANCHOR_DEFERRALS records shipped_value {}, but bound event '{}''s own from_year is now {} (re-dated without updating/removing the deferral entry?)",
                    v.anchor_id, v.table_year, v.event_id, v.event_year
                )
            } else {
                format!(
                    "anchor '{}': table year {} != bound event '{}''s own from_year {} -- fix the date (with anchor-table justification) or register a typed ANCHOR_DEFERRALS entry if this is genuinely time-bounded drift pending HOTFIX-7",
                    v.anchor_id, v.table_year, v.event_id, v.event_year
                )
            }
        })
        .collect();
    lines.sort();
    let joined = lines.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n");
    bail!("chronology anchor-equality validation failed with {} error(s):\n{}", violations.len(), joined);
}

/// The era-window guard, in two independent failure modes reported together: a book some dated event actually
/// cites having no curated narration window at all, and a dated event's year falling outside a witness book's
/// window without an individual exemption.
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

/// Needs the full `AtlasData`, unlike the checks above, because it reasons about GLOBAL TIMELINE POSITION rather
/// than raw years.
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

#[cfg(test)]
mod gaz1_r1_tests {
    use super::*;

    fn place(id: &str, name: &str) -> Place {
        Place { id: id.into(), name: name.into(), lat: 0.0, lon: 0.0, verse_links: vec![] }
    }

    fn alias(id: &str, name: &str, verses: &[&str]) -> PlaceNameAlias {
        PlaceNameAlias {
            id: id.into(),
            translations: HashMap::from([(DEFAULT_TRANSLATION.to_string(), name.to_string())]),
            verses: verses.iter().map(|v| v.to_string()).collect(),
        }
    }

    #[test]
    fn red_then_green_verbatim_substring_law() {
        let places = vec![place("lebo-hamath", "Lebo-hamath")];
        let verses = HashMap::from([(
            "NUM.34.8".to_string(),
            "From mount Hor ye shall point out your border unto the entrance of Hamath; and the goings forth of the border shall be to Zedad:".to_string(),
        )]);

        let wrong = vec![alias("lebo-hamath", "entering in of Hamath", &["NUM.34.8"])];
        let err = run_place_names_kjv(&wrong, &places, &verses).unwrap_err();
        assert!(err.to_string().contains("not a verbatim substring"), "{err}");

        let right = vec![alias("lebo-hamath", "entrance of Hamath", &["NUM.34.8"])];
        assert!(run_place_names_kjv(&right, &places, &verses).is_ok());
    }

    #[test]
    fn verbatim_substring_law_normalizes_en_dash_to_hyphen() {
        let places = vec![place("rock-of-escape", "Rock of Escape")];
        let verses = HashMap::from([("1SA.23.28".to_string(), "...therefore they called that place Sela\u{2013}hammahlekoth.".to_string())]);
        let aliases = vec![alias("rock-of-escape", "Sela-hammahlekoth", &["1SA.23.28"])];
        assert!(run_place_names_kjv(&aliases, &places, &verses).is_ok(), "a plain-hyphen alias must match an en-dash verse under dash normalization");
    }

    #[test]
    fn multiple_aliases_on_the_same_id_are_lawful_when_names_differ() {
        let places = vec![place("lebo-hamath", "Lebo-hamath")];
        let verses = HashMap::from([
            ("NUM.34.8".to_string(), "...unto the entrance of Hamath; and the goings forth...".to_string()),
            ("JOS.13.5".to_string(), "...unto the entering into Hamath.".to_string()),
        ]);
        let aliases = vec![
            alias("lebo-hamath", "entrance of Hamath", &["NUM.34.8"]),
            alias("lebo-hamath", "entering into Hamath", &["JOS.13.5"]),
        ];
        assert!(run_place_names_kjv(&aliases, &places, &verses).is_ok());
    }

    #[test]
    fn the_exact_same_id_and_name_repeated_is_still_a_duplicate() {
        let places = vec![place("lebo-hamath", "Lebo-hamath")];
        let verses = HashMap::from([("NUM.34.8".to_string(), "...unto the entrance of Hamath...".to_string())]);
        let aliases = vec![alias("lebo-hamath", "entrance of Hamath", &["NUM.34.8"]), alias("lebo-hamath", "entrance of Hamath", &["NUM.34.8"])];
        let err = run_place_names_kjv(&aliases, &places, &verses).unwrap_err();
        assert!(err.to_string().contains("duplicate"), "{err}");
    }
}
