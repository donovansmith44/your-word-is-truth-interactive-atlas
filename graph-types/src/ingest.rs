//! Ingestion vocabulary: uniform provenance for every asserted edge.
//!
//! HISTORY (2026-08-24, owner-ordered): the spec's full ingestion
//! machinery (`SourceAdapter`, the `Assertion` IR, `BoundaryLaw`) was
//! declared here unimplemented and deleted under the dead-code law --
//! the pipeline delivers those contracts concretely (per-source adapter
//! modules over the compiled-JSON boundary; fidelity laws inline in
//! atlas-graph). Re-derive from
//! docs/superpowers/specs/2026-08-22-explorable-graph-types.md if a
//! plugin-style source contract is ever wanted.

use crate::id::{Interned, SourceId};

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
