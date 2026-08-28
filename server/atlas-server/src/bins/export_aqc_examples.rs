//! Batch AQC-1's own provider-side exporter (spec §3: "a provider-side
//! exporter draws descriptors from the REAL graph ... and emits/refreshes
//! the committed `Examples:` tables. The generator is code; its OUTPUT is
//! Gherkin"). Regenerates the two `Scenario Outline`-bearing corpus files
//! (`focus-query.feature`, `exploration-roundtrip.feature`) in full, from a
//! fixed, disclosed SEED list -- one id per real `NodeKind` the committed
//! graph materializes (11 kinds; `GEN-KINDS` below), each one VERIFIED live
//! against the real compiled graph before it is ever written out. A seed
//! id that no longer resolves (a curated record renamed/removed) makes
//! this binary panic -- fail loud, never silently emit a stale example.
//!
//! Deterministic by construction (never wall-clock random): the SEED list
//! is a fixed const array in this file, and every downstream string is a
//! pure function of it plus whatever the graph itself reports for each
//! id (its own `NodeKind::Debug` string) -- re-running this binary against
//! an unchanged graph reproduces byte-identical `.feature` files every
//! time (the determinism proof the batch report cites: `git diff` is
//! empty after a re-run).
//!
//! Outside `src/bin/` (this crate's own root `.gitignore` has a broad
//! `**/bin/` rule, meant for the Blazor client's own .NET build output --
//! same disclosed workaround `atlas-graph`'s `compile_graph.rs` and
//! `atlas-etl`'s `gen_sources.rs` already use): source lives at
//! `src/bins/export_aqc_examples.rs`, declared explicitly in this crate's
//! `Cargo.toml` `[[bin]]` table.
//!
//! Run from `server/`: `cargo run -p atlas-server --bin export_aqc_examples`.

use std::path::Path;
use std::sync::Arc;

use atlas_core::data::AtlasData;
use atlas_graph::GraphService;
use atlas_graph_types::store::GraphQuery;
use atlas_server::graph_wire::{decode_node_id, encode_node_id};
use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

/// One (NodeKind Debug string, wire id) seed per real node kind this
/// contract samples -- SAME id list `graph_wire.rs`'s own round-trip unit
/// test and `tests/graph_api.rs`'s own real-data HTTP tests already prove
/// live against the committed graph (`ab_ur`, `aaron_1`, `kretzmann/0.1.0`,
/// ... -- not invented for this exporter). `commandment-1` (CatechismItem)
/// is `data/curated/catechism.toml`'s own first commandment item id;
/// `latin_vulgate` (Translation) is one of the six CORP-1a-ingested
/// parallel editions -- deliberately NOT "kjv", which
/// `brainfuel_layers.rs::no_kjv_translation_node_is_authored_this_batch`
/// proves does NOT exist as a Translation node (the KJV is the canonical
/// TextUnit layer itself, never a Translation-kind node of its own).
const SEEDS: &[(&str, &str)] = &[
    ("TextUnit", "text-unit:JHN.3.16"),
    ("Event", "Event:ab_ur"),
    ("Narrative", "Narrative:abraham-migration"),
    ("Anchor", "Anchor:solomon-crowned"),
    ("Place", "Place:ur-1"),
    ("Era", "Era:primeval"),
    ("Polity", "Polity:egypt"),
    ("Person", "Person:aaron_1"),
    ("Translation", "Translation:latin_vulgate"),
    ("CommentaryItem", "CommentaryItem:kretzmann/0.1.0"),
    ("CatechismItem", "CatechismItem:commandment-1"),
];

fn examples_table(indent: &str) -> String {
    let kind_w = SEEDS.iter().map(|(k, _)| k.len()).chain(["kind".len()]).max().unwrap();
    let id_w = SEEDS.iter().map(|(_, i)| i.len()).chain(["id".len()]).max().unwrap();
    let mut out = String::new();
    out.push_str(&format!("{indent}| {:<kind_w$} | {:<id_w$} |\n", "kind", "id"));
    for (kind, id) in SEEDS {
        out.push_str(&format!("{indent}| {kind:<kind_w$} | {id:<id_w$} |\n"));
    }
    out
}

