use serde::Serialize;

use atlas_core::canon::Testament;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Contents {
    pub corpus: Corpus,
    pub version: String,
    pub roots: Vec<ContentsRoot>,
}

/// Every body of text this app serves a reading spine for. `id` is the
/// graph's own corpus key, so the wire label and the spine key can never
/// disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub enum Corpus {
    #[serde(rename = "bible")]
    Bible,
    #[serde(rename = "concord")]
    Concord,
}

impl Corpus {
    pub const ALL: [Corpus; 2] = [Corpus::Bible, Corpus::Concord];

    pub fn named(name: &str) -> Option<Corpus> {
        Corpus::ALL.into_iter().find(|corpus| corpus.id() == name)
    }

    pub fn id(self) -> &'static str {
        match self {
            Corpus::Bible => atlas_graph::kjv_adapter::BIBLE_CORPUS,
            Corpus::Concord => atlas_graph::concord_adapter::CONCORD_CORPUS,
        }
    }
}

/// What a top-level entry of a corpus's contents IS: a book of the Bible, or
/// a document of the Book of Concord.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub enum ContentsRootKind {
    #[serde(rename = "book")]
    Book,
    #[serde(rename = "document")]
    Document,
}

impl ContentsRootKind {
    pub const ALL: [ContentsRootKind; 2] = [ContentsRootKind::Book, ContentsRootKind::Document];
}

/// What a second-level entry IS: a chapter of a book, or an article of a
/// document. The tree stops here (D4: "Stop at the level of ARTICLE (BoC) or
/// TOPIC (Small Catechism)").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub enum ContentsChildKind {
    #[serde(rename = "chapter")]
    Chapter,
    #[serde(rename = "article")]
    Article,
}

impl ContentsChildKind {
    pub const ALL: [ContentsChildKind; 2] = [ContentsChildKind::Chapter, ContentsChildKind::Article];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_corpus_serialises_as_the_graphs_own_reading_spine_keys() {
        // Arrange
        let every_variant = Corpus::ALL;
        // Act
        let json = serde_json::to_string(&every_variant).unwrap();
        // Assert
        assert_eq!(json, r#"["bible","concord"]"#);
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
    fn a_contents_root_kind_serialises_as_the_two_kinds_of_top_level_entry() {
        // Arrange
        let every_variant = ContentsRootKind::ALL;
        // Act
        let json = serde_json::to_string(&every_variant).unwrap();
        // Assert
        assert_eq!(json, r#"["book","document"]"#);
    }

    #[test]
    fn a_contents_child_kind_serialises_as_the_two_kinds_of_second_level_entry() {
        // Arrange
        let every_variant = ContentsChildKind::ALL;
        // Act
        let json = serde_json::to_string(&every_variant).unwrap();
        // Assert
        assert_eq!(json, r#"["chapter","article"]"#);
    }
}
