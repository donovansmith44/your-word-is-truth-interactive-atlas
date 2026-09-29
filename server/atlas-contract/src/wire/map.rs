use atlas_core::data::PolityDelta;
use atlas_graph_types::id::{EraId, PolityId};
use serde::Serialize;
use utoipa::openapi::schema::{ArrayBuilder, ObjectBuilder, SchemaType, Type};
use utoipa::openapi::{RefOr, Schema};
use utoipa::{PartialSchema, ToSchema};

#[derive(Debug, Serialize)]
pub struct Point(pub f64, pub f64);

const POINT: &str = "One point of a border: latitude then longitude, in degrees.";
const POINT_COORDINATES: usize = 2;

/// Written out rather than derived: a Rust pair publishes as a fixed-length tuple,
/// which OpenAPI spells with a per-position schema list that a generated client
/// refuses to read, while what this actually is -- two numbers -- is an array.
impl PartialSchema for Point {
    fn schema() -> RefOr<Schema> {
        ArrayBuilder::new()
            .items(ObjectBuilder::new().schema_type(SchemaType::Type(Type::Number)))
            .min_items(Some(POINT_COORDINATES))
            .max_items(Some(POINT_COORDINATES))
            .description(Some(POINT))
            .into()
    }
}

impl ToSchema for Point {}

/// Closed rings of points, in the form the served structs carry them.
pub fn rings(curated: &[Vec<(f64, f64)>]) -> Vec<Vec<Point>> {
    curated.iter().map(|ring| ring.iter().map(|&(lat, lon)| Point(lat, lon)).collect()).collect()
}

/// The coastline geometry border washes are clipped against, so no polity's
/// colour spills into open sea.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LandMask {
    /// Closed rings of [latitude, longitude] points, in degrees.
    pub rings: Vec<Vec<Point>>,
}

/// A named stretch of this atlas's timeline.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Era {
    pub id: EraId,
    pub name: String,
    /// The first year of the era, negative for BC.
    pub from_year: i32,
    /// The last year of the era.
    pub to_year: i32,
    /// The era's years, labelled.
    pub window: super::TimeRange,
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
    pub id: PolityId,
    /// The polity's name during this era.
    pub name: String,
    /// The first year of this era, negative for BC.
    pub from: i32,
    /// The last year of this era.
    pub to: i32,
    /// This era's years, labelled.
    pub reign: super::TimeRange,
    /// This era's border, as closed rings of [latitude, longitude] points, in
    /// degrees.
    pub rings: Vec<Vec<Point>>,
    /// A number fixed per polity, so its eras can be coloured consistently.
    pub color_key: u8,
    /// The event that opened this era, absent when none is recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<PolityDelta>,
    /// The event that ended this polity for good, absent when none is recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fall: Option<PolityDelta>,
}
