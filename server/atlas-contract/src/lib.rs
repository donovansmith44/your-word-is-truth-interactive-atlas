//! The Bible Atlas HTTP API, as one axum router over the compiled atlas.

pub mod app;
pub mod aqc_export;
pub mod catechism;
pub mod contents;
pub mod document;
pub mod error;
pub mod events;
pub mod graph;
pub mod graph_wire;
pub mod load;
pub mod map;
pub mod meta;
pub mod provenance;
pub mod query;
pub mod reading;
pub mod reference;
pub mod wire;

/// The components no response body registers, and which a reference to them
/// therefore has nothing to resolve against until they are declared here: the
/// vocabularies that reach this document only as query parameters, and the body
/// every route's refusals are answered with.
#[derive(utoipa::OpenApi)]
#[openapi(components(schemas(wire::TextScope, atlas_graph::window::WindowDir, error::ErrorBody, error::ErrorInner, error::ErrorCode)))]
struct ReferencedElsewhere;

pub fn openapi_router() -> utoipa_axum::router::OpenApiRouter<app::AppState> {
    use utoipa::OpenApi;
    utoipa_axum::router::OpenApiRouter::with_openapi(ReferencedElsewhere::openapi())
        .merge(meta::routes())
        .merge(map::routes())
        .merge(reading::routes())
        .merge(catechism::routes())
        .merge(events::routes())
        .merge(graph::routes())
        .merge(contents::routes())
}

/// `document::openapi` is the one published form of this, so nothing outside this
/// crate reads the raw one.
pub(crate) fn openapi() -> utoipa::openapi::OpenApi {
    openapi_router().split_for_parts().1
}
