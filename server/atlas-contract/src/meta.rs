use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use atlas_core::sources::SourcesDocument;

use crate::wire;

#[utoipa::path(get, path = "/health", responses((status = 200, body = String, content_type = "text/plain")), tag = "meta")]
pub async fn health() -> &'static str {
    "ok"
}

/// `GET /api/sources` (batch-s-brief.md requirement 3): the Sources
/// page's entire single source of truth, straight off
/// `data/compiled/sources.json` (itself generated 1:1 from LICENSES.md by
/// `atlas_etl::sources`'s own fail-loud drift check -- see the
/// `gen_sources` binary). The client renders this directly; nothing here
/// is a hardcoded duplicate list.
#[utoipa::path(get, path = "/api/sources", responses((status = 200, body = atlas_core::sources::SourcesDocument)), tag = "meta")]
pub async fn sources(State(sources): State<Arc<SourcesDocument>>) -> Json<SourcesDocument> {
    Json((*sources).clone())
}

/// Pre-launch there is no range of earlier contract versions to support, so the
/// minimum and the maximum are the one version this server implements.
pub const MIN_SUPPORTED_VERSION: &str = "0.8.0";
pub const MAX_SUPPORTED_VERSION: &str = "0.8.0";

#[utoipa::path(get, path = "/api/contract", responses((status = 200, body = wire::Contract)), tag = "meta")]
pub async fn contract() -> Json<wire::Contract> {
    Json(wire::Contract {
        min_version: MIN_SUPPORTED_VERSION.to_string(),
        max_version: MAX_SUPPORTED_VERSION.to_string(),
        manifest_schema: atlas_graph::sqlite::manifest::MANIFEST_SCHEMA,
        section_schema_version: atlas_graph::sections::SECTION_SCHEMA_VERSION,
    })
}

/// The published contract itself, served from the same document
/// `contracts/openapi.yaml` is generated from -- a consumer reads the
/// running server's contract without reaching for the repository. Rendered
/// once: the document cannot change while the process runs.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn advertises_the_pinned_aqc_version_range() {
        let Json(body) = contract().await;
        assert_eq!(body.min_version, "0.8.0");
        assert_eq!(body.max_version, "0.8.0");
        assert_eq!((body.manifest_schema, body.section_schema_version), (1, 14));
    }
}