fn focus_query_feature() -> String {
    format!(
        r#"# AQC v0.1.0 -- FocusQuery(descriptor) -> Focus (spec §2, §3).
# GET /api/node/{{id}} -- server/atlas-server/src/graph_handlers.rs::node_card.
#
# The Examples: table below is GENERATED, not hand-authored -- see
# server/atlas-etl/src/bin/export_aqc_examples.rs. It draws one seed id per
# NODE KIND the real committed graph materializes (spec §3: "every node kind
# sampled from the graph"), verified live against that graph at export time
# (a stale seed id fails the exporter loud, not silently). Re-running the
# exporter against an unchanged graph reproduces this table byte-identical
# (deterministic; never wall-clock random).
Feature: FocusQuery -- one node's card, by descriptor

  Scenario Outline: every sampled node kind resolves to a valid Focus card
    Given a node of kind "<kind>" with id "<id>"
    When I run FocusQuery for "<id>"
    Then the response is a valid "NodeCardOut"
    And the response "id" field equals "<id>"
    And the response "kind" field equals "<kind>"
    And every frontier group is a relations! family

    Examples:
{examples}
  Scenario: an id that parses but names no real node is not_found
    Given a node of kind "Person" with id "Person:nonexistent-xyz"
    When I run FocusQuery for "Person:nonexistent-xyz"
    Then the request fails with status 404 and code "not_found"

  Scenario: a malformed id is bad_ref
    When I run FocusQuery for "not-even-a-colon-pair"
    Then the request fails with status 400 and code "bad_ref"
"#,
        examples = examples_table("      ")
    )
}

fn exploration_roundtrip_feature() -> String {
    format!(
        r#"# AQC v0.1.0 -- ExplorationRoundTrip(descriptor): descriptor -> node ->
# descriptor identity (the G2 seam, now a contract law -- spec §2). At the
# WIRE level this is graph_wire::encode_node_id/decode_node_id's own
# bijection: the id a FocusQuery/TraversalQuery response hands back always
# re-resolves the SAME node when fed straight back into FocusQuery.
#
# The Examples: table is GENERATED by the same exporter as focus-query.feature
# (server/atlas-etl/src/bin/export_aqc_examples.rs) -- one seed id per real
# node kind, verified live against the committed graph.
Feature: ExplorationRoundTrip -- descriptor -> node -> descriptor identity

  Scenario Outline: a captured focus reference round-trips identically
    Given a node of kind "<kind>" with id "<id>"
    When I run FocusQuery for "<id>"
    And I capture the returned focus reference
    And I run FocusQuery again for the captured reference
    Then the focus reference round-trips identically

    Examples:
{examples}
  Scenario: a traversal target's own id round-trips too
    Given a node of kind "TextUnit" with id "text-unit:JHN.3.16"
    When I run TraversalQuery for "text-unit:JHN.3.16" frontier "cites"
    And I capture the returned focus reference
    And I run FocusQuery again for the captured reference
    Then the focus reference round-trips identically
"#,
        examples = examples_table("      ")
    )
}

/// Percent-encodes the one wire-id character that collides with axum's own
/// path-segment routing (`/`, e.g. `CommentaryItem:kretzmann/0.1.0`) --
/// same convention `tests/graph_api.rs`'s own literal
/// `"CommentaryItem:kretzmann%2F0.1.0"` URIs already establish.
fn path_encode(id: &str) -> String {
    id.replace('/', "%2F")
}

/// Provider-exported response FIXTURES (brief deliverable 3, "Fixtures:
/// provider-exported response fixtures committed pact-style"): one JSON
/// file per DISTINCT real HTTP request this contract's Gherkin corpus
/// makes across ALL SIX feature files (deduplicated -- a request repeated
/// across two scenarios, e.g. TextUnit's FocusQuery in both
/// focus-query.feature and exploration-roundtrip.feature, gets ONE
/// fixture, reused). Captured via the SAME in-process
/// `tower::ServiceExt::oneshot` idiom the Rust cucumber harness uses --
/// real committed-graph HTTP responses, never hand-typed JSON. Each file
/// is `{"status": <u16>, "body": <json>}` so error-path scenarios (400/404)
/// are provable from a fixture too, not just success shapes. The C#
/// harness (thin, contract-ignorant) deserializes these through the
/// client's own DTOs -- it never talks to a live server.
const FIXTURES: &[(&str, &str)] = &[
    // FocusQuery error cases (the 11 success cases are derived from SEEDS
    // below -- one "focus-<kind-lowercase>" fixture per real node kind).
    ("focus-not-found", "/api/node/Person:nonexistent-xyz"),
    ("focus-bad-ref", "/api/node/not-even-a-colon-pair"),
    // exploration-roundtrip.feature's own "a traversal target's own id
    // round-trips too" scenario captures a TraversalQuery entry's own id
    // (text-unit:ROM.5.8, JHN.3.16's own first "cites" target) and
    // re-runs FocusQuery on it -- this is that second FocusQuery's fixture.
    ("focus-traversal-target", "/api/node/text-unit:ROM.5.8"),
    // TraversalQuery
    ("traversal-cites", "/api/node/text-unit:JHN.3.16/edges?kind=cites"),
    ("traversal-cites-limit1", "/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=1"),
    ("traversal-located-at", "/api/node/Event:ab_ur/edges?kind=located-at"),
    ("traversal-bad-kind", "/api/node/text-unit:JHN.3.16/edges?kind=not-a-real-kind"),
    // TextWindowQuery
    ("text-window-single", "/api/text?ref=JHN.3.16&n=1"),
    ("text-window-multi", "/api/text?ref=JHN.3.16&n=3"),
    ("text-window-mat-4-19", "/api/text?ref=MAT.4.19&n=1"),
    ("text-window-mat-5-4", "/api/text?ref=MAT.5.4&n=1"),
    ("text-window-chapter-backward-bad-dir", "/api/text?ref=JHN.3&scope=chapter&dir=backward"),
    ("text-window-bad-corpus", "/api/text?ref=JHN.3.16&n=1&corpus=not-a-real-corpus"),
    // SceneQuery
    ("scene-time", "/api/scene?from=-2100&to=-2000"),
    ("scene-scripture", "/api/scene/scripture?ref=JHN.3.16"),
    ("scene-bad-window", "/api/scene?from=100&to=-100"),
    ("scene-bad-ref", "/api/scene/scripture?ref=not-a-ref-at-all"),
    // Versioning -- the one new behavioral endpoint (contract.rs).
    ("contract", "/api/contract"),
];

