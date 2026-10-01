use std::path::PathBuf;
use std::process::{Command, Output};

use atlas_contract::document::{contracts_root, RenderedDocument, GENERATED_DOCUMENTS};

const EXPORTER: &str = env!("CARGO_BIN_EXE_export_contract");
const CHECK_ARGUMENT: &str = "--check";
const UNRECOGNISED_ARGUMENT: &str = "--write";
const CLEAN_EXIT: i32 = 0;
const MISUSE_EXIT: i32 = 2;
const USAGE_LINE: &str = "usage: export_contract [--check]\n";
const NOTHING: &str = "";

/// Words that name this project's own development rather than what it serves: a
/// batch, a ruling, a fix round, a ticket, a version stamp, a date, a numbered
/// requirement. No document a reader outside this repository reads may carry one,
/// and neither may the contract corpus, which is read by whoever has to keep the
/// two harnesses agreeing.
const NAMES_THIS_PROJECTS_OWN_HISTORY: [&str; 26] = [
    "2026",
    "aqc v",
    "batch",
    "brief",
    "controller",
    "design doc",
    "fix round",
    "fixme",
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
    "red-1",
    "requirement ",
    " review",
    "ruling",
    "spec ",
    "task ",
    "todo",
    "\u{a7}",
];

/// Words that name the code behind this API rather than the API. A consumer of the
/// published document has none of it in hand, so naming it there says nothing; the
/// contract corpus, which is read from inside this repository, names it freely.
const NAMES_THE_CODE_BEHIND_THE_API: [&str; 7] = [".md", ".rs", "atlas_contract", "atlas_core", "atlas_graph", "graph_wire", "graphquery"];

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
struct ForbiddenWordHit {
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

fn committed_document(rendered: RenderedDocument) -> CommittedDocument {
    let freshness = match std::fs::read_to_string(&rendered.path) {
        Ok(committed) if committed == rendered.contents => Freshness::ByteIdenticalToWhatTheRustRenders,
        Ok(_) => Freshness::DiffersFromWhatTheRustRenders,
        Err(_) => Freshness::NoCommittedFileAtThisPath,
    };
    CommittedDocument { path: rendered.path, freshness }
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
            { "name": "AuthoredBy",   "forward": "authored-by",     "inverse": "authored" },
            { "name": "Shows",        "forward": "shows",           "inverse": "shown-on" },
        ],
        "symmetric": [
            { "name": "Analogue",          "label": "analogous-to" },
            { "name": "CatechismLink",     "label": "catechism-link" },
            { "name": "Corresponds",       "label": "corresponds-to" },
            { "name": "Parallel",          "label": "parallel" },
            { "name": "TemporalAdjacency", "label": "temporal-adjacency" },
            { "name": "Spouses",           "label": "spouse-of" },
            { "name": "Brethren",          "label": "brethren-of" },
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
    let version = std::fs::read_to_string(atlas_contract::document::contracts_root().join("atlas-query-contract/VERSION")).expect("the AQC VERSION file exists");
    let expected = serde_json::json!({
        "title": "Bible Atlas API",
        "description": "The Bible Atlas HTTP API, generated from the Rust that serves it. The `x-atlas-relations` extension lists every relation this atlas joins two nodes by, each with the label its forward and its inverse frontier is asked for, so a consumer builds its own frontier vocabulary from that list rather than writing one out.",
        "version": version.trim(),
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
    // Act
    let unresolved = unresolved_references(&schema, "#/$defs/", &schema["$defs"]);
    // Assert
    assert_eq!(unresolved, Vec::<String>::new());
}

#[test]
fn every_reference_in_the_published_document_resolves() {
    // Arrange
    let document = published_document();
    // Act
    let unresolved = unresolved_references(&document, "#/components/schemas/", &document["components"]["schemas"]);
    // Assert
    assert_eq!(unresolved, Vec::<String>::new());
}

#[test]
fn every_object_the_published_document_publishes_is_closed() {
    // Arrange
    let document = published_document();
    // Act
    let open = open_objects(&document, String::new());
    // Assert
    assert_eq!(open, Vec::<String>::new());
}

#[test]
fn every_family_a_route_names_is_published_with_its_own_sentence() {
    // Arrange
    let document = published_document();
    let mut published: Vec<String> = document["tags"].as_array().expect("the document publishes a tag list").iter().map(|tag| tag["name"].as_str().expect("a tag is named").to_string()).collect();
    published.sort_unstable();
    // Act
    let named_by_a_route = families_every_route_names(&document);
    // Assert
    assert_eq!((named_by_a_route, published), (families_declared(), families_declared()));
}

fn published_document() -> serde_json::Value {
    serde_json::to_value(atlas_contract::document::openapi()).expect("the published document serialises to JSON")
}

fn families_declared() -> Vec<String> {
    let mut declared: Vec<String> = atlas_contract::document::FAMILIES.iter().map(|(name, _)| (*name).to_string()).collect();
    declared.sort_unstable();
    declared
}

fn families_every_route_names(document: &serde_json::Value) -> Vec<String> {
    let mut named: Vec<String> = document["paths"]
        .as_object()
        .expect("the document publishes a path map")
        .values()
        .flat_map(|route| route.as_object().expect("a route publishes a method map").values())
        .flat_map(|operation| operation["tags"].as_array().expect("an operation names its family").iter())
        .map(|family| family.as_str().expect("a family is a name").to_string())
        .collect();
    named.sort_unstable();
    named.dedup();
    named
}

fn open_objects(value: &serde_json::Value, at: String) -> Vec<String> {
    let mut open = Vec::new();
    match value {
        serde_json::Value::Object(map) => {
            let is_object = map.get("type") == Some(&serde_json::json!("object"));
            if is_object && map.get("additionalProperties") != Some(&serde_json::json!(false)) && !map.contains_key("discriminator") {
                open.push(at.clone());
            }
            let closes_its_members = map.get("unevaluatedProperties") == Some(&serde_json::json!(false));
            for (key, child) in map {
                match (key.as_str(), child) {
                    ("allOf", serde_json::Value::Array(members)) if closes_its_members => open.extend(members.iter().enumerate().flat_map(|(index, member)| {
                        member.as_object().into_iter().flatten().flat_map(|(key, child)| open_objects(child, format!("{at}/allOf/{index}/{key}"))).collect::<Vec<_>>()
                    })),
                    _ => open.extend(open_objects(child, format!("{at}/{key}"))),
                }
            }
        }
        serde_json::Value::Array(items) => open.extend(items.iter().enumerate().flat_map(|(index, item)| open_objects(item, format!("{at}/{index}")))),
        _ => {}
    }
    open
}

fn unresolved_references(value: &serde_json::Value, prefix: &str, defined: &serde_json::Value) -> Vec<String> {
    let names: std::collections::BTreeSet<&str> = defined.as_object().expect("a document defines its shapes under one key").keys().map(String::as_str).collect();
    let mut references = std::collections::BTreeSet::new();
    collect_references(value, &mut references);
    references.into_iter().filter(|reference| reference.strip_prefix(prefix).is_none_or(|name| !names.contains(name))).collect()
}

fn collect_references(value: &serde_json::Value, out: &mut std::collections::BTreeSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                match (key.as_str(), child) {
                    ("$ref", serde_json::Value::String(target)) => {
                        out.insert(target.to_string());
                    }
                    ("mapping", serde_json::Value::Object(subtypes)) => out.extend(subtypes.values().filter_map(serde_json::Value::as_str).map(str::to_string)),
                    _ => collect_references(child, out),
                }
            }
        }
        serde_json::Value::Array(items) => items.iter().for_each(|item| collect_references(item, out)),
        _ => {}
    }
}

