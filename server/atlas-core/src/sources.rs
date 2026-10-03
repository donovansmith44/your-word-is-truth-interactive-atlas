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
    pub fn title_of(&self, provenance: &str) -> Option<&str> {
        self.0.get(split_provenance_id(provenance).0).map(String::as_str)
    }

    pub fn rows(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(id, title)| (id.as_str(), title.as_str()))
    }
}

impl FromIterator<(String, String)> for ProvenanceTitles {
    fn from_iter<I: IntoIterator<Item = (String, String)>>(rows: I) -> Self {
        ProvenanceTitles(rows.into_iter().collect())
    }
}

impl SourcesDocument {
    pub fn provenance_titles(&self) -> Result<ProvenanceTitles, String> {
        self.provenances
            .iter()
            .map(|p| match self.sources.iter().find(|s| s.id == p.source) {
                Some(source) => Ok((p.id.clone(), source.title.clone())),
                None => Err(format!("provenance {} names the source {}, which the registry does not list", p.id, p.source)),
            })
            .collect()
    }
}

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
    fn a_provenance_is_titled_by_the_source_it_names_whatever_locator_it_carries() {
        // Arrange
        let registry = SourcesDocument {
            categories: vec![],
            sources: vec![SourceEntry {
                id: "kretzmann-commentary".into(),
                category: "lutheran-texts".into(),
                title: "Kretzmann's Popular Commentary".into(),
                what_it_is: String::new(),
                what_we_built: String::new(),
                license: "Public domain".into(),
                link: None,
                licenses_row_key: "Kretzmann".into(),
            }],
            provenances: vec![ProvenanceEntry { id: "kretzmann".into(), source: "kretzmann-commentary".into(), confidence: Confidence::Imported, locator: None }],
        };

        // Act
        let titles = registry.provenance_titles().unwrap();

        // Assert
        assert_eq!(
            [titles.title_of("kretzmann"), titles.title_of("kretzmann/jeremiah/1"), titles.title_of("kjv")],
            [Some("Kretzmann's Popular Commentary"), Some("Kretzmann's Popular Commentary"), None]
        );
    }

    #[test]
    fn a_provenance_naming_no_listed_source_has_no_title_and_is_refused() {
        // Arrange
        let registry = SourcesDocument {
            categories: vec![],
            sources: vec![],
            provenances: vec![ProvenanceEntry { id: "kjv".into(), source: "kjv-text".into(), confidence: Confidence::CanonicalText, locator: None }],
        };

        // Act
        let titles = registry.provenance_titles();

        // Assert
        assert_eq!(titles, Err("provenance kjv names the source kjv-text, which the registry does not list".to_string()));
    }

    #[test]
    fn a_suffixed_provenance_id_splits_at_the_first_slash_only() {
        assert_eq!(split_provenance_id("kretzmann/jeremiah/1"), ("kretzmann", Some("jeremiah/1")));
    }
}
