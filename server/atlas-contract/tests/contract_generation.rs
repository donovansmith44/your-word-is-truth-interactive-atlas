//! CONTRACT-1a (spec D8): the published contract documents are GENERATED
//! from the Rust and committed, and this file is the gate -- it regenerates
//! every one of them and fails on any byte of difference, so a hand edit to
//! `contracts/openapi.yaml`, `aqc.schema.json` or `graph-vocabulary.json`
//! cannot survive (PRINCIPLES.md 13).

use std::path::PathBuf;
use std::process::{Command, Output};

/// Every document the exporter publishes, under `contracts/`. A document that
/// stops being generated is a promise quietly withdrawn, so the set is named
/// here and not merely iterated.
const GENERATED_DOCUMENTS: [&str; 3] = [
    "openapi.yaml",
    "atlas-query-contract/aqc.schema.json",
    "atlas-graph-contract/fixtures/graph-vocabulary.json",
];
const BYTE_IDENTICAL: &str = "byte-identical to the committed file";
const DIFFERS: &str = "DIFFERS from the committed file -- run `cargo run -p atlas-contract --bin export_contract`";
const ABSENT: &str = "ABSENT -- no committed file at this path";

const EXPORTER: &str = env!("CARGO_BIN_EXE_export_contract");
const CHECK_ARGUMENT: &str = "--check";
const UNRECOGNISED_ARGUMENT: &str = "--write";
const CLEAN_EXIT: i32 = 0;
const MISUSE_EXIT: i32 = 2;
const USAGE_LINE: &str = "usage: export_contract [--check]\n";
const NOTHING: &str = "";

#[test]
fn every_generated_document_is_byte_identical_to_the_committed_one() {
    // Arrange
    let expected: Vec<(PathBuf, &str)> = GENERATED_DOCUMENTS.iter().map(|name| (contracts_root().join(name), BYTE_IDENTICAL)).collect();
    // Act
    let actual: Vec<(PathBuf, &str)> = atlas_contract::document::generated_files()
        .into_iter()
        .map(|(path, generated)| {
            let state = match std::fs::read_to_string(&path) {
                Ok(committed) if committed == generated => BYTE_IDENTICAL,
                Ok(_) => DIFFERS,
                Err(_) => ABSENT,
            };
            (path, state)
        })
        .collect();
    // Assert
    assert_eq!(actual, expected);
}

fn contracts_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contracts")
}

#[test]
fn the_exporter_asked_to_check_exits_clean_and_silent_while_every_document_is_current() {
    // Arrange
    let mut exporter = Command::new(EXPORTER);
    exporter.arg(CHECK_ARGUMENT);
    // Act
    let run = exporter.output().expect("the export_contract binary runs");
    // Assert
    assert_eq!(outcome(&run), (Some(CLEAN_EXIT), NOTHING.to_string(), NOTHING.to_string()));
}

#[test]
fn the_exporter_refuses_an_unrecognised_argument_with_the_usage_line_rather_than_writing() {
    // Arrange
    let mut exporter = Command::new(EXPORTER);
    exporter.arg(UNRECOGNISED_ARGUMENT);
    // Act
    let run = exporter.output().expect("the export_contract binary runs");
    // Assert
    assert_eq!(outcome(&run), (Some(MISUSE_EXIT), NOTHING.to_string(), USAGE_LINE.to_string()));
}

