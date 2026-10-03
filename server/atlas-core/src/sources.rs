use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(description = "One heading the sources are grouped under.")]
pub struct SourceCategory {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(description = "One source this atlas is built from: what it is, what was built from it, and how it is licensed.")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default, utoipa::ToSchema)]
#[schema(description = "Everything this atlas is built from: the sources, the headings they are grouped under, and the ids that tie individual records back to a source.")]
pub struct SourcesDocument {
    pub categories: Vec<SourceCategory>,
    pub sources: Vec<SourceEntry>,
    #[serde(default)]
    pub provenances: Vec<ProvenanceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(description = "The join between a provenance id as records carry it and the source that names, describes and licenses it.")]
pub struct ProvenanceEntry {
    pub id: String,
    pub source: String,
    pub confidence: Confidence,
    #[serde(default)]
    pub locator: Option<String>,
}

pub use atlas_graph_types::ingest::Confidence;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProvenanceTitles(BTreeMap<String, String>);

impl ProvenanceTitles {
    pub fn from_rows(rows: impl IntoIterator<Item = (String, String)>) -> Result<Self, DuplicateProvenance> {
        let mut titles = BTreeMap::new();
        for (id, title) in rows {
            if titles.contains_key(&id) {
                return Err(DuplicateProvenance { id });
            }
            titles.insert(id, title);
        }
        Ok(ProvenanceTitles(titles))
    }

    pub fn title_of(&self, provenance: &str) -> Option<&str> {
        self.0.get(split_provenance_id(provenance).0).map(String::as_str)
    }

    pub fn rows(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(id, title)| (id.as_str(), title.as_str()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateProvenance {
    pub id: String,
}

impl std::fmt::Display for DuplicateProvenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the provenance {} is titled more than once", self.id)
    }
}

impl std::error::Error for DuplicateProvenance {}

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
