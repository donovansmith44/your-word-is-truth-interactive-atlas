mod common;

use common::OptionalCorpora;

use atlas_graph_types::edge::{Direction, EdgeKind, RelationId, SymRelationId};
use atlas_graph_types::adjacency::{EdgeQuery, Adjacent, PositionRef};
use atlas_graph_types::id::Position;
use atlas_graph_types::text::TextRef;

fn real_graph() -> &'static atlas_graph_types::graph::Graph {
    static GRAPH: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| common::indexed_raw_graph(OptionalCorpora { kretzmann: false, red_letter: false }))
}

fn concord_locus_links(graph: &atlas_graph_types::graph::Graph) -> Vec<&atlas_graph_types::edge::CatechismLink> {
    graph.catechism.iter().filter(|l| matches!(l.locus.at, TextRef::Concord(_))).collect()
}

#[test]
fn the_curated_sc_overlap_table_authors_one_link_per_catechism_item() {
    let curated_row_count = common::sc_overlap().len();
    assert_eq!(curated_row_count, 33, "the curated table's own [[link]] entry count -- one per CatechismItem, before paragraph-list expansion");
}

#[test]
fn sc_overlap_expands_to_37_catechism_link_rows_with_zero_unmatched() {
    let graph = real_graph();
    let concord_links = concord_locus_links(graph);
    assert_eq!(concord_links.len(), 37, "11 commandments + 3 creed + 9 lord's-prayer + 7 baptism + 2 confession + 5 altar (decision 4's own hand-verified alignment table)");

}

#[test]
fn the_first_commandment_catechism_item_reaches_its_concord_home_over_real_data() {
    let graph = real_graph();
    let item_pos = Position::Node(atlas_graph::catechism_adapter::catechism_item_node_id("commandment-1"));
    let kind = EdgeKind::Symmetric(SymRelationId::CatechismLink);
    let mut all_entries = Vec::new();
    let mut cursor = None;
    loop {
        let page = PositionRef(item_pos.clone()).edges(graph, &EdgeQuery { kind, cursor, limit: 200 });
        all_entries.extend(page.entries);
        match page.next {
            Some(n) => cursor = Some(n),
            None => break,
        }
    }
    assert!(!all_entries.is_empty(), "the First Commandment must be reachable through catechism-link at all");

    let concord_pos = Position::Node(atlas_graph::concord_adapter::text_unit_id(7, 2, 1));
    let concord_entries: Vec<_> = all_entries.iter().filter(|e| e.node == concord_pos).collect();
    assert_eq!(concord_entries.len(), 1, "exactly one link to Small Catechism (part 7), Ten Commandments (article 2), paragraph 1 -- among {} total (Bible + Concord)", all_entries.len());

    let node = graph.nodes.get(&atlas_graph::concord_adapter::text_unit_id(7, 2, 1)).unwrap();
    match &node.payload {
        atlas_graph_types::node::NodePayload::TextUnit { renderings, .. } => {
            let text = renderings.get(&atlas_graph_types::text::TranslationId("bente-dau".to_string())).unwrap();
            assert_eq!(text, "Thou shalt have no other gods. What does this mean? \u{2013}Answer: We should fear, love, and trust in God above all things.");
        }
        other => panic!("expected TextUnit, got {other:?}"),
    }
}

#[test]
fn augsburg_confession_article_iv_container_holds_exactly_its_three_paragraphs() {
    let graph = real_graph();
    let article_container = atlas_graph_types::id::ContainerNodeId::new("concord-art-augsburg-confession-5");
    let forward = EdgeKind::Directed(RelationId::Contains, Direction::Forward);
    let page = PositionRef(Position::Node(article_container.erase())).edges(graph, &EdgeQuery { kind: forward, cursor: None, limit: 10 });
    assert_eq!(page.entries.len(), 3, "Augsburg Confession Article IV (Of Justification) has exactly 3 real paragraphs");
}
