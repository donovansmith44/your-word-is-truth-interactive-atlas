//! CONTRACT-1a: NO ROUTE WITHOUT A CONTRACT.
//!
//! `contracts/openapi.yaml` publishes every route this server serves, and a
//! published route is a promise. This file is the law that a promise is
//! backed by an executable expectation: every documented path must be named
//! somewhere in the contract corpus, so a route cannot be added, published
//! and then left with nobody holding us to it.
//!
//! The corpus is read as text rather than parsed, deliberately. A suite
//! names a route in a `When I GET` line where the vendored runner drives it
//! (`atlas-graph-contract`, `atlas-edge`) and in its own feature's header
//! where a different harness does (`atlas-query-contract`'s `ContentsQuery`
//! scenarios reach `/api/contents/{corpus}` through a step phrased in the
//! query's own domain language). Both are a suite taking responsibility for
//! the route; only one of them is a step line.

use std::path::{Path, PathBuf};

const CONTRACT_SUITES: [&str; 3] = ["atlas-graph-contract", "atlas-query-contract", "atlas-edge"];

/// `/api/openapi.yaml` serves the contract itself and `/health` answers a
/// liveness string; neither is a promise a consumer can hold us to.
const NOT_A_PROMISE: [&str; 2] = ["/health", "/api/openapi.yaml"];

#[test]
fn every_served_route_is_referenced_by_a_contract_scenario() {
    // Arrange
    let features = every_feature_file_concatenated();
    let doc = atlas_contract::document::openapi();
    // Act
    let uncovered: Vec<String> = doc
        .paths
        .paths
        .keys()
        .filter(|path| !NOT_A_PROMISE.contains(&path.as_str()))
        .map(|path| route_prefix(path))
        .filter(|prefix| !features.contains(prefix.as_str()))
        .collect();
    // Assert
    assert_eq!(uncovered, Vec::<String>::new());
}

fn every_feature_file_concatenated() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts");
    CONTRACT_SUITES
        .iter()
        .flat_map(|suite| files_under(&root.join(suite)))
        .filter(|file| file.extension().is_some_and(|extension| extension == "feature"))
        .map(|file| std::fs::read_to_string(&file).expect("a feature file this walk found must be readable"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .expect("a contract suite must be a readable directory")
        .flatten()
        .map(|entry| entry.path())
        .flat_map(|path| if path.is_dir() { files_under(&path) } else { vec![path] })
        .collect()
}

fn route_prefix(path: &str) -> String {
    path.split('{').next().expect("splitting never yields nothing").trim_end_matches('/').to_string()
}
