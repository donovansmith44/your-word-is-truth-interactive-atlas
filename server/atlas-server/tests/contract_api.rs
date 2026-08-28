//! Batch AQC-1: `GET /api/contract`'s own HTTP-level test -- the ONE new
//! behavioral surface this batch adds (design spec §2's versioning law).
//! Mirrors `tests/api.rs`'s own no-socket-bound `oneshot` idiom, over
//! `atlas_core::data::demo_fixture()` (this endpoint is static -- it needs
//! no real graph data, unlike `tests/graph_api.rs`'s own `real_app()`).

use std::sync::Arc;

use atlas_core::data::demo_fixture;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn app() -> axum::Router {
    let data = demo_fixture();
    let graph = atlas_graph::GraphService::from_canon_and_verses(&data.canon, &data.verses, "", &data).expect("fixture graph must build");
    atlas_server::app::build(Arc::new(data), Arc::new(graph), None)
}

#[tokio::test]
async fn api_contract_advertises_the_pinned_aqc_version_range() {
    let response = app().oneshot(Request::builder().uri("/api/contract").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["min_version"], "0.1.0");
    assert_eq!(body["max_version"], "0.1.0");
}

#[tokio::test]
async fn api_contract_carries_no_other_fields() {
    let response = app().oneshot(Request::builder().uri("/api/contract").body(Body::empty()).unwrap()).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let obj = body.as_object().expect("ContractOut must serialize as a JSON object");
    let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, vec!["max_version", "min_version"]);
}
