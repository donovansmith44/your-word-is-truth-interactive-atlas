use serde::Serialize;

use atlas_core::canon::Testament;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Contents {
    pub corpus: Corpus,
    pub version: String,
    pub roots: Vec<ContentsRoot>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ContentsRoot {
    pub id: String,
    pub title: String,
    pub kind: ContentsRootKind,
    /// Absent for a Concord document, which belongs to neither testament.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<Testament>,
    /// The navigation target: the root's own first child's ref.
    #[serde(rename = "ref")]
    pub sref: String,
    pub children: Vec<ContentsChild>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ContentsChild {
    pub id: String,
    pub title: String,
    pub kind: ContentsChildKind,
    /// `GEN.1` for a chapter; `BoC 7.2.1` (its first paragraph) for an article.
    #[serde(rename = "ref")]
    pub sref: String,
    /// The child's own members: verses of a chapter, paragraphs of an article.
    pub count: usize,
}

atlas_core::vocabulary! {
    /// Every body of text this app serves a reading spine for. Each name IS
    /// the graph's own spine key, named here rather than re-typed, so a
    /// request that resolves to a corpus reaches that corpus's own spine.
    Corpus {
        Bible => atlas_graph::kjv_adapter::BIBLE_CORPUS,
        Concord => atlas_graph::concord_adapter::CONCORD_CORPUS,
    }
}

atlas_core::vocabulary! {
    /// What a top-level entry of a corpus's contents IS: a book of the Bible,
    /// or a document of the Book of Concord.
    ContentsRootKind {
        Book => "book",
        Document => "document",
    }
}

atlas_core::vocabulary! {
    /// What a second-level entry IS: a chapter of a book, or an article of a
    /// document. The tree stops here (D4: "Stop at the level of ARTICLE (BoC)
    /// or TOPIC (Small Catechism)").
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
