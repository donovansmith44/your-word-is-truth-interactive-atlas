use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Contract {
    pub min_version: String,
    pub max_version: String,
    /// DB-4c: the served identity is the manifest's (spec §9): its schema
    /// and the sections' `PRAGMA user_version`. Additive beside the graph
    /// vocabulary's `artifact_format_version` (retired at DB-5).
    pub manifest_schema: u32,
    pub section_schema_version: u32,
}
