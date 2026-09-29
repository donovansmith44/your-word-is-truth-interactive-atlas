//! Source lives under `src/bins/` rather than `src/bin/` because the repository
//! root's `.gitignore` carries a broad `**/bin/` rule for the client's build
//! output; the `[[bin]]` table in `Cargo.toml` names this path explicitly.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use atlas_core::data::AtlasData;
use atlas_graph::GraphService;
use atlas_graph_types::store::GraphQuery;
use atlas_contract::aqc_export::{self, FIXTURES, FOCUS_IDENTITY_EXTRA, SEEDS};
use atlas_contract::graph_wire::{decode_node_id, encode_node_id};
use axum::body::Body;
use axum::http::Request;
use tower::ServiceExt;

const USAGE: &str = "usage: export_aqc_examples [CONTRACT_DIR]";
const MISUSE: i32 = 2;

async fn capture(app: &axum::Router, uri: &str) -> serde_json::Value {
    let response = app.clone().oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = if bytes.is_empty() { serde_json::Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    serde_json::json!({ "status": status, "body": body })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.join("../..");
    let contract_dir = match Vec::from_iter(std::env::args().skip(1)).as_slice() {
        [] => repo_root.join("contracts").join("atlas-query-contract"),
        [dir] => PathBuf::from(dir),
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(MISUSE);
        }
    };
    let data_dir = repo_root.join("data");
    let raw_dir = data_dir.join("raw");

    let compiled = atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
        .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first");
    let data: AtlasData = compiled.data;
    let graph = GraphService::build(&raw_dir, &data).expect("data/raw/{kjv.json,xrefs/cross_references.txt} must exist and satisfy the fidelity law");
    let snap = graph.snapshot();

    for (kind, wire_id) in SEEDS {
        let decoded = decode_node_id(wire_id).unwrap_or_else(|| panic!("export_aqc_examples: seed id '{wire_id}' does not even PARSE via graph_wire::decode_node_id -- fix the SEEDS list"));
        let re_encoded = encode_node_id(&decoded);
        assert_eq!(&re_encoded, wire_id, "export_aqc_examples: seed id '{wire_id}' does not round-trip (got '{re_encoded}') -- the G2 wire bijection is broken for this id");
        let actual_kind = decoded.kind.name();
        assert_eq!(&actual_kind, kind, "export_aqc_examples: seed id '{wire_id}' decodes to kind '{actual_kind}', expected '{kind}'");
        snap.node(&decoded).unwrap_or_else(|| panic!("export_aqc_examples: seed id '{wire_id}' does not resolve against the real committed graph -- the curated record it names may have been renamed or removed; pick a new seed"));
    }

    let features_dir = contract_dir.join("features");
    std::fs::create_dir_all(&features_dir)?;
    std::fs::write(features_dir.join("focus-query.feature"), aqc_export::focus_query_feature())?;
    std::fs::write(features_dir.join("exploration-roundtrip.feature"), aqc_export::exploration_roundtrip_feature())?;

    let app = atlas_contract::app::build(Arc::new(data), Arc::new(graph), None);
    let fixtures_dir = contract_dir.join("fixtures");
    std::fs::create_dir_all(&fixtures_dir)?;

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
