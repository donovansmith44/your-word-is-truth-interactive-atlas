use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use atlas_core::sources::SourcesDocument;

use crate::wire;

/// Reports that the server is up, as the plain text `ok`.
#[utoipa::path(get, path = "/health", responses((status = 200, body = String, content_type = "text/plain")), tag = "meta")]
pub async fn health() -> &'static str {
    "ok"
}

/// Every source this atlas is built from, and how each one is licensed.
///
/// `categories` are the headings the `sources` are grouped under, and
/// `provenances` join the provenance id that rides on an individual record
/// back to the source that asserts it.
#[utoipa::path(get, path = "/api/sources", responses((status = 200, body = atlas_core::sources::SourcesDocument)), tag = "meta")]
pub async fn sources(State(sources): State<Arc<SourcesDocument>>) -> Json<SourcesDocument> {
    Json((*sources).clone())
}

/// Pre-launch there is no range of earlier contract versions to support, so the
/// minimum and the maximum are the one version this server implements.
pub const MIN_SUPPORTED_VERSION: &str = "0.10.0";
pub const MAX_SUPPORTED_VERSION: &str = "0.10.0";

/// The range of contract versions this server answers for.
///
/// The two schema versions identify the compiled data set behind the
/// responses.
#[utoipa::path(get, path = "/api/contract", responses((status = 200, body = wire::Contract)), tag = "meta")]
pub async fn contract() -> Json<wire::Contract> {
    Json(wire::Contract {
        min_version: MIN_SUPPORTED_VERSION.to_string(),
        max_version: MAX_SUPPORTED_VERSION.to_string(),
        manifest_schema: atlas_graph::sqlite::manifest::MANIFEST_SCHEMA,
        section_schema_version: atlas_graph::sections::SECTION_SCHEMA_VERSION,
    })
}

/// This API's own OpenAPI document, as YAML.
///
/// A consumer reads the contract off the running server rather than out of the
/// repository the server was built from.
#[utoipa::path(get, path = "/api/openapi.yaml", responses((status = 200, body = String, content_type = "application/yaml")), tag = "meta")]
pub async fn openapi_yaml() -> ([(axum::http::HeaderName, &'static str); 1], String) {
    // The document cannot change while the process runs.
    static RENDERED: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let document = RENDERED.get_or_init(crate::document::openapi_yaml);
    ([(axum::http::header::CONTENT_TYPE, "application/yaml")], document.clone())
}

pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(health))
        .routes(routes!(contract))
        .routes(routes!(sources))
        .routes(routes!(openapi_yaml))
}

