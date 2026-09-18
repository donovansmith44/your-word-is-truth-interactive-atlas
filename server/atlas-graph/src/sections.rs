//! The section map (spec §2.1) and the per-section logical dump / version
//! root (spec §3.4) live in `atlas_graph_types::sections` since DB-4a, so
//! `MemStore::publish` can stamp the manifest root. This module is the
//! re-export every existing caller keeps using.
pub use atlas_graph_types::sections::*;
