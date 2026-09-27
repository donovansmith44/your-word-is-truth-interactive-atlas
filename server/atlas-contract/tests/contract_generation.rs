//! CONTRACT-1a (spec D8): the published contract documents are GENERATED
//! from the Rust and committed, and this file is the gate -- it regenerates
//! every one of them and fails on any byte of difference, so a hand edit to
//! `contracts/openapi.yaml`, `aqc.schema.json` or `graph-vocabulary.json`
//! cannot survive (PRINCIPLES.md 13).

#[test]
fn every_generated_document_is_byte_identical_to_the_committed_one() {
    // Arrange
    let files = atlas_contract::document::generated_files();
    // Act
    let stale: Vec<String> = files
        .iter()
        .filter(|(path, expected)| std::fs::read_to_string(path).ok().as_deref() != Some(expected.as_str()))
        .map(|(path, _)| path.display().to_string())
        .collect();
    // Assert
    assert!(stale.is_empty(), "stale: {stale:?} -- run `cargo run -p atlas-contract --bin export_contract`");
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
