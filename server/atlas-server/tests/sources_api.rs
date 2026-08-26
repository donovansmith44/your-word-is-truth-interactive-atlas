//! HTTP-level tests for `GET /api/sources` (batch-s-brief.md requirement
//! 3/4). Uses `demo_fixture()` + a minimal graph for `data`/`graph` (this
//! endpoint's own response never touches either) alongside the REAL
//! `data/curated/sources.toml`, parsed the same way `gen_sources` does --
//! so this proves the endpoint really serves the Sources page's real
//! curated content end to end, without paying for a full raw+curated
//! `AtlasData` compile the way `graph_api.rs`'s own `real_app()` does.

use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use atlas_core::data::{demo_fixture, AtlasData};
use atlas_core::sources::SourcesDocument;
use atlas_graph::GraphService;

fn minimal_graph(data: &AtlasData) -> Arc<GraphService> {
    Arc::new(
        GraphService::from_canon_and_verses(&data.canon, &data.verses, "", data)
            .expect("demo_fixture()'s own canon+verses must build a graph"),
    )
}

fn real_sources_document() -> SourcesDocument {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/curated/sources.toml");
    let input = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let doc = atlas_etl::sources::parse_sources(&input).expect("data/curated/sources.toml must parse");
    atlas_etl::sources::validate_structure(&doc).expect("data/curated/sources.toml structural validation");
    doc
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let response = app.clone().oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("GET {uri} did not return JSON: {e}"));
    (status, json)
}

#[tokio::test]
async fn get_api_sources_returns_every_real_curated_source_and_category() {
    let data = demo_fixture();
    let graph = minimal_graph(&data);
    let sources = Arc::new(real_sources_document());
    let expected_categories = sources.categories.len();
    let expected_sources = sources.sources.len();

    let app = atlas_server::app::build_with_sources(Arc::new(data), graph, sources, None);

    let (status, body) = get(&app, "/api/sources").await;
    assert_eq!(status, StatusCode::OK);

    let categories = body["categories"].as_array().expect("categories array");
    let rows = body["sources"].as_array().expect("sources array");
    assert_eq!(categories.len(), expected_categories, "GET /api/sources category count must match data/curated/sources.toml");
    assert_eq!(rows.len(), expected_sources, "GET /api/sources source count must match data/curated/sources.toml");

    // Every source row carries the fields the Sources page needs to
    // render honest, non-hardcoded prose -- a wire-shape regression here
    // would silently break the page without any Rust-side type error
    // (the client deserializes this same JSON independently).
    for row in rows {
        for field in ["id", "category", "title", "what_it_is", "what_we_built", "license"] {
            assert!(
                row.get(field).and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty()),
                "source row {row} is missing a non-empty '{field}'"
            );
        }
    }
}

#[tokio::test]
async fn api_sources_defaults_to_empty_when_build_called_without_sources() {
    // `app::build` (the plain, pre-existing signature every other test in
    // this crate keeps using) delegates to `build_with_sources` with an
    // empty `SourcesDocument` -- proves that delegation actually happens,
    // not just that it compiles.
    let data = demo_fixture();
    let graph = minimal_graph(&data);
    let app = atlas_server::app::build(Arc::new(data), graph, None);

    let (status, body) = get(&app, "/api/sources").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["categories"].as_array().unwrap().len(), 0);
    assert_eq!(body["sources"].as_array().unwrap().len(), 0);
}
