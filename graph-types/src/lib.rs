#![allow(dead_code)]

pub mod id;
pub mod text;
pub mod node;
pub mod edge;
pub mod chrono;
pub mod ingest;
pub mod graph;
pub mod explore;
pub mod frontier;
pub mod present;
pub mod store;
#[cfg(any(feature = "serde", feature = "openapi"))]
pub mod wire_form;
pub mod canon;
pub mod raw_manifest;
pub mod sections;
pub mod sha256;
pub mod vocabulary;

pub use edge::{dual, Direction, EdgeId, EdgeKind, RelationId, SymRelationId};
pub use explore::{Explorable, Holdings};
pub use graph::Graph;
pub use store::{GraphPublisher, GraphSnapshot, GraphStore, GraphVersion, MemStore};
pub use id::{AnyNodeId, NodeKind, Pid, Position, PositionKind};
pub use text::{BibleLocus, Locus, TextLocus, TextRef, VerseRef};

pub mod covenant {
    pub use crate::chrono::{
        ChronoTarget, DatePlacement, DatedBy, Duration, PlacementBasis, ResolvedDate,
        ResolvedPlacement, SeqKey, TimePoint, Year,
    };
    pub use crate::edge::{Ground, GroundTarget, Justification};
    pub use crate::id::{AnyNodeId, ContentAddressed, ContentHash, NodeId, Pid, Position, PositionKind};
    pub use crate::id::{EventId, PlaceId, SourceId};
    pub use crate::ingest::{Confidence, Provenance, ProvenanceId};
    pub use crate::text::{
        BibleLocus, BibleLocusRange, BibleTag, Corpus, Locus, LocusRange, LocusSet,
        TextLocus, TextRef, VerseRef,
    };
}

#[cfg(test)]
mod tests;
