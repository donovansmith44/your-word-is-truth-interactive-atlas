use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;

use atlas_core::data::{AtlasData, Era, Landmark, Narrative, PolityDelta};
use atlas_core::refs::ScriptureRef;
use atlas_core::scene::{compose_scripture_scene, compose_time_scene};
use atlas_core::time::TimeRange;
use atlas_core::wire::Scene;
use atlas_graph::GraphService;
use atlas_graph_types::store::GraphQuery;

use crate::error::ApiError;
use crate::wire;

/// `GET /api/scene?from=&to=`. `from`/`to` are read out of a
/// `Query<HashMap<String, String>>` rather than a strongly-typed `Query<T>`
/// specifically so a missing or unparseable value can never trigger axum's
/// own extractor-rejection response (ruling 1) — `HashMap<String, String>`
/// cannot itself fail to deserialize on these inputs, so every failure mode
/// (missing, non-integer, zero, inverted) is handled by this function and
/// always yields the typed `bad_window` body.
///
/// OVERLAY-1 Task 5: the scene's DATA now comes from the graph port --
/// `graph.scene_source(&data)`, an `atlas_graph::scene_source::
/// GraphSceneSource` built once at load from `GraphService`'s own snapshot,
/// not from `AtlasData`'s former graph-derived `events`/`places`/
/// `narratives` fields (which `legacy::atlas_data_overlay` used to
/// reconstruct at boot, and which are no longer written on any serving
/// path). `data` is still extracted because that source reads two genuinely
/// curated-JSON sidecars through it (`place-history.json`,
/// `place-names-kjv.json`). The composed bytes are unchanged -- `tests/
/// scene_byte_identity.rs`'s 25 pinned hashes are the gate on that.
#[utoipa::path(get, path = "/api/scene", params(("from" = i32, Query), ("to" = i32, Query)), responses((status = 200, body = atlas_core::wire::Scene), ApiError), tag = "map")]
pub async fn scene_time(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Scene>, ApiError> {
    let from = parse_year(&params, "from")?;
    let to = parse_year(&params, "to")?;
    let window = TimeRange::new(from, to).map_err(|_| ApiError::bad_window())?;
    Ok(Json(compose_time_scene(graph.scene_source(&data), window)))
}

/// `GET /api/scene/scripture?ref=`.
///
/// ruling-3-policy: a `ref` that fails to *parse* (unknown book code, zero
/// chapter/verse, empty segment, inverted range — i.e. `ScriptureRef::parse`
/// returns `Err`) is a structurally bad ref and always 400s as `bad_ref`,
/// same as a missing `ref` param (treated as parsing the empty string, which
/// also fails to parse). A `ref` that *parses* but names coordinates outside
/// the loaded canon (e.g. a chapter number past the end of the book) is
/// deliberately NOT an error: `compose_scripture_scene` only ever matches it
/// against verses that actually exist on events/places, so an out-of-canon
/// ref naturally composes an empty-but-valid scene, exactly mirroring
/// ruling 2's "don't reject out-of-span time windows" for the sibling
/// endpoint. This needs no extra bounds-checking code — it falls out of not
/// adding any.
///
/// OVERLAY-1 Task 5: composes from `graph.scene_source(&data)`, exactly as
/// `scene_time` above does -- see that handler's own doc comment.
#[utoipa::path(get, path = "/api/scene/scripture", params(("ref" = String, Query)), responses((status = 200, body = atlas_core::wire::Scene), ApiError), tag = "map")]
pub async fn scene_scripture(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Scene>, ApiError> {
    let raw = params.get("ref").map(String::as_str).unwrap_or("");
    let r = ScriptureRef::parse(raw).map_err(|_| ApiError::bad_ref(raw))?;
    Ok(Json(compose_scripture_scene(graph.scene_source(&data), &r)))
}

/// `GET /api/eras` (Batch M-C, controller decision 7: "map migration...
/// become views over graph queries"). BATCH M-C: re-implemented as a VIEW
/// over the graph's own Era nodes (`atlas_graph::era_adapter`) instead of
/// `AtlasData.eras` -- `GraphService.era_ids` is the companion enumeration
/// `GraphQuery`'s own minimal port surface doesn't model (no "list every
/// node of kind K" primitive; see that field's own doc comment), already
/// held in chronological order; every actual FIELD comes from a real
/// `GraphQuery::node` call on this request's own snapshot. WIRE SHAPE
/// UNCHANGED: `Era { id, name, from_year, to_year }`, same order, same
/// JSON -- `AtlasData.eras`/`eras.json` retire this batch (deletion
/// inventory) with this endpoint as their only production reader.
#[utoipa::path(get, path = "/api/eras", responses((status = 200, body = Vec<atlas_core::data::Era>)), tag = "map")]
pub async fn eras(State(graph): State<Arc<GraphService>>) -> Json<Vec<Era>> {
    use atlas_graph_types::node::NodePayload;

    // DB-3: enumeration through the port (`nodes_of_kind`, id order); the
    // chronological wire order the retired `era_ids` companion carried is
    // this handler's own sort now -- by `(from_year, id)`, the same key the
    // companion sorted by, so the response bytes are unchanged
    // (`port_widening_real_data.rs` pins the equivalence).
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

/// `GET /api/narratives` (M-C2, definitive surface list). Re-implemented as
/// a VIEW over the graph's own Narrative nodes -- `GraphService.
/// narrative_ids` is the companion enumeration (same "port doesn't model
/// 'list every node of kind K'" class as `era_ids`/`polity_ids`, already
/// confirmed to match `data/curated/narratives/`'s own sorted-by-filename
/// compiled order -- see that field's own doc comment); `legs` comes from
/// `narrative_legs` (the `succession` relation's own row `chain`, the
/// single source -- never duplicated onto the payload). WIRE SHAPE
/// UNCHANGED: `atlas_core::data::Narrative { id, name, color, legs }`.
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

#[utoipa::path(get, path = "/api/landmarks", responses((status = 200, body = Vec<atlas_core::data::Landmark>)), tag = "map")]
pub async fn landmarks(State(data): State<Arc<AtlasData>>) -> Json<Vec<Landmark>> {
    Json(data.landmarks.clone())
}

#[utoipa::path(get, path = "/api/land-mask", responses((status = 200, body = wire::LandMask)), tag = "map")]
pub async fn land_mask(State(data): State<Arc<AtlasData>>) -> Json<wire::LandMask> {
    Json(wire::LandMask { rings: data.land_mask.clone() })
}

/// `GET /api/polities?from=&to=`. `from`/`to` share `scene_time`'s lenient
/// parsing (ruling 1: missing/unparseable/zero/inverted -> 400
/// `bad_window`), via the same `parse_year` helper and `TimeRange::new`
/// validity check.
///
/// Once the window is valid, emits every era (of every polity) whose own
/// `[from,to]` intersects it -- a polity with several eras in view (a
/// window spanning a border change) contributes one row PER intersecting
/// era, all sharing that polity's own `id`/`color_key`; a window matching no
/// era at all (out-of-span, or the `demo_fixture()`/pre-ETL case where
/// `data.polities` is empty) is not an error, mirroring `scene_time`'s own
/// ruling-2 spirit -- it 200s with an empty `polities` array. Deterministic
/// order: by polity id, then by era `from` -- so a multi-era window always
/// lists a polity's OLDER era before its newer one (the exact order
/// map.js's `BorderLayer` needs to paint older-under-newer and pick the
/// dotted/lightest era correctly without re-sorting client-side).
/// BATCH M-C (controller decision 7): re-implemented as a VIEW over the
/// graph's own Polity nodes (`atlas_graph::polity_adapter`) instead of
/// `AtlasData.polities` -- border data as node payloads (controller
/// decision 2), the map consuming payloads directly, per era, off each
/// Polity node's own `NodePayload::Polity.eras`. `GraphService.polity_ids`
/// is the companion enumeration (same status as `era_ids`, see that
/// field's own doc comment). WIRE SHAPE UNCHANGED: `Polity`/
/// `PolityDelta`, same fields, same conditional presence, same
/// deterministic sort -- `AtlasData.polities`/`polities.json` stay
/// standing (deliberately NOT this batch's deletion target: still the
/// adapter's own curated source, same status as `event_world`'s own
/// `atlas.events`/`.narratives`).
#[utoipa::path(get, path = "/api/polities", params(("from" = i32, Query), ("to" = i32, Query)), responses((status = 200, body = wire::Polities), ApiError), tag = "map")]
pub async fn polities(
    State(graph): State<Arc<GraphService>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<wire::Polities>, ApiError> {
    use atlas_graph_types::node::NodePayload;

    let from = parse_year(&params, "from")?;
    let to = parse_year(&params, "to")?;
    let window = TimeRange::new(from, to).map_err(|_| ApiError::bad_window())?;

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
                    rings: era.rings.clone(),
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

/// `for_era_from` is UNOBSERVABLE here -- it is `skip_serializing`, so nothing
/// reads what this writes. It is passed because the domain struct has the
/// field, and `era_from` is the only value that could ever be right for it.
fn curated_delta(d: &atlas_graph_types::node::PolityDeltaPayload, era_from: atlas_core::time::Year) -> PolityDelta {
    PolityDelta { event: d.event.clone(), verses: d.verses.clone(), ref_note: d.ref_note.clone(), for_era_from: era_from }
}

fn parse_year(params: &HashMap<String, String>, key: &str) -> Result<i32, ApiError> {
    params.get(key).and_then(|s| s.parse::<i32>().ok()).ok_or_else(ApiError::bad_window)
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
