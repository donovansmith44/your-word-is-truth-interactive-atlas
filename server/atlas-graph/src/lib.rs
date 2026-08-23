//! atlas-graph: storage/adapters/build over `atlas-graph-types` (Batch M-A).
//!
//! `graph-types/` stays pure types+laws (the owner-approved compiling
//! authority; nothing in it changes here). This crate is the pragmatic M-A
//! materialization the controller ordered (ruling 2): it builds the
//! Explorable Graph IN MEMORY, once, at server startup, from the same raw
//! sources `atlas-etl` reads -- the KJV text (`kjv_adapter`) and
//! openbible.info's cross-references (`xref_adapter`, the first relation
//! proving the generic edge path end-to-end, ruling 3).
//!
//! STORAGE PORTABILITY (design doc §9a; fix round 1, C1): the storage/
//! query port -- `GraphQuery`/`GraphSnapshot`/`GraphStore`/`GraphPublisher`
//! -- is OWNER-APPROVED and lives entirely in `atlas_graph_types::store`
//! (commit `ab75c8b`). This crate defines NO competing trait -- `window.rs`
//! (the window/text-path logic) is generic over `&dyn
//! atlas_graph_types::store::GraphQuery` only, and `service::GraphService`
//! (this batch's implementation #1) is a thin adapter-side wrapper around
//! the owner's own `MemStore`/`MemSnapshot`, plus the one companion index
//! the port itself doesn't model (a reading-spine reverse lookup -- see
//! `service.rs`'s own doc comment for why that's a disclosed companion,
//! not scope creep). `fidelity` is the KJV adapter's fail-loud boundary
//! law (bijection + reconstruction, design doc P3), enforced automatically
//! inside `GraphService::from_sources`/`build` -- an ingestion-time concern
//! of this backend, not part of the generic port.
//!
//! The serialized one-artifact form (design doc P1, fully realized) lands
//! at M-C when the parallel `data/compiled/*.json` artifacts retire; until
//! then this crate and `atlas-etl` both derive from the same `data/raw/`,
//! disclosed as a strangler deviation, not a quiet one.

pub mod build;
pub mod catechism_adapter;
pub mod era_adapter;
pub mod event_world;
pub mod fidelity;
pub mod kjv_adapter;
pub mod law_check;
pub mod pipeline;
pub mod place_adapter;
pub mod polity_adapter;
pub mod service;
pub mod window;
pub mod xref_adapter;

pub use build::BuildStats;
pub use event_world::{Chronology, ChronologyDerivation, EventWorldStats};
pub use fidelity::{check_kjv_fidelity, FidelityViolation};
pub use pipeline::{pipeline as compiler_pipeline, BuildCtx, Pass};
pub use service::GraphService;
pub use window::WindowDir;

/// The wire/ETag form of a `GraphVersion` -- fixed-width lowercase hex, so
/// equal stamps are byte-identical strings (no whitespace/case ambiguity
/// for `If-None-Match` comparisons). A free function here, not a new impl
/// added directly inside `graph-types` itself (deliberately -- the port is
/// owner-approved and this batch touches no `graph-types` source file at
/// all, even for a small additive `Display` impl; `GraphVersion`'s own
/// `.0.0` field access is public, so this needs no such impl to exist).
pub fn version_hex(v: atlas_graph_types::store::GraphVersion) -> String {
    format!("{:016x}", v.0 .0)
}
