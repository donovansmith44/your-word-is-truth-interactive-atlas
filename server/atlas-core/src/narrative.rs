//! Batch N ("narratives as first-class graph structure"): originally
//! resolved a verse's or an event's own position(s) within the narrative
//! graph for the reader popover's own PRIOR EVENT / FOLLOWING EVENT
//! sections via this module's own `positions_for_events`/
//! `NarrativePosition`. M-D3 (owner ruling R5, "get [legacy machinery] out
//! of the app"): that resolver retired -- Batch M-B/M-C re-implemented the
//! live `GET /api/narrative/event/{id}` handler as a view over the GRAPH's
//! own succession edges (`atlas_server::handlers::narrative_event_positions`,
//! `EdgeMeta::Narrative`-tagged `follows-in`/`precedes-in` pages), and
//! nothing has called `positions_for_events` from a live server response
//! since. `adjacent_event` below (the shared PRESENTATION builder, id ->
//! label/places/verse_groups) survives and is now called DIRECTLY by that
//! handler, unchanged. `global_timeline_position` (below) also survives --
//! not because it is live-reached either, but because
//! `server/atlas-graph/tests/timeline_equivalence.rs` keeps it as a real,
//! proven LOCKSTEP ORACLE against the graph's own reconstructed timeline
//! order (the same class of exemption `AtlasData::heading_for_verse`
//! earns, per that module's own doc comment) -- `positions_for_events` had
//! no such cross-checked test, only its own in-isolation unit tests
//! (deleted alongside it; recoverable from git history at the commit
//! immediately preceding this one).
//!
//! ONE-GRAPH property (the user's own words: "we have one graph
//! representing narratives"): the ONLY input this module reads is
//! `AtlasData::narratives`/`AtlasData::events` -- the exact same data
//! `scene::build_arrows` walks to build the map's own `SceneArrow`s, and
//! `adjacent_event` below builds each adjacent event's `verse_groups` via
//! `scene::to_scene_event`, the SAME function `scene::lit_places`/
//! `handlers::verse` already use for every OTHER "an event's own verses on
//! the wire" case in this codebase. A verse popover's PRIOR/FOLLOWING
//! section and a map arrow's own endpoint therefore always show the
//! IDENTICAL verse content for the same event id -- not by convention, but
//! because both are the same function call over the same data (see
//! `adjacent_event_verse_groups_equal_the_map_arrows_own_scene_event`'s own
//! test, below, for a direct, asserted proof of this, not just an
//! argument).

use crate::data::AtlasData;
use crate::wire::VerseGroup;

/// One event ADJACENT to the position being described (the PRIOR or the
/// FOLLOWING leg) -- carries everything the popover needs to render it
/// (label + verses, via the shared passage-list component) and everything
/// needed to traverse further (its own `id`, for the event-id-keyed lookup
/// below). `places` are this event's own anchor-and-companion place ids
/// (`Event::places`, unchanged) -- carried for wire completeness (the
/// brief's own "id, label, place(s), and verse groups") even though this
/// batch's own client UI does not render them textually (the event's own
/// `label` already names the moment in house prose; a future batch could
/// use `places` to jump the map to it without a wire change here).
#[derive(Debug, Clone, PartialEq)]
pub struct NarrativeAdjacentEvent {
    pub id: String,
    pub label: String,
    pub places: Vec<String>,
    pub verse_groups: Vec<VerseGroup>,
}

// M-D3 (owner ruling R5): `NarrativePosition` retired alongside
// `positions_for_events` (below `adjacent_event`) -- see this module's own
// top doc comment for the full retirement.

