use serde::Serialize;

/// One catechism item citing a verse or a span: enough to name it and to fetch
/// it in full.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatechismRef {
    pub id: String,
    pub name: String,
    /// The question the verse was cited under, absent when the item cites it
    /// directly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    /// The sources behind this response's catechism citations. Every element of one
    /// response carries the same set; `/api/sources` names each id.
    pub provenance: Vec<String>,
}

impl CatechismRef {
    pub(crate) fn attributed(c: atlas_core::catechism::CatechismRef, provenance: &[String]) -> Self {
        CatechismRef { id: c.id, name: c.name, question: c.question, provenance: provenance.to_vec() }
    }
}

/// One catechism item in full, with the chief part of the catechism it sits
/// under.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatechismItem {
    pub id: String,
    pub name: String,
    /// The chief part this item belongs to, such as Baptism.
    pub part_title: String,
    /// The item's own words -- a commandment, a petition, an article -- absent for
    /// an item that has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The question the explanation answers, such as "What does this mean?".
    pub explanation_heading: String,
    pub explanation: String,
    /// Where in Scripture the item's own words are written, absent when it quotes
    /// none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub where_written: Option<String>,
    pub verses: Vec<CatechismProofVerse>,
}

/// One proof verse of a catechism item, carrying the verse in full rather than
/// a preview.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatechismProofVerse {
    pub vref: String,
    pub text: String,
    /// The question this verse was cited under, absent when the item cites it
    /// directly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}
