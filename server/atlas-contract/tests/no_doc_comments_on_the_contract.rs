mod common;

use common::source_scan::{read, rust_sources_under, scanned_text, shown};

const CONTRACT_SOURCES: [&str; 1] = ["server/atlas-contract/src/wire"];

const SCHEMA_ROOTS: [&str; 6] = [
    "server/atlas-contract/src",
    "server/atlas-core/src",
    "server/atlas-graph/src",
    "server/atlas-server/src",
    "graph-types/src",
    "server/atlas-cli/src",
];

fn is_doc_comment(comment: &str) -> bool {
    comment.starts_with("///") && !comment.starts_with("////") || comment.starts_with("//!") || comment.starts_with("/**") && !comment.starts_with("/***") || comment.starts_with("/*!")
}

fn doc_comments_in(source: &str) -> Vec<usize> {
    scanned_text(source).into_iter().filter(|(_, comment)| is_doc_comment(comment)).map(|(line, _)| line).collect()
}

fn derives_a_schema(source: &str) -> bool {
    source.contains("ToSchema")
}

#[test]
fn no_wire_source_and_no_schema_deriving_source_carries_a_doc_comment() {
    // Arrange
    let wire = rust_sources_under(&CONTRACT_SOURCES);
    let schema_sources: Vec<_> = rust_sources_under(&SCHEMA_ROOTS).into_iter().filter(|path| derives_a_schema(&read(path))).collect();

    // Act
    let offences: Vec<String> = wire
        .iter()
        .chain(schema_sources.iter())
        .flat_map(|path| doc_comments_in(&read(path)).into_iter().map(move |line| format!("{}:{line}", shown(path))))
        .collect();

    // Assert
    assert_eq!((wire.is_empty(), schema_sources.is_empty(), offences), (false, false, Vec::<String>::new()));
}

#[test]
fn a_doc_comment_of_any_form_is_found_and_a_plain_comment_or_a_string_is_not() {
    // Arrange
    let source = "//! module\n/// field\n/** block */\n// plain\n//// rule\nfn f() { let s = \"/// in a string\"; }\n";

    // Act
    let lines = doc_comments_in(source);

    // Assert
    assert_eq!(lines, vec![1, 2, 3]);
}
