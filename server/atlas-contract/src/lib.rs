//! atlas-contract library crate: the axum HTTP API over a loaded `AtlasData`.
//! `main.rs` is a thin binary shell (CLI parsing + startup) around
//! `app::build`; integration tests (`tests/api.rs`) exercise the same
//! `app::build` directly via `tower::ServiceExt::oneshot`, which is why this
//! logic lives in a library target rather than only in the binary.

pub mod app;
pub mod aqc_export;
pub mod catechism;
pub mod contents;
pub mod document;
pub mod error;
pub mod events;
pub mod graph;
pub mod graph_wire;
/// CDC-1 fix round 1 (review C-3): the ONE assembly path from a `data_dir`
/// to a serving `Router`, shared by `main.rs` and by the contract-pact
/// recorder so the recorded evidence cannot drift from what the real server
/// serves. See the module's own header for the two green-suite fidelity
/// bugs that made it necessary.
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

/// The router's own raw document. `document::openapi` is the one published
/// form of it -- named, versioned and carrying the relation manifest -- so
/// nothing outside this crate reads the raw one.
pub(crate) fn openapi() -> utoipa::openapi::OpenApi {
    openapi_router().split_for_parts().1
}
