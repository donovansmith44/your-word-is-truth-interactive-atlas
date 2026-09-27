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
pub mod reading;
pub mod wire;

pub fn openapi_router() -> utoipa_axum::router::OpenApiRouter<app::AppState> {
    utoipa_axum::router::OpenApiRouter::new()
        .merge(meta::routes())
        .merge(map::routes())
        .merge(reading::routes())
        .merge(catechism::routes())
        .merge(places::routes())
        .merge(events::routes())
        .merge(graph::routes())
        .merge(contents::routes())
}

/// The router's raw document. `document::openapi` is the one published form
/// of it, so nothing outside this crate reads the raw one.
pub(crate) fn openapi() -> utoipa::openapi::OpenApi {
    openapi_router().split_for_parts().1
}
