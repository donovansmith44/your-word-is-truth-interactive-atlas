use crate::data::{year_index, PlaceBlurbEntry, PlaceHistory, PlaceNameAlias, PlaceNameEntry};
use crate::time::{TimeRange, Year};

/// `established.from_year` and `destroyed.to_year`: a curated range means the place
/// plausibly existed from as early as the one and still stood as late as the other. A
/// bound that is present widens (never narrows) to cover every curated name range.
pub fn resolve_existence(history: Option<&PlaceHistory>) -> (Option<Year>, Option<Year>) {
    let Some(h) = history else { return (None, None) };
    let from = h.established.as_ref().map(|c| {
        let earliest_name = h.names.iter().map(|n| n.when.from_year).min();
        earliest_name.map_or(c.when.from_year, |n| c.when.from_year.min(n))
    });
    let to = h.destroyed.as_ref().map(|c| {
        let latest_name = h.names.iter().map(|n| n.when.to_year).max();
        latest_name.map_or(c.when.to_year, |n| c.when.to_year.max(n))
    });
    (from, to)
}

/// True only when `window` falls entirely outside the bounds; an absent bound never
/// gates on its side. Gating hides the label and keeps the dot, so the client re-applies
/// this same both-ends-inclusive comparison at paint time.
pub fn existence_gates_label(existence_from: Option<Year>, existence_to: Option<Year>, window: TimeRange) -> bool {
    if let Some(from) = existence_from {
        if window.to_year < from {
            return true;
        }
    }
    if let Some(to) = existence_to {
        if window.from_year > to {
            return true;
        }
    }
    false
}

/// Floors toward the earlier (more BC) year on an even-length window, so a window always
/// has exactly one canonical midpoint to test range coverage against.
fn window_midpoint(w: TimeRange) -> Year {
    let mid_idx = (year_index(w.from_year) + year_index(w.to_year)).div_euclid(2);
    if mid_idx >= 0 {
        (mid_idx + 1) as Year
    } else {
        mid_idx as Year
    }
}

/// `candidates` must already intersect `window` -- callers filter first. Entries within
/// one candidate set never overlap (the ETL validates this), so at most one can cover
/// the midpoint and "latest `from_year`" and "latest `to_year`" always agree.
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

/// Precedence: a curated period name for `window`, else the curated translation alias,
/// else the stripped default. `window` is `None` when there is no span to resolve
/// against, which skips period resolution but still reaches the alias tier.
pub fn resolve_display_name(default_name: &str, history: Option<&PlaceHistory>, window: Option<TimeRange>, alias: Option<&PlaceNameAlias>) -> String {
    resolve_display_name_and_canonical(default_name, history, window, alias).0
}

/// The second value is `Some` only when an alias is the reason the returned name differs
/// from the canonical one -- there is nothing else for a provenance note to disclose.
pub fn resolve_display_name_and_canonical(
    default_name: &str,
    history: Option<&PlaceHistory>,
    window: Option<TimeRange>,
    alias: Option<&PlaceNameAlias>,
) -> (String, Option<String>) {
    let stripped_default = strip_disambiguation_suffix(default_name);
    if let (Some(h), Some(w)) = (history, window) {
        let intersecting: Vec<&PlaceNameEntry> = h.names.iter().filter(|n| n.when.intersects(&w)).collect();
        if let Some(i) = pick_by_window(&intersecting, w, |n| n.when) {
            return (intersecting[i].name.clone(), None);
        }
    }
    match alias.and_then(|a| crate::translation::resolve_name(&a.translations, crate::translation::DEFAULT_TRANSLATION).ok()) {
        // An alias record with no entry for the translation degrades to the plain
        // fallback rather than panicking.
        Some(kjv_name) => (kjv_name.to_string(), Some(stripped_default.to_string())),
        None => (stripped_default.to_string(), None),
    }
}

