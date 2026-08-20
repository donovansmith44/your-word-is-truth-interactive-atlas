//! Batch E (time-accurate places): resolves a curated `PlaceHistory` record
//! against a caller's window into the ONE display name / ONE blurb that
//! should show for it, per the brief's resolution rules (batch-e-brief.md
//! Requirement 3). Pure functions over `atlas_core::data` types -- no I/O,
//! no `AtlasData` dependency beyond what's passed in, so both
//! `scene::compose_time_scene` (per-scene-place display names) and
//! atlas-server's `/api/place/{id}` handler (one place's full history
//! payload) share exactly the same resolution logic.

use crate::data::{year_index, PlaceBlurbEntry, PlaceHistory, PlaceNameEntry};
use crate::time::{TimeRange, Year};

/// The zero-aware "midpoint year" of a window: floors toward the earlier
/// (more-BC) year when the window spans an even number of years, so there
/// is always exactly one canonical midpoint year to test range coverage
/// against (mirrors `AtlasData::nearest_border_year`'s own earlier-wins tie
/// policy, applied here to picking a single midpoint instead of a nearest
/// snapshot).
fn window_midpoint(w: TimeRange) -> Year {
    let mid_idx = (year_index(w.from_year) + year_index(w.to_year)).div_euclid(2);
    if mid_idx >= 0 {
        (mid_idx + 1) as Year
    } else {
        mid_idx as Year
    }
}

/// Shared by name and blurb resolution: among `candidates` that already all
/// intersect `window` (callers filter first), picks the one the window's
/// own rules prefer -- the entry covering the window's zero-aware midpoint
/// year if one does, else the entry with the latest `from_year` ("the
/// latest intersecting" per the brief). Entries within one candidate set
/// never overlap each other (ETL-validated -- see `validate::run_place_history`),
/// so "latest `from_year`" and "latest `to_year`" always agree and at most
/// one candidate can ever cover the midpoint.
fn pick_by_window<T>(candidates: &[&T], window: TimeRange, when: impl Fn(&T) -> TimeRange) -> Option<usize> {
    match candidates.len() {
        0 => None,
        1 => Some(0),
        _ => {
            let mid = window_midpoint(window);
            if let Some(i) = candidates.iter().position(|c| when(c).contains_year(mid)) {
                return Some(i);
            }
            candidates
                .iter()
                .enumerate()
                .max_by_key(|(_, c)| when(c).from_year)
                .map(|(i, _)| i)
        }
    }
}

/// NAME-1: resolves the period-true display name for `history` (if any)
/// over `window` (if any). `window` is `None` for scripture-mode scenes and
/// for a plain `/api/place/{id}` call with no `from`/`to` -- both cases
/// always return `default_name` unresolved, deliberately: scripture mode
/// lights a place via its (already name-appropriate, per the KJV text
/// itself) geocoded verse links, not a calendar window, so there is no
/// window to resolve a period name against, and resolving one anyway risks
/// showing a curated period name that contradicts the very verse text
/// displayed alongside it (see place-history.toml's own file-header comment
/// on proleptic naming).
pub fn resolve_display_name(default_name: &str, history: Option<&PlaceHistory>, window: Option<TimeRange>) -> String {
    let default_name = strip_disambiguation_suffix(default_name);
    let (Some(h), Some(w)) = (history, window) else {
        return default_name.to_string();
    };
    let intersecting: Vec<&PlaceNameEntry> = h.names.iter().filter(|n| n.when.intersects(&w)).collect();
    match pick_by_window(&intersecting, w, |n| n.when) {
        Some(i) => intersecting[i].name.clone(),
        None => default_name.to_string(),
    }
}

/// Batch E2 folded-in fix 1: strips a trailing " <digits>" ETL slug-
/// disambiguation suffix from a DEFAULT display name -- "Beersheba 2",
/// "Succoth 2", "Moab 2" were leaking onto the plate verbatim (313 of 1375
/// compiled places carry one, e.g. `beersheba-2`/name "Beersheba 2";
/// atlas-etl::geo's own doc comment: upstream `friendly_id` values already
/// disambiguate same-named real-world sites this way -- the suffix exists
/// to keep two places' NAMES apart in the *source* data, mirrored into our
/// own `-2`/`-3`... id suffixes, never meant for a user-facing label). Only
/// ever applied to the plain `default_name` a caller passes in -- CURATED
/// period names (`PlaceNameEntry::name`, hand-written) are already clean and
/// never run through this, so a curated name that happens to end in a
/// number (none do today, but nothing here would assume otherwise) is never
/// touched. Two same-named places may now show identical labels on the
/// plate after this strip -- that's correct cartography (their ids, e.g.
/// `beersheba-1`/`beersheba-2`, stay distinct; only the DISPLAYED text
/// converges).
fn strip_disambiguation_suffix(name: &str) -> &str {
    match name.rsplit_once(' ') {
        Some((base, suffix)) if !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()) => base,
        _ => name,
    }
}

