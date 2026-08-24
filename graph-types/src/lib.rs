//! The Explorable Graph — type system base point.
//!
//! Realizes docs/superpowers/specs/2026-08-22-explorable-graph-types.md as
//! compiling Rust. The rule stands: every type either makes an illegal
//! state unrepresentable or names the law a fail-loud validator enforces.
//! The tests at the bottom run the monad laws, the dual involution, the
//! bijection witness, and temporal totality over a toy graph — if this
//! crate compiles and its tests pass, the design composes.
//!
//! Skeleton notes (deliberate, documented simplifications — none change
//! shapes): `Interned` is a String; the content hash is a 64-bit std
//! hash (production: a real multihash); tables are Vecs.

#![allow(dead_code)]

pub mod id;
pub mod text;
pub mod node;
pub mod edge;
pub mod chrono;
pub mod ingest;
pub mod graph;
pub mod explore;
pub mod present;
pub mod store;

pub use edge::{dual, Direction, EdgeId, EdgeKind, RelationId, SymRelationId};
pub use explore::{Explorable, Holdings};
pub use graph::Graph;
pub use store::{GraphPublisher, GraphSnapshot, GraphStore, GraphVersion, MemStore};
pub use id::{AnyNodeId, NodeKind, Pid, Position, PositionKind};
pub use text::{BibleLocus, Locus, TextLocus, TextRef, VerseRef};

/// C1 -- the COVENANT surface (map-system contract set, 2026-08-24):
/// the one import path a sibling system binds to. Everything here is a
/// PROMISE -- names are stable, semantics are stable, and additions are
/// deliberate acts; nothing else in this crate is covenanted. Explicit
/// list on purpose: a wildcard would grow the promise silently.
pub mod covenant {
    /// Chronology authority (contract C2): placements resolve dates;
    /// there is no second dating path.
    pub use crate::chrono::{
        ChronoTarget, DatePlacement, DatedBy, Duration, PlacementBasis, ResolvedDate,
        ResolvedPlacement, SeqKey, TimePoint, Year,
    };
    /// Justification everywhere (covenant rule): a claim carries its why.
    pub use crate::edge::{Ground, GroundTarget, Justification};
    /// Content addressing + typed positions.
    pub use crate::id::{AnyNodeId, ContentAddressed, NodeId, Pid, Position, PositionKind};
    /// Provenance vocabulary.
    pub use crate::ingest::{Confidence, Provenance, ProvenanceId};
    /// Loci: where in the text a claim stands.
    pub use crate::text::{
        BibleLocus, BibleLocusRange, BibleTag, Corpus, Locus, LocusRange, LocusSet,
        TextLocus, TextRef, VerseRef,
    };
    // The `relations!` macro is covenanted too -- `#[macro_export]`
    // hoists it to the crate root (`atlas_graph_types::relations!`);
    // see its own C1 doc comment in edge.rs.
}

#[cfg(test)]
mod tests;
