//! atlas-server library crate: the axum HTTP API over a loaded `AtlasData`.
//! `main.rs` is a thin binary shell (CLI parsing + startup) around
//! `app::build`; integration tests (`tests/api.rs`) exercise the same
//! `app::build` directly via `tower::ServiceExt::oneshot`, which is why this
//! logic lives in a library target rather than only in the binary.

pub mod app;
pub mod error;
pub mod handlers;
