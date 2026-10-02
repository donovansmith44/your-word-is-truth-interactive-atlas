use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "The schema versions of the data behind this server's responses.")]
pub struct Contract {
    #[schema(minimum = 1)]
    pub manifest_schema: u32,
    #[schema(minimum = 1)]
    pub section_schema_version: u32,
}