async fn capture(app: &axum::Router, uri: &str) -> serde_json::Value {
    let response = app.clone().oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = if bytes.is_empty() { serde_json::Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    serde_json::json!({ "status": status, "body": body })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Repo layout: server/atlas-server (this crate) -> ../../data,
    // ../../contracts -- same relative shape `tests/graph_api.rs`'s own
    // `real_app()` already uses for `data/`.
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.join("../..");
    let data_dir = repo_root.join("data");
    let raw_dir = data_dir.join("raw");

    let compiled = atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
        .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first");
    let data: AtlasData = compiled.data;
    let graph = GraphService::build(&raw_dir, &data).expect("data/raw/{kjv.json,xrefs/cross_references.txt} must exist and satisfy the fidelity law");
    let snap = graph.snapshot();

    // FAIL LOUD: every seed must round-trip AND resolve against the real
    // committed graph before anything is written out.
    for (kind, wire_id) in SEEDS {
        let decoded = decode_node_id(wire_id).unwrap_or_else(|| panic!("export_aqc_examples: seed id '{wire_id}' does not even PARSE via graph_wire::decode_node_id -- fix the SEEDS list"));
        let re_encoded = encode_node_id(&decoded);
        assert_eq!(&re_encoded, wire_id, "export_aqc_examples: seed id '{wire_id}' does not round-trip (got '{re_encoded}') -- the G2 wire bijection is broken for this id");
        let actual_kind = format!("{:?}", decoded.kind);
        assert_eq!(&actual_kind, kind, "export_aqc_examples: seed id '{wire_id}' decodes to kind '{actual_kind}', expected '{kind}'");
        snap.node(&decoded).unwrap_or_else(|| panic!("export_aqc_examples: seed id '{wire_id}' does not resolve against the real committed graph -- the curated record it names may have been renamed or removed; pick a new seed"));
    }

    let features_dir = repo_root.join("contracts").join("atlas-query-contract").join("features");
    std::fs::write(features_dir.join("focus-query.feature"), focus_query_feature())?;
    std::fs::write(features_dir.join("exploration-roundtrip.feature"), exploration_roundtrip_feature())?;

    // Fixtures: build the SAME real router (app::build) and capture real
    // HTTP responses -- one file per SEED kind, plus every FIXTURES entry.
    let app = atlas_server::app::build(Arc::new(data), Arc::new(graph), None);
    let fixtures_dir = repo_root.join("contracts").join("atlas-query-contract").join("fixtures");
    std::fs::create_dir_all(&fixtures_dir)?;

    let mut written = 0usize;
    for (kind, wire_id) in SEEDS {
        let name = format!("focus-{}", kind.to_lowercase());
        let uri = format!("/api/node/{}", path_encode(wire_id));
        let value = capture(&app, &uri).await;
        std::fs::write(fixtures_dir.join(format!("{name}.json")), serde_json::to_string_pretty(&value)?)?;
        written += 1;
    }
    for (name, uri) in FIXTURES {
        let value = capture(&app, uri).await;
        std::fs::write(fixtures_dir.join(format!("{name}.json")), serde_json::to_string_pretty(&value)?)?;
        written += 1;
    }

    println!(
        "export_aqc_examples: verified {} seeds against the real committed graph; wrote focus-query.feature + exploration-roundtrip.feature + {written} fixture files",
        SEEDS.len()
    );
    Ok(())
}