/// Builds one event's own `NarrativeAdjacentEvent` -- `None` only if `id`
/// names no real event (should never happen: ETL's own
/// `validate::run_narratives` already rejects a dangling leg id before this
/// data ever reaches a running server; handled here rather than assumed,
/// same "trust but verify" stance `scene::build_arrows`'s own
/// `filter_map(|id| d.event_by_id(id))` already takes for the identical
/// input).
///
/// Batch M-B: made `pub` (was private) so `atlas_server::handlers::
/// narrative_event_positions`'s own graph-view re-implementation can reuse
/// this EXACT presentation builder (id -> label/places/verse_groups) rather
/// than duplicating it -- the view's own TOPOLOGY (which events are prior/
/// following, in which narratives, and on the global timeline) now comes
/// from the graph (the generic port's own `EdgeMeta::Narrative` tags plus
/// `atlas_graph::Chronology`); this function's job was
/// always presentation, not topology, and stays exactly as it was,
/// untouched beyond this visibility widening (a safe, non-breaking change:
/// every existing caller/test in this file keeps compiling and passing
/// unmodified).
pub fn adjacent_event(d: &AtlasData, event_id: &str) -> Option<NarrativeAdjacentEvent> {
    let e = d.event_by_id(event_id)?;
    let se = crate::scene::to_scene_event(e); // SAME function every other "event's own verses on the wire" case uses
    Some(NarrativeAdjacentEvent { id: se.id, label: se.label, places: e.places.clone(), verse_groups: se.verse_groups })
}

// M-D3 (owner ruling R5, "why not just hash the legacy stuff... and get it
// out of the app"): `positions_for_events` retired -- the resolver it was
// ("the core resolver, shared by BOTH the verse-keyed and the event-id-
// keyed lookup") is genuinely unreached by any live server response (see
// this module's own top doc comment), and carried no cross-checked
// lockstep test the way `global_timeline_position` below does. Recoverable
// from git history at the commit immediately preceding this one.

/// Batch HOTFIX-4 requirement 1 ("generalize the ONE resolver -- traversal
/// by time for every dated event"): the GLOBAL chronological PRIOR/
/// FOLLOWING for one event id, independent of narrative membership --
/// "the previous/next event is the one that is chronologically NEXT," the
/// owner's own law, applied to the FULL set of dated events rather than one
/// narrative's own leg chain. `prior`/`following` reuse the SAME
/// `NarrativeAdjacentEvent` shape and the SAME `adjacent_event` builder
/// this module's own narrative-scoped lookups use too (one graph, seen a
/// second way -- never a parallel verse-groups derivation).
#[derive(Debug, Clone, PartialEq)]
pub struct TimelinePosition {
    pub prior: Option<NarrativeAdjacentEvent>,
    pub following: Option<NarrativeAdjacentEvent>,
}

/// `None` when `event_id` names no event at all, OR a real but
/// general-kind (undated) one -- requirement 2's own "general-kind
/// containers are NOT part of time traversal... fabricating a date is
/// forbidden," resolved by simple ABSENCE from `AtlasData::timeline_position`'s
/// own index (built from `kind == "event"` entries only), never a
/// special-cased branch here. Otherwise `Some`, with `prior`/`following`
/// independently `None` only at the true first/last position of the WHOLE
/// atlas's own dated-event timeline (conditional presence, never a
/// disabled stub) -- everywhere else, EVERY dated event has both, so a
/// user can always keep walking, per the owner's own "traversed by
/// time... arbitrarily far, until the end of the graph."
pub fn global_timeline_position(d: &AtlasData, event_id: &str) -> Option<TimelinePosition> {
    let idx = d.timeline_position(event_id)?;
    let prior = idx.checked_sub(1).and_then(|i| d.timeline_event_at(i)).and_then(|e| adjacent_event(d, &e.id));
    let following = d.timeline_event_at(idx + 1).and_then(|e| adjacent_event(d, &e.id));
    Some(TimelinePosition { prior, following })
}

#[cfg(test)]
mod tests {
    use super::*;

    // demo_fixture's own narrative shape (data.rs's own doc comment,
    // unchanged by this batch): `conquest` = [e1 Gilgal, e2 Jericho
    // besieged, e3 Jericho falls, e4 Ai defeated]; `patriarchs-demo` =
    // [e2] alone (a single-leg narrative -- e2's own position in IT has
    // neither prior nor following, a real, deliberate edge case the
    // fixture already carries, not one this batch invented). e5 (Sarah
    // buried at Machpelah) is a leg of NEITHER narrative.

    // M-D3 (owner ruling R5): the seven `positions_for_events`-exercising
    // tests formerly here (mid_chain_event_has_both_neighbors,
    // first_leg_has_no_prior, last_leg_has_no_following,
    // event_in_two_narratives_returns_both_positions_independently,
    // adjacent_event_with_zero_verses_resolves_with_empty_verse_groups,
    // event_in_no_narrative_returns_no_positions,
    // unknown_or_narrative_less_event_ids_yield_no_positions) are deleted
    // alongside the function itself -- see this module's own top doc
    // comment for the full retirement. `adjacent_event`'s own coverage
    // (the presentation builder these tests exercised only indirectly, via
    // `positions_for_events`' wrapping) continues below via
    // `global_timeline_position`'s own tests and the ONE-GRAPH EQUALITY
    // PROOF, both of which call `adjacent_event` directly.

