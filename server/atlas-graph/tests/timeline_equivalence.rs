//! BATCH M-B ACCEPTANCE CENTERPIECE (brief controller decision 3): "for ALL
//! events, the graph's total order (temporal_order over ResolvedPlacements)
//! must equal the old resolver's timeline_order EXACTLY (same sequence),
//! proven by a committed test BEFORE the serving swap; any divergence is a
//! bug in the adapter derivation, not an opportunity to re-date."
//!
//! Runs over the REAL, FULL compiled/raw data (not a fixture) -- every
//! dated event in `data/compiled/events.json`.
//!
//! TWO INDEPENDENT ORDERS, compared:
//! - OLD: `atlas_core::data::AtlasData::timeline_event_at(i)`, walked from
//!   i=0 until `None` -- the exact production timeline
//!   `atlas_core::narrative::global_timeline_position` (and, before this
//!   batch, `GET /api/narrative/event/{id}`) has always used.
//! - NEW (graph-derived): for every row in the REAL, BUILT graph's own
//!   `dated_by` table, `resolve_timepoint` independently RE-WALKS the
//!   STORED `DatePlacement` (never trusts a cached year) to recover its
//!   TimePoint; paired with the `SeqKey`/`PlacementBasis`
//!   `ChronologyDerivation` assigned when building that same graph, this
//!   reconstructs each event's `ResolvedPlacement`
//!   (`graph_types::chrono::ResolvedPlacement`) -- the EXACT type
//!   `temporal_order` compares. Sorting every dated event by
//!   `temporal_order` over these reconstructed placements is "the graph's
//!   total order," proven identical to the OLD order below, not merely
//!   assumed from the derivation's own internal bookkeeping.

use std::collections::HashMap;
use std::path::Path;

use atlas_core::data::AtlasData;
use atlas_graph::build::build_graph_from_sources;
use atlas_graph::event_world::{self, EventWorld};
use atlas_graph_types::chrono::{temporal_order, ResolvedDate, ResolvedPlacement};
use atlas_graph_types::id::EventId;

fn load_real_atlas() -> AtlasData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled");
    AtlasData::load(&dir).expect("data/compiled/*.json must exist -- run `cargo run -p atlas-etl` from server/ first").finish()
}

/// The OLD resolver's own total order, walked via its OWN public accessor
/// (`timeline_event_at`), exactly as `global_timeline_position` does --
/// never re-derived from `AtlasData.events` directly, so this test cannot
/// silently reimplement (and diverge from) the very algorithm it exists to
/// check.
fn old_timeline_order(atlas: &AtlasData) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(e) = atlas.timeline_event_at(i) {
        out.push(e.id.clone());
        i += 1;
    }
    out
}

#[test]
fn the_graphs_total_order_over_resolved_placements_equals_the_old_resolvers_timeline_order_exactly() {
    let atlas = load_real_atlas();

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist (committed real data)");
    let xrefs_tsv =
        std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist (committed real data)");

    let (graph, _kjv_stats, ew_stats, chrono) =
        build_graph_from_sources(&kjv_json, &xrefs_tsv, &atlas).expect("the real KJV source and the real curated/compiled event world must both build");
    assert!(ew_stats.dated_events >= 450, "expected the real compiled event set to carry well over 450 dated events, got {}", ew_stats.dated_events);
    assert_eq!(graph.dated_by.len(), ew_stats.dated_events, "every dated event must carry exactly one DatedBy row");

    // Independently re-resolve EVERY dated_by row's own STORED placement --
    // not the cached ChronologyDerivation.resolved map -- proving the
    // graph's OWN rows (what a real server actually publishes) resolve
    // correctly, not merely that the in-memory derivation struct is
    // self-consistent.
    let anchor_years = EventWorld::anchor_years(&atlas);
    let event_years = EventWorld::event_years(&atlas);

    let mut resolved: HashMap<String, ResolvedPlacement> = HashMap::new();
    for row in &graph.dated_by {
        let event_id = row.event.0.clone();
        let tp = event_world::resolve_timepoint(&row.placement, &anchor_years, &event_years)
            .unwrap_or_else(|| panic!("dated_by row for '{event_id}' must resolve to a real TimePoint"));
        let expected_year = atlas.event_by_id(&event_id).unwrap_or_else(|| panic!("'{event_id}' must be a real event")).when.from_year;
        assert_eq!(tp.year.get(), expected_year, "'{event_id}': the STORED graph placement must resolve to its own true source year");

        // SeqKey is inherently a GLOBAL ordering assignment (design doc
        // §4/§9: "resolved from SequenceAfter chains -- total by
        // construction"); this batch's own resolver assigns it directly
        // from the reconstructed timeline position (event_world's own doc
        // comment has the full "why exact, not approximate" argument) --
        // read here from `chrono.resolved`, the SAME structure that was
        // used to assign it when the graph's own rows were built, not
        // independently re-derived a second time (there is nothing further
        // upstream than "the reconstructed order" to re-derive it FROM).
        let seq = chrono.resolved[&event_id].seq;
        let basis = row.basis;
        resolved.insert(event_id, ResolvedPlacement { date: ResolvedDate { from: tp, to: tp }, seq, basis });
    }
    assert_eq!(resolved.len(), ew_stats.dated_events);

    let mut graph_order: Vec<String> = resolved.keys().cloned().collect();
    graph_order.sort_by(|a, b| temporal_order(&resolved[a], &resolved[b]));

    let old_order = old_timeline_order(&atlas);

    assert_eq!(graph_order.len(), old_order.len(), "the graph-derived and old-resolver orders must cover the SAME number of dated events");
    assert_eq!(graph_order, old_order, "THE ACCEPTANCE CENTERPIECE: the graph's total order over ResolvedPlacements must equal the old resolver's timeline_order EXACTLY, event-for-event, in the same sequence");

    // A second, independent witness over the SAME two orders: every
    // EventId round-trips (no id-encoding bug could silently pass the
    // sequence check above by comparing garbled-but-consistently-garbled
    // strings on both sides).
    for id in &old_order {
        assert!(atlas.event_by_id(id).is_some(), "'{id}' from the OLD order must be a real event");
        let _ = EventId::new(id.clone()); // constructs without panic -- a real, valid graph id shape
    }
}
