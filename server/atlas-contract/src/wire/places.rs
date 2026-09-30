use serde::Serialize;

use atlas_core::data::PlaceDateClaim;
use atlas_core::wire::SceneEvent;

/// One place: where it is, the events that happened there, and what is known
/// about its name and its history.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PlacePage {
    pub id: String,
    pub name: String,
    /// Latitude in degrees, north positive.
    pub lat: f64,
    /// Longitude in degrees, east positive.
    pub lon: f64,
    pub events: Vec<SceneEvent>,
    /// Absent for a place with no curated history.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<History>,
    /// The place's plain canonical name, present only when the name being shown is
    /// a period name or a translation's own wording and so differs from it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_name: Option<String>,
    /// Public-domain dictionary prose about the place, absent when none is
    /// recorded. Never invented.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// What is known about a place's name and existence over time, resolved for the
/// years asked about.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct History {
    /// The name this place bore in the years asked about, falling back to its
    /// default name when no period name applies.
    pub display_name: String,
    /// A short description of the place in those years, absent when none applies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blurb: Option<String>,
    /// When the place was founded, absent when that is not recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub established: Option<PlaceDateClaim>,
    /// When the place was destroyed, absent when that is not recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destroyed: Option<PlaceDateClaim>,
}
