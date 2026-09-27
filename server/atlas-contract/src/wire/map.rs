use atlas_core::data::PolityDelta;
use serde::Serialize;

/// The coastline geometry border washes are clipped against, so no polity's
/// colour spills into open sea.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LandMask {
    /// Closed rings of [latitude, longitude] points, in degrees.
    pub rings: Vec<Vec<(f64, f64)>>,
}

/// The polity borders in view for the span of years asked about.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Polities {
    pub polities: Vec<Polity>,
}

/// One era of one polity's border. The `id` and `color_key` are the polity's
/// and hold across every era row it contributes; the name, the years and the
/// rings belong to this era alone.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Polity {
    pub id: String,
    /// The polity's name during this era.
    pub name: String,
    /// The first year of this era, negative for BC.
    pub from: i32,
    /// The last year of this era.
    pub to: i32,
    /// This era's border, as closed rings of [latitude, longitude] points, in
    /// degrees.
    pub rings: Vec<Vec<(f64, f64)>>,
    /// A number fixed per polity, so its eras can be coloured consistently.
    pub color_key: u8,
    /// The event that opened this era, absent when none is recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<PolityDelta>,
    /// The event that ended this polity for good, absent when none is recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fall: Option<PolityDelta>,
}
