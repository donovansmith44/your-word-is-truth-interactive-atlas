use std::sync::Arc;

use atlas_core::data::demo_fixture;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn app() -> axum::Router {
    let data = demo_fixture();
    let graph = atlas_graph::GraphService::from_canon_and_verses(&data.canon, &data.verses, "", &data).expect("fixture graph must build");
    atlas_contract::app::build(Arc::new(data), Arc::new(graph), None)
}

#[tokio::test]
async fn the_contract_declares_only_the_schema_versions_it_was_built_with() {
    // Arrange
    let app = app();
    let expected = serde_json::json!({
        "manifest_schema": 1,
        "section_schema_version": 24,
    });
    // Act
    let answered = get_json(&app, "/api/contract").await;
    // Assert
    assert_eq!(answered, (StatusCode::OK, expected));
}

#[test]
fn every_served_route_is_documented() {
    // Arrange
    let expected = [
        "/health", "/api/contract", "/api/scene", "/api/scene/scripture", "/api/books",
        "/api/chapter/{cref}", "/api/kretzmann/chapter/{cref}", "/api/xrefs/{sref}",
        "/api/catechism/item/{id}", "/api/catechism/{sref}", "/api/narratives",
        "/api/narrative/event/{id}", "/api/event/{id}", "/api/eras", "/api/polities", "/api/landmarks",
        "/api/land-mask", "/api/sources", "/api/node/{id}", "/api/node/{id}/edges", "/api/elements", "/api/text",
        "/api/contents/{corpus}", "/api/openapi.yaml",
    ];
    // Act
    let doc = atlas_contract::document::openapi();
    let mut paths: Vec<&str> = doc.paths.paths.keys().map(String::as_str).collect();
    paths.sort_unstable();
    // Assert
    let mut want = expected.to_vec();
    want.sort_unstable();
    assert_eq!(paths, want);
}

#[tokio::test]
async fn the_served_openapi_document_equals_the_committed_one() {
    // Arrange
    let app = app();
    let committed = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../contracts/openapi.yaml")).expect("contracts/openapi.yaml must exist");
    // Act
    let served = get_text(&app, "/api/openapi.yaml").await;
    // Assert
    assert_eq!(served, committed);
}

async fn get_json(app: &axum::Router, path: &str) -> (StatusCode, serde_json::Value) {
    let response = app.clone().oneshot(Request::builder().uri(path).body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).expect("the body is JSON"))
}

async fn get_text(app: &axum::Router, path: &str) -> String {
    let response = app.clone().oneshot(Request::builder().uri(path).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK, "GET {path}");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).expect("the body is UTF-8 text")
}

#[cfg(feature = "dev-docs")]
#[tokio::test]
async fn the_developer_docs_page_is_served_when_dev_docs_is_on() {
    // Arrange
    let app = app();
    // Act
    let response = app.oneshot(Request::builder().uri("/swagger-ui/").body(Body::empty()).unwrap()).await.unwrap();
    // Assert
    assert_eq!(response.status(), StatusCode::OK);
}
