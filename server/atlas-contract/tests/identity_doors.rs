mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::source_scan::{code_of, is_ident_char, read, rust_sources_under, shown};

const SERVED_AND_TOOL_SOURCES: [&str; 6] = ["server/atlas-contract/src", "server/atlas-core/src", "server/atlas-graph/src", "server/atlas-server/src", "server/atlas-cli/src", "server/atlas-etl/src"];

const CONSTRUCTORS: [&str; 5] = ["NodeId::asked", "NodeId::of_place", "ArtifactRoot::of", "EdgePageCursor::at", "ElementPageCursor::at"];

const TEST_MODULE: &str = "#[cfg(test)]";

fn names(code: &str, constructor: &str) -> bool {
    code.match_indices(constructor).any(|(at, _)| !code[at + constructor.len()..].starts_with(is_ident_char))
}

fn application_code(source: &str) -> String {
    let code = code_of(source);
    code.split(TEST_MODULE).next().unwrap_or_default().to_string()
}

#[test]
fn every_identity_is_made_only_at_its_own_doors() {
    // Arrange
    let sources = rust_sources_under(&SERVED_AND_TOOL_SOURCES);

    // Act
    let callers: BTreeMap<&str, BTreeSet<String>> = CONSTRUCTORS
        .iter()
        .map(|constructor| (*constructor, sources.iter().filter(|path| names(&application_code(&read(path)), constructor)).map(|path| shown(path)).collect()))
        .collect();

    // Assert
    assert_eq!(
        callers,
        BTreeMap::from([
            ("ArtifactRoot::of", BTreeSet::from(["server/atlas-contract/src/contents.rs", "server/atlas-contract/src/graph.rs", "server/atlas-contract/src/reading.rs", "server/atlas-graph/src/bins/compile_graph.rs", "server/atlas-server/src/main.rs"].map(String::from))),
            ("EdgePageCursor::at", BTreeSet::from(["server/atlas-contract/src/graph.rs".to_string()])),
            ("ElementPageCursor::at", BTreeSet::from(["server/atlas-contract/src/graph.rs".to_string()])),
            ("NodeId::asked", BTreeSet::from(["server/atlas-contract/src/reference.rs".to_string()])),
            ("NodeId::of_place", BTreeSet::from(["server/atlas-core/src/data.rs".to_string()])),
        ])
    );
}

#[test]
fn a_constructor_named_in_a_test_module_or_a_string_is_not_a_door() {
    // Arrange
    let source = "fn f() -> NodeId { NodeId::asked(\"x\") }\nfn h() { pages.map(ElementPageCursor::at); NodeId::of_placed(); }\nconst S: &str = \"ArtifactRoot::of(v)\";\n#[cfg(test)]\nmod tests { fn g() { EdgePageCursor::at(Cursor(1)); } }\n";

    // Act
    let found: Vec<&str> = CONSTRUCTORS.iter().copied().filter(|constructor| names(&application_code(source), constructor)).collect();

    // Assert
    assert_eq!(found, vec!["NodeId::asked", "ElementPageCursor::at"]);
}