#[test]
fn the_published_contract_names_no_internal_history_and_none_of_the_code_behind_it() {
    // Arrange
    let document = atlas_contract::document::openapi_yaml();
    // Act
    let hits = forbidden_words_in(&document, &[&NAMES_THIS_PROJECTS_OWN_HISTORY, &NAMES_THE_CODE_BEHIND_THE_API]);
    // Assert
    assert_eq!(hits, Vec::<ForbiddenWordHit>::new());
}

#[test]
fn no_comment_in_the_query_contract_corpus_names_this_projects_own_history() {
    // Arrange
    let features = contracts_root().join("atlas-query-contract").join("features");
    // Act
    let hits: Vec<ForbiddenWordHit> = feature_comments(&features).iter().flat_map(|comments| forbidden_words_in(comments, &[&NAMES_THIS_PROJECTS_OWN_HISTORY])).collect();
    // Assert
    assert_eq!(hits, Vec::<ForbiddenWordHit>::new());
}

/// Every comment line of every feature file, one string per file, so a hit names
/// the line the corpus itself carries.
fn feature_comments(features: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(features)
        .expect("the query contract keeps its features in one directory")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "feature"))
        .map(|path| std::fs::read_to_string(&path).expect("a feature file this walk found must be readable"))
        .map(|text| text.lines().filter(|line| line.trim_start().starts_with('#')).collect::<Vec<_>>().join("\n"))
        .collect()
}

fn forbidden_words_in(text: &str, vocabularies: &[&[&'static str]]) -> Vec<ForbiddenWordHit> {
    text.lines()
        .enumerate()
        .flat_map(|(number, text)| {
            let line = number + 1;
            let haystack = text.to_ascii_lowercase();
            vocabularies
                .iter()
                .flat_map(|vocabulary| vocabulary.iter())
                .filter(move |token| haystack.contains(**token))
                .map(move |token| ForbiddenWordHit { line, token, text: text.trim().to_string() })
        })
        .collect()
}
