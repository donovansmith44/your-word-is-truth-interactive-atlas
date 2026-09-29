//! Storage, adapters and the in-memory graph build over `atlas-graph-types`, whose `store` module owns the one
//! storage/query port: this crate defines no competing trait, only implementations of it.

pub mod attestation_pending;
pub mod bible_container_adapter;
pub mod brainfuel_adapter;
pub mod build;
pub mod catechism_adapter;
pub mod concord_adapter;
pub mod corpus_root;
pub mod description_adapter;
pub mod era_adapter;
pub mod event_world;
pub mod exports;
pub mod fidelity;
pub mod fulfillment_adapter;
pub mod heading;
pub mod kjv_adapter;
pub mod kretzmann_adapter;
pub mod law_check;
pub mod legacy;
pub mod lexicon_adapter;
pub mod map_adapter;
pub mod peoples_adapter;
pub mod person_adapter;
pub mod pipeline;
pub mod place_adapter;
pub mod polity_adapter;
pub mod provenance;
pub mod red_letter_adapter;
pub mod red_letter_spans;
pub mod scene_source;
pub mod sections;
pub mod service;
pub mod sqlite;
pub mod window;
pub mod xref_adapter;

pub use build::BuildStats;
pub use event_world::{Chronology, ChronologyDerivation, EventWorldStats};
pub use fidelity::{check_kjv_fidelity, FidelityViolation};
pub use pipeline::{pipeline as compiler_pipeline, BuildCtx, Pass};
pub use service::GraphService;
pub use window::WindowDir;

/// The wire/ETag form of a `GraphVersion`: fixed-width lowercase hex, so equal stamps compare
/// byte-identical as `If-None-Match` strings. The width follows `ContentHash`, which the
/// `canon-ids` feature widens, so it is never spelled out here.
pub fn version_hex(v: atlas_graph_types::store::GraphVersion) -> String {
    v.0.hex()
}
