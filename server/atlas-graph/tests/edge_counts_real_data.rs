mod common;

use std::collections::BTreeSet;

use atlas_graph::sqlite::reload::committed_graph;
use atlas_graph_types::adjacency::EdgeSummary;
use atlas_graph_types::edge::{Direction, EdgeKind};
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;

#[test]
fn every_compiled_edge_count_is_the_count_recomputed_from_the_artifacts_rows() {
    // Arrange
    let (g, snap) = committed_graph(&common::compiled_dir()).unwrap();
    let inhabited: BTreeSet<EdgeKind> = g
        .indexes
        .iter()
        .flat_map(|(rel, ix)| [(Direction::Forward, ix.fwd.is_empty()), (Direction::Inverse, ix.inv.is_empty())].into_iter().filter(|(_, empty)| !empty).map(move |(dir, _)| EdgeKind::Directed(*rel, dir)))
        .chain(g.symmetric_indexes.iter().filter(|(_, ix)| !ix.fwd.is_empty()).map(|(rel, _)| EdgeKind::Symmetric(*rel)))
        .collect();

    // Act
    let mut served_kinds: BTreeSet<EdgeKind> = BTreeSet::new();
    let mut disagreeing: Vec<(Position, EdgeSummary, EdgeSummary)> = Vec::new();
    for position in g.positions() {
        let served = snap.edge_summary(&position);
        let recounted = g.edge_summary(&position);
        served_kinds.extend(served.keys().copied());
        if served != recounted {
            disagreeing.push((position, served, recounted));
        }
    }

    // Assert
    assert_eq!((disagreeing.into_iter().take(5).collect::<Vec<_>>(), served_kinds), (Vec::new(), inhabited));
}
