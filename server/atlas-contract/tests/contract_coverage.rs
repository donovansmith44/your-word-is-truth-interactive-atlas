use std::path::{Path, PathBuf};

const CONTRACT_SUITES: [&str; 3] = ["atlas-graph-contract", "atlas-query-contract", "atlas-edge"];

const NOT_A_PROMISE: [&str; 2] = ["/health", "/api/openapi.yaml"];

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

fn every_path_the_scenarios_request(corpus: &str) -> Vec<String> {
    corpus.lines().filter_map(requested_path).collect()
}

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
