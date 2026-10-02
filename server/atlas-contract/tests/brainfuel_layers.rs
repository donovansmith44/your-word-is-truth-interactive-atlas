use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use atlas_core::data::AtlasData;
use atlas_graph::GraphService;
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::TranslationId;

/// The real corpus and the graph built from it, both shared: every reader below only reads them,
/// and building either one per test cost this file six compiles and six graph builds.
fn real_atlas() -> (Arc<AtlasData>, Arc<GraphService>) {
    static CACHED: std::sync::OnceLock<(Arc<AtlasData>, Arc<GraphService>)> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            let data = atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
                .data;
            let graph = GraphService::build(&data_dir.join("raw"), &data)
                .expect("data/raw/{kjv.json,xrefs/cross_references.txt,brain-fuel-bible} must exist and satisfy the fidelity law");
            (Arc::new(data), Arc::new(graph))
        })
        .clone()
}

fn real_graph() -> Arc<GraphService> {
    real_atlas().1
}

fn real_app() -> axum::Router {
    let (data, graph) = real_atlas();
    atlas_contract::app::build(data, graph, None)
}

async fn get_json(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let resp = app.oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = if bytes.is_empty() { serde_json::Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (status, json)
}

fn renderings_of(snap: &impl GraphQuery, id: &atlas_graph_types::id::AnyNodeId) -> HashMap<String, String> {
    let node = snap.node(id).expect("node must exist");
    match node.payload {
        NodePayload::TextUnit { renderings, .. } => renderings.into_iter().map(|(TranslationId(k), v)| (k, v)).collect(),
        other => panic!("expected TextUnit, got {other:?}"),
    }
}

#[test]
fn genesis_1_1_carries_every_ot_applicable_edition_on_the_real_graph() {
    let svc = real_graph();
    let snap = svc.snapshot();
    let id = atlas_graph::kjv_adapter::verse_node_id(0, 1, 1);
    let r = renderings_of(&snap, &id);

    assert_eq!(r.get("kjv").map(String::as_str), Some("In the beginning God created the heaven and the earth."), "canonical layer untouched");
    assert_eq!(r.get("latin_vulgate").map(String::as_str), Some("In principio creavit Deus cælum et terram. "));
    assert_eq!(r.get("hebrew_masoretic").map(String::as_str), Some("בְּרֵאשִׁ֖ית בָּרָ֣א אֱלֹהִ֑ים אֵ֥ת הַשָּׁמַ֖יִם וְאֵ֥ת הָאָֽרֶץ׃"));
    assert_eq!(r.get("douay_rheims").map(String::as_str), Some("In the beginning God created heaven, and earth."));
    assert_eq!(r.get("finnish_biblia").map(String::as_str), Some("Alussa loi Jumala taivaan ja maan. "));
    assert_eq!(r.get("swedish_karl_xii").map(String::as_str), Some("J Begynnelsen skapade Gudh Himmel och Jord."));
    assert!(!r.contains_key("greek_textus_receptus"), "TR must not apply to the OT: {r:?}");
    assert_eq!(r.len(), 6, "kjv + 5 -- no stray layers: {r:?}");
}

#[test]
fn john_1_1_carries_every_nt_applicable_edition_on_the_real_graph() {
    let svc = real_graph();
    let snap = svc.snapshot();
    let id = atlas_graph::kjv_adapter::verse_node_id(42, 1, 1);
    let r = renderings_of(&snap, &id);

    assert_eq!(r.get("kjv").map(String::as_str), Some("In the beginning was the Word, and the Word was with God, and the Word was God."));
    assert_eq!(r.get("greek_textus_receptus").map(String::as_str), Some("ἐν ἀρχῇ ἦν ὁ λόγος καὶ ὁ λόγος ἦν πρὸς τὸν θεόν καὶ θεὸς ἦν ὁ λόγος"));
    assert!(r.get("latin_vulgate").is_some(), "Vulgate applies to the NT too");
    assert!(r.get("finnish_biblia").is_some());
    assert!(r.get("swedish_karl_xii").is_some());
    assert!(!r.contains_key("hebrew_masoretic"), "WLC must not apply to the NT: {r:?}");
    assert!(!r.contains_key("douay_rheims"), "Douay-Rheims does not apply to the NT in this dataset: {r:?}");
    assert_eq!(r.len(), 5, "kjv + 4 -- no stray layers: {r:?}");
}

#[tokio::test]
async fn a_verses_record_still_projects_only_the_canonical_kjv_text() {
    let (status, body) = get_json(real_app(), "/api/node/text-unit:GEN.1.1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["text"]["text"], "In the beginning God created the heaven and the earth.");
    assert!(body.get("renderings").is_none());
    assert!(body.get("latin_vulgate").is_none());
}

#[tokio::test]
async fn api_text_window_still_projects_only_the_canonical_kjv_text() {
    let (status, body) = get_json(real_app(), "/api/text?ref=JHN.1.1&scope=verse").await;
    assert_eq!(status, StatusCode::OK);
    let units = body["units"].as_array().expect("units array");
    assert_eq!(units.len(), 1);
    assert_eq!(units[0]["body"]["text"], "In the beginning was the Word, and the Word was with God, and the Word was God.");
    assert!(units[0].get("renderings").is_none());
}

#[tokio::test]
async fn every_ingested_translation_node_is_reachable_through_the_existing_generic_node_endpoint() {
    let app = real_app();
    for (slug, label) in [
        ("latin_vulgate", "Clementine Vulgate"),
        ("hebrew_masoretic", "Westminster Leningrad Codex"),
        ("douay_rheims", "Douay-Rheims (Challoner)"),
        ("finnish_biblia", "Biblia 1776"),
        ("swedish_karl_xii", "Karl XII:s Bibel (1703)"),
        ("greek_textus_receptus", "Greek Textus Receptus"),
    ] {
        let uri = format!("/api/node/Translation:{slug}");
        let (status, body) = get_json(app.clone(), &uri).await;
        assert_eq!(status, StatusCode::OK, "GET {uri} must succeed -- a rendering's own TranslationId must resolve to a real, reachable node");
        assert_eq!(body["kind"], "Translation");
        assert_eq!(body["label"], label);
        assert_eq!(body["provenance"], "brainfuel");
    }
}

#[tokio::test]
async fn no_kjv_translation_node_is_authored_this_batch() {
    let (status, _) = get_json(real_app(), "/api/node/Translation:kjv").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
