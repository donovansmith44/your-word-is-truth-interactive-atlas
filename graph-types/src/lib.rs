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

#[cfg(test)]
mod tests;
