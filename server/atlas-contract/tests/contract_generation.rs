use std::path::PathBuf;
use std::process::{Command, Output};

use atlas_contract::document::{contracts_root, GENERATED_DOCUMENTS};

const EXPORTER: &str = env!("CARGO_BIN_EXE_export_contract");
const CHECK_ARGUMENT: &str = "--check";
const UNRECOGNISED_ARGUMENT: &str = "--write";
const CLEAN_EXIT: i32 = 0;
const MISUSE_EXIT: i32 = 2;
const USAGE_LINE: &str = "usage: export_contract [--check]\n";
const NOTHING: &str = "";

const FORBIDDEN_IN_PUBLISHED_PROSE: [&str; 31] = [
    ".md",
    ".rs",
    "2026",
    "atlas_contract",
    "atlas_core",
    "atlas_graph",
    "batch",
    "brief",
    "controller",
    "design doc",
    "fix round",
    "fixme",
    "graph_wire",
    "graphquery",
    "hotfix",
    "m-a",
    "m-b",
    "m-c",
    "m-d",
    "overlay-",
    "owner",
    "principles",
    "prov-",
    "q-",
    "requirement ",
    " review",
    "ruling",
    "spec ",
    "task ",
    "todo",
    "\u{a7}",
];

#[derive(Debug, PartialEq)]
enum Freshness {
    ByteIdenticalToWhatTheRustRenders,
    DiffersFromWhatTheRustRenders,
    NoCommittedFileAtThisPath,
}

#[derive(Debug, PartialEq)]
struct CommittedDocument {
    path: PathBuf,
    freshness: Freshness,
}

#[derive(Debug, PartialEq)]
struct InternalHistoryHit {
    line: usize,
    token: &'static str,
    text: String,
}

#[test]
fn every_generated_document_is_byte_identical_to_the_committed_one() {
    // Arrange
    let expected: Vec<CommittedDocument> = GENERATED_DOCUMENTS
        .iter()
        .map(|document| CommittedDocument { path: contracts_root().join(document.path), freshness: Freshness::ByteIdenticalToWhatTheRustRenders })
        .collect();
    // Act
    let actual: Vec<CommittedDocument> = atlas_contract::document::generated_files().into_iter().map(committed_document).collect();
    // Assert
    assert_eq!(actual, expected, "run `cargo run -p atlas-contract --bin export_contract`");
}

fn committed_document((path, rendered): (PathBuf, String)) -> CommittedDocument {
    let freshness = match std::fs::read_to_string(&path) {
        Ok(committed) if committed == rendered => Freshness::ByteIdenticalToWhatTheRustRenders,
        Ok(_) => Freshness::DiffersFromWhatTheRustRenders,
        Err(_) => Freshness::NoCommittedFileAtThisPath,
    };
    CommittedDocument { path, freshness }
}

#[test]
fn the_exporter_asked_to_check_exits_clean_and_silent_while_every_document_is_current() {
    // Arrange
    let mut exporter = Command::new(EXPORTER);
    exporter.arg(CHECK_ARGUMENT);
    // Act
    let run = exporter.output().expect("the export_contract binary runs");
    // Assert
    assert_eq!(
        outcome(&run),
        ProcessOutcome { exit_code: Some(CLEAN_EXIT), stdout: NOTHING.to_string(), stderr: NOTHING.to_string() }
    );
}

#[test]
fn the_exporter_refuses_an_unrecognised_argument_with_the_usage_line_rather_than_writing() {
    // Arrange
    let mut exporter = Command::new(EXPORTER);
    exporter.arg(UNRECOGNISED_ARGUMENT);
    // Act
    let run = exporter.output().expect("the export_contract binary runs");
    // Assert
    assert_eq!(
        outcome(&run),
        ProcessOutcome { exit_code: Some(MISUSE_EXIT), stdout: NOTHING.to_string(), stderr: USAGE_LINE.to_string() }
    );
}

#[derive(Debug, PartialEq)]
struct ProcessOutcome {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn outcome(run: &Output) -> ProcessOutcome {
    ProcessOutcome {
        exit_code: run.status.code(),
        stdout: String::from_utf8_lossy(&run.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&run.stderr).into_owned(),
    }
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

#[test]
fn the_published_contract_names_no_internal_history() {
    // Arrange
    let document = atlas_contract::document::openapi_yaml();
    // Act
    let hits: Vec<InternalHistoryHit> = document
        .lines()
        .enumerate()
        .flat_map(|(number, text)| {
            let line = number + 1;
            let haystack = text.to_ascii_lowercase();
            FORBIDDEN_IN_PUBLISHED_PROSE
                .iter()
                .filter(move |token| haystack.contains(**token))
                .map(move |token| InternalHistoryHit { line, token, text: text.trim().to_string() })
        })
        .collect();
    // Assert
    assert_eq!(hits, Vec::<InternalHistoryHit>::new());
}
