//! atlas-graph: storage/adapters/build over `atlas-graph-types` (Batch M-A).
//!
//! `graph-types/` stays pure types+laws (the owner-approved compiling
//! authority; nothing in it changes here). This crate is the pragmatic M-A
//! materialization the controller ordered (ruling 2): it builds the
//! Explorable Graph IN MEMORY, once, at server startup, from the same raw
//! sources `atlas-etl` reads -- the KJV text (`kjv_adapter`) and
//! openbible.info's cross-references (`xref_adapter`, the first relation
//! proving the generic edge path end-to-end, ruling 3). `state::GraphState`
//! is the one handle `atlas-server` holds; `fidelity` is the KJV adapter's
//! fail-loud boundary law (bijection + reconstruction, design doc P3).
//!
//! The serialized one-artifact form (design doc P1, fully realized) lands
//! at M-C when the parallel `data/compiled/*.json` artifacts retire; until
//! then this crate and `atlas-etl` both derive from the same `data/raw/`,
//! disclosed as a strangler deviation, not a quiet one.

pub mod build;
pub mod fidelity;
pub mod kjv_adapter;
pub mod state;
pub mod version;
pub mod xref_adapter;

pub use build::BuildStats;
pub use fidelity::{check_kjv_fidelity, FidelityViolation};
pub use state::{GraphState, WindowDir};
pub use version::GraphVersion;
