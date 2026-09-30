mod common;

#[test]
fn justified_by_grand_total_over_the_real_compiled_data_is_pinned() {
    let inputs = common::PipelineInputs::read();

    let ctx = inputs.run();

    println!("EDGE-1a/JB-1 justified-by grand total (real compiled data): {}", ctx.justified_by_count);
    assert_eq!(
        ctx.justified_by_count, 76,
        "justified-by grand total regressed or grew over the real committed data -- if this is a genuine data/curated change (a new DatedBy/fulfills/typology/named_after row, or a new ground on an existing row), update this pin in the SAME commit"
    );
}

#[test]
fn an_edge_position_is_indexed_under_justified_by_and_no_other_relation() {
    use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
    use atlas_graph_types::id::Position;
    use std::collections::BTreeSet;

    // Arrange
    let inputs = common::PipelineInputs::read();
    let ctx = inputs.run();
    let holds_an_edge = |side: &std::collections::BTreeMap<Position, atlas_graph_types::explore::Frontier>| {
        side.iter().any(|(position, frontier)| matches!(position, Position::Edge(_)) || frontier.edges().any(|e| matches!(e.node, Position::Edge(_))))
    };

    // Act
    let directed = ctx.graph.indexes.iter().flat_map(|(rel, index)| {
        [(Direction::Forward, &index.fwd), (Direction::Inverse, &index.inv)].into_iter().filter(|(_, side)| holds_an_edge(side)).map(move |(dir, _)| EdgeKind::Directed(*rel, dir))
    });
    let symmetric = ctx.graph.symmetric_indexes.iter().filter(|(_, index)| holds_an_edge(&index.fwd)).map(|(rel, _)| EdgeKind::Symmetric(*rel));
    let kinds_with_an_edge_position: BTreeSet<EdgeKind> = directed.chain(symmetric).collect();

    // Assert
    assert_eq!(
        kinds_with_an_edge_position,
        BTreeSet::from([EdgeKind::Directed(RelationId::JustifiedBy, Direction::Forward), EdgeKind::Directed(RelationId::JustifiedBy, Direction::Inverse)])
    );
}
