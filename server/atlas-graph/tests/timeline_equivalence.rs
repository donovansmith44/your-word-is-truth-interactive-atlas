mod common;

use std::collections::HashMap;

use atlas_core::data::AtlasData;
use atlas_graph::event_world::{self};
use atlas_graph::Chronology;
use atlas_graph_types::chrono::{temporal_order, ResolvedDate, ResolvedPlacement};
use atlas_graph_types::id::EventId;

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
    let atlas = common::real_atlas();

    let (graph, _kjv_stats, ew_stats, chrono) = common::kjv_and_atlas_build(&[]);
    assert!(ew_stats.dated_events >= 450, "expected the real compiled event set to carry well over 450 dated events, got {}", ew_stats.dated_events);
    assert_eq!(graph.dated_by.len(), ew_stats.dated_events, "every dated event must carry exactly one DatedBy row");

    let anchor_years = Chronology::anchor_years(atlas);
    let event_years = Chronology::event_years(atlas);

    let mut resolved: HashMap<String, ResolvedPlacement> = HashMap::new();
    for row in &graph.dated_by {
        let event_id = row.event.0.clone();
        let tp = event_world::resolve_timepoint(&row.placement, &anchor_years, &event_years)
            .unwrap_or_else(|| panic!("dated_by row for '{event_id}' must resolve to a real TimePoint"));
        let expected_year = atlas.event_by_id(&event_id).unwrap_or_else(|| panic!("'{event_id}' must be a real event")).when.from_year;
        assert_eq!(tp.year.get(), expected_year, "'{event_id}': the STORED graph placement must resolve to its own true source year");

        let seq = chrono.resolved[&event_id].seq;
        let basis = row.basis;
        resolved.insert(event_id, ResolvedPlacement { date: ResolvedDate { from: tp, to: tp }, seq, basis });
    }
    assert_eq!(resolved.len(), ew_stats.dated_events);

    let mut graph_order: Vec<String> = resolved.keys().cloned().collect();
    graph_order.sort_by(|a, b| temporal_order(&resolved[a], &resolved[b]));

    let old_order = old_timeline_order(atlas);

    assert_eq!(graph_order.len(), old_order.len(), "the graph-derived and old-resolver orders must cover the SAME number of dated events");
    assert_eq!(graph_order, old_order, "THE ACCEPTANCE CENTERPIECE: the graph's total order over ResolvedPlacements must equal the old resolver's timeline_order EXACTLY, event-for-event, in the same sequence");

    for id in &old_order {
        assert!(atlas.event_by_id(id).is_some(), "'{id}' from the OLD order must be a real event");
        let _ = EventId::new(id.clone());
    }
}
