use serde::Serialize;

use atlas_core::canon::Testament;

/// One corpus's contents, as a tree two levels deep.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Contents {
    pub corpus: Corpus,
    /// A stamp identifying the data set this tree was read from.
    pub version: String,
    pub roots: Vec<ContentsRoot>,
}

/// A top-level entry: a book of the Bible, or a document of the Book of
/// Concord.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ContentsRoot {
    pub id: String,
    pub title: String,
    pub kind: ContentsRootKind,
    /// Which half of the canon the book belongs to; absent for a Concord document,
    /// which belongs to neither.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<Testament>,
    /// The reference to open when this entry is chosen: its first child's.
    pub r#ref: String,
    pub locus: super::TextRef,
    /// The entry's chapters, or its articles.
    pub children: Vec<ContentsChild>,
}

/// A second-level entry: a chapter of a book, or an article of a document.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ContentsChild {
    pub id: String,
    pub title: String,
    pub kind: ContentsChildKind,
    /// The reference to open for this entry: `GEN.1` for a chapter, or an
    /// article's first paragraph, such as `BoC 7.2.1`.
    pub r#ref: String,
    pub locus: super::TextRef,
    /// How many members it holds: verses of a chapter, paragraphs of an article.
    pub count: usize,
}

atlas_graph_types::vocabulary! {
    /// A body of text this API serves a reading spine for: Scripture, or the
    /// Book of Concord.
    Corpus {
        Bible => atlas_graph::kjv_adapter::BIBLE_CORPUS,
        Concord => atlas_graph::concord_adapter::CONCORD_CORPUS,
    }
}

atlas_graph_types::vocabulary! {
    /// Whether a top-level entry of a corpus's contents is a book of the Bible
    /// or a document of the Book of Concord.
    ContentsRootKind {
        Book => "book",
        Document => "document",
    }
}

atlas_graph_types::vocabulary! {
    /// Whether a second-level entry of a corpus's contents is a chapter of a book
    /// or an article of a document. The contents tree goes no deeper than this.
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
