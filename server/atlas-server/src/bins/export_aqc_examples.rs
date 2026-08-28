//! Batch AQC-1's own provider-side exporter (spec §3: "a provider-side
//! exporter draws descriptors from the REAL graph ... and emits/refreshes
//! the committed `Examples:` tables. The generator is code; its OUTPUT is
//! Gherkin"). Regenerates the two `Scenario Outline`-bearing corpus files
//! (`focus-query.feature`, `exploration-roundtrip.feature`) in full,
//! captures the committed pact-style fixtures, and writes the identity
//! INDEX (fix round 1, S-1) -- all from the fixed, disclosed
//! `atlas_server::aqc_export` module, which owns every PURE (no I/O)
//! generation function this binary calls. Each seed id is VERIFIED live
//! against the real compiled graph before anything is written out. A seed
//! id that no longer resolves (a curated record renamed/removed) makes
//! this binary panic -- fail loud, never silently emit a stale example.
//!
//! Deterministic by construction (never wall-clock random): every output
//! is a pure function of `aqc_export`'s own fixed consts plus whatever the
//! real graph/router report for each id -- re-running this binary against
//! an unchanged graph reproduces byte-identical output every time (the
//! determinism proof the batch report cites: `git diff` is empty after a
//! re-run). `tests/aqc_corpus_generation.rs` pins this as a standing test
//! (fix round 1, Q-6): it calls `aqc_export`'s own generation functions
//! directly (no HTTP, no graph) and asserts byte-equality against the
//! committed `.feature` files.
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
use atlas_server::aqc_export::{self, FIXTURES, FOCUS_IDENTITY_EXTRA, SEEDS};
use atlas_server::graph_wire::{decode_node_id, encode_node_id};
use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

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
    std::fs::write(features_dir.join("focus-query.feature"), aqc_export::focus_query_feature())?;
    std::fs::write(features_dir.join("exploration-roundtrip.feature"), aqc_export::exploration_roundtrip_feature())?;

    // Fixtures: build the SAME real router (app::build) and capture real
    // HTTP responses -- one file per SEED kind, plus every FIXTURES entry.
    let app = atlas_server::app::build(Arc::new(data), Arc::new(graph), None);
    let fixtures_dir = repo_root.join("contracts").join("atlas-query-contract").join("fixtures");
    std::fs::create_dir_all(&fixtures_dir)?;

    // S-1 fix (fix round 1): wire id -> fixture-name identity INDEX, keyed
    // by the REQUEST id (never the response's own echoed `id`) -- see
    // `aqc_export::FOCUS_IDENTITY_EXTRA`'s own doc comment for why this
    // makes the C# harness's round-trip proof genuinely failable.
    let mut identity_index: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();

    let mut written = 0usize;
    for (kind, wire_id) in SEEDS {
        let name = format!("focus-{}", kind.to_lowercase());
        let uri = format!("/api/node/{}", aqc_export::path_encode(wire_id));
        let value = capture(&app, &uri).await;
        std::fs::write(fixtures_dir.join(format!("{name}.json")), serde_json::to_string_pretty(&value)?)?;
        identity_index.insert((*wire_id).to_string(), name);
        written += 1;
    }
    for (wire_id, name) in FOCUS_IDENTITY_EXTRA {
        identity_index.insert((*wire_id).to_string(), (*name).to_string());
    }
    for (name, uri) in FIXTURES {
        let value = capture(&app, uri).await;
        std::fs::write(fixtures_dir.join(format!("{name}.json")), serde_json::to_string_pretty(&value)?)?;
        written += 1;
    }

    std::fs::write(fixtures_dir.join("index.json"), serde_json::to_string_pretty(&identity_index)?)?;

    println!(
        "export_aqc_examples: verified {} seeds against the real committed graph; wrote focus-query.feature + exploration-roundtrip.feature + {written} fixture files + index.json ({} identity entries)",
        SEEDS.len(),
        identity_index.len()
    );
    Ok(())
}
