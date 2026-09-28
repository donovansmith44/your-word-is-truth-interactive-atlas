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
pub mod places;
pub mod query;
pub mod reading;
pub mod reference;
pub mod wire;

/// The vocabularies that reach this document only as query parameters: no response
/// body carries one, so nothing else registers them, and a parameter's reference to
/// one must resolve.
#[derive(utoipa::OpenApi)]
#[openapi(components(schemas(wire::TextScope, atlas_graph::window::WindowDir)))]
struct QueryVocabularies;

pub fn openapi_router() -> utoipa_axum::router::OpenApiRouter<app::AppState> {
    use utoipa::OpenApi;
    utoipa_axum::router::OpenApiRouter::with_openapi(QueryVocabularies::openapi())
        .merge(meta::routes())
        .merge(map::routes())
        .merge(reading::routes())
        .merge(catechism::routes())
        .merge(places::routes())
        .merge(events::routes())
        .merge(graph::routes())
        .merge(contents::routes())
}

/// `document::openapi` is the one published form of this, so nothing outside this
/// crate reads the raw one.
pub(crate) fn openapi() -> utoipa::openapi::OpenApi {
    openapi_router().split_for_parts().1
}
