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
}
