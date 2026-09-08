//! Shared types for the Sources page (batch-s-brief.md, "document our
//! sources for everything and give it a dedicated page on the site").
//! `data/curated/sources.toml` (curated, hand-authored) compiles 1:1 into
//! `data/compiled/sources.json` (this SAME shape -- see
//! `server/atlas-etl/src/sources.rs`'s own `parse_sources`/`gen_sources`
//! bin), which atlas-server serves at `GET /api/sources` and the client's
//! `Sources.razor` page renders directly: "the page renders from data, not
//! hardcoded duplicate prose" (requirement 3).
//!
//! Deliberately its OWN small module, never a field on [`crate::data::
//! AtlasData`]: this data has nothing to do with the Explorable Graph or
//! any of `AtlasData`'s own place/event/narrative machinery, and
//! batch-s-brief.md's own finalization block is explicit that any Rust
//! helper this batch adds must stay OUTSIDE the graph pipeline, so
//! `graph.bin`/`data/exports/` stay byte-untouched by anything here.

use serde::{Deserialize, Serialize};

/// One heading group on the Sources page (e.g. "Scripture & Text"). `id`
/// is the join key [`SourceEntry::category`] points at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCategory {
    pub id: String,
    pub label: String,
}

/// One row of LICENSES.md's own "## Per-source table", curated by hand
/// into page-ready prose (positive tone throughout, per the KJV inerrancy
/// directive; license wording copied from LICENSES.md, never
/// embellished, per the citation-integrity rule).
///
/// `licenses_row_key` is never rendered on the page -- it is a literal
/// substring of this source's own row in LICENSES.md's Source column,
/// read ONLY by `atlas_etl::sources::validate_against_licenses`'s own
/// fail-loud drift check (requirement 3: "a LICENSES.md row absent from
/// the page (or vice versa) fails the build or a test").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEntry {
    pub id: String,
    pub category: String,
    pub title: String,
    pub what_it_is: String,
    pub what_we_built: String,
    pub license: String,
    #[serde(default)]
    pub link: Option<String>,
    pub licenses_row_key: String,
}

/// The whole compiled/curated shape. `data/curated/sources.toml`'s
/// `[[category]]`/`[[source]]` arrays and `data/compiled/sources.json`'s
/// top-level object are the SAME shape (no per-format renaming), so one
/// struct serves both directions -- `atlas_etl::sources::parse_sources`
/// reads the TOML shape, `gen_sources` writes this straight to JSON, and
/// `GET /api/sources` serves that JSON back out unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SourcesDocument {
    pub categories: Vec<SourceCategory>,
    pub sources: Vec<SourceEntry>,
    /// Batch PROV-1: see [`ProvenanceEntry`]. `#[serde(default)]` so a
    /// `sources.json` written before this batch still deserializes (the
    /// same additive-only discipline the HTTP wire follows) -- the
    /// committed one is regenerated with this table populated.
    #[serde(default)]
    pub provenances: Vec<ProvenanceEntry>,
}

