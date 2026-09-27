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

/// One heading the sources are grouped under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SourceCategory {
    /// The key a source's `category` points at.
    pub id: String,
    pub label: String,
}

/// One source this atlas is built from: what it is, what was built from it, and
/// how it is licensed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SourceEntry {
    pub id: String,
    /// The id of the heading this source is grouped under.
    pub category: String,
    /// The source's own name.
    pub title: String,
    /// What the source is, in a sentence or two.
    pub what_it_is: String,
    /// What this atlas built from it.
    pub what_we_built: String,
    /// The licence it is used under, quoted rather than paraphrased.
    pub license: String,
    /// Where to find the source, absent when it has no public home.
    #[serde(default)]
    pub link: Option<String>,
    /// A key that ties this entry to its row in the project's licence file, so the
    /// two cannot drift apart. Not meant to be displayed.
    pub licenses_row_key: String,
}

/// Everything this atlas is built from: the sources, the headings they are
/// grouped under, and the ids that tie individual records back to a source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, utoipa::ToSchema)]
pub struct SourcesDocument {
    pub categories: Vec<SourceCategory>,
    pub sources: Vec<SourceEntry>,
    /// The join from the provenance id a record carries to the source that asserts
    /// it.
    #[serde(default)]
    pub provenances: Vec<ProvenanceEntry>,
}

/// The join between a provenance id as records carry it and the source that
/// names, describes and licenses it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProvenanceEntry {
    /// The provenance id exactly as a record carries it.
    pub id: String,
    /// The id of the source this is a claim by.
    pub source: String,
    /// How the claim was arrived at: `CanonicalText`, `Curated`, `Imported` or
    /// `Derived`.
    pub confidence: String,
    /// What inside the named source this id draws on, absent when the source's own
    /// description already says.
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