    // --- global_timeline_position (Batch HOTFIX-4 requirement 1) -----------
    //
    // demo_fixture's own global timeline (chronological, ties broken by
    // stable original-array order -- see `AtlasData::finish`'s own
    // `timeline_order` doc comment): e5 (-2000, no narrative) -> e1 (-1406)
    // -> e2 (-1406) -> e3 (-1405) -> e4 (-1405). e1/e2 and e3/e4 tie on
    // `(from_year, order_key)` (both default order_key 0); the stable sort
    // keeps them in `demo_fixture`'s own declared order.

    #[test]
    fn global_timeline_mid_event_has_both_neighbors() {
        let d = crate::data::demo_fixture();
        let pos = global_timeline_position(&d, "e2").expect("e2 is a dated event");
        assert_eq!(pos.prior.as_ref().unwrap().id, "e1");
        assert_eq!(pos.following.as_ref().unwrap().id, "e3");
    }

    #[test]
    fn global_timeline_true_first_of_the_whole_atlas_has_no_prior() {
        let d = crate::data::demo_fixture();
        // e5 (-2000) is the earliest event in the fixture AND a leg of no
        // narrative at all (per data.rs's own fixture doc comment) --
        // exactly the "narrative-less dated event" case requirement 1
        // exists for.
        let pos = global_timeline_position(&d, "e5").expect("e5 is a dated event");
        assert!(pos.prior.is_none(), "e5 is the fixture's true first dated event -- no prior, not a disabled stub");
        assert_eq!(pos.following.as_ref().unwrap().id, "e1", "chronologically next regardless of narrative membership");
    }

    #[test]
    fn global_timeline_true_last_of_the_whole_atlas_has_no_following() {
        let d = crate::data::demo_fixture();
        let pos = global_timeline_position(&d, "e4").expect("e4 is a dated event");
        assert_eq!(pos.prior.as_ref().unwrap().id, "e3");
        assert!(pos.following.is_none(), "e4 is the fixture's true last dated event -- no following, not a disabled stub");
    }

    #[test]
    fn global_timeline_position_is_independent_of_narrative_membership() {
        let d = crate::data::demo_fixture();
        // e2 is a leg of two narratives (conquest AND patriarchs-demo, per
        // data.rs's own fixture doc comment) -- its OWN global timeline
        // position must be the SAME single answer regardless, not one per
        // narrative.
        let via_e1 = global_timeline_position(&d, "e2").unwrap();
        assert_eq!(via_e1.prior.as_ref().unwrap().id, "e1");
        // And a narrative-less event (e5) still gets a real position --
        // requirement 1's own "generalize the ONE resolver... for EVERY
        // dated event, not just narrative members."
        assert!(global_timeline_position(&d, "e5").is_some());
    }

    #[test]
    fn global_timeline_unknown_id_returns_none() {
        let d = crate::data::demo_fixture();
        assert!(global_timeline_position(&d, "no-such-event").is_none());
    }

