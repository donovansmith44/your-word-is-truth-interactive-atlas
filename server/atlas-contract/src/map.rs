use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use atlas_core::data::{AtlasData, Era, Landmark, Narrative, PolityDelta};
use atlas_core::refs::ScriptureRef;
use atlas_core::scene::{compose_scripture_scene, compose_time_scene};
use atlas_core::time::{TimeRange, Year};
use atlas_core::wire::Scene;
use atlas_graph::GraphService;
use atlas_graph_types::store::GraphQuery;

use crate::error::{ApiError, ReferenceRefusals, WindowRefusals};
use crate::query::{self, Contract, ContractParams};
use crate::wire;

/// The map for a span of years: the places events light up in that span, the quiet places around them, and the arrows narratives draw between them.
///
/// `from` and `to` are years on this atlas's scale -- negative for BC, no year
/// zero -- and both are required, with `from` no later than `to`; anything else
/// is `bad_window`. A span no event falls in composes an empty scene rather
/// than failing.
#[utoipa::path(get, path = "/api/scene", params(SceneWindow), responses((status = 200, body = atlas_core::wire::Scene), WindowRefusals), tag = "map")]
pub async fn scene_time(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Contract(asked): Contract<SceneWindow>,
) -> Result<Json<Scene>, ApiError> {
    Ok(Json(compose_time_scene(graph.scene_source(&data), asked.span()?)))
}

/// The span of years a map answer covers.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SceneWindow {
    #[param(value_type = i32)]
    pub from: Year,
    #[param(value_type = i32)]
    pub to: Year,
}

impl SceneWindow {
    pub fn span(&self) -> Result<TimeRange, ApiError> {
        query::span(self.from, self.to)
    }
}

impl ContractParams for SceneWindow {
    /// Both years are the one window, so which of them could not be read makes no
    /// difference to the refusal -- and `bad_window` is the only refusal this route
    /// publishes, so a name it does not know answers the same rather than failing on a
    /// word that came off the caller's own query.
    fn unreadable(_parameter: &str, _asked_with: Option<&str>) -> ApiError {
        ApiError::bad_window()
    }
}

/// The map for a passage: the places its verses name, and the arrows narratives draw between them.
///
/// `ref` is a dotted reference -- `GEN.12`, `GEN.12.1` or `GEN.12.1-5` -- and a
/// missing or unparseable one is `bad_ref`. A reference that parses but names
/// coordinates outside this atlas's canon composes an empty scene rather than
/// failing.
#[utoipa::path(get, path = "/api/scene/scripture", params(ScripturePassage), responses((status = 200, body = atlas_core::wire::Scene), ReferenceRefusals), tag = "map")]
pub async fn scene_scripture(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Contract(asked): Contract<ScripturePassage>,
) -> Result<Json<Scene>, ApiError> {
    let raw = asked.r#ref.as_str();
    let r = ScriptureRef::parse(raw).map_err(|_| ApiError::bad_ref(raw))?;
    Ok(Json(compose_scripture_scene(graph.scene_source(&data), &r)))
}

/// The passage a map answer covers.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ScripturePassage {
    pub r#ref: String,
}

impl ContractParams for ScripturePassage {
    /// The passage is this route's only parameter and `bad_ref` its only refusal, so a
    /// name it does not know answers the same rather than failing on a word that came
    /// off the caller's own query.
    fn unreadable(_parameter: &str, asked_with: Option<&str>) -> ApiError {
        ApiError::bad_ref(asked_with.unwrap_or_default())
    }
}

/// Every named stretch of this atlas's timeline, with its year bounds, oldest first.
#[utoipa::path(get, path = "/api/eras", responses((status = 200, body = Vec<atlas_core::data::Era>)), tag = "map")]
pub async fn eras(State(graph): State<Arc<GraphService>>) -> Json<Vec<Era>> {
    use atlas_graph_types::node::NodePayload;

    // `ids_of_kind` answers in id order; this response's order is chronological.
    let snap = graph.snapshot();
    let mut eras: Vec<(i32, Era)> = graph
        .ids_of_kind(atlas_graph_types::id::NodeKind::Era)
        .into_iter()
        .filter_map(|id| {
            let node = snap.node(&id)?;
            match node.payload {
                NodePayload::Era { label, from_year, to_year } => Some((from_year, Era { id: id.raw.clone(), name: label, from_year, to_year })),
                _ => None,
            }
        })
        .collect();
    eras.sort_by(|a, b| (a.0, &a.1.id).cmp(&(b.0, &b.1.id)));
    Json(eras.into_iter().map(|(_, e)| e).collect())
}

