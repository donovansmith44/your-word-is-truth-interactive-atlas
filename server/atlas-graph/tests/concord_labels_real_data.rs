mod common;

use common::OptionalCorpora;

use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::Position;

fn real_graph() -> &'static Graph {
    static GRAPH: std::sync::OnceLock<Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| common::indexed_raw_graph(OptionalCorpora { kretzmann: false, red_letter: false }))
}

#[test]
fn a_concord_paragraph_is_labelled_by_its_triglot_code_and_by_its_article_where_numbering_differs() {
    // Arrange
    let g = real_graph();
    let paragraphs = [(4, 4, 48), (3, 29, 44), (6, 1, 47), (10, 5, 61), (7, 2, 1), (1, 1, 3)];
    // Act
    let labels: Vec<Option<&str>> = paragraphs
        .iter()
        .map(|(part, article, paragraph)| g.labels.get(&Position::Node(atlas_graph::concord_adapter::text_unit_id(*part, *article, *paragraph))).map(String::as_str))
        .collect();
    // Assert
    assert_eq!(labels, vec![Some("Ap IV 48"), Some("AC XXVIII 44"), Some("Tr 47"), Some("SD III 61"), Some("SC I"), Some("Pref")]);
}