/// Batch PROV-1 (owner order 1, verbatim: "one thing we definitely need
/// for EVERY PIECE OF DATA is the source from which it came. openbible,
/// etc."): THE JOIN ROW between a `ProvenanceId` as the graph actually
/// carries it -- the interned adapter-side string on every node and every
/// authored/imported row (`"kjv"`, `"theographic"`, `"curated-eras"`, ...)
/// -- and the [`SourceEntry`] that names, describes and licenses it for a
/// reader.
///
/// This table exists because the two vocabularies are genuinely DIFFERENT,
/// and always were: an adapter's provenance id is an INGEST fact (which
/// pipeline stage asserted this row), while a source id is a PUBLICATION
/// fact (which corpus a reader is being cited to). Several ingest ids
/// legitimately map to ONE source -- every hand-authored `curated-*` id is
/// this project's own work -- and rewriting the graph to carry publication
/// ids instead would erase that distinction. So the mapping is DECLARED
/// here, curated, and checked in BOTH directions
/// (`atlas_etl::sources::validate_structure` for internal consistency;
/// `atlas-graph/tests/provenance_registry_real_data.rs` for the real
/// artifact), rather than guessed by string munging at the UI.
///
/// [`confidence`](Self::confidence) is the reader-facing half of
/// `atlas_graph_types::ingest::Confidence`, which the graph's own row
/// structs do NOT store (they carry the id alone -- and that enum's own
/// doc comment says why: confidence "derives from its role at the
/// registry," and this IS the registry). Spelled exactly as that enum's
/// variants are: `"CanonicalText"` | `"Curated"` | `"Imported"` |
/// `"Derived"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    /// The `ProvenanceId` as the compiled graph carries it, verbatim.
    pub id: String,
    /// The [`SourceEntry::id`] this ingest id is a claim BY.
    pub source: String,
    /// `atlas_graph_types::ingest::Confidence`, spelled as its own variant
    /// name -- see this struct's doc comment. Validated against the closed
    /// vocabulary by [`crate::sources::CONFIDENCE_VOCABULARY`].
    pub confidence: String,
    /// The optional "locator" half of `ingest::Provenance`, at the level
    /// this registry can honestly state it: WHAT INSIDE the named source
    /// this id draws on ("cross_references.txt", "Bible-Geocoding-Data",
    /// ...). Omitted (not an empty string) when the source's own
    /// `what_we_built` already says it and a locator would add nothing --
    /// never a fabricated or padded string.
    #[serde(default)]
    pub locator: Option<String>,
}

/// The closed `Confidence` vocabulary, spelled as
/// `atlas_graph_types::ingest::Confidence`'s own variants. Lives here
/// rather than as a `use` of that enum on purpose: `atlas-core` does not
/// depend on the graph-types crate (nothing in this module does), and the
/// value on the wire is a STRING either way -- so this is the one place a
/// typo in `sources.toml` gets caught, and
/// `atlas-graph/tests/provenance_registry_real_data.rs` (which DOES see
/// both crates) reconciles this list against the enum itself.
pub const CONFIDENCE_VOCABULARY: &[&str] = &["CanonicalText", "Curated", "Imported", "Derived"];

/// THE RESOLUTION RULE, in one function -- the ONE place the id grammar
/// lives on this side of the wire (the client's own
/// `ProvenanceResolver.SplitId` mirrors it exactly, and
/// `atlas-graph/tests/provenance_registry_real_data.rs` asserts the law
/// through this function, never through a second copy of the rule).
///
/// A `ProvenanceId` is `kind` or `kind/locator`. The KIND is the
/// [`ProvenanceEntry::id`] registry key; the remainder, when present, is
/// THIS ROW's own locator -- which is exactly the `{source, locator}`
/// shape `atlas_graph_types::ingest::Provenance` has declared since the
/// ingest vocabulary was written.
///
/// The suffix half is not a convenience. `atlas_graph::kretzmann_adapter`
/// mints one provenance id PER commentary unit
/// (`format!("{KRETZMANN_PROVENANCE_KIND}/{slug}/{n}")`), so an
/// exact-match-only registry would need tens of thousands of curated rows
/// to state one true fact. Splitting at the FIRST slash (not the last)
/// keeps a multi-segment locator like `jeremiah/1` intact.
pub fn split_provenance_id(id: &str) -> (&str, Option<&str>) {
    match id.split_once('/') {
        Some((kind, locator)) => (kind, Some(locator)),
        None => (id, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_provenance_id_is_all_kind_and_no_locator() {
        assert_eq!(split_provenance_id("theographic"), ("theographic", None));
        // The one real id carrying a dot rather than a slash -- it must
        // NOT be split (dots are part of the kind).
        assert_eq!(split_provenance_id("openbible.info-cross-references"), ("openbible.info-cross-references", None));
    }

    #[test]
    fn a_suffixed_provenance_id_splits_at_the_first_slash_only() {
        // `kretzmann_adapter`'s own real shape: kind + a TWO-segment
        // locator. Splitting at the last slash would mis-read the kind as
        // "kretzmann/jeremiah".
        assert_eq!(split_provenance_id("kretzmann/jeremiah/1"), ("kretzmann", Some("jeremiah/1")));
    }
}
