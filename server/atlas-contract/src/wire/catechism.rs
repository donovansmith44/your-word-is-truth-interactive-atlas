use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One catechism item citing a verse or a span: enough to name it and to fetch it in full.")]
pub struct CatechismRef {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    pub provenance: Vec<super::Provenance>,
}

impl CatechismRef {
    pub(crate) fn attributed(c: atlas_core::catechism::CatechismRef, provenance: &[super::Provenance]) -> Self {
        CatechismRef { id: c.id, name: c.name, question: c.question, provenance: provenance.to_vec() }
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One catechism item in full, with the chief part of the catechism it sits under.")]
pub struct CatechismItem {
    pub id: String,
    pub name: String,
    pub part_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub explanation_heading: String,
    pub explanation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub where_written: Option<String>,
    pub verses: Vec<CatechismProofVerse>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One proof verse of a catechism item, carrying the verse in full rather than a preview.")]
pub struct CatechismProofVerse {
    pub vref: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}
