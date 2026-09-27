//! CONTRACT-1a: NO ROUTE WITHOUT A CONTRACT.
//!
//! `contracts/openapi.yaml` publishes every route this server serves, and a
//! published route is a promise. This file is the law that a promise is
//! backed by an executable expectation: every documented path must be the
//! path some contract scenario actually requests, so a route cannot be
//! added, published and then left with nobody holding us to it.
//!
//! A ROUTE IS MATCHED SEGMENT BY SEGMENT, NEVER AS A SUBSTRING. The first
//! version of this law asked whether the corpus text CONTAINED the part of
//! the documented path before its first `{`, and that is satisfiable by an
//! unrelated route: `/api/catechism/{sref}` was "covered" by
//! `/api/catechism/item/commandment-1`, so deleting the one scenario that
//! reads the catechism of a verse would have left this law green.
//! `a_route_whose_only_request_is_deleted_is_named_uncovered` below is that
//! exact mutant, killed by a test rather than by inspection.

use std::path::{Path, PathBuf};

const CONTRACT_SUITES: [&str; 3] = ["atlas-graph-contract", "atlas-query-contract", "atlas-edge"];

/// `/api/openapi.yaml` serves the contract itself and `/health` answers a
/// liveness string; neither is a promise a consumer can hold us to.
const NOT_A_PROMISE: [&str; 2] = ["/health", "/api/openapi.yaml"];

/// The one step line in the whole corpus that requests the catechism of a
/// verse, and therefore the only thing standing between `/api/catechism/{sref}`
/// and being an unpromised route.
const THE_ONLY_REQUEST_FOR_A_VERSE_S_CATECHISM: &str = "When I GET /api/catechism/MAT.28.19";
const THE_ROUTE_IT_COVERS: &str = "/api/catechism/{sref}";

#[test]
fn every_served_route_is_referenced_by_a_contract_scenario() {
    // Arrange
    let requested = every_path_the_scenarios_request(&every_feature_file_concatenated());
    let documented = every_published_route();
    // Act
    let uncovered = routes_no_scenario_requests(&documented, &requested);
    // Assert
    assert_eq!(uncovered, Vec::<String>::new());
}

#[test]
fn a_route_whose_only_request_is_deleted_is_named_uncovered() {
    // Arrange
    let corpus = every_feature_file_concatenated().replace(THE_ONLY_REQUEST_FOR_A_VERSE_S_CATECHISM, "");
    let requested = every_path_the_scenarios_request(&corpus);
    let documented = every_published_route();
    // Act
    let uncovered = routes_no_scenario_requests(&documented, &requested);
    // Assert
    assert_eq!(uncovered, vec![THE_ROUTE_IT_COVERS.to_string()]);
}

/// Every path the corpus's scenarios ask for, with any query string dropped.
fn every_path_the_scenarios_request(corpus: &str) -> Vec<String> {
    corpus.lines().filter_map(requested_path).collect()
}

/// The two request phrasings this corpus uses, and there is no third: the
/// vendored runner's `I GET <path>` (which may bind the answer with a
/// trailing ` as <name>`, and whose path never contains whitespace), and the
/// Atlas Query Contract's `I query "<path>"`, which both of that suite's own
/// harnesses define. A phrasing that names no path -- `I run SceneQuery for
/// the time window ...` -- requests a route this law cannot see, which is
/// why the routes behind those steps carry a named `I query` step of their
/// own in the suite that owns them.
fn requested_path(line: &str) -> Option<String> {
    let step = line.trim();
    let body = STEP_KEYWORDS.iter().find_map(|keyword| step.strip_prefix(keyword))?;
    let named = REQUEST_PHRASINGS.iter().find_map(|phrasing| body.strip_prefix(phrasing))?;
    let path = named.split_whitespace().next()?.trim_matches('"');
    Some(path.split('?').next().unwrap_or(path).to_string())
}

const STEP_KEYWORDS: [&str; 2] = ["When ", "And "];
const REQUEST_PHRASINGS: [&str; 2] = ["I GET ", "I query "];

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

fn every_published_route() -> Vec<String> {
    atlas_contract::document::openapi()
        .paths
        .paths
        .into_keys()
        .filter(|path| !NOT_A_PROMISE.contains(&path.as_str()))
        .collect()
}

fn routes_no_scenario_requests(documented: &[String], requested: &[String]) -> Vec<String> {
    documented
        .iter()
        .filter(|route| !requested.iter().any(|path| route_answers(route, path)))
        .cloned()
        .collect()
}

/// A documented route answers a requested path when the two have the same
/// number of segments and agree on every segment the document spells
/// literally; a `{param}` segment stands for exactly one segment of any
/// spelling. Same segment count is what keeps `/api/catechism/{sref}` and
/// `/api/catechism/item/{id}` two separate promises.
fn route_answers(route: &str, path: &str) -> bool {
    let route = segments(route);
    let path = segments(path);
    route.len() == path.len()
        && route.iter().zip(&path).all(|(declared, asked)| is_parameter(declared) || declared == asked)
}

fn segments(path: &str) -> Vec<&str> {
    path.split('/').filter(|segment| !segment.is_empty()).collect()
}

fn is_parameter(segment: &str) -> bool {
    segment.starts_with('{') && segment.ends_with('}')
}
