use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;

use atlas_core::data::AtlasData;
use atlas_core::history::{resolve_blurb, resolve_display_name_and_canonical};
use atlas_core::scene::to_scene_event;
use atlas_core::time::TimeRange;
use atlas_core::wire::SceneEvent;
use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::id::Position;

use crate::error::ApiError;
use crate::reading::drain_edges;
use crate::wire;

/// `GET /api/place/{id}?from=&to=`. Unknown id -> 404 `not_found` (a place
/// id is an exact identifier, not a ref that can be "out of canon but still
/// valid shape" — there's no parsing/shape question here at all).
///
/// `from`/`to` are OPTIONAL (Batch E) and, unlike `scene_time`/`borders`'s
/// own `from`/`to`, never themselves cause a 400: this endpoint's core
/// resource (the place, its events) is fully meaningful with neither
/// present, so a missing or malformed window just means `history` (when the
/// place has one at all) reports its default display name and no blurb,
/// rather than rejecting the whole request over an optional refinement.
/// M-C2 (definitive surface list): `place`/`events` now come from the
/// graph -- `atlas_graph::legacy::place_from_node` for the place itself,
/// the `site-of` (`located-at` INVERSE) frontier at the place's own
/// position for its own events (every LocatedAt row event_world.rs builds
/// for this place, order-independent here since the explicit `sort_by_key`
/// below already re-establishes the from_year order regardless).
/// `place_history_for`/`place_name_alias_for` stay on `AtlasData`
/// deliberately -- `place-history.json`/`place-names-kjv.json` are not
/// this batch's deletion target.
#[utoipa::path(get, path = "/api/place/{id}", params(("id" = String, Path), ("from" = Option<i32>, Query), ("to" = Option<i32>, Query)), responses((status = 200, body = wire::PlaceDetail), ApiError), tag = "places")]
pub async fn place(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
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

    let window = match (params.get("from").and_then(|s| s.parse::<i32>().ok()), params.get("to").and_then(|s| s.parse::<i32>().ok()))
    {
        (Some(from), Some(to)) => TimeRange::new(from, to).ok(),
        _ => None,
    };

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

pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(place))
}
