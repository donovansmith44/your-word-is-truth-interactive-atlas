mod common;

use common::source_scan::{code_of, read, rust_sources_under, shown};

const SERVED_CRATES: [&str; 2] = ["server/atlas-contract/src", "server/atlas-server/src"];

const LABEL_COMPOSERS: [&str; 4] = ["atlas_graph::labels", "labels::", "node::label", "::cite"];

fn composers_in(code: &str) -> Vec<&'static str> {
    LABEL_COMPOSERS.iter().copied().filter(|composer| code.contains(composer)).collect()
}

#[test]
fn no_served_crate_composes_a_label() {
    // Arrange
    let sources = rust_sources_under(&SERVED_CRATES);

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
