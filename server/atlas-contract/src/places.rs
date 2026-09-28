use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use atlas_core::data::AtlasData;
use atlas_core::history::{resolve_blurb, resolve_display_name_and_canonical};
use atlas_core::scene::to_scene_event;
use atlas_core::time::{TimeRange, Year};
use atlas_core::wire::SceneEvent;
use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::id::Position;

use crate::error::{ApiError, WindowRefusals};
use crate::query::{Contract, ContractParams};
use crate::reading::drain_edges;
use crate::wire;

/// One place: where it is, the events that happened there oldest first, and its curated history where it has one.
///
/// `{id}` is a place id handed back by another response; an id naming no place
/// is `not_found`. The optional `from` and `to` years choose the period whose
/// name and description the history reports -- neither is required, and a period
/// only one of them names simply leaves the place's default name in place. A year
/// that is not a year is `bad_window`.
#[utoipa::path(get, path = "/api/place/{id}", params(("id" = String, Path), PlacePeriod), responses((status = 200, body = wire::PlaceDetail), WindowRefusals), tag = "places")]
pub async fn place(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(id): Path<String>,
    Contract(asked): Contract<PlacePeriod>,
) -> Result<Json<wire::PlaceDetail>, ApiError> {
    let snap = graph.snapshot();
    let place_id = atlas_graph::event_world::place_stub_node_id(&id);
    let place = atlas_graph::legacy::place_from_node(&place_id, &snap).ok_or_else(|| ApiError::not_found("place"))?;

    let mut events: Vec<SceneEvent> = drain_edges(&snap, &Position::Node(place_id.clone()), EdgeKind::Directed(RelationId::LocatedAt, Direction::Inverse))
        .into_iter()
        .filter_map(|entry| match entry.node {
            Position::Node(eid) => atlas_graph::legacy::event_from_node(&eid, &snap, &graph.chronology.chrono),
            Position::Edge(_) => None,
        })
        .map(|e| to_scene_event(&e))
        .collect();
    events.sort_by_key(|e| e.when.from_year);

    let window = asked.period();

    // Resolved before `history`, and independently of it: a place with no curated
    // history record at all still has a translation alias to resolve.
    let alias = data.place_name_alias_for(&id);
    let (display_name, canonical_name) = resolve_display_name_and_canonical(&place.name, data.place_history_for(&id), window, alias);

    let history = data.place_history_for(&id).map(|h| wire::History {
        display_name: display_name.clone(),
        blurb: window.and_then(|w| resolve_blurb(&h.blurbs, w)).map(|b| b.text.clone()),
        established: h.established.clone(),
        destroyed: h.destroyed.clone(),
    });

    let description = crate::graph::node_description(&place_id, &snap);

    Ok(Json(wire::PlaceDetail { id: place.id.clone(), name: place.name.clone(), lat: place.lat, lon: place.lon, events, history, canonical_name, description }))
}

/// The period a place's own history is reported for.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PlacePeriod {
    #[serde(default)]
    #[param(value_type = Option<i32>)]
    pub from: Option<Year>,
    #[serde(default)]
    #[param(value_type = Option<i32>)]
    pub to: Option<Year>,
}

impl PlacePeriod {
    /// A period only one year names is no period at all, and a zero or inverted one
    /// names none either: both leave the place's default name standing.
    fn period(&self) -> Option<TimeRange> {
        TimeRange::new(self.from?, self.to?).ok()
    }
}

impl ContractParams for PlacePeriod {
    /// Both years are the one period, so which of them could not be read makes no
    /// difference to the refusal.
    fn unreadable(_parameter: &str, _asked_with: Option<&str>) -> ApiError {
        ApiError::bad_window()
    }
}

pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(place))
}
