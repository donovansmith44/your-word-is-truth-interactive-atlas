//! Uniform provenance for every asserted edge.

use crate::id::{Interned, SourceId};

pub type ProvenanceId = Interned;

/// Who asserts: source + locator. (WHY a claim stands is Justification —
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    pub source: SourceId,
    pub locator: String,
    pub confidence: Confidence,
}

/// Confidence for a corpus's TEXT derives from its role at the registry —
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Confidence {
    CanonicalText,
    Curated,
    Imported,
    Derived,
}
