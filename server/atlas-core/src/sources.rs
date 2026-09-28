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
    pub confidence: Confidence,
    /// What inside the named source this id draws on, absent when the source's own
    /// description already says.
    #[serde(default)]
    pub locator: Option<String>,
}

pub use atlas_graph_types::ingest::Confidence;

/// A provenance id is `kind` or `kind/locator`, split at the FIRST slash so a
/// multi-segment locator such as `jeremiah/1` stays intact. The kind half is the
/// [`ProvenanceEntry::id`] registry key.
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
        assert_eq!(split_provenance_id("openbible.info-cross-references"), ("openbible.info-cross-references", None));
    }

    #[test]
    fn a_suffixed_provenance_id_splits_at_the_first_slash_only() {
        assert_eq!(split_provenance_id("kretzmann/jeremiah/1"), ("kretzmann", Some("jeremiah/1")));
    }
}
