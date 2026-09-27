//! The published contract documents, all of them derived from the Rust that
//! serves the API (spec D8). `contracts/openapi.yaml` is the contract; the
//! Atlas Query Contract's shapes and the Atlas Graph Contract's vocabulary
//! fixture are derivations of it at their own committed paths.

use std::path::PathBuf;

use atlas_graph_types::{NodeKind, RelationId, SymRelationId};
use serde::Serialize;
use serde_json::{json, Value};
use utoipa::openapi::extensions::ExtensionsBuilder;
use utoipa::openapi::{Info, OpenApi};

const API_TITLE: &str = "Bible Atlas API";
const API_DESCRIPTION: &str = "The Bible Atlas HTTP API, generated from the Rust that serves it.";
const RELATIONS_EXTENSION: &str = "x-atlas-relations";
const COMPONENT_REFERENCE: &str = "#/components/schemas/";
const SHAPE_REFERENCE: &str = "#/$defs/";

pub fn generated_files() -> Vec<(PathBuf, String)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contracts");
    vec![
        (root.join("openapi.yaml"), openapi_yaml()),
        (root.join("atlas-query-contract/aqc.schema.json"), aqc_schema_json()),
        (root.join("atlas-graph-contract/fixtures/graph-vocabulary.json"), graph_vocabulary_json()),
    ]
}

pub fn openapi_yaml() -> String {
    openapi().to_yaml().expect("the OpenAPI document serialises")
}

/// The router's document, named and versioned as the PUBLISHED contract:
/// left alone it carries utoipa-axum's own crate metadata (that crate's
/// title, author and licence), which would be published as though it
/// described this API.
pub fn openapi() -> OpenApi {
    let mut doc = crate::openapi();
    let mut info = Info::new(API_TITLE, crate::meta::MAX_SUPPORTED_VERSION);
    info.description = Some(API_DESCRIPTION.to_string());
    doc.info = info;
    doc.extensions = Some(ExtensionsBuilder::new().add(RELATIONS_EXTENSION, relations_json()).build());
    doc
}

pub fn relations_json() -> Value {
    json!({
        "directed": RelationId::ALL.iter().map(|r| json!({"name": format!("{r:?}"), "forward": r.forward_label(), "inverse": r.inverse_label()})).collect::<Vec<_>>(),
        "symmetric": SymRelationId::ALL.iter().map(|s| json!({"name": format!("{s:?}"), "label": s.label()})).collect::<Vec<_>>(),
    })
}

/// The shapes both Atlas Query Contract harnesses validate responses
/// against. They resolve a shape as `#/$defs/<Shape>`, which is where a
/// JSON Schema document keeps its definitions; the OpenAPI document keeps
/// the same schemas under `#/components/schemas/`, so every reference is
/// rewritten to the one the harnesses follow.
pub fn aqc_schema_json() -> String {
    let doc = serde_json::to_value(openapi()).expect("the document serialises to JSON");
    let shapes = doc["components"]["schemas"].to_string().replace(COMPONENT_REFERENCE, SHAPE_REFERENCE);
    let out = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://bible-atlas/contracts/atlas-query-contract/aqc.schema.json",
        "title": "Atlas Query Contract -- shapes",
        "description": "Derived from contracts/openapi.yaml by export_contract; do not edit.",
        "version": crate::meta::MAX_SUPPORTED_VERSION,
        "$defs": serde_json::from_str::<Value>(&shapes).expect("the rewritten shapes are still JSON"),
    });
    serde_json::to_string_pretty(&out).expect("the AQC schema serialises") + "\n"
}

pub fn graph_vocabulary_json() -> String {
    let relations = relations_json();
    blessed_fixture(&json!({
        "manifest_schema": atlas_graph::sqlite::manifest::MANIFEST_SCHEMA,
        "section_schema_version": atlas_graph::sections::SECTION_SCHEMA_VERSION,
        "node_kinds": NodeKind::ALL.iter().map(|k| k.name()).collect::<Vec<_>>(),
        "relations": relations["directed"],
        "symmetric": relations["symmetric"],
    }))
}

/// The Atlas Graph Contract's runner blesses a fixture with aeson-pretty --
/// four-space indent, no trailing newline. This fixture is now blessed from
/// both sides, so it renders the runner's way; otherwise the two blessers
/// would rewrite each other's whitespace and one gate would always be red.
fn blessed_fixture(value: &Value) -> String {
    let mut rendered = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut rendered, serde_json::ser::PrettyFormatter::with_indent(b"    "));
    value.serialize(&mut serializer).expect("a fixture serialises");
    String::from_utf8(rendered).expect("serde_json emits UTF-8")
}
