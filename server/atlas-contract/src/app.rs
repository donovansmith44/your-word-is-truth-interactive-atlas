use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::FromRef;
use axum::http::StatusCode;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use atlas_core::data::AtlasData;
use atlas_core::sources::SourcesDocument;
use atlas_graph::GraphService;

#[derive(Clone)]
pub struct AppState {
    pub data: Arc<AtlasData>,
    pub graph: Arc<GraphService>,
    pub sources: Arc<SourcesDocument>,
}

impl FromRef<AppState> for Arc<AtlasData> {
    fn from_ref(s: &AppState) -> Self {
        s.data.clone()
    }
}

impl FromRef<AppState> for Arc<GraphService> {
    fn from_ref(s: &AppState) -> Self {
        s.graph.clone()
    }
}

impl FromRef<AppState> for Arc<SourcesDocument> {
    fn from_ref(s: &AppState) -> Self {
        s.sources.clone()
    }
}

pub fn build(data: Arc<AtlasData>, graph: Arc<GraphService>, static_dir: Option<PathBuf>) -> Router {
    build_with_sources(data, graph, Arc::new(SourcesDocument::default()), static_dir)
}

pub fn build_with_sources(
    data: Arc<AtlasData>,
    graph: Arc<GraphService>,
    sources: Arc<SourcesDocument>,
    static_dir: Option<PathBuf>,
) -> Router {
    let state = AppState { data, graph, sources };

    let (api, _) = crate::openapi_router().split_for_parts();
    let api = api.with_state(state);

    #[cfg(feature = "dev-docs")]
    let api = api.merge(utoipa_swagger_ui::SwaggerUi::new("/swagger-ui").url("/api/openapi.json", crate::document::openapi()));

    let router = match static_dir {
        Some(dir) => {
            let index = dir.join("index.html");
            let serve_dir = ServeDir::new(dir).not_found_service(ServeFile::new(index));
            // `ServeDir` tags its not-found-service fallback 404, but a deep link to
            // a client-side route is not an error, so the fallback answers 200.
            let serve_dir = tower::ServiceExt::<axum::extract::Request>::map_response(serve_dir, |mut res| {
                if res.status() == StatusCode::NOT_FOUND {
                    *res.status_mut() = StatusCode::OK;
                }
                res
            });
            api.fallback_service(serve_dir)
        }
        None => api,
    };

    // `Router::layer` wraps only the routes present when it is called, and
    // `fallback_service` installs a route that bypasses an earlier layer, so the
    // static fallback is attached above this line rather than below it.
    router.layer(CorsLayer::permissive())
}
