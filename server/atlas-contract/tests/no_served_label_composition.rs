mod common;

use common::source_scan::{code_of, read, rust_sources_under, shown};

const SERVED_SOURCES: [&str; 3] = ["server/atlas-contract/src", "server/atlas-server/src", "server/atlas-graph/src/node_ref.rs"];

const COMPOSERS: [&str; 6] = ["atlas_graph::labels", "labels::", "node::label", "::cite", "dot_ref", "tokenize("];

fn composers_in(code: &str) -> Vec<&'static str> {
    COMPOSERS.iter().copied().filter(|composer| code.contains(composer)).collect()
}

#[test]
fn no_served_crate_composes_a_label_or_a_reference() {
    // Arrange
    let sources = rust_sources_under(&SERVED_SOURCES);

    // Act
    let offences: Vec<String> = sources
        .iter()
        .flat_map(|path| composers_in(&code_of(&read(path))).into_iter().map(move |composer| format!("{}: {composer}", shown(path))))
        .collect();

    // Assert
    assert_eq!((sources.is_empty(), offences), (false, Vec::<String>::new()));
}

#[test]
fn a_composer_named_in_code_is_found_and_one_named_in_a_comment_or_a_string_is_not() {
    // Arrange
    let source = "// node::label(n)\nfn f(n: &Node) -> String { let s = \"labels::compile\"; atlas_graph_types::node::label(n) }\nfn g(r: &ConcordRef) -> String { ConcordTag::cite(r) }\n";

    // Act
    let found = composers_in(&code_of(source));

    // Assert
    assert_eq!(found, vec!["node::label", "::cite"]);
}

#[test]
fn a_reference_composer_named_in_code_is_found() {
    // Arrange
    let source = "fn f(b: u8, c: u16, v: u16) -> String { kjv_adapter::dot_ref(b, c, v) }\nfn g(text: &str) -> usize { tokens::tokenize(text).len() }\n";

    // Act
    let found = composers_in(&code_of(source));

    // Assert
    assert_eq!(found, vec!["dot_ref", "tokenize("]);
}
