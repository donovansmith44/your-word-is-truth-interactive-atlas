mod common;

use common::source_scan::{read, rust_sources_under, shown, string_literals_of};

const SERVED_CRATES: [&str; 3] = ["server/atlas-graph/src", "server/atlas-contract/src", "server/atlas-server/src"];

fn counts_over_the_adjacency(literal: &str) -> bool {
    let sql = literal.to_lowercase();
    sql.contains("edge_index") && sql.contains("count(")
}

fn edge_counting_queries_in(source: &str) -> Vec<String> {
    string_literals_of(source).into_iter().filter(|literal| counts_over_the_adjacency(literal)).collect()
}

#[test]
fn no_served_crate_counts_edges_over_the_adjacency() {
    // Arrange
    let sources = rust_sources_under(&SERVED_CRATES);

    // Act
    let offences: Vec<String> = sources
        .iter()
        .flat_map(|path| edge_counting_queries_in(&read(path)).into_iter().map(move |query| format!("{}: {query}", shown(path))))
        .collect();

    // Assert
    assert_eq!((sources.is_empty(), offences), (false, Vec::<String>::new()));
}

#[test]
fn a_count_over_the_adjacency_is_found_in_a_string_and_not_in_a_comment_or_another_table() {
    // Arrange
    let source = "// SELECT COUNT(*) FROM edge_index\nfn f() { let a = \"SELECT rel, COUNT(DISTINCT edge_id) FROM all_edge_index WHERE subject = ?1\"; let b = r#\"SELECT count(*) FROM node\"#; let c = \"SELECT rel, dir, count FROM all_edge_count\"; }\n";

    // Act
    let found = edge_counting_queries_in(source);

    // Assert
    assert_eq!(found, vec!["SELECT rel, COUNT(DISTINCT edge_id) FROM all_edge_index WHERE subject = ?1".to_string()]);
}