/// Every narrative in this atlas: its name, the colour its arrows are drawn in, and the ordered events it runs through.
#[utoipa::path(get, path = "/api/narratives", responses((status = 200, body = Vec<atlas_core::data::Narrative>)), tag = "map")]
pub async fn narratives(State(graph): State<Arc<GraphService>>) -> Json<Vec<Narrative>> {
    let snap = graph.snapshot();
    let empty_legs: Vec<String> = Vec::new();
    let out: Vec<Narrative> = graph
        .ids_of_kind(atlas_graph_types::id::NodeKind::Narrative)
        .iter()
        .filter_map(|id| atlas_graph::legacy::narrative_from_node(id, &snap, graph.narrative_legs.get(&id.raw).unwrap_or(&empty_legs)))
        .collect();
    Json(out)
}

/// The always-on map labels -- waters, mountains and regions -- each with the point it is drawn at.
#[utoipa::path(get, path = "/api/landmarks", responses((status = 200, body = Vec<atlas_core::data::Landmark>)), tag = "map")]
pub async fn landmarks(State(data): State<Arc<AtlasData>>) -> Json<Vec<Landmark>> {
    Json(data.landmarks.clone())
}

/// The coastline geometry border washes are clipped against, so no polity's colour spills into open sea.
#[utoipa::path(get, path = "/api/land-mask", responses((status = 200, body = wire::LandMask)), tag = "map")]
pub async fn land_mask(State(data): State<Arc<AtlasData>>) -> Json<wire::LandMask> {
    Json(wire::LandMask { rings: wire::rings(&data.land_mask) })
}

/// The polity borders in view for a span of years: one row per era of a polity whose own years overlap the span.
///
/// `from` and `to` are years on this atlas's scale -- negative for BC, no year
/// zero -- and both are required, with `from` no later than `to`; anything else
/// is `bad_window`. Rows come ordered by polity and then oldest era first, so a
/// border change paints older beneath newer. A span no era overlaps answers an
/// empty list.
#[utoipa::path(get, path = "/api/polities", params(SceneWindow), responses((status = 200, body = wire::Polities), WindowRefusals), tag = "map")]
pub async fn polities(
    State(graph): State<Arc<GraphService>>,
    Contract(asked): Contract<SceneWindow>,
) -> Result<Json<wire::Polities>, ApiError> {
    use atlas_graph_types::node::NodePayload;

    let window = asked.span()?;

    let snap = graph.snapshot();
    let mut out: Vec<wire::Polity> = Vec::new();
    for id in &graph.ids_of_kind(atlas_graph_types::id::NodeKind::Polity) {
        let Some(node) = snap.node(id) else { continue };
        let NodePayload::Polity { color_key, eras, .. } = node.payload else { continue };
        for era in &eras {
            let era_range = TimeRange { from_year: era.from_year, to_year: era.to_year };
            if window.intersects(&era_range) {
                out.push(wire::Polity {
                    id: id.raw.clone(),
                    name: era.name.clone(),
                    from: era.from_year,
                    to: era.to_year,
                    rings: wire::rings(&era.rings),
                    color_key,
                    transition: era.transition.as_ref().map(|d| curated_delta(d, era.from_year)),
                    fall: era.fall.as_ref().map(|d| curated_delta(d, era.from_year)),
                });
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id).then(a.from.cmp(&b.from)));

    Ok(Json(wire::Polities { polities: out }))
}

/// `for_era_from` is unobservable here -- it is `skip_serializing`, so nothing
/// reads what this writes. It is passed because the domain struct has the field,
/// and `era_from` is the only value that could ever be right for it.
fn curated_delta(d: &atlas_graph_types::node::PolityDeltaPayload, era_from: atlas_core::time::Year) -> PolityDelta {
    PolityDelta { event: d.event.clone(), verses: d.verses.clone(), ref_note: d.ref_note.clone(), for_era_from: era_from }
}


pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(scene_time))
        .routes(routes!(scene_scripture))
        .routes(routes!(eras))
        .routes(routes!(narratives))
        .routes(routes!(landmarks))
        .routes(routes!(land_mask))
        .routes(routes!(polities))
}