fn outcome(run: &Output) -> (Option<i32>, String, String) {
    (
        run.status.code(),
        String::from_utf8_lossy(&run.stdout).into_owned(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
    )
}

#[test]
fn x_atlas_relations_is_the_relations_manifest_in_declaration_order() {
    // Arrange
    let expected = serde_json::json!({
        "directed": [
            { "name": "Contains",     "forward": "contains",        "inverse": "member-of" },
            { "name": "Attests",      "forward": "attested-in",     "inverse": "attests" },
            { "name": "Succession",   "forward": "follows-in",      "inverse": "precedes-in" },
            { "name": "DatedBy",      "forward": "dated-by",        "inverse": "dates" },
            { "name": "LocatedAt",    "forward": "located-at",      "inverse": "site-of" },
            { "name": "Mentions",     "forward": "mentions",        "inverse": "mentioned-in" },
            { "name": "Cites",        "forward": "cites",           "inverse": "cited-by" },
            { "name": "Quotes",       "forward": "quotes",          "inverse": "quoted-by" },
            { "name": "Confesses",    "forward": "confesses",       "inverse": "confessed-in" },
            { "name": "Fulfillment",  "forward": "fulfilled-in",    "inverse": "fulfills" },
            { "name": "Typology",     "forward": "prefigures",      "inverse": "prefigured-by" },
            { "name": "NamedAfter",   "forward": "named-after",     "inverse": "namesake-of" },
            { "name": "JustifiedBy",  "forward": "justified-by",    "inverse": "justifies" },
            { "name": "CommentsOn",   "forward": "comments-on",     "inverse": "commented-on-by" },
            { "name": "SpokenBy",     "forward": "spoken-by",       "inverse": "speech-of" },
            { "name": "SpokenAt",     "forward": "spoken-at",       "inverse": "site-of-speech" },
            { "name": "DerivedFrom",  "forward": "derived-from",    "inverse": "derives" },
            { "name": "Occurs",       "forward": "occurs-in",       "inverse": "words" },
            { "name": "ParentOf",     "forward": "parent-of",       "inverse": "child-of" },
            { "name": "Participates", "forward": "participates-in", "inverse": "participants" },
        ],
        "symmetric": [
            { "name": "Analogue",          "label": "analogous-to" },
            { "name": "CatechismLink",     "label": "catechism-link" },
            { "name": "Corresponds",       "label": "corresponds-to" },
            { "name": "Parallel",          "label": "parallel" },
            { "name": "TemporalAdjacency", "label": "temporal-adjacency" },
            { "name": "Partners",          "label": "partner-of" },
        ],
    });
    // Act
    let actual = atlas_contract::document::relations_json();
    // Assert
    assert_eq!(actual, expected);
}

#[test]
fn the_published_document_names_this_api_and_the_contract_version_it_serves() {
    // Arrange
    let expected = serde_json::json!({
        "title": "Bible Atlas API",
        "description": "The Bible Atlas HTTP API, generated from the Rust that serves it.",
        "version": atlas_contract::meta::MAX_SUPPORTED_VERSION,
    });
    // Act
    let actual = serde_json::to_value(atlas_contract::document::openapi().info).expect("the document's info serialises");
    // Assert
    assert_eq!(actual, expected);
}

#[test]
fn every_reference_in_the_aqc_schema_resolves_under_defs() {
    // Arrange
    let schema: serde_json::Value = serde_json::from_str(&atlas_contract::document::aqc_schema_json()).expect("the AQC schema is valid JSON");
    let shapes: std::collections::BTreeSet<&str> = schema["$defs"].as_object().expect("the AQC schema has a $defs object").keys().map(String::as_str).collect();
    // Act
    let mut references = std::collections::BTreeSet::new();
    collect_references(&schema, &mut references);
    // Assert
    let unresolved: Vec<&String> = references
        .iter()
        .filter(|r| r.strip_prefix("#/$defs/").is_none_or(|shape| !shapes.contains(shape)))
        .collect();
    assert!(unresolved.is_empty(), "the AQC harnesses resolve every $ref under #/$defs/, but these do not resolve there: {unresolved:?}");
}

fn collect_references(value: &serde_json::Value, out: &mut std::collections::BTreeSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                match child.as_str() {
                    Some(target) if key == "$ref" => {
                        out.insert(target.to_string());
                    }
                    _ => collect_references(child, out),
                }
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|item| collect_references(item, out)),
        _ => {}
    }
}
