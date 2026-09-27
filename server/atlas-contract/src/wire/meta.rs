use serde::Serialize;

/// The range of contract versions this server answers for, and the schema
/// versions of the data behind them.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    // A consumer parses the advertised range as MAJOR.MINOR.PATCH, so the
    // published contract says so -- written out twice because
    // `#[schema(pattern = ..)]` takes a literal, not a constant.
    /// The oldest contract version this server answers for, as MAJOR.MINOR.PATCH.
    #[schema(pattern = r"^[0-9]+\.[0-9]+\.[0-9]+$")]
    pub min_version: String,
    /// The newest contract version this server answers for.
    #[schema(pattern = r"^[0-9]+\.[0-9]+\.[0-9]+$")]
    pub max_version: String,
    /// The schema version of the compiled data set the responses are read from.
    #[schema(minimum = 1)]
    pub manifest_schema: u32,
    /// The schema version of the individual sections that data set is made of.
    #[schema(minimum = 1)]
    pub section_schema_version: u32,
}
