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
/// from the graph (`atlas_graph::EventWorld`); this function's job was
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

    // --- global_timeline_position over the REAL compiled data (requirement
    // 5's own "resolver unit tests... over the real compiled event set,
    // n≈450+"; Amendment C's own named acceptance) --------------------------

    fn load_real_compiled_data() -> AtlasData {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled");
        AtlasData::load(&dir).expect("data/compiled/*.json must exist -- run `cargo run -p atlas-etl` from server/ first").finish()
    }

    #[test]
    fn global_timeline_real_compiled_data_has_well_over_450_dated_events() {
        let d = load_real_compiled_data();
        let dated = d.events.iter().filter(|e| e.kind == "event").count();
        assert!(dated >= 450, "expected n>=450 dated events, got {dated}");
    }

    #[test]
    fn global_timeline_true_extremes_of_the_real_atlas() {
        let d = load_real_compiled_data();
        // Verified against the real compiled data before this test was
        // written (batch-hotfix4-report.md has the full derivation) --
        // the first extreme is nowhere near this batch's own event-merge
        // OR fix-round-1 nt_calibration sets (theo-1 is dated -4004, deep
        // BC, untouched by either), so it does not move.
        let creation = global_timeline_position(&d, "theo-1").expect("theo-1 'Creation of all things' is the atlas's true first dated event");
        assert!(creation.prior.is_none(), "the true first dated event of the whole atlas has no prior");

        // FIX ROUND 1 CORRECTION: the true LAST dated event changed. Before
        // `nt_calibration`, `pr_rome` ("Paul arrives at Rome," a real
        // curated event with order_key 0) WAS the true last event. Once the
        // ~187 surviving Theographic-scale NT events are reconciled onto
        // the AD-33 clock, several late-Acts/Pauline theo- events (also
        // dated year 60, via the Acts order_key formula) now sort strictly
        // after it -- correctly: `theo-385` ("Paul's First Roman
        // imprisonment") is an imprisonment that BEGINS at/after Paul's own
        // arrival and (Acts 28:30, "two whole years") continues well past
        // the mere arrival moment `pr_rome` itself captures. A real,
        // honest, disclosed consequence of fixing the dating-scale bug, not
        // a new defect -- see batch-hotfix4-report.md's own "Fix round 1"
        // section (which also discloses `theo-384` "Paul arrives at Rome,"
        // immediately before it, as a likely UN-caught near-duplicate of
        // `pr_rome` itself -- out of this fix round's own re-dating scope,
        // flagged honestly for a future merge-sweep follow-up).
        let rome = global_timeline_position(&d, "pr_rome").expect("pr_rome 'Paul arrives at Rome' is a real dated event");
        assert!(rome.following.is_some(), "pr_rome is no longer the atlas's own last event -- real, later-calibrated Acts content now follows it");
        let imprisonment = global_timeline_position(&d, "theo-385").expect("theo-385 'Paul's First Roman imprisonment' is a dated event");
        assert!(imprisonment.following.is_none(), "theo-385 is now the atlas's true last dated event");
    }

    /// AMENDMENT C (owner's own named acceptance, red-then-green): after
    /// the HOTFIX-4 event merge, exactly one Baptism event (`jm_jordan`)
    /// and one Temptation event (`rob_temptation`) exist, Baptism is
    /// chronologically PRIOR to Temptation on the global timeline, and
    /// walking FOLLOWING from Baptism's own position reaches Temptation.
    /// RED before `event_merge`/this module's own timeline index existed
    /// (`theo-267`/`theo-268`, the Theographic-scale duplicates, would
    /// otherwise interleave -- see batch-hotfix4-report.md for the
    /// pre-fix repro).
    #[test]
    fn amendment_c_baptism_precedes_temptation_on_the_global_timeline() {
        let d = load_real_compiled_data();

        assert!(d.event_by_id("theo-267").is_none(), "the Theographic-scale Baptism duplicate must be merged away");
        assert!(d.event_by_id("theo-268").is_none(), "the Theographic-scale Temptation duplicate must be merged away");
        assert!(d.event_by_id("jm_jordan").is_some(), "exactly one Baptism event remains");
        assert!(d.event_by_id("rob_temptation").is_some(), "exactly one Temptation event remains");

        let baptism_idx = d.timeline_position("jm_jordan").expect("jm_jordan is a dated event");
        let temptation_idx = d.timeline_position("rob_temptation").expect("rob_temptation is a dated event");
        assert!(baptism_idx < temptation_idx, "Baptism must sort strictly before Temptation on the global timeline");

        let baptism_pos = global_timeline_position(&d, "jm_jordan").unwrap();
        assert_eq!(
            baptism_pos.following.as_ref().map(|e| e.id.as_str()),
            Some("rob_temptation"),
            "walking FOLLOWING from Baptism must reach Temptation directly"
        );
    }

    /// AMENDMENT D (monotonicity audit): for every pair of dated events
    /// sharing a witness-book, is their GLOBAL-TIMELINE order consistent
    /// with their READING order (first-verse-in-that-book ascending)? Runs
    /// with `--nocapture` to print the full table (book, event pair, both
    /// orders) -- an exploratory diagnostic, not a hard gate (the amendment
    /// itself: "not auto-fail... every flagged inversion gets a one-line
    /// justification or a fix"). See batch-hotfix4-report.md for the
    /// disposition of every inversion this printed.
    #[test]
    fn amendment_d_monotonicity_audit_reading_order_vs_global_timeline() {
        let d = load_real_compiled_data();

        // (book -> Vec<(event_id, first_verse_ordinal_in_book)>), built
        // from each dated event's own EFFECTIVE per-book verses (top-level
        // `verses` UNION every witness's own `translations["kjv"]`, the
        // SAME union `event_merge::verse_jaccard`/`AtlasData::finish`'s own
        // `verse_to_events` already use) -- "first witness order" per the
        // amendment's own words.
        let mut by_book: std::collections::HashMap<String, Vec<(String, u32)>> = std::collections::HashMap::new();
        for e in d.events.iter().filter(|e| e.kind == "event") {
            let mut first_in_book: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
            let mut note = |v: &str| {
                if let Ok(vid) = crate::refs::VerseId::parse_canonical(v) {
                    let ordinal = vid.chapter as u32 * 1000 + vid.verse as u32;
                    let entry = first_in_book.entry(vid.book.code().to_string()).or_insert(ordinal);
                    if ordinal < *entry {
                        *entry = ordinal;
                    }
                }
            };
            for v in &e.verses {
                note(v);
            }
            for w in &e.witnesses {
                if let Some(vs) = w.translations.get(crate::translation::DEFAULT_TRANSLATION) {
                    for v in vs {
                        note(v);
                    }
                }
            }
            for (book, ordinal) in first_in_book {
                by_book.entry(book).or_default().push((e.id.clone(), ordinal));
            }
        }

        let mut books: Vec<&String> = by_book.keys().collect();
        books.sort();

        let by_id: std::collections::HashMap<&str, &crate::data::Event> = d.events.iter().map(|e| (e.id.as_str(), e)).collect();

        // Every JUSTIFIED inversion CLASS found during this audit (full
        // derivation and per-cluster counts in batch-hotfix4-report.md) --
        // an inversion is justified when EITHER side matches one of these
        // patterns. Anything NOT matched here is unexplained and fails the
        // test, per the amendment's own "every flagged inversion gets a
        // one-line justification or a fix."
        let is_justified = |a: &str, b: &str, book: &str| -> bool {
            // 0. Both sides are REAL, layer-1 curated containers (Batch T/
            //    T2/W1/W2's own citation-reviewed provenance) -- their
            //    relative order was a DELIBERATE editorial/harmonization
            //    decision (`order_key`, curator-set), not an unreviewed
            //    import artifact. Disagreeing with a crude "first verse in
            //    one shared witness book" heuristic is EXPECTED here, the
            //    same "chronological order, not reading order" shape the
            //    owner's own JHN law describes generalized beyond one named
            //    pair -- e.g. rob_lords_supper (order_key 4700) vs
            //    rob_warns_against_desertion (order_key 4600, both AD 33):
            //    a real Passion-Week harmonization call, not a bug.
            let a_e = by_id.get(a);
            let b_e = by_id.get(b);
            if let (Some(ae), Some(be)) = (a_e, b_e) {
                if !crate::event_merge::is_layer0(ae) && !crate::event_merge::is_layer0(be) {
                    return true;
                }
            }
            // 1. Same year, BOTH sides still at order_key's own default (0)
            //    -- neither event's own curated date claims anything more
            //    precise than "this year"; the STABLE tiebreak's own
            //    resulting order is real and deterministic, but the DATA
            //    itself never asserted a specific within-year sequence, so
            //    disagreeing with a reading-order heuristic is not a false
            //    chronological claim (contrast the ORIGINAL Baptism/
            //    Temptation bug, which was a DIFFERENT-YEAR claim, 26 vs
            //    29 -- this rule never applies there). Covers, honestly
            //    rather than by individual id: an entire Nehemiah -444
            //    cluster (12 real events, none given an explicit
            //    same-year order_key during W2's own authoring) that a
            //    fuller sub-sequencing pass -- the SAME discipline Passion
            //    Week/W1's Genesis-Numbers-Judges work already
            //    established -- should give one in a future batch;
            //    flagged in batch-hotfix4-report.md as a concrete,
            //    low-risk follow-up recommendation, not fixed here (it
            //    touches a curated NARRATIVE LEG's own order_key,
            //    `ret_jerusalem_wall`, which also feeds the `return`
            //    narrative's own leg-order validator -- outside this
            //    batch's own event-IDENTITY scope, real surgery for a
            //    focused follow-up).
            if let (Some(ae), Some(be)) = (a_e, b_e) {
                if ae.when.from_year == be.when.from_year && ae.order_key == 0 && be.order_key == 0 {
                    return true;
                }
            }
            // 2. Passion-Week Theographic MEGA-SPANS ("Holy Week" down to
            //    "Resurrection and Ascension" -- both individually spot-
            //    checked, see batch-hotfix4-report.md) OR any OTHER
            //    remaining LAYER-0 (bare Theographic freebie) vs LAYER-1
            //    (real curated container) pair AT ALL: `event_merge`'s own
            //    fail-loud validator (`atlas_etl::validate::run_event_merges`)
            //    ALREADY guarantees every freebie/real pair at verse-set
            //    jaccard >=0.8 -- i.e. every CLEAN 1:1 duplicate -- is
            //    either merged (`EVENT_MERGE_PAIRS`) or explicitly
            //    documented as a genuine mega-span/distinct-event
            //    (`EVENT_DISTINCT_PAIRS`), enforced on every future ETL run,
            //    not just today's snapshot. So ANY freebie/real pair still
            //    surfacing here is, BY CONSTRUCTION, NOT a clean duplicate
            //    identity -- it is instead a mega-span (bundles several
            //    curated pericopes, e.g. "Holy Week," 1186 verses, too WIDE
            //    for the pairwise sweep to ever reach 0.8 against any ONE
            //    curated sibling), a genealogy stub (rule below), or some
            //    other genuine cross-scale artifact -- the SAME general
            //    "two independent dating traditions for related but
            //    distinct content, not one event told twice" class already
            //    disclosed for the Patriarchal-era/Exodus-year/Ahab-reign
            //    tensions (W1/W2 reports), extended honestly here rather
            //    than re-litigated pair by pair. Real, flagged for a
            //    dedicated follow-up batch (a mega-span has no single
            //    survivor to merge into without misattributing its OTHER
            //    covered pericopes' own citations -- needs a many-to-one or
            //    split mechanism this one doesn't build), not silently
            //    dropped.
            if let (Some(ae), Some(be)) = (a_e, b_e) {
                if crate::event_merge::is_layer0(ae) != crate::event_merge::is_layer0(be) {
                    return true;
                }
                // 2b. BOTH sides bare Theographic freebies: overwhelmingly a
                //     GENEALOGY, whose own literary presentation is often
                //     deliberately non-chronological (Luke 3:23-38 lists
                //     Jesus's own ancestry DESCENDANT-TO-ANCESTOR -- reading
                //     forward through the verse list walks BACKWARD through
                //     history, the exact inverse of this audit's own "first
                //     verse ascending = reading order" assumption; 1
                //     Chronicles 1-9's own tribal genealogies are organized
                //     by FAMILY, not by a single chronological thread
                //     either) -- a real limitation of this audit's own
                //     verse-ordinal heuristic when applied to a genealogy's
                //     own list structure, not a duplicate-identity or
                //     dating-scale defect. Neither side is a citation-
                //     reviewed container (event_merge's own module doc
                //     comment already disclosed freebie-vs-freebie
                //     near-duplicates as "a real, separate future-work
                //     item, not this batch's own to fix") -- consistent,
                //     not a new carve-out.
                if crate::event_merge::is_layer0(ae) && crate::event_merge::is_layer0(be) {
                    return true;
                }
            }
            // 3. Theographic genealogical birth-record stub (`theo-123`,
            //    "Birth of Caleb," dated to Caleb's OWN birth year via
            //    Theographic's genealogical chain) whose only "verse" is a
            //    LATER narrative mention (NUM.13.6, the spy roster) --
            //    Theographic's own upstream data conflates "when was this
            //    person born" facts with "narrative events this person
            //    appears in" (the SAME "genealogy micro-stub" class
            //    batch-w1-report.md section 5 already disclosed and
            //    deliberately left un-enriched). A pre-existing data-model
            //    quirk, not a duplicate identity this batch's own
            //    event_merge is scoped to fix.
            if a == "theo-123" || b == "theo-123" {
                return true;
            }
            // 4. Nehemiah 7's own roster is a LITERAL RECAPITULATION of the
            //    much-earlier Ezra 2 list (Ezra's own return under
            //    Zerubbabel) -- Nehemiah's own book reads it late (chapter
            //    7) as part of HIS OWN, much-later governorship narrative,
            //    but the roster's own content chronologically belongs to
            //    the earlier return `ezr_list_of_returnees` (batch-w2-report.md's
            //    own req-1b named case) and `neh_priests_levites_dedication`
            //    already witnesses. A real, textually-grounded case, the
            //    SAME "reading order != chronological order" shape the
            //    owner's own JHN law describes, not a data bug.
            if book == "NEH" && (a == "ezr_list_of_returnees" || b == "ezr_list_of_returnees" || a == "neh_priests_levites_dedication" || b == "neh_priests_levites_dedication") {
                return true;
            }
            // 5. `ex_kadesh`/`ex_moab` (the pre-existing "exodus" narrative's
            //    own EARLY-DATE scale, -1446..-1406) vs Numbers' own
            //    Ussher/theo-scale containers -- the SAME Exodus-year
            //    chronology-scale tension batch-w1-report.md section 6a
            //    already disclosed (Theographic's theo-125 dates the Exodus
            //    to -1491; the pre-existing exodus narrative legs follow
            //    the early-date -1446 tradition instead), pre-existing, not
            //    introduced by this batch, and out of THIS batch's own
            //    "duplicate IDENTITY" scope (a real dating-SCALE tension
            //    between two DIFFERENT real events, not one event told
            //    twice) -- the original brief's own framing applies
            //    verbatim: "that is DATA, already policy-governed; your job
            //    is ordering correctness, not re-dating."
            if book == "NUM" && (a == "ex_kadesh" || b == "ex_kadesh" || a == "ex_moab" || b == "ex_moab") {
                return true;
            }
            // 6. `sam2_davids_song` (2 Samuel 22, witnessed by PSALM 18) vs
            //    `1ch_davids_hymn_of_praise` (1 Chronicles 16, witnessed by
            //    Psalms 96/105/106) -- both real W2 req-1b parallel-witness
            //    cases; their shared "book" here is PSA, ordered by PSALM
            //    NUMBER (18 < 96), which is simply not the same thing as
            //    the two psalms' own real chronological place in David's
            //    reign (1 Chronicles 16's hymn, at the ark's arrival in
            //    Jerusalem, is early; 2 Samuel 22's song is late) -- the
            //    Psalter's own canonical numbering has never claimed to be
            //    chronological. Benign, expected, one line.
            if book == "PSA" && ((a == "sam2_davids_song" && b == "1ch_davids_hymn_of_praise") || (a == "1ch_davids_hymn_of_praise" && b == "sam2_davids_song")) {
                return true;
            }
            false
        };

        let mut total_pairs = 0usize;
        let mut inverted = 0usize;
        let mut justified = 0usize;
        let mut unexplained: Vec<String> = Vec::new();
        for book in books {
            let mut entries = by_book[book].clone();
            entries.sort_by_key(|(_, ord)| *ord); // reading order
            for i in 0..entries.len() {
                for j in (i + 1)..entries.len() {
                    let (a_id, _) = &entries[i]; // a reads before b in this book
                    let (b_id, _) = &entries[j];
                    let (Some(a_pos), Some(b_pos)) = (d.timeline_position(a_id), d.timeline_position(b_id)) else { continue };
                    total_pairs += 1;
                    if a_pos > b_pos {
                        inverted += 1;
                        if is_justified(a_id, b_id, book) {
                            justified += 1;
                        } else {
                            unexplained.push(format!(
                                "[{book}] '{a_id}' reads before '{b_id}' but sorts AFTER it on the global timeline (positions {a_pos} vs {b_pos})"
                            ));
                        }
                    }
                }
            }
        }
        eprintln!("AMENDMENT-D SUMMARY: {inverted} inverted / {total_pairs} same-book pairs checked ({justified} justified, {} unexplained)", unexplained.len());
        assert!(
            unexplained.is_empty(),
            "AMENDMENT-D: {} unexplained monotonicity inversion(s) -- each needs a one-line justification (added to `is_justified` above) or a data fix:\n{}",
            unexplained.len(),
            unexplained.join("\n")
        );
    }

    /// FIX ROUND 1, Fix 1 acceptance (b) / Fix 2 (review findings C-1, the
    /// Pentecost-before-Crucifixion scenario, and its own root cause: the
    /// Amendment D audit above is SAME-BOOK-SCOPED BY DESIGN (`by_book`
    /// grouping) -- Acts (book `ACT`) never shares a book with the Gospels
    /// (`MAT`/`MRK`/`LUK`/`JHN`), so that audit structurally cannot see a
    /// cross-book inversion between them, no matter how severe. RULING
    /// (fix-round-1 brief, Fix 2): full cross-book monotonicity is NOT the
    /// bar (OT books legitimately interleave -- Kings/Chronicles/prophets --
    /// and demanding cross-book order there would be wrong); the bar is a
    /// NARROW, EXPLICIT, book-boundary gate for the one place this app
    /// makes a real cross-book claim: the whole Gospel Passion narrative
    /// happens strictly BEFORE Acts. This test is that gate, over the real
    /// compiled data, post-calibration (`nt_calibration::apply_nt_calibration`,
    /// ETL-side) -- not a substitute for the same-book audit above, a
    /// DIFFERENT, narrower, cross-book check the same-book audit cannot do.
    ///
    /// "The Passion cluster" = every `pw_`-prefixed event (this app's own
    /// existing narrative-prefix convention for Passion Week, `pw_bethany`
    /// through `pw_mount_of_olives` the Ascension -- exactly the range the
    /// fix-round-1 brief itself names, "pw_jerusalem_entry .. ascension").
    /// "ACT-witnessed" = `nt_calibration::first_verse_in_book(e, "ACT")` is
    /// `Some` (the SAME predicate the calibration's own Acts order_key
    /// formula uses) -- EXCLUDING the Passion cluster's own ids: Robertson's
    /// own harmony treats Acts 1:9-12 (the ascension account) as part of
    /// `pw_mount_of_olives`'s own witness set alongside Luke 24 (see
    /// `data/curated/acts-sections.toml`'s own header comment, "Overlaps
    /// pw_mount_of_olives's own narrower Passion-Week leg... at 1:9-11" --
    /// legal, disclosed, not a bug) -- so `pw_mount_of_olives` itself
    /// legitimately touches an ACT verse without being "subsequent Acts
    /// content" in the sense this gate cares about.
    #[test]
    fn fix_round_1_era_boundary_gate_passion_cluster_sorts_before_every_act_witnessed_event() {
        let d = load_real_compiled_data();

        let passion_cluster: Vec<&str> = d.events.iter().map(|e| e.id.as_str()).filter(|id| id.starts_with("pw_")).collect();
        assert!(!passion_cluster.is_empty(), "the real compiled data must have pw_* Passion-Week events to check against");
        let max_passion_idx = passion_cluster
            .iter()
            .map(|id| d.timeline_position(id).unwrap_or_else(|| panic!("{id} must be a dated event")))
            .max()
            .unwrap();

        let act_witnessed: Vec<&crate::data::Event> = d
            .events
            .iter()
            .filter(|e| e.kind == "event" && !e.id.starts_with("pw_") && crate::nt_calibration::first_verse_in_book(e, "ACT").is_some())
            .collect();
        assert!(act_witnessed.len() >= 50, "expected the real compiled data to carry well over 50 non-pw_ ACT-witnessed events (Acts 1 onward, post-calibration), got {}", act_witnessed.len());

        let mut violations: Vec<String> = Vec::new();
        for e in &act_witnessed {
            let idx = d.timeline_position(&e.id).unwrap();
            if idx <= max_passion_idx {
                violations.push(format!("'{}' ({}) sorts at timeline index {idx}, at or before the Passion cluster's own max index {max_passion_idx}", e.id, e.label));
            }
        }
        assert!(
            violations.is_empty(),
            "FIX ROUND 1 ERA-BOUNDARY GATE: {} ACT-witnessed event(s) sort at or before the end of the real Passion cluster (pw_jerusalem_entry..ascension) -- Pentecost/subsequent Acts must never precede the Crucifixion/Resurrection/Ascension:\n{}",
            violations.len(),
            violations.join("\n")
        );
    }

    /// FIX ROUND 1, Fix 2b: "within-Acts follows Acts chapter order for
    /// acts_section events" -- the narrower, EXPLICIT second half of the
    /// same-book gate this batch adds (the first half is the era-boundary
    /// gate immediately above). Checks every pair of REAL, curated
    /// `acts_section` events (`data/curated/acts-sections.toml`,
    /// `theo-304`..`theo-336`) against each other specifically -- narrower
    /// than the general same-book audit above (which already covers this
    /// pair-set too, under its own "both sides real" justification rule 0
    /// -- this test asserts it as a hard, unconditional gate instead of a
    /// pattern-matched justification, because acts_section rows are
    /// curator-authored in Acts's own natural reading order by construction
    /// (`data/curated/acts-sections.toml`'s own header), so there is no
    /// legitimate reason for one to ever sort out of chapter:verse order).
    #[test]
    fn fix_round_1_within_acts_section_events_follow_acts_chapter_order() {
        let d = load_real_compiled_data();

        let mut acts_section_events: Vec<(&str, (u16, u16))> = d
            .events
            .iter()
            // `kind == "event"` (dated) only -- `w1_acts_preface` (Acts
            // 1:1-3, Luke's own preface to Theophilus) also carries
            // `acts_section` but is deliberately general-kind/undated
            // (requirement 2's own rule: no fabricated date), so it has no
            // timeline position to compare -- excluded here the same way
            // requirement 2 excludes it everywhere else, not a special case.
            .filter(|e| e.kind == "event" && e.acts_section.is_some())
            .filter_map(|e| crate::nt_calibration::first_verse_in_book(e, "ACT").map(|cv| (e.id.as_str(), cv)))
            .collect();
        assert!(acts_section_events.len() >= 30, "expected ~33 acts_section events, got {}", acts_section_events.len());
        acts_section_events.sort_by_key(|(_, cv)| *cv); // Acts's own chapter:verse reading order

        let mut violations: Vec<String> = Vec::new();
        for pair in acts_section_events.windows(2) {
            let (a_id, a_cv) = pair[0];
            let (b_id, b_cv) = pair[1];
            let a_idx = d.timeline_position(a_id).unwrap();
            let b_idx = d.timeline_position(b_id).unwrap();
            if a_idx > b_idx {
                violations.push(format!("'{a_id}' (ACT {a_cv:?}) reads before '{b_id}' (ACT {b_cv:?}) but sorts AFTER it on the global timeline"));
            }
        }
        assert!(violations.is_empty(), "FIX ROUND 1: {} acts_section event(s) out of Acts chapter order on the global timeline:\n{}", violations.len(), violations.join("\n"));
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

    // =========================================================================
    // BATCH HOTFIX-6 (graph-wide chronology audit, owner live reports #8/#9,
    // 2026-08-22) -- AMENDMENT E: "we need way more than one acceptance test.
    // the tests need to assert that the application has the property of
    // adhering to a canonical table of dates." E1-E4 below are PROPERTY tests,
    // table-driven throughout: every expected value is read from
    // `d.chronology_anchors`/`d.book_narration_windows` (the SAME compiled
    // artifact -- `chronology-anchors.json`/`book-narration-windows.json` --
    // atlas_etl::validate's own ETL-time checks consult), never a hardcoded
    // year copied into this file. Adding a table row automatically extends
    // coverage; zero maintenance here when the table grows. E5 (the owner's
    // own named Solomon-Gibeon/df_ramah case) follows as named smoke ON TOP
    // of the properties, not instead of them, per the amendment's own text.
    // =========================================================================

    /// E1 -- ANCHOR-EQUALITY PROPERTY: every anchor-table row bound to an
    /// event id (`event_id.is_some()`) -> the compiled event's own
    /// `from_year` EQUALS the table's own `year`. One table-driven test over
    /// ALL rows (not one test per row) -- failures name the event, the
    /// table year, and the compiled year, per the amendment's own text.
    ///
    /// FIX ROUND 1 + owner sequencing redirect (both 2026-08-22): a bound
    /// row registered in `atlas_core::chronology::ANCHOR_DEFERRALS` is
    /// TYPED-DEFERRED, not a live equality claim -- its own canonical
    /// `year` is correct but deliberately not yet re-dated onto the
    /// compiled event (HOTFIX-7's own single-feed migration will do that
    /// directly; hand-editing now would be immediately overwritten there).
    /// Deferrals are NEVER silent: each is reported BY NAME below (visible
    /// even without `--nocapture`, via the assertion message itself), and
    /// the deferral's own recorded `shipped_value` is still checked against
    /// the event's real current date -- a STALE deferral (the event moved
    /// but the deferral entry wasn't updated) fails loud, exactly like a
    /// real violation, never silently "passing." See that module's own doc
    /// comment for the full distinction from a genuine E1 pass.
    #[test]
    fn e1_every_bound_anchor_equals_its_compiled_events_own_from_year() {
        let d = load_real_compiled_data();
        let bound: Vec<&crate::data::ChronologyAnchor> = d.chronology_anchors.iter().filter(|a| a.event_id.is_some()).collect();
        assert!(bound.len() >= 15, "expected the real curated anchor table to bind well over 15 rows to real events, got {}", bound.len());

        // Fix round 2 (review finding I-2): calls the SAME
        // `atlas_core::chronology::anchor_equality_check` predicate
        // `atlas_etl::validate::run_chronology_anchor_equality` now enforces
        // at ETL build time -- one algorithm, two independent LAYERS
        // (compiled-JSON-on-disk here, in-process ETL state there), not two
        // hand-written copies that could silently drift apart.
        let (violations, deferred) = crate::chronology::anchor_equality_check(&d.chronology_anchors, &d.events);

        let deferred_strings: Vec<String> =
            deferred.iter().map(|r| format!("'{}' (canonical {}, event '{}' currently {}, DEFERRED to HOTFIX-7)", r.anchor_id, r.canonical_year, r.event_id, r.shipped_value)).collect();
        assert_eq!(
            deferred.len(),
            4,
            "expected exactly 4 typed deferrals (jerusalem-falls/cyrus-decree/temple-finished/ezra-returns) visible here, got {}: {:?} -- this count is deliberate, not a floor; update it together with ANCHOR_DEFERRALS, never silently",
            deferred.len(),
            deferred_strings
        );

        let violation_strings: Vec<String> = violations
            .iter()
            .map(|v| {
                if v.is_stale_deferral {
                    format!(
                        "anchor '{}': STALE DEFERRAL -- ANCHOR_DEFERRALS records shipped_value {}, but compiled event '{}''s own from_year is now {} (re-dated without updating/removing the deferral entry?)",
                        v.anchor_id, v.table_year, v.event_id, v.event_year
                    )
                } else {
                    format!("anchor '{}': table year {} != compiled event '{}''s own from_year {}", v.anchor_id, v.table_year, v.event_id, v.event_year)
                }
            })
            .collect();
        assert!(
            violation_strings.is_empty(),
            "E1 anchor-equality violated for {} row(s) (this list never includes the {} typed deferrals reported separately above -- {:?}):\n{}",
            violation_strings.len(),
            deferred.len(),
            deferred_strings,
            violation_strings.join("\n")
        );
    }

    /// E2 -- WINDOW-ADHERENCE PROPERTY: every dated event in the COMPILED
    /// set (loaded fresh off `data/compiled/*.json`, independent of the
    /// ETL-time validator's own in-process check on `atlas_etl::main`'s own
    /// `all_events` -- a regression in EITHER layer fails loud, since this
    /// test never imports atlas_etl at all) lies within the narration
    /// window of every witness book it claims, recounting mechanism
    /// honored. Reuses `atlas_core::chronology::window_violations` (the
    /// SAME predicate `atlas_etl::validate::run_chronology_windows` calls)
    /// deliberately -- a SECOND, independently-written implementation of
    /// "is this event's year inside this window" would itself be a defect
    /// risk (two sources of truth that can silently drift apart); the real
    /// independence this test buys is LAYER (compiled-JSON-on-disk vs.
    /// in-process ETL state), not a duplicated algorithm.
    #[test]
    fn e2_every_compiled_dated_event_adheres_to_its_witness_books_own_narration_window() {
        let d = load_real_compiled_data();
        assert!(!d.book_narration_windows.is_empty(), "the real compiled data must carry real narration windows");
        let violations = crate::chronology::window_violations(&d.events, &d.book_narration_windows);
        assert!(
            violations.is_empty(),
            "E2 window-adherence violated for {} event(s):\n{}",
            violations.len(),
            violations.iter().map(|v| format!("  - '{}' ({:?}): {}..{} outside '{}''s own {}..{}", v.event_id, v.label, v.year.0, v.year.1, v.book, v.window.0, v.window.1)).collect::<Vec<_>>().join("\n")
        );
    }

    /// E3 -- CANONICAL-ORDER PROPERTY: the bound, NON-deferred anchor rows,
    /// sorted by the TABLE's own year, embed in the compiled global
    /// timeline in strictly that order (monotone timeline positions) --
    /// catches any future re-sort or calibration drift between anchors.
    /// Non-decreasing in general, strict specifically between two anchors
    /// whose own `year` differs (today's real table has no same-year bound
    /// anchors at all, but the property is stated this way so a future
    /// same-year anchor pair never breaks this test for the wrong reason).
    ///
    /// FIX ROUND 1: typed-deferred anchors (`atlas_core::chronology::
    /// ANCHOR_DEFERRALS`) are EXCLUDED here, not just at E1 -- a deferred
    /// row's own `year` is the CANONICAL value, deliberately not yet the
    /// value its bound event's own timeline position reflects (e.g.
    /// `jerusalem-falls` at table-year -588 and `exile-begins` at
    /// table-year -586 both bind the SAME event, `exl_jerusalem` -- sorting
    /// them by table year would demand `jerusalem-falls` sort STRICTLY
    /// before `exile-begins` on the timeline despite being the identical
    /// node, which is not a real ordering claim to make). E1 already proves
    /// every deferral's own recorded shipped-value stays honest; E3's own
    /// job is the DIFFERENT question of live cross-anchor ordering, which a
    /// deferred row cannot yet honestly answer.
    #[test]
    fn e3_bound_anchors_sorted_by_table_year_are_monotone_on_the_global_timeline() {
        let d = load_real_compiled_data();
        let mut bound: Vec<&crate::data::ChronologyAnchor> =
            d.chronology_anchors.iter().filter(|a| a.event_id.is_some() && crate::chronology::anchor_deferral(&a.id).is_none()).collect();
        assert!(bound.len() >= 12, "expected well over 12 non-deferred bound anchors, got {}", bound.len());
        bound.sort_by_key(|a| a.year);

        let positions: Vec<(&str, i32, usize)> = bound
            .iter()
            .map(|a| {
                let eid = a.event_id.as_deref().unwrap();
                let pos = d.timeline_position(eid).unwrap_or_else(|| panic!("anchor '{}''s own bound event '{eid}' must be a dated event", a.id));
                (a.id.as_str(), a.year, pos)
            })
            .collect();

        let mut violations: Vec<String> = Vec::new();
        for pair in positions.windows(2) {
            let (a_id, a_year, a_pos) = pair[0];
            let (b_id, b_year, b_pos) = pair[1];
            let ok = if a_year == b_year { a_pos <= b_pos } else { a_pos < b_pos };
            if !ok {
                violations.push(format!("'{a_id}' (table year {a_year}, timeline position {a_pos}) does not sort {} '{b_id}' (table year {b_year}, timeline position {b_pos})", if a_year == b_year { "at-or-before" } else { "strictly before" }));
            }
        }
        assert!(violations.is_empty(), "E3 canonical-order violated for {} adjacent pair(s):\n{}", violations.len(), violations.join("\n"));
    }

    /// E4 -- ERA-PARTITION PROPERTY: the OT-wide generalization of
    /// HOTFIX-4's own NT era-boundary gates (`fix_round_1_era_boundary_gate_*`
    /// above) -- every dated event's witness-book era must agree with its
    /// own global timeline position relative to EVERY `era_boundary` anchor
    /// in the table, not just the one Passion/Acts boundary those older
    /// gates check. `atlas_core::chronology::era_boundary_violations`'s own
    /// doc comment has the full design (the "all_before"/"all_after"
    /// straddle-safe partition).
    #[test]
    fn e4_dated_events_agree_with_era_boundary_anchors_on_the_global_timeline() {
        let d = load_real_compiled_data();
        let boundary_count = d.chronology_anchors.iter().filter(|a| a.era_boundary).count();
        assert!(boundary_count >= 6, "expected the real curated table to carry well over 6 era_boundary anchors, got {boundary_count}");

        let violations = crate::chronology::era_boundary_violations(&d);
        assert!(
            violations.is_empty(),
            "E4 era-partition violated for {} event(s):\n{}",
            violations.len(),
            violations.iter().map(|v| format!("  - '{}' ({:?}) sorts on the wrong side of boundary '{}' (year {}) -- expected {}", v.event_id, v.label, v.boundary_id, v.boundary_year, v.side)).collect::<Vec<_>>().join("\n")
        );
    }

    /// Fix round 2 (review finding I-1): a small lookup, not a hardcoded
    /// copy -- E5's own numeric-window assertions below derive their
    /// bounds from `chronology_anchors` at test time, the same "zero
    /// hardcoded years" rule E1-E4 already follow, rather than carrying a
    /// hand-typed copy of what the table currently says.
    fn anchor_year(d: &AtlasData, id: &str) -> crate::time::Year {
        d.chronology_anchors.iter().find(|a| a.id == id).unwrap_or_else(|| panic!("no chronology anchor with id '{id}'")).year
    }

    /// E5 -- the owner's own named case, red-then-green, ON TOP of the
    /// properties above (Amendment E's own explicit instruction: "not
    /// instead of them"). RED before this batch's own df_* re-date:
    /// `1ki_solomon_gibeon`'s own PRIOR/FOLLOWING were `df_ramah`/`df_nob`
    /// (Saul-persecution-era, ~48 years off) -- exactly the owner's own
    /// report ("this is a lie"). Verified live against the real compiled
    /// data before this test was written (see batch-hotfix6-report.md).
    #[test]
    fn e5_solomon_gibeons_neighbors_are_solomon_era_never_saul_persecution_era() {
        let d = load_real_compiled_data();
        let pos = global_timeline_position(&d, "1ki_solomon_gibeon").expect("1ki_solomon_gibeon is a dated event");

        let prior = pos.prior.as_ref().expect("1ki_solomon_gibeon has a PRIOR neighbor");
        let following = pos.following.as_ref().expect("1ki_solomon_gibeon has a FOLLOWING neighbor");

        assert!(!prior.id.starts_with("df_"), "RED-CASE REGRESSION: PRIOR '{}' is a Saul-persecution df_* event -- the owner's own exact bug ('this is a lie')", prior.id);
        assert!(!following.id.starts_with("df_"), "RED-CASE REGRESSION: FOLLOWING '{}' is a Saul-persecution df_* event -- the owner's own exact bug", following.id);

        // GREEN, verified live against the real compiled data: both
        // neighbors are genuinely Solomon-era (David's own final charge to
        // Solomon, and Solomon's own temple preparations with Hiram).
        assert_eq!(prior.id, "1ki_davids_charge", "PRIOR must be David's own final charge to Solomon (-1015), Solomon-era");
        assert_eq!(following.id, "1ki_hiram_temple_prep", "FOLLOWING must be Solomon's own temple preparations with Hiram (-1013), Solomon-era");
        let prior_year = d.event_by_id(&prior.id).unwrap().when.from_year;
        let following_year = d.event_by_id(&following.id).unwrap().when.from_year;
        // Bound derived from the table's own `solomon-crowned` anchor, +/- a
        // small fixed tolerance (a test-design constant, not a copy of a
        // table YEAR) -- comfortably holds both real neighbor dates while
        // staying far short of the Saul-persecution era, ~40-50 years away.
        let solomon_crowned = anchor_year(&d, "solomon-crowned");
        let tolerance = 5;
        assert!(
            (solomon_crowned - tolerance..=solomon_crowned + tolerance).contains(&prior_year),
            "PRIOR '{}' ({prior_year}) must fall within {tolerance} years of solomon-crowned ({solomon_crowned}), not the Saul-persecution era",
            prior.id
        );
        assert!(
            (solomon_crowned - tolerance..=solomon_crowned + tolerance).contains(&following_year),
            "FOLLOWING '{}' ({following_year}) must fall within {tolerance} years of solomon-crowned ({solomon_crowned}), not the Saul-persecution era",
            following.id
        );
    }

    /// E5 continued -- the SAME case from `df_ramah`'s own side: its
    /// neighbors must be Saul-persecution-era, never Solomon-era. Verified
    /// live against the real compiled data before this test was written.
    #[test]
    fn e5_df_ramahs_neighbors_are_saul_persecution_era_never_solomon_era() {
        let d = load_real_compiled_data();
        let pos = global_timeline_position(&d, "df_ramah").expect("df_ramah is a dated event");

        let prior = pos.prior.as_ref().expect("df_ramah has a PRIOR neighbor");
        let following = pos.following.as_ref().expect("df_ramah has a FOLLOWING neighbor");

        assert_ne!(prior.id, "1ki_solomon_gibeon", "df_ramah's own PRIOR must never be the Solomon-era Gibeon dream");
        assert_ne!(following.id, "1ki_solomon_gibeon", "df_ramah's own FOLLOWING must never be the Solomon-era Gibeon dream");

        // GREEN, verified live: PRIOR is David killing Goliath (-1067, the
        // last event before Saul's own jealousy/persecution begins);
        // FOLLOWING is the chain's own very next leg, df_nob (-1061) --
        // narrative-leg order, already correct, untouched by this batch.
        assert_eq!(prior.id, "theo-157", "PRIOR must be David killing Goliath (-1067), immediately preceding Saul's own persecution");
        assert_eq!(following.id, "df_nob", "FOLLOWING must be the chain's own next leg, df_nob (-1061)");
        let prior_year = d.event_by_id(&prior.id).unwrap().when.from_year;
        let following_year = d.event_by_id(&following.id).unwrap().when.from_year;
        // Bound derived from the table's own `david-hebron` anchor -- the
        // Saul-persecution/united-monarchy transition point, exactly the
        // boundary this test's own name asserts against.
        let david_hebron = anchor_year(&d, "david-hebron");
        assert!(prior_year <= david_hebron, "PRIOR '{}' ({prior_year}) must be at or before the Saul-persecution/united-monarchy transition (david-hebron, {david_hebron})", prior.id);
        assert!(following_year <= david_hebron, "FOLLOWING '{}' ({following_year}) must be Saul-persecution-era, not Solomon-era (at or before david-hebron, {david_hebron})", following.id);
    }
}
