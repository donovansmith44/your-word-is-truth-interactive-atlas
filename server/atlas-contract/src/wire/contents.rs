use serde::Serialize;

use atlas_core::canon::Testament;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One corpus's contents, as a tree two levels deep.")]
pub struct Contents {
    pub corpus: Corpus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub version: super::ArtifactRoot,
    pub roots: Vec<ContentsRoot>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A top-level entry: a book of the Bible, or a document of the Book of Concord.")]
pub struct ContentsRoot {
    pub id: super::NodeId,
    pub title: String,
    pub kind: ContentsRootKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<Testament>,
    pub r#ref: super::ContentsReference,
    pub locus: super::TextRef,
    pub children: Vec<ContentsChild>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A second-level entry: a chapter of a book, or an article of a document.")]
pub struct ContentsChild {
    pub id: super::NodeId,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_title: Option<String>,
    pub kind: ContentsChildKind,
    pub r#ref: super::ContentsReference,
    pub locus: super::TextRef,
    pub count: usize,
}

atlas_graph_types::vocabulary! {
    #[doc = "Which corpus of text something belongs to: the Bible or the Book of Concord."]
    Corpus {
        Bible => atlas_graph::kjv_adapter::BIBLE_CORPUS,
        Concord => atlas_graph::concord_adapter::CONCORD_CORPUS,
    }
}

atlas_graph_types::vocabulary! {
    ContentsRootKind {
        Book => "book",
        Document => "document",
    }
}

atlas_graph_types::vocabulary! {
    ContentsChildKind {
        Chapter => "chapter",
        Article => "article",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_corpus_round_trips_through_the_graphs_own_reading_spine_keys() {
        // Arrange
        let every_variant = Corpus::ALL;
        // Act
        let json = serde_json::to_string(&every_variant).unwrap();
        let back: Vec<Corpus> = serde_json::from_str(&json).unwrap();
        // Assert
        assert_eq!(json, r#"["bible","concord"]"#);
        assert_eq!(back, every_variant.to_vec());
    }

    #[test]
    fn a_corpus_is_named_by_the_spine_key_it_serialises_as() {
        // Arrange
        let requested = ["bible", "concord", "nope"];
        // Act
        let resolved: Vec<Option<Corpus>> = requested.iter().map(|name| Corpus::named(name)).collect();
        // Assert
        assert_eq!(resolved, vec![Some(Corpus::Bible), Some(Corpus::Concord), None]);
    }

    #[test]
    fn a_contents_root_kind_round_trips_through_the_two_kinds_of_top_level_entry() {
        // Arrange
        let every_variant = ContentsRootKind::ALL;
        // Act
        let json = serde_json::to_string(&every_variant).unwrap();
        let back: Vec<ContentsRootKind> = serde_json::from_str(&json).unwrap();
        // Assert
        assert_eq!(json, r#"["book","document"]"#);
        assert_eq!(back, every_variant.to_vec());
    }

    #[test]
    fn a_contents_child_kind_round_trips_through_the_two_kinds_of_second_level_entry() {
        // Arrange
        let every_variant = ContentsChildKind::ALL;
        // Act
        let json = serde_json::to_string(&every_variant).unwrap();
        let back: Vec<ContentsChildKind> = serde_json::from_str(&json).unwrap();
        // Assert
        assert_eq!(json, r#"["chapter","article"]"#);
        assert_eq!(back, every_variant.to_vec());
    }
}