/// Strips the trailing " <digits>" the upstream geodata adds to keep same-named sites
/// apart. Applied only to a default name: curated period names are already clean. `pub`
/// because the ETL validator reuses this same rule instead of re-deriving it.
pub fn strip_disambiguation_suffix(name: &str) -> &str {
    match name.rsplit_once(' ') {
        Some((base, suffix)) if !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()) => base,
        _ => name,
    }
}

/// Exactly one blurb or none, never a stack. Every curated range is inclusive on both
/// ends, so a blurb whose text narrates a year must curate a range reaching that year.
/// A window spanning two or more era ranges prefers a broad entry over an era one.
pub fn resolve_blurb(blurbs: &[PlaceBlurbEntry], window: TimeRange) -> Option<&PlaceBlurbEntry> {
    let era: Vec<&PlaceBlurbEntry> = blurbs.iter().filter(|b| b.breadth == "era" && b.when.intersects(&window)).collect();
    let broad: Vec<&PlaceBlurbEntry> = blurbs.iter().filter(|b| b.breadth == "broad" && b.when.intersects(&window)).collect();

    if era.len() <= 1 {
        if let Some(e) = era.first() {
            return Some(e);
        }
        // A window in a gap between curated eras is still inside the place's whole
        // history, so a broad summary is a truer answer here than no blurb at all.
        return pick_by_window(&broad, window, |b| b.when).map(|i| broad[i]);
    }

    if let Some(i) = pick_by_window(&broad, window, |b| b.when) {
        return Some(broad[i]);
    }
    pick_by_window(&era, window, |b| b.when).map(|i| era[i])
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashMap;

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

    fn alias(kjv_name: &str) -> PlaceNameAlias {
        PlaceNameAlias { id: "x".into(), translations: HashMap::from([("kjv".to_string(), kjv_name.to_string())]), verses: vec![] }
    }

    #[test]
    fn no_history_falls_back_to_default() {
        assert_eq!(resolve_display_name("Bethel 1", None, Some(range(-2000, -1900)), None), "Bethel");
    }

    #[test]
    fn scripture_mode_no_history_uses_alias_when_curated() {
        let a = alias("Ethiopia");
        assert_eq!(resolve_display_name("Cush 2", None, None, Some(&a)), "Ethiopia");
    }

    #[test]
    fn time_mode_no_history_uses_alias_too() {
        let a = alias("Ethiopia");
        assert_eq!(resolve_display_name("Cush 2", None, Some(range(-4004, -3000)), Some(&a)), "Ethiopia");
    }

    #[test]
    fn no_alias_curated_still_falls_back_to_stripped_default() {
        assert_eq!(resolve_display_name("Cush 2", None, None, None), "Cush");
    }

    #[test]
    fn active_curated_period_name_wins_over_alias() {
        let h = history(vec![name("Luz", -4004, -2092)]);
        let a = alias("Some Alias");
        assert_eq!(resolve_display_name("Bethel", Some(&h), Some(range(-3000, -2500)), Some(&a)), "Luz");
    }

    #[test]
    fn alias_wins_when_history_exists_but_no_range_is_active() {
        let h = history(vec![name("Jebus", -4004, -1004)]);
        let a = alias("Some Alias");
        assert_eq!(resolve_display_name("Jerusalem", Some(&h), Some(range(-1000, -900)), Some(&a)), "Some Alias");
    }

    #[test]
    fn alias_record_without_a_kjv_entry_falls_back_to_default_not_panic() {
        let a = PlaceNameAlias { id: "x".into(), translations: HashMap::new(), verses: vec![] };
        assert_eq!(resolve_display_name("Cush 2", None, None, Some(&a)), "Cush");
    }

    #[test]
    fn trailing_numeral_stripped_from_default_name_with_no_history() {
        assert_eq!(resolve_display_name("Beersheba 2", None, Some(range(-2000, -1900)), None), "Beersheba");
        assert_eq!(resolve_display_name("Succoth 2", None, None, None), "Succoth");
    }

    #[test]
    fn trailing_numeral_stripped_only_when_curated_name_does_not_apply() {
        let h = history(vec![name("Luz", -4004, -2092)]);
        assert_eq!(resolve_display_name("Bethel 2", Some(&h), Some(range(-3000, -2500)), None), "Luz");
        assert_eq!(resolve_display_name("Bethel 2", Some(&h), Some(range(-1000, -900)), None), "Bethel");
    }

    #[test]
    fn multi_digit_and_no_suffix_cases() {
        assert_eq!(resolve_display_name("Aphek 12", None, None, None), "Aphek");
        assert_eq!(resolve_display_name("Jerusalem", None, None, None), "Jerusalem");
        assert_eq!(resolve_display_name("Antioch of Pisidia", None, None, None), "Antioch of Pisidia");
    }

    #[test]
    fn no_window_always_falls_back_to_default_even_with_history() {
        let h = history(vec![name("Luz", -4004, -2092)]);
        assert_eq!(resolve_display_name("Bethel", Some(&h), None, None), "Bethel");
    }

    #[test]
    fn window_fully_inside_one_name_range_uses_that_name() {
        let h = history(vec![name("Luz", -4004, -2092), name("Bethel", -2091, 100)]);
        assert_eq!(resolve_display_name("Bethel 1", Some(&h), Some(range(-3000, -2500)), None), "Luz");
        assert_eq!(resolve_display_name("Bethel 1", Some(&h), Some(range(-1930, -1930)), None), "Bethel");
    }

    #[test]
    fn window_outside_every_curated_range_falls_back_to_default() {
        let h = history(vec![name("Jebus", -4004, -1004)]);
        assert_eq!(resolve_display_name("Jerusalem", Some(&h), Some(range(-1000, -900)), None), "Jerusalem");
    }

    #[test]
    fn luz_bethel_boundary_years_pin_exactly() {
        let h = history(vec![name("Luz", -4004, -2092), name("Bethel", -2091, 100)]);
        assert_eq!(resolve_display_name("Bethel 1", Some(&h), Some(range(-2092, -2092)), None), "Luz");
        assert_eq!(resolve_display_name("Bethel 1", Some(&h), Some(range(-2091, -2091)), None), "Bethel");
    }

    #[test]
    fn window_spanning_both_ranges_picks_the_one_covering_the_midpoint() {
        let h = history(vec![name("Luz", -4004, -2092), name("Bethel", -2091, 100)]);
        let got = resolve_display_name("Bethel 1", Some(&h), Some(range(-2093, -2090)), None);
        assert!(got == "Luz" || got == "Bethel");
        assert_eq!(got, resolve_display_name("Bethel 1", Some(&h), Some(range(-2093, -2090)), None));
    }

    #[test]
    fn several_intersecting_falls_back_to_latest_when_none_covers_midpoint() {
        let h = history(vec![name("A", -300, -200), name("B", -50, 50)]);
        assert_eq!(resolve_display_name("Default", Some(&h), Some(range(-300, 50)), None), "B");
    }

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
        let got = resolve_blurb(&blurbs, range(-300, 50));
        assert!(got.is_some());
    }

    #[test]
    fn blurb_range_is_inclusive_on_both_ends() {
        let blurbs = vec![blurb("only", -200, -100, "era")];
        assert_eq!(resolve_blurb(&blurbs, range(-200, -200)).map(|b| b.text.as_str()), Some("only"));
        assert_eq!(resolve_blurb(&blurbs, range(-100, -100)).map(|b| b.text.as_str()), Some("only"));
        assert_eq!(resolve_blurb(&blurbs, range(-201, -201)), None);
        assert_eq!(resolve_blurb(&blurbs, range(-99, -99)), None);
    }

    #[test]
    fn zero_era_hits_falls_back_to_broad_by_design() {
        let blurbs = vec![
            blurb("first half", -4004, -800, "era"),
            blurb("second half", -400, 100, "era"),
            blurb("whole sweep", -4004, 100, "broad"),
        ];
        assert_eq!(resolve_blurb(&blurbs, range(-799, -401)).map(|b| b.text.as_str()), Some("whole sweep"));
    }

    #[test]
    fn zero_era_hits_and_no_broad_curated_returns_none() {
        let blurbs = vec![blurb("first half", -4004, -800, "era"), blurb("second half", -400, 100, "era")];
        assert_eq!(resolve_blurb(&blurbs, range(-799, -401)), None);
    }

    #[test]
    fn narrated_boundary_year_resolves_to_the_specific_blurb_not_broad() {
        let blurbs = vec![
            blurb("Once a stronghold, the city fell in 586 BC.", -4004, -586, "era"),
            blurb("Rebuilt after the exile.", -538, 100, "era"),
            blurb("The whole sweep, Canaanite era to today.", -4004, 100, "broad"),
        ];
        let destroyed_year = range(-586, -586);
        assert_eq!(resolve_blurb(&blurbs, destroyed_year).map(|b| b.text.as_str()), Some("Once a stronghold, the city fell in 586 BC."));
    }

    #[test]
    fn exactly_one_or_none_never_more() {
        let blurbs = vec![
            blurb("e1", -4004, -2167, "era"),
            blurb("e2", -586, -539, "era"),
            blurb("e3", -538, -536, "era"),
        ];
        for w in [range(-4004, 100), range(-600, -500), range(-4004, -2167), range(1, 50)] {
            let _ = resolve_blurb(&blurbs, w);
        }
    }

    use crate::data::PlaceDateClaim;

    fn claim(from: Year, to: Year) -> PlaceDateClaim {
        PlaceDateClaim { when: range(from, to), verses: vec![], note: None, event: None }
    }

    fn history_with_dates(established: Option<PlaceDateClaim>, destroyed: Option<PlaceDateClaim>) -> PlaceHistory {
        PlaceHistory { id: "x".into(), names: vec![], blurbs: vec![], established, destroyed }
    }

    #[test]
    fn resolve_existence_no_history_is_unbounded() {
        assert_eq!(resolve_existence(None), (None, None));
    }

    #[test]
    fn resolve_existence_history_with_neither_claim_is_unbounded() {
        let h = history_with_dates(None, None);
        assert_eq!(resolve_existence(Some(&h)), (None, None));
    }

    #[test]
    fn resolve_existence_reads_established_from_year_and_destroyed_to_year() {
        let h = history_with_dates(Some(claim(-1399, -1399)), Some(claim(-1104, -1050)));
        assert_eq!(resolve_existence(Some(&h)), (Some(-1399), Some(-1050)));
    }

    #[test]
    fn resolve_existence_established_only_is_open_ended_on_destroyed_side() {
        let h = history_with_dates(Some(claim(-2000, -2000)), None);
        assert_eq!(resolve_existence(Some(&h)), (Some(-2000), None));
    }

    #[test]
    fn resolve_existence_destroyed_only_is_open_ended_on_established_side() {
        let h = history_with_dates(None, Some(claim(-586, -586)));
        assert_eq!(resolve_existence(Some(&h)), (None, Some(-586)));
    }

    #[test]
    fn resolve_existence_established_is_widened_by_an_earlier_curated_name() {
        let mut h = history_with_dates(Some(claim(-1003, -1003)), Some(claim(-586, -586)));
        h.names = vec![name("Jebus", -4004, -1004)];
        assert_eq!(resolve_existence(Some(&h)), (Some(-4004), Some(-586)));
    }

    #[test]
    fn resolve_existence_never_widens_past_the_later_bound() {
        let mut h = history_with_dates(Some(claim(-1003, -1003)), Some(claim(-586, -586)));
        h.names = vec![name("Jebus", -4004, -1004)];
        let (_, to) = resolve_existence(Some(&h));
        assert_eq!(to, Some(-586));
    }

    #[test]
    fn resolve_existence_destroyed_is_widened_by_a_later_curated_name() {
        let mut h = history_with_dates(Some(claim(-1003, -1003)), Some(claim(-700, -600)));
        h.names = vec![name("Later Name", -600, -400)];
        assert_eq!(resolve_existence(Some(&h)), (Some(-1003), Some(-400)));
    }

    #[test]
    fn resolve_existence_names_alone_never_introduce_a_bound() {
        let mut h = history_with_dates(None, None);
        h.names = vec![name("Old Name", -4004, -1004)];
        assert_eq!(resolve_existence(Some(&h)), (None, None));
    }

    #[test]
    fn resolve_existence_with_no_names_is_unaffected_by_widening() {
        let h = history_with_dates(Some(claim(-1399, -1399)), Some(claim(-1104, -1050)));
        assert_eq!(resolve_existence(Some(&h)), (Some(-1399), Some(-1050)));
    }

    #[test]
    fn existence_gates_label_no_bounds_never_gates() {
        assert!(!existence_gates_label(None, None, range(1, 100)));
        assert!(!existence_gates_label(None, None, range(-4004, -4004)));
    }

    #[test]
    fn existence_gates_label_window_entirely_before_established() {
        assert!(existence_gates_label(Some(-1399), Some(-1050), range(-2000, -1400)));
        assert!(!existence_gates_label(Some(-1399), Some(-1050), range(-2000, -1399)));
    }

    #[test]
    fn existence_gates_label_window_entirely_after_destroyed() {
        assert!(existence_gates_label(Some(-1399), Some(-1050), range(-900, -800)));
        assert!(!existence_gates_label(Some(-1399), Some(-1050), range(-1050, -900)));
    }

    #[test]
    fn existence_gates_label_window_overlapping_existence_never_gates() {
        assert!(!existence_gates_label(Some(-1399), Some(-1050), range(-1399, -1050)));
        assert!(!existence_gates_label(Some(-1399), Some(-1050), range(-4004, 100)));
        assert!(!existence_gates_label(Some(-1399), Some(-1050), range(-1200, -1100)));
    }

    #[test]
    fn existence_gates_label_open_ended_bound_never_gates_on_that_side() {
        assert!(!existence_gates_label(Some(-2000), None, range(1, 100)));
        assert!(existence_gates_label(Some(-2000), None, range(-4004, -2001)));
        assert!(!existence_gates_label(None, Some(-586), range(-4004, -4004)));
        assert!(existence_gates_label(None, Some(-586), range(-500, -400)));
    }

    proptest! {
        #[test]
        fn existence_gating_agrees_with_time_range_intersects(
            est in -4004i32..=100, dest_delta in 0i32..500, w in window_strategy(),
        ) {
            let est = if est == 0 { 1 } else { est };
            let dest = (est + dest_delta).min(100);
            let dest = if dest == 0 { 1 } else { dest };
            prop_assume!(est <= dest);
            let existence = TimeRange::new(est, dest).unwrap();
            let gated = existence_gates_label(Some(est), Some(dest), w);
            prop_assert_eq!(gated, !existence.intersects(&w));
        }
    }

    fn window_strategy() -> impl Strategy<Value = TimeRange> {
        (-4004i32..=100, -4004i32..=100)
            .prop_filter("no zero", |(a, b)| *a != 0 && *b != 0)
            .prop_map(|(a, b)| TimeRange::new(a.min(b), a.max(b)).unwrap())
    }

    proptest! {
        #[test]
        fn name_resolution_is_deterministic_and_intersecting(w in window_strategy()) {
            let h = history(vec![
                name("Luz", -4004, -2092),
                name("Bethel", -2091, -1004),
                name("Jerusalem-ish", -1003, 100),
            ]);
            let a = resolve_display_name("Default", Some(&h), Some(w), None);
            let b = resolve_display_name("Default", Some(&h), Some(w), None);
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
