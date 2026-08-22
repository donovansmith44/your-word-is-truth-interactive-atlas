//! Ingestion: source-agnostic assertions with uniform provenance;
//! boundary laws prove fidelity at the only moment source and graph
//! coexist.

use crate::chrono::{DatePlacement, PlacementBasis};
use crate::edge::Justification;
use crate::id::{AnyNodeId, EventId, Interned, SourceId};
use crate::text::{BibleLocusRange, LayerMap, TextLocus, TranslationId, VerseRef};

pub type ProvenanceId = Interned;

/// Who asserts: source + locator. (WHY a claim stands is Justification —
/// a different thing, deliberately.)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    pub source: SourceId,
    pub locator: String,
    pub confidence: Confidence,
}

/// Confidence for a corpus's TEXT derives from its role at the registry —
/// CanonicalText iff NormaNormans; an adapter cannot claim canonical
/// standing for extrabiblical text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Confidence {
    CanonicalText,
    Curated,
    Imported,
    Derived,
}

#[derive(Clone, Debug)]
pub struct Asserted<T> {
    pub fact: T,
    pub provenance: Provenance,
}

/// The assertion vocabulary — the source-neutral IR every adapter emits.
/// (Representative set; extended per the manifest as node/edge kinds
/// grow.)
#[derive(Clone, Debug)]
pub enum Assertion {
    TextUnit { corpus: &'static str, unit: TextLocus, renderings: LayerMap },
    Container { id: AnyNodeId, title: String },
    Event { id: EventId, label: String },
    Attestation { event: EventId, attestation: BibleLocusRange, justification: Justification },
    SuccessionChain { narrative: AnyNodeId, chain: Vec<EventId> },
    Anchor { id: AnyNodeId, citation: String },
    Placement { event: EventId, placement: DatePlacement, basis: PlacementBasis, justification: Justification },
    Mention { locus: TextLocus, entity: AnyNodeId },
    CrossRef { from: TextLocus, to: TextLocus, votes: u32 },
    Quote { quoting: TextLocus, quoted: BibleLocusRange },
    Confession { confessing: TextLocus, confessed: BibleLocusRange, justification: Justification },
    Correspondence { a: TextLocus, b: TextLocus, layer_a: TranslationId, layer_b: TranslationId },
    VersificationMap { native: String, canonical: VerseRef },
    Exemption { kind: ExemptionKind, subject: AnyNodeId, reason: String },
}

/// Exemptions are typed, reasoned, enumerable — never silent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExemptionKind {
    Recounting,
    DeferredMigration,
}

pub struct SourceBytes(pub Vec<u8>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterError(pub String);

/// A source adapter turns ONE source's bytes into typed assertions.
/// TOML is one impl; JSON another; nothing downstream knows the
/// difference.
pub trait SourceAdapter {
    fn source(&self) -> SourceId;
    fn emit(&self, input: &SourceBytes) -> Result<Vec<Asserted<Assertion>>, AdapterError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LawViolation(pub String);

/// Fidelity proven at the boundary: e.g., the Bible adapter's bijection
/// (every source verse becomes exactly one skeleton unit rendering) and
/// reconstruction (the reading-order walk reproduces the source text).
pub trait BoundaryLaw {
    fn check(&self, source: &SourceBytes, built: &crate::graph::Graph) -> Result<(), LawViolation>;
}
