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

pub fn rings(curated: &[Vec<(f64, f64)>]) -> Vec<Vec<Point>> {
    curated.iter().map(|ring| ring.iter().map(|&(lat, lon)| Point(lat, lon)).collect()).collect()
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "The coastline geometry border washes are clipped against, so no polity's colour spills into open sea.")]
pub struct LandMask {
    pub rings: Vec<Vec<Point>>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A named stretch of this atlas's timeline: its first and last years (negative for BC) and the same years labelled.")]
pub struct Era {
    pub id: EraId,
    pub node: super::NodeRef,
    pub name: String,
    pub from_year: i32,
    pub to_year: i32,
    pub window: super::TimeRange,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "The polity borders in view for the span of years asked about.")]
pub struct Polities {
    pub polities: Vec<Polity>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One era of one polity's border. The `id` and `color_key` are the polity's and hold across every era row it contributes; the name, the years and the rings belong to this era alone.")]
pub struct Polity {
    pub id: PolityId,
    pub node: super::NodeRef,
    pub name: String,
    pub from: i32,
    pub to: i32,
    pub reign: super::TimeRange,
    pub rings: Vec<Vec<Point>>,
    pub color_key: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<PolityDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fall: Option<PolityDelta>,
}