/// BLURB-1: resolves the ONE blurb (if any) that should show for `blurbs`
/// over `window`. Both `TimeRange::intersects`/`contains_year` (and
/// therefore every curated range below) are inclusive on BOTH ends -- the
/// year model's general convention (see `time.rs`) -- so a range's own
/// `to` year is itself covered, not excluded; a blurb whose text narrates a
/// specific year must curate its range to REACH that year, not stop one
/// short of it (fix round 1, M1: Jerusalem's own -586 destruction blurb had
/// exactly this off-by-one -- see place-history.toml's own comment there
/// and batch-e-report.md's "Fix round 1" section for the full fencepost
/// sweep this bug prompted across every other curated range).
///
/// Era-breadth entries win whenever exactly one of them intersects
/// `window`; when `window` spans MORE than one era-breadth range (>=2
/// intersect), a broad-breadth entry is preferred instead (falling back to
/// the era set, same `pick_by_window` tie rule, if no broad entry
/// intersects either) -- "a broad period -> a broad blurb; don't stack
/// everything" (user direction, 2026-08-19). Exactly one blurb or none,
/// never a stack: every branch below returns at most one reference.
pub fn resolve_blurb(blurbs: &[PlaceBlurbEntry], window: TimeRange) -> Option<&PlaceBlurbEntry> {
    let era: Vec<&PlaceBlurbEntry> = blurbs.iter().filter(|b| b.breadth == "era" && b.when.intersects(&window)).collect();
    let broad: Vec<&PlaceBlurbEntry> = blurbs.iter().filter(|b| b.breadth == "broad" && b.when.intersects(&window)).collect();

    if era.len() <= 1 {
        if let Some(e) = era.first() {
            return Some(e);
        }
        // era.len() == 0: window touches NONE of this place's own
        // era-breadth ranges (a genuine gap between curated eras, or a
        // place with no era blurbs at all) -- deliberate design choice
        // (fix round 1, M1, documented here because the reviewer correctly
        // flagged this branch as unlabeled): fall back to a broad summary
        // if one intersects, rather than showing no blurb. A window inside
        // an unnarrated gap is still, truthfully, inside the place's whole
        // history, and the broad blurb is written to be true of the whole
        // span -- hiding it here would trade a correct, useful answer for
        // a blank one. CONTRACT.md's BLURB-1 note documents this case
        // explicitly (was previously silent on it, per the review).
        return pick_by_window(&broad, window, |b| b.when).map(|i| broad[i]);
    }

    // era.len() >= 2: window spans more than one of this place's own
    // era-breadth ranges -> prefer a broad summary.
    if let Some(i) = pick_by_window(&broad, window, |b| b.when) {
        return Some(broad[i]);
    }
    // No broad blurb curated/intersecting despite the multi-era span --
    // degrade gracefully to the same era pick a narrower window would have
    // used, rather than showing nothing at all.
    pick_by_window(&era, window, |b| b.when).map(|i| era[i])
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn range(from: Year, to: Year) -> TimeRange {
        TimeRange::new(from, to).unwrap()
    }

    fn name(n: &str, from: Year, to: Year) -> PlaceNameEntry {
        PlaceNameEntry { name: n.into(), when: range(from, to), verses: vec![] }
    }

    fn blurb(text: &str, from: Year, to: Year, breadth: &str) -> PlaceBlurbEntry {
        PlaceBlurbEntry { text: text.into(), when: range(from, to), breadth: breadth.into() }
    }

    fn history(names: Vec<PlaceNameEntry>) -> PlaceHistory {
        PlaceHistory { id: "x".into(), names, blurbs: vec![], established: None, destroyed: None }
    }

    // --- resolve_display_name ---------------------------------------------

    #[test]
    fn no_history_falls_back_to_default() {
        // Batch E2 folded-in fix 1: this pinned "Bethel 1" verbatim before
        // the fix (the exact leak the fix closes) -- now the fallback path
        // ALWAYS strips the disambiguation suffix first, so the observable
        // fallback is "Bethel", not the raw default. See this file's own
        // `trailing_numeral_stripped_*` tests below for the fix in isolation.
        assert_eq!(resolve_display_name("Bethel 1", None, Some(range(-2000, -1900))), "Bethel");
    }

    // --- Batch E2 folded-in fix 1: strip_disambiguation_suffix -------------

    #[test]
    fn trailing_numeral_stripped_from_default_name_with_no_history() {
        // Real shapes from the compiled data (data/compiled/places.json):
        // "Beersheba 2"/"Succoth 2" leaked onto the plate verbatim before
        // this fix -- both must now resolve to the clean, unsuffixed name.
        assert_eq!(resolve_display_name("Beersheba 2", None, Some(range(-2000, -1900))), "Beersheba");
        assert_eq!(resolve_display_name("Succoth 2", None, None), "Succoth");
    }

    #[test]
    fn trailing_numeral_stripped_only_when_curated_name_does_not_apply() {
        // A window inside the curated range still resolves to the curated
        // name unchanged; a window OUTSIDE every curated range falls back to
        // the stripped default -- the strip only ever touches the fallback
        // branch, never a curated `PlaceNameEntry::name`.
        let h = history(vec![name("Luz", -4004, -2092)]);
        assert_eq!(resolve_display_name("Bethel 2", Some(&h), Some(range(-3000, -2500))), "Luz");
        assert_eq!(resolve_display_name("Bethel 2", Some(&h), Some(range(-1000, -900))), "Bethel");
    }

    #[test]
    fn multi_digit_and_no_suffix_cases() {
        assert_eq!(resolve_display_name("Aphek 12", None, None), "Aphek"); // multi-digit suffix
        assert_eq!(resolve_display_name("Jerusalem", None, None), "Jerusalem"); // no suffix: untouched
        assert_eq!(resolve_display_name("Antioch of Pisidia", None, None), "Antioch of Pisidia"); // trailing word, not digits: untouched
    }

    #[test]
    fn no_window_always_falls_back_to_default_even_with_history() {
        let h = history(vec![name("Luz", -4004, -2092)]);
        assert_eq!(resolve_display_name("Bethel", Some(&h), None), "Bethel");
    }

    #[test]
    fn window_fully_inside_one_name_range_uses_that_name() {
        let h = history(vec![name("Luz", -4004, -2092), name("Bethel", -2091, 100)]);
        assert_eq!(resolve_display_name("Bethel 1", Some(&h), Some(range(-3000, -2500))), "Luz");
        assert_eq!(resolve_display_name("Bethel 1", Some(&h), Some(range(-1930, -1930))), "Bethel");
    }

    #[test]
    fn window_outside_every_curated_range_falls_back_to_default() {
        let h = history(vec![name("Jebus", -4004, -1004)]);
        assert_eq!(resolve_display_name("Jerusalem", Some(&h), Some(range(-1000, -900))), "Jerusalem");
    }

    #[test]
    fn luz_bethel_boundary_years_pin_exactly() {
        let h = history(vec![name("Luz", -4004, -2092), name("Bethel", -2091, 100)]);
        assert_eq!(resolve_display_name("Bethel 1", Some(&h), Some(range(-2092, -2092))), "Luz");
        assert_eq!(resolve_display_name("Bethel 1", Some(&h), Some(range(-2091, -2091))), "Bethel");
    }

    #[test]
    fn window_spanning_both_ranges_picks_the_one_covering_the_midpoint() {
        let h = history(vec![name("Luz", -4004, -2092), name("Bethel", -2091, 100)]);
        // Window [-2093,-2090]: zero-aware midpoint is -2092 (or -2091,
        // depending on rounding) -- either way exactly one of the two
        // curated ranges covers it, so the result must be deterministic and
        // equal to whichever one does.
        let got = resolve_display_name("Bethel 1", Some(&h), Some(range(-2093, -2090)));
        assert!(got == "Luz" || got == "Bethel");
        // Re-resolving the SAME window must always produce the SAME name
        // (API property: window -> name is a pure function).
        assert_eq!(got, resolve_display_name("Bethel 1", Some(&h), Some(range(-2093, -2090))));
    }

    #[test]
    fn several_intersecting_falls_back_to_latest_when_none_covers_midpoint() {
        // Three disjoint name ranges; a window spanning the first two only
        // (never reaching the third) with a midpoint that lands in the GAP
        // between them (no curated range covers it) must resolve to the
        // LATER (second) of the two it actually intersects.
        let h = history(vec![name("A", -300, -200), name("B", -50, 50)]);
        // Window [-300,50] intersects BOTH; the gap [-199,-51] contains no
        // curated range, but the true zero-aware midpoint of this window
        // (index range covers -300..49 inclusive -> mid index near -125)
        // falls inside that gap, so this must fall through to "latest
        // intersecting" = B.
        assert_eq!(resolve_display_name("Default", Some(&h), Some(range(-300, 50))), "B");
    }

    // --- resolve_blurb ------------------------------------------------------

    #[test]
    fn no_blurbs_returns_none() {
        assert_eq!(resolve_blurb(&[], range(-100, -50)), None);
    }

    #[test]
    fn single_intersecting_era_blurb_wins() {
        let blurbs = vec![blurb("early", -200, -100, "era")];
        assert_eq!(resolve_blurb(&blurbs, range(-150, -120)).map(|b| b.text.as_str()), Some("early"));
    }

    #[test]
    fn window_outside_every_blurb_returns_none() {
        let blurbs = vec![blurb("early", -200, -100, "era")];
        assert_eq!(resolve_blurb(&blurbs, range(1, 50)), None);
    }

    #[test]
    fn window_spanning_two_era_blurbs_prefers_broad() {
        let blurbs = vec![
            blurb("first half", -4004, -587, "era"),
            blurb("second half", -538, 100, "era"),
            blurb("whole sweep", -4004, 100, "broad"),
        ];
        // Spans both era ranges -> exactly the broad one shows.
        assert_eq!(resolve_blurb(&blurbs, range(-4004, 100)).map(|b| b.text.as_str()), Some("whole sweep"));
    }

    #[test]
    fn window_inside_one_era_range_ignores_the_broad_one() {
        let blurbs = vec![
            blurb("first half", -4004, -587, "era"),
            blurb("second half", -538, 100, "era"),
            blurb("whole sweep", -4004, 100, "broad"),
        ];
        assert_eq!(resolve_blurb(&blurbs, range(-1000, -900)).map(|b| b.text.as_str()), Some("first half"));
    }

    #[test]
    fn multi_era_span_with_no_broad_falls_back_to_an_era_pick() {
        let blurbs = vec![blurb("A", -300, -200, "era"), blurb("B", -50, 50, "era")];
        // No broad entry at all -- exactly one blurb (not none, not both).
        let got = resolve_blurb(&blurbs, range(-300, 50));
        assert!(got.is_some());
    }

    #[test]
    fn blurb_range_is_inclusive_on_both_ends() {
        // Fix round 1 (M1): pins the boundary semantics CONTRACT.md's
        // BLURB-1 note now documents explicitly -- a blurb's own `to` (and
        // `from`) year is itself covered, not excluded.
        let blurbs = vec![blurb("only", -200, -100, "era")];
        assert_eq!(resolve_blurb(&blurbs, range(-200, -200)).map(|b| b.text.as_str()), Some("only")); // lower edge
        assert_eq!(resolve_blurb(&blurbs, range(-100, -100)).map(|b| b.text.as_str()), Some("only")); // upper edge
        assert_eq!(resolve_blurb(&blurbs, range(-201, -201)), None); // one before: excluded
        assert_eq!(resolve_blurb(&blurbs, range(-99, -99)), None); // one after: excluded
    }

    #[test]
    fn zero_era_hits_falls_back_to_broad_by_design() {
        // Fix round 1 (M1): the branch the review flagged as unlabeled --
        // a window touching NONE of this place's era ranges but that a
        // broad range still intersects shows the broad summary, not "no
        // blurb" (see resolve_blurb's own doc comment for the reasoning).
        // Locks the decision in: changing this requires deliberately
        // updating this test, not an accidental behavior drift.
        let blurbs = vec![
            blurb("first half", -4004, -800, "era"),
            blurb("second half", -400, 100, "era"),
            blurb("whole sweep", -4004, 100, "broad"),
        ];
        // [-799,-401] touches NEITHER era range (a genuine gap) but the
        // broad range still covers it.
        assert_eq!(resolve_blurb(&blurbs, range(-799, -401)).map(|b| b.text.as_str()), Some("whole sweep"));
    }

    #[test]
    fn zero_era_hits_and_no_broad_curated_returns_none() {
        // Symmetric counterpart to the test above: a gap with no broad
        // entry at all curated for this place still correctly falls
        // through to "no blurb" (the brief's own "fallback default/none"
        // case), not a panic or a spurious pick.
        let blurbs = vec![blurb("first half", -4004, -800, "era"), blurb("second half", -400, 100, "era")];
        assert_eq!(resolve_blurb(&blurbs, range(-799, -401)), None);
    }

    #[test]
    fn narrated_boundary_year_resolves_to_the_specific_blurb_not_broad() {
        // Fix round 1 (M1): a direct, synthetic reproduction of the exact
        // shape of the shipped bug -- Jerusalem's own era-1 blurb text
        // named "586 BC" as its own closing event, but its curated range
        // stopped at -587, one year short, so the window reached via the
        // shipped "Destroyed 586 BC" -> "Show this time on the map" link
        // (from=-586&to=-586) fell into the zero-era-hits fallback and
        // showed the generic broad summary instead. This pins the FIXED
        // shape: a blurb's range reaching the exact year its own text
        // narrates resolves to that specific blurb, never broad.
        let blurbs = vec![
            blurb("Once a stronghold, the city fell in 586 BC.", -4004, -586, "era"),
            blurb("Rebuilt after the exile.", -538, 100, "era"),
            blurb("The whole sweep, Canaanite era to today.", -4004, 100, "broad"),
        ];
        let destroyed_year = range(-586, -586); // place.destroyed.when, mirrored
        assert_eq!(resolve_blurb(&blurbs, destroyed_year).map(|b| b.text.as_str()), Some("Once a stronghold, the city fell in 586 BC."));
    }

    #[test]
    fn exactly_one_or_none_never_more() {
        // BLURB-1 sanity: resolve_blurb's return type itself (Option<&T>)
        // makes "more than one" structurally impossible, but exercise a
        // busy, multi-breadth history to confirm no panic/ambiguity.
        let blurbs = vec![
            blurb("e1", -4004, -2167, "era"),
            blurb("e2", -586, -539, "era"),
            blurb("e3", -538, -536, "era"),
        ];
        for w in [range(-4004, 100), range(-600, -500), range(-4004, -2167), range(1, 50)] {
            let _ = resolve_blurb(&blurbs, w); // must not panic for any window shape
        }
    }

    // --- properties -----------------------------------------------------

    fn window_strategy() -> impl Strategy<Value = TimeRange> {
        (-4004i32..=100, -4004i32..=100)
            .prop_filter("no zero", |(a, b)| *a != 0 && *b != 0)
            .prop_map(|(a, b)| TimeRange::new(a.min(b), a.max(b)).unwrap())
    }

    proptest! {
        // API property (batch-e-brief.md Requirement 5): window -> name
        // resolution is deterministic (same window always yields the same
        // name) and, whenever it resolves to a curated name at all, that
        // name's own range truly intersects the window.
        #[test]
        fn name_resolution_is_deterministic_and_intersecting(w in window_strategy()) {
            let h = history(vec![
                name("Luz", -4004, -2092),
                name("Bethel", -2091, -1004),
                name("Jerusalem-ish", -1003, 100),
            ]);
            let a = resolve_display_name("Default", Some(&h), Some(w));
            let b = resolve_display_name("Default", Some(&h), Some(w));
            prop_assert_eq!(&a, &b);
            if a != "Default" {
                let entry = h.names.iter().find(|n| n.name == a).unwrap();
                prop_assert!(entry.when.intersects(&w));
            }
        }

        #[test]
        fn blurb_resolution_is_deterministic_and_intersecting(w in window_strategy()) {
            let blurbs = vec![
                blurb("e1", -4004, -587, "era"),
                blurb("e2", -538, 100, "era"),
                blurb("broad", -4004, 100, "broad"),
            ];
            let a = resolve_blurb(&blurbs, w).map(|b| b.text.clone());
            let b = resolve_blurb(&blurbs, w).map(|b| b.text.clone());
            prop_assert_eq!(&a, &b);
            if let Some(text) = a {
                let entry = blurbs.iter().find(|bl| bl.text == text).unwrap();
                prop_assert!(entry.when.intersects(&w));
            }
        }
    }
}
