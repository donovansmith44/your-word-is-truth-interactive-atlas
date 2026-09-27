use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    // A consumer parses the advertised range as MAJOR.MINOR.PATCH and fails
    // loud on anything else, so the published contract says so -- written out
    // twice because `#[schema(pattern = ..)]` takes a literal, not a constant.
    #[schema(pattern = r"^[0-9]+\.[0-9]+\.[0-9]+$")]
    pub min_version: String,
    #[schema(pattern = r"^[0-9]+\.[0-9]+\.[0-9]+$")]
    pub max_version: String,
    /// DB-4c: the served identity is the manifest's (spec §9): its schema
    /// and the sections' `PRAGMA user_version`. Additive beside the graph
    /// vocabulary's `artifact_format_version` (retired at DB-5).
    #[schema(minimum = 1)]
    pub manifest_schema: u32,
    #[schema(minimum = 1)]
    pub section_schema_version: u32,
}
