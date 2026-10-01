mod common;

use atlas_core::data::Canon;
use atlas_graph::runs::coalesce;
use atlas_graph_types::text::{BibleLocusRange, Locus, VerseRef};
use serde_json::Value;

const ATTESTATION_RUNS_VECTORS: &str = "attestation-runs.json";

#[test]
fn every_attestation_runs_vector_coalesces_to_the_runs_it_names() {
    // Arrange
    let vectors: Value = serde_json::from_str(&std::fs::read_to_string(common::contract_vectors_dir().join(ATTESTATION_RUNS_VECTORS)).expect("the attestation-runs vectors are committed"))
        .expect("the vectors are JSON");
    let cases = vectors["cases"].as_array().expect("the vectors list their cases");
    let expected: Vec<(&str, Vec<BibleLocusRange>)> = cases.iter().map(|case| (name_of(case), ranges(&case["runs"]))).collect();
    // Act
    let coalesced: Vec<(&str, Vec<BibleLocusRange>)> = cases.iter().map(|case| (name_of(case), coalesce(&ranges(&case["attested"]), &canon_of(case)))).collect();
    // Assert
    assert_eq!(coalesced, expected);
}

fn name_of(case: &Value) -> &str {
    case["name"].as_str().expect("a case is named")
}

fn canon_of(case: &Value) -> Canon {
    serde_json::from_value(case["canon"].clone()).expect("a case carries the canon it is read against")
}

fn ranges(spans: &Value) -> Vec<BibleLocusRange> {
    spans.as_array().expect("a list of spans").iter().map(|span| BibleLocusRange { from: Locus::whole(verse(&span["from"]["unit"])), to: Locus::whole(verse(&span["to"]["unit"])) }).collect()
}

fn verse(unit: &Value) -> VerseRef {
    let number = |part: &str| u16::try_from(unit[part].as_u64().expect("a verse names its chapter and verse")).expect("a chapter or verse number fits a u16");
    VerseRef {
        book: atlas_core::canon::resolve_alias(unit["book"].as_str().expect("a verse names its book")).expect("a book is named by its canon code").0,
        chapter: number("chapter"),
        verse: number("verse"),
    }
}
