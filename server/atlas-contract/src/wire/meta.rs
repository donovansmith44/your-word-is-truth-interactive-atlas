use serde::Serialize;

/// The schema versions of the data behind this server's responses.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    /// The schema version of the compiled data set the responses are read from.
    #[schema(minimum = 1)]
    pub manifest_schema: u32,
    /// The schema version of the individual sections that data set is made of.
    #[schema(minimum = 1)]
    pub section_schema_version: u32,
}
