//! Batch N ("narratives as first-class graph structure"): resolves a
//! verse's or an event's own position(s) within the narrative graph --
//! which narrative(s), which event, and the PRIOR/FOLLOWING events in leg
//! order -- for the reader popover's own PRIOR EVENT / FOLLOWING EVENT
//! sections. Pure, no HTTP/handler concerns (mirrors `crate::xrefs`/
//! `crate::catechism`'s own "business logic lives in atlas-core, the
//! handler is a thin wrapper" shape).
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
//! `positions_for_events`'s own tests for a direct, asserted proof of this,
//! not just an argument).

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

/// One (narrative, event) position a queried verse or event touches. A
/// verse cited by >1 event, or an event that is itself a leg of >1
/// narrative (both real, not hypothetical -- see this module's own tests
/// and, in the real compiled data, `EXO.12.37`, cited by BOTH the exodus
/// narrative's `ex_rameses` AND `ex_succoth` legs) yields one
/// `NarrativePosition` per distinct (narrative, event) pair touched --
/// "a verse in multiple narratives returns all positions," generalized to
/// "and an event in multiple narratives, or a verse in multiple events,
/// too" -- never silently collapsed to one.
///
/// `prior`/`following` are `None` exactly at the narrative's own first/last
/// leg (conditional presence -- no disabled stub) -- see
/// `positions_for_events`'s own doc comment for the walk itself.
#[derive(Debug, Clone, PartialEq)]
pub struct NarrativePosition {
    pub narrative_id: String,
    pub narrative_name: String,
    /// The event id `event_ids` matched IN THIS narrative's own leg chain --
    /// not necessarily the only event a queried verse touches (see
    /// `NarrativePosition`'s own doc comment), and not a duplicate of the
    /// verse's/event's OWN text (already on screen as the popover's
    /// subject) -- present for two reasons: (1) the map-focus-sync
    /// mechanism needs to know exactly which arrow endpoints are "the
    /// current leg" (client: `ExplorerPopover`'s own `INarrativeAware`
    /// hook), and (2) disambiguating two positions that share a
    /// `narrative_id` (the `EXO.12.37` case above) needs `event_label` to
    /// tell them apart in a section heading.
    pub event_id: String,
    pub event_label: String,
    pub prior: Option<NarrativeAdjacentEvent>,
    pub following: Option<NarrativeAdjacentEvent>,
}

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

/// The core resolver, shared by BOTH the verse-keyed and the event-id-keyed
/// lookup (requirement 1's own "endpoint/payload also supports event-id
/// lookup" -- traversal steps resolve by event, not by re-searching
/// verses): `event_ids` is `AtlasData::events_for_verse(vref)` for the
/// verse-keyed caller, a single-element slice for the event-id-keyed one --
/// ONE walk, so the two lookup paths can never drift apart.
///
/// For every narrative, for every leg (event id) that appears in
/// `event_ids`, the PRIOR position is `legs[idx-1]` and the FOLLOWING is
/// `legs[idx+1]` -- a plain adjacent-array-index walk over
/// `Narrative::legs`, the EXACT ordered chain `scene::build_arrows` walks
/// to build arrows (that function's own `kept.windows(2)` is the identical
/// "look at this leg and its immediate neighbor" idea, just already
/// filtered to a time/scripture window there; this module deliberately
/// ignores window filtering entirely -- a narrative position is asked "by
/// verse" or "by event," never "by window," so EVERY leg is a candidate,
/// unlike a scene's own filtered arrow list). `idx == 0` (the narrative's
/// own first leg) has no prior; `idx == legs.len() - 1` (the last) has no
/// following -- conditional presence, never a disabled stub, matching
/// requirement 2 verbatim ("ending naturally at the narrative's first/last
/// event").
pub fn positions_for_events(d: &AtlasData, event_ids: &[String]) -> Vec<NarrativePosition> {
    let mut out = Vec::new();
    for n in &d.narratives {
        for (idx, eid) in n.legs.iter().enumerate() {
            if !event_ids.iter().any(|e| e == eid) {
                continue;
            }
            let Some(current) = d.event_by_id(eid) else { continue }; // dangling leg -- ETL-validated not to happen; skip rather than panic
            let prior = idx.checked_sub(1).and_then(|i| n.legs.get(i)).and_then(|id| adjacent_event(d, id));
            let following = n.legs.get(idx + 1).and_then(|id| adjacent_event(d, id));
            out.push(NarrativePosition {
                narrative_id: n.id.clone(),
                narrative_name: n.name.clone(),
                event_id: eid.clone(),
                event_label: current.label.clone(),
                prior,
                following,
            });
        }
    }
    out
}

