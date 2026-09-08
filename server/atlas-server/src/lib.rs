//! atlas-server library crate: the axum HTTP API over a loaded `AtlasData`.
//! `main.rs` is a thin binary shell (CLI parsing + startup) around
//! `app::build`; integration tests (`tests/api.rs`) exercise the same
//! `app::build` directly via `tower::ServiceExt::oneshot`, which is why this
//! logic lives in a library target rather than only in the binary.

pub mod app;
pub mod aqc_export;
pub mod contract;
pub mod error;
pub mod graph_handlers;
pub mod graph_wire;
pub mod handlers;
/// CDC-1 fix round 1 (review C-3): the ONE assembly path from a `data_dir`
/// to a serving `Router`, shared by `main.rs` and by the contract-pact
/// recorder so the recorded evidence cannot drift from what the real server
/// serves. See the module's own header for the two green-suite fidelity
/// bugs that made it necessary.
pub mod load;
