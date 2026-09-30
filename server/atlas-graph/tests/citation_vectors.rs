mod common;

use atlas_graph::citations::{scan, target_display};
use serde_json::Value;

const CITATION_GRAMMAR_VECTORS: &str = "citation-grammar.json";

#[test]
fn the_ported_citation_grammar_agrees_with_every_recorded_case() {
    // Arrange
    let vectors: Value = serde_json::from_str(
        &std::fs::read_to_string(common::contract_vectors_dir().join(CITATION_GRAMMAR_VECTORS)).expect("the citation-grammar vectors are committed"),
    )
    .expect("the vectors are JSON");
    let cases = vectors["cases"].as_array().expect("the vectors list their cases");
    let expected: Vec<(&str, Vec<(usize, usize, String)>)> = cases.iter().map(|case| (name_of(case), citations(&case["citations"]))).collect();
    // Act
    let scanned: Vec<(&str, Vec<(usize, usize, String)>)> = cases
        .iter()
        .map(|case| (name_of(case), scan(text_of(case)).into_iter().map(|c| (c.chars.start, c.chars.end, target_display(&c.cites))).collect()))
        .collect();
    // Assert
    assert_eq!(scanned, expected);
}

fn name_of(case: &Value) -> &str {
    case["name"].as_str().expect("a case is named")
}

fn text_of(case: &Value) -> &str {
    case["text"].as_str().expect("a case carries its text")
}

fn citations(found: &Value) -> Vec<(usize, usize, String)> {
    let offset = |c: &Value, key: &str| usize::try_from(c[key].as_u64().expect("a citation names its character range")).expect("an offset fits a usize");
    found
        .as_array()
        .expect("a case lists the citations it finds")
        .iter()
        .map(|c| (offset(c, "start"), offset(c, "end"), c["cites"].as_str().expect("a citation names what it cites").to_string()))
        .collect()
}