/// Batch HOTFIX-4 requirement 1 ("generalize the ONE resolver -- traversal
/// by time for every dated event"): the GLOBAL chronological PRIOR/
/// FOLLOWING for one event id, independent of narrative membership --
/// "the previous/next event is the one that is chronologically NEXT," the
/// owner's own law, applied to the FULL set of dated events rather than one
/// narrative's own leg chain. `prior`/`following` reuse the SAME
/// `NarrativeAdjacentEvent` shape and the SAME `adjacent_event` builder
/// `positions_for_events` above already uses (one graph, seen a second way
/// -- never a parallel verse-groups derivation).
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

    #[test]
    fn mid_chain_event_has_both_neighbors() {
        let d = crate::data::demo_fixture();
        let positions = positions_for_events(&d, &["e2".to_string()]);
        let conquest = positions.iter().find(|p| p.narrative_id == "conquest").expect("e2 is a conquest leg");
        assert_eq!(conquest.event_id, "e2");
        assert_eq!(conquest.event_label, "Jericho besieged");
        let prior = conquest.prior.as_ref().expect("e2 is not conquest's first leg");
        assert_eq!(prior.id, "e1");
        assert_eq!(prior.label, "Camp at Gilgal");
        assert_eq!(prior.places, vec!["gilgal".to_string()]);
        let following = conquest.following.as_ref().expect("e2 is not conquest's last leg");
        assert_eq!(following.id, "e3");
        assert_eq!(following.label, "Jericho falls");
    }

    #[test]
    fn first_leg_has_no_prior() {
        let d = crate::data::demo_fixture();
        let positions = positions_for_events(&d, &["e1".to_string()]);
        let conquest = positions.iter().find(|p| p.narrative_id == "conquest").unwrap();
        assert!(conquest.prior.is_none(), "e1 is conquest's first leg -- no prior, not a disabled stub");
        assert_eq!(conquest.following.as_ref().unwrap().id, "e2");
    }

    #[test]
    fn last_leg_has_no_following() {
        let d = crate::data::demo_fixture();
        let positions = positions_for_events(&d, &["e4".to_string()]);
        let conquest = positions.iter().find(|p| p.narrative_id == "conquest").unwrap();
        assert_eq!(conquest.prior.as_ref().unwrap().id, "e3");
        assert!(conquest.following.is_none(), "e4 is conquest's last leg -- no following, not a disabled stub");
    }

    /// The multi-narrative case, verbatim per the brief ("a verse in
    /// multiple narratives returns all positions"): e2 is a leg of BOTH
    /// conquest (mid-chain, both neighbors present) AND patriarchs-demo (its
    /// own ONLY leg -- neither neighbor present). Both positions must come
    /// back from the SAME call, neither one silently dropping the other.
    #[test]
    fn event_in_two_narratives_returns_both_positions_independently() {
        let d = crate::data::demo_fixture();
        let positions = positions_for_events(&d, &["e2".to_string()]);
        assert_eq!(positions.len(), 2, "e2 belongs to two narratives: {positions:?}");

        let conquest = positions.iter().find(|p| p.narrative_id == "conquest").unwrap();
        assert!(conquest.prior.is_some() && conquest.following.is_some());

        let patriarchs = positions.iter().find(|p| p.narrative_id == "patriarchs-demo").unwrap();
        assert_eq!(patriarchs.narrative_name, "Patriarchs (demo)");
        assert!(patriarchs.prior.is_none(), "patriarchs-demo's only leg has no prior");
        assert!(patriarchs.following.is_none(), "patriarchs-demo's only leg has no following");
    }

    /// An event with ZERO curated verses (a real, if currently only
    /// theoretical, shape -- no event in the real compiled data has an
    /// empty `verses` list today, but nothing in `Narrative.legs`/`Event`
    /// forbids it, and `scene::build_arrows`'s own arrows never depend on
    /// an event having verses either) still resolves a real position with
    /// an empty `verse_groups` -- never panics, never fabricates a
    /// placeholder verse. Standalone fixture (not `demo_fixture`): every
    /// real event there has >=1 verse, so this case needs its own tiny,
    /// purpose-built dataset.
    #[test]
    fn adjacent_event_with_zero_verses_resolves_with_empty_verse_groups() {
        use crate::data::{Canon, Event, Narrative, Place};
        use std::collections::HashMap;

        let places = vec![Place { id: "p1".into(), name: "P1".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let events = vec![
            Event {
                id: "silent-1".into(),
                label: "An undocumented moment".into(),
                when: crate::time::TimeRange::new(-1000, -1000).unwrap(),
                places: vec!["p1".into()],
                verses: vec![], // the case under test
                ..Default::default()
            },
            Event {
                id: "silent-2".into(),
                label: "The next moment".into(),
                when: crate::time::TimeRange::new(-999, -999).unwrap(),
                places: vec!["p1".into()],
                verses: vec![],
                ..Default::default()
            },
        ];
        let narratives = vec![Narrative {
            id: "silent".into(),
            name: "A Silent Narrative".into(),
            color: "#000000".into(),
            legs: vec!["silent-1".into(), "silent-2".into()],
        }];
        let d = AtlasData::new(Canon { books: vec![] }, places, events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();

        let positions = positions_for_events(&d, &["silent-1".to_string()]);
        let position = positions.iter().find(|p| p.narrative_id == "silent").unwrap();
        assert!(position.prior.is_none());
        let following = position.following.as_ref().expect("silent-1 -> silent-2 is a real leg");
        assert_eq!(following.id, "silent-2");
        assert!(following.verse_groups.is_empty(), "zero curated verses -> empty verse_groups, not a panic or a placeholder");
    }

    /// A verse touched by an event that is a leg of NO narrative at all
    /// (e5 -- Sarah buried at Machpelah, a bare event, per data.rs's own
    /// fixture doc comment) returns an empty position list -- the
    /// "non-narrative verse shows neither section" case, resolved purely by
    /// an empty Vec, no special-cased sentinel.
    #[test]
    fn event_in_no_narrative_returns_no_positions() {
        let d = crate::data::demo_fixture();
        let positions = positions_for_events(&d, &["e5".to_string()]);
        assert!(positions.is_empty(), "e5 is not a leg of any narrative: {positions:?}");
    }

    /// A verse-keyed call (multiple candidate event ids, mirroring
    /// `AtlasData::events_for_verse`'s own return shape) that matches
    /// nothing in any narrative's legs -- same empty-Vec outcome as the
    /// single-event case above, proving the verse-keyed path doesn't need
    /// its own special casing.
    #[test]
    fn unknown_or_narrative_less_event_ids_yield_no_positions() {
        let d = crate::data::demo_fixture();
        let positions = positions_for_events(&d, &["e5".to_string(), "no-such-event".to_string()]);
        assert!(positions.is_empty());
    }

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
        // narrative at all (positions_for_events returns [] for it, see
        // `event_in_no_narrative_returns_no_positions` above) -- exactly
        // the "narrative-less dated event" case requirement 1 exists for.
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
        // e2 is a leg of two narratives (conquest AND patriarchs-demo,
        // see `event_in_two_narratives_returns_both_positions_independently`
        // above) -- its OWN global timeline position must be the SAME
        // single answer regardless, not one per narrative.
        let via_e1 = global_timeline_position(&d, "e2").unwrap();
        assert_eq!(via_e1.prior.as_ref().unwrap().id, "e1");
        // And a narrative-less event (e5) still gets a real position --
        // requirement 1's own "generalize the ONE resolver... for EVERY
        // dated event, not just narrative members."
        assert!(global_timeline_position(&d, "e5").is_some());
        assert!(positions_for_events(&d, &["e5".to_string()]).is_empty(), "e5 has no NARRATIVE position");
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
    /// (a) THIS module's own `positions_for_events`, the reader popover's
    /// wire source, and (b) `scene::compose_time_scene`, the MAP's own wire
    /// source (a lit place's `events[].verse_groups`, exactly what an arrow
    /// endpoint's own place card would show) -- must be byte-for-byte
    /// `assert_eq!`, not just "look similar." e3 (Jericho falls, JOS.6.20/
    /// .21/.24) is chosen because its own window (-1405) is disjoint from
    /// e1/e2's (-1406), so a narrow time-mode window isolates it cleanly on
    /// the map side.
    #[test]
    fn adjacent_event_verse_groups_equal_the_map_arrows_own_scene_event() {
        let d = crate::data::demo_fixture();

        // (a) the reader popover's own source: ask for e2 (Jericho
        // besieged)'s position in conquest, and read its own FOLLOWING
        // event's (e3's) verse_groups.
        let positions = positions_for_events(&d, &["e2".to_string()]);
        let conquest = positions.iter().find(|p| p.narrative_id == "conquest").unwrap();
        let e3_via_narrative = &conquest.following.as_ref().expect("e2 -> e3 in conquest").verse_groups;

        // (b) the map's own source: e3's window is -1405 alone; the
        // resulting scene's own "jericho" place carries e3 among its
        // events (jericho also carries e2, hence the `find`).
        let scene = crate::scene::compose_time_scene(&d, crate::time::TimeRange::new(-1405, -1405).unwrap());
        let jericho = scene.places.iter().find(|p| p.id == "jericho").expect("jericho is lit at -1405 (e3)");
        let e3_via_scene = &jericho.events.iter().find(|e| e.id == "e3").expect("e3 present in the -1405 scene").verse_groups;

        assert_eq!(e3_via_narrative, e3_via_scene, "the popover's PRIOR/FOLLOWING verse groups must equal the map arrow endpoint's own -- one graph, seen twice");
    }

}
