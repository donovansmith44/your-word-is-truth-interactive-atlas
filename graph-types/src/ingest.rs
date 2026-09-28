//! Uniform provenance for every asserted edge.

use crate::id::{Interned, SourceId};

pub type ProvenanceId = Interned;

/// Who asserts: source plus locator. WHY a claim stands is a justification, a different thing
/// and deliberately so.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    pub source: SourceId,
    pub locator: String,
    pub confidence: Confidence,
}

/// A corpus's TEXT takes its confidence from that corpus's role, never from a per-assertion
/// claim: `CanonicalText` if and only if the role is `NormaNormans`, so an adapter cannot
/// claim canonical standing for extrabiblical text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Confidence {
    CanonicalText,
    Curated,
    Imported,
    Derived,
}
