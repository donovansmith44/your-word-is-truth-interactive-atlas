use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Contents {
    pub corpus: String,
    pub version: String,
    pub roots: Vec<ContentsRoot>,
}

#[derive(Debug, Serialize)]
pub struct ContentsRoot {
    pub id: String,
    pub title: String,
    /// `book` | `document`.
    pub kind: String,
    /// `OT` | `NT` for a Bible book; absent for a Concord document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// The navigation target: the root's own first child's ref.
    #[serde(rename = "ref")]
    pub sref: String,
    pub children: Vec<ContentsChild>,
}

#[derive(Debug, Serialize)]
pub struct ContentsChild {
    pub id: String,
    pub title: String,
    /// `chapter` | `article`.
    pub kind: String,
    /// `GEN.1` for a chapter; `BoC 7.2.1` (its first paragraph) for an article.
    #[serde(rename = "ref")]
    pub sref: String,
    /// The child's own members: verses of a chapter, paragraphs of an article.
    pub count: usize,
}