    /// Requirement 2 ("general-kind containers: NOT part of time
    /// traversal... fabricating one is forbidden"): a real, standalone
    /// fixture (not `demo_fixture`, which carries no general-kind event) --
    /// `kind: "general"` events are excluded from `timeline_order` entirely
    /// by `AtlasData::finish()`.
    #[test]
    fn global_timeline_general_kind_event_returns_none() {
        use crate::data::{Canon, Event, Place};
        use std::collections::HashMap;

        let places = vec![Place { id: "p1".into(), name: "P1".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let events = vec![
            Event {
                id: "dated-1".into(),
                label: "A dated event".into(),
                when: crate::time::TimeRange::new(1, 1).unwrap(),
                places: vec!["p1".into()],
                verses: vec![],
                kind: "event".into(),
                ..Default::default()
            },
            Event {
                id: "general-1".into(),
                label: "A general-kind container".into(),
                when: crate::time::TimeRange::undated(),
                places: vec![],
                verses: vec![],
                kind: "general".into(),
                ..Default::default()
            },
        ];
        let d = AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();

        assert!(global_timeline_position(&d, "general-1").is_none(), "general-kind: no timeline position at all, not a stub");
        assert!(global_timeline_position(&d, "dated-1").is_some());
    }

    /// Same-`(from_year, order_key)` runs are common (order_key defaults to
    /// 0 outside deliberately-curated sub-sequencing) -- the global timeline
    /// must still resolve them deterministically: a STABLE sort, ties keep
    /// original array order (`AtlasData::finish`'s own `timeline_order` doc
    /// comment) -- the same "equal on all tiers keeps first-wins" precedent
    /// `heading_precedence` already establishes, not a new invented rule.
    #[test]
    fn global_timeline_same_date_run_resolves_by_stable_original_order() {
        use crate::data::{Canon, Event, Place};
        use std::collections::HashMap;

        let places = vec![Place { id: "p1".into(), name: "P1".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let ev = |id: &str| Event {
            id: id.into(),
            label: id.into(),
            when: crate::time::TimeRange::new(-4004, -4004).unwrap(), // ALL the same year, all default order_key 0
            places: vec!["p1".into()],
            verses: vec![],
            ..Default::default()
        };
        let events = vec![ev("alpha"), ev("beta"), ev("gamma")]; // declared order -- what the stable tiebreak must preserve
        let d = AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();

        let alpha = global_timeline_position(&d, "alpha").unwrap();
        assert!(alpha.prior.is_none());
        assert_eq!(alpha.following.as_ref().unwrap().id, "beta");
        let beta = global_timeline_position(&d, "beta").unwrap();
        assert_eq!(beta.prior.as_ref().unwrap().id, "alpha");
        assert_eq!(beta.following.as_ref().unwrap().id, "gamma");
        let gamma = global_timeline_position(&d, "gamma").unwrap();
        assert_eq!(gamma.prior.as_ref().unwrap().id, "beta");
        assert!(gamma.following.is_none());
    }

    /// THE ONE-GRAPH EQUALITY PROOF (not merely an argument): the SAME
    /// event's `verse_groups`, reached two completely independent ways --
    /// (a) `adjacent_event`, called DIRECTLY the way the LIVE reader-
    /// popover handler calls it today (M-D3: `atlas_server::handlers::
    /// narrative_event_positions` reads succession topology off the GRAPH's
    /// own `follows-in`/`precedes-in` edges and calls `adjacent_event`
    /// straight from there -- this module's own retired `positions_for_events`
    /// used to be the wrapper in between, but was never more than a
    /// presentation pass-through around this exact call, and is gone; see
    /// this module's own top doc comment), and (b) `scene::
    /// compose_time_scene`, the MAP's own wire source (a lit place's
    /// `events[].verse_groups`, exactly what an arrow endpoint's own place
    /// card would show) -- must be byte-for-byte `assert_eq!`, not just
    /// "look similar." e3 (Jericho falls, JOS.6.20/.21/.24) is chosen
    /// because its own window (-1405) is disjoint from e1/e2's (-1406), so
    /// a narrow time-mode window isolates it cleanly on the map side.
    #[test]
    fn adjacent_event_verse_groups_equal_the_map_arrows_own_scene_event() {
        let d = crate::data::demo_fixture();

        // (a) the reader popover's own source, called the SAME direct way
        // the live handler does: e3 is conquest's own leg immediately
        // following e2 (Jericho besieged) -- its own verse_groups via
        // `adjacent_event` directly, no narrative-position wrapper.
        let e3_via_narrative = &adjacent_event(&d, "e3").expect("e3 is a real event").verse_groups;

        // (b) the map's own source: e3's window is -1405 alone; the
        // resulting scene's own "jericho" place carries e3 among its
        // events (jericho also carries e2, hence the `find`).
        let scene = crate::scene::compose_time_scene(&d, crate::time::TimeRange::new(-1405, -1405).unwrap());
        let jericho = scene.places.iter().find(|p| p.id == "jericho").expect("jericho is lit at -1405 (e3)");
        let e3_via_scene = &jericho.events.iter().find(|e| e.id == "e3").expect("e3 present in the -1405 scene").verse_groups;

        assert_eq!(e3_via_narrative, e3_via_scene, "the popover's PRIOR/FOLLOWING verse groups must equal the map arrow endpoint's own -- one graph, seen twice");
    }

}
