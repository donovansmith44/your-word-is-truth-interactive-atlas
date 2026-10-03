//! Storage, adapters and the in-memory graph build over `atlas-graph-types`, whose `store` module owns the one
//! storage/query port: this crate defines no competing trait, only implementations of it.

pub mod attestation_pending;
pub mod bible_container_adapter;
pub mod brainfuel_adapter;
pub mod build;
pub mod catechism_adapter;
pub mod citations;
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
pub mod labels;
pub mod geography;
pub mod law_check;
pub mod legacy;
pub mod mention_spans;
pub mod node_ref;
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
pub mod references;
pub mod runs;
pub mod scene_source;
pub mod sections;
pub mod service;
pub mod sqlite;
pub mod tokens;
pub mod window;
pub mod xref_adapter;

pub use build::BuildStats;
pub use event_world::{Chronology, ChronologyDerivation, EventWorldStats};
pub use fidelity::{check_kjv_fidelity, FidelityViolation};
pub use pipeline::{pipeline as compiler_pipeline, BuildCtx, Pass};
pub use service::GraphService;
pub use window::WindowDir;

