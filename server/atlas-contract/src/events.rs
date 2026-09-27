use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;

use atlas_core::data::{AtlasData, Event};
use atlas_core::history::resolve_display_name;
use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;

use crate::error::ApiError;
use crate::reading::drain_edges;
use crate::wire;

/// `GET /api/narrative/event/{id}` (Batch N requirement 1's own "endpoint/
/// payload also supports event-id lookup" half; Batch T requirement 2: the
/// resolver itself is UNCHANGED -- `positions_for_events`'s own leg-array-
/// adjacency walk was already exactly "chronologically adjacent given a
/// validated leg order," so no new logic was needed; what changed is WHO
/// calls this endpoint -- `client/Explore/EventNode.cs` replaces the
/// retired `NarrativeEventNode.cs` as its own caller -- and that ETL now
/// ALSO validates same-year legs via `Event::order_key`, not just
/// `when.from_year`, see `atlas_etl::validate::run`): every narrative
/// position the given event id occupies -- mirrors `GET
/// /api/catechism/item/{id}`'s own precedent exactly (an id-keyed follow-on
/// lookup). Reached by the client ONLY with an event id already handed back
/// by a prior response (never typed by a user), so an id that names no real
/// event at all is a genuine "not found," same `places::place`/
/// `catechism::catechism_item` precedent as every other exact-identifier
/// lookup in this crate;
/// ruling-3-policy still applies one layer in -- a REAL event that simply
/// isn't a leg of any narrative 200s with an empty array (the "no results"
/// case, not the "bad identifier" case), same as `positions_for_events`
/// itself naturally returns for a bare, narrative-less event (see
/// `narrative::tests::event_in_no_narrative_returns_no_positions`).
///
/// BATCH M-B (controller decision 3): re-implemented as a VIEW over graph
/// queries -- temporal neighbors come from `atlas_graph::Chronology::
/// temporal_neighbors` (built from `ChronologyDerivation::order`, which
/// `tests/timeline_equivalence.rs` proves is EXACTLY
/// `atlas_core::narrative::global_timeline_position`'s own timeline order --
/// the acceptance centerpiece). The bespoke resolvers this endpoint used to
/// call (`positions_for_events`/`global_timeline_position`) RETIRE from this
/// production call site -- `atlas_core::narrative`'s own module is
/// otherwise completely untouched (its OWN tests, including E1-E5, stay
/// green, unmodified) and its two topology functions remain `pub`, still
/// directly unit-tested, simply no longer reached by any live server
/// response.
///
/// BATCH M-C (controller decision 1): succession duals now come straight
/// off the GENERIC PORT instead of a companion index. `graph-types` commit
/// `13184e1` (owner-approved, "EdgeMeta -- per-entry relation metadata")
/// tags every `follows-in`/`precedes-in` entry with the `NarrativeId` it
/// belongs to (`Graph::build_indexes`'s own `Succession` pairing), so "every
/// narrative this event is a leg of, and its neighbor in each" is answerable
/// by draining both direction's pages at this event's own Position and
/// grouping entries by `EdgeMeta::Narrative` -- exactly the SAME
/// `Narrative.legs`-derived data the graph's own `Succession` rows were
/// always built from, just read back through the port instead of a
/// second, hand-maintained index (`atlas_graph::event_world::EventWorld`'s
/// own `narrative_positions` field, RETIRED this batch -- see that
/// module's own `Chronology` doc comment). This closes the M-B review's I-3
/// (validation bypass): there is now only one representation of "which
/// narrative is this leg in," so it cannot silently diverge from the
/// graph's own rows.
///
/// WIRE SHAPE IS BYTE-IDENTICAL (hard requirement, verified by the
/// pre-existing Playwright `event-timeline.spec.ts`/`popover-sections.spec.ts`
/// suites, which exercise this exact endpoint through the unmodified
/// client): `NarrativeEventPositions`/`NarrativePosition`/
/// `TimelinePosition`/`NarrativeAdjacentEvent` are UNCHANGED. Each
/// adjacent event's own presentation (label/places/verse_groups) still
/// calls `atlas_core::narrative::adjacent_event` directly (made `pub` at
/// M-B, see that function's own doc comment) -- the SAME presentation
/// builder the OLD resolver used, so label/places/verse_group formatting
/// cannot drift from what shipped before; only the TOPOLOGY (which ids are
/// prior/following, in which narratives) now originates from the graph.
#[utoipa::path(get, path = "/api/narrative/event/{id}", params(("id" = String, Path)), responses((status = 200, body = wire::NarrativeEventPositions), ApiError), tag = "events")]
pub async fn narrative_event_positions(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Path(id): Path<String>,
) -> Result<Json<wire::NarrativeEventPositions>, ApiError> {
    use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
    use atlas_graph_types::explore::EdgeMeta;
    use atlas_graph_types::id::{NarrativeId, Position};
    use atlas_graph_types::node::NodePayload;
    use std::collections::BTreeSet;

    let snap = graph.snapshot();
    let event_id = atlas_graph::event_world::event_node_id(&id);
    let Some(node) = snap.node(&event_id) else {
        return Err(ApiError::not_found("event"));
    };
    let event_label = match node.payload {
        NodePayload::Event { label, .. } => label,
        _ => String::new(),
    };
    let event_pos = Position::Node(event_id);

    // `follows-in` (Forward) at this event's position is its FOLLOWING leg;
    // `precedes-in` (Inverse) is its PRIOR one -- a succession row reads from the
    // earlier leg to the later one.
    let following_entries = drain_edges(&snap, &event_pos, EdgeKind::Directed(RelationId::Succession, Direction::Forward));
    let prior_entries = drain_edges(&snap, &event_pos, EdgeKind::Directed(RelationId::Succession, Direction::Inverse));

    let mut narrative_ids: BTreeSet<NarrativeId> = following_entries
        .iter()
        .chain(prior_entries.iter())
        .filter_map(|e| match &e.meta {
            EdgeMeta::Narrative(nid) => Some(nid.clone()),
            _ => None,
        })
        .collect();
    let src = graph.scene_source(&data);

    // A narrative whose `legs` names exactly one event produces no succession row
    // at all, so its membership is invisible to the pages above and is read off the
    // narrative list instead.
    for n in src.narrative_list() {
        if n.legs.len() == 1 && n.legs[0] == id {
            narrative_ids.insert(NarrativeId::new(n.id.clone()));
        }
    }

    let leg_event_id = |entries: &[atlas_graph_types::explore::EdgeEntry], nid: &NarrativeId| -> Option<String> {
        entries.iter().find(|e| matches!(&e.meta, EdgeMeta::Narrative(n) if n == nid)).and_then(|e| match &e.node {
            Position::Node(id) => Some(id.raw.clone()),
            Position::Edge(_) => None,
        })
    };

    let narrative: Vec<wire::NarrativePosition> = narrative_ids
        .into_iter()
        .map(|nid| {
            let narrative_name = snap
                .node(&nid.erase())
                .map(|n| match n.payload {
                    NodePayload::Narrative { label, .. } => label,
                    _ => String::new(),
                })
                .unwrap_or_default();
            let prior = leg_event_id(&prior_entries, &nid);
            let following = leg_event_id(&following_entries, &nid);
            wire::NarrativePosition {
                narrative_id: nid.0,
                narrative_name,
                event_id: id.clone(),
                event_label: event_label.clone(),
                prior: prior.and_then(|pid| atlas_core::narrative::adjacent_event(src, &pid)),
                following: following.and_then(|pid| atlas_core::narrative::adjacent_event(src, &pid)),
            }
        })
        .collect();

    let timeline = graph.temporal_neighbors_of(&id).map(|(prior, following)| atlas_core::narrative::TimelinePosition {
        prior: prior.as_deref().and_then(|pid| atlas_core::narrative::adjacent_event(src, pid)),
        following: following.as_deref().and_then(|pid| atlas_core::narrative::adjacent_event(src, pid)),
    });

    Ok(Json(wire::NarrativeEventPositions { narrative, timeline }))
}

/// `GET /api/event/{id}` (Batch T requirement 4): the EVENT node's own rich
/// fetch -- id-keyed, same exact-identifier "unknown id -> 404 not_found"
/// precedent `narrative_event_positions`/`catechism_item`/`place` already
/// set (never a user-typed id; always one a prior response, or a reader
/// heading, already handed back). Reads the graph's own Event node directly
/// (M-C2: via `legacy::event_from_node`, not the scene/narrative machinery,
/// and no longer `data.event_by_id` -- see the M-C2 comment inside this
/// function) since this is a passage's own STANDALONE content --
/// title/date/places/witnesses -- not anything scoped to a window or a
/// narrative position.
#[utoipa::path(get, path = "/api/event/{id}", params(("id" = String, Path)), responses((status = 200, body = wire::EventDetail), ApiError), tag = "events")]
pub async fn event(State(data): State<Arc<AtlasData>>, State(graph): State<Arc<GraphService>>, Path(id): Path<String>) -> Result<Json<wire::EventDetail>, ApiError> {
    let snap = graph.snapshot();
    let e: Event = atlas_graph::legacy::event_from_node(&atlas_graph::event_world::event_node_id(&id), &snap, &graph.chronology.chrono).ok_or_else(|| ApiError::not_found("event"))?;
    let e: &Event = &e;

    // A general-kind passage carries `TimeRange::undated()` -- the whole atlas span
    // -- which would intersect every curated period-name range and let a period name
    // be picked for a passage that has no date at all.
    let window = if e.kind == "event" { Some(e.when) } else { None };
    let places = e
        .places
        .iter()
        .filter_map(|pid| atlas_graph::legacy::place_from_node(&atlas_graph::event_world::place_stub_node_id(pid), &snap))
        .map(|p| wire::PlaceRef {
            id: p.id.clone(),
            name: resolve_display_name(&p.name, data.place_history_for(&p.id), window, data.place_name_alias_for(&p.id)),
        })
        .collect();
    let witnesses = atlas_core::scene::witnesses_for(e);
    let when = window;

    let event_pos = Position::Node(atlas_graph::event_world::event_node_id(&e.id));
    // The frontier answers in curated-file order, so a stable reading order needs
    // this sort.
    let mut mentioned_in: Vec<String> = drain_edges(&snap, &event_pos, EdgeKind::Directed(RelationId::Mentions, Direction::Inverse))
        .into_iter()
        .filter_map(|entry| match entry.node {
            Position::Node(id) => id.raw.strip_prefix("bible/").and_then(|rest| {
                let mut parts = rest.split('.');
                let book: u8 = parts.next()?.parse().ok()?;
                let chapter: u16 = parts.next()?.parse().ok()?;
                let verse: u16 = parts.next()?.parse().ok()?;
                let code = atlas_core::refs::BookId(book).code();
                Some(((book, chapter, verse), format!("{code}.{chapter}.{verse}")))
            }),
            Position::Edge(_) => None,
        })
        .collect::<std::collections::BTreeMap<_, _>>()
        .into_values()
        .collect();
    mentioned_in.dedup();
    let analogues: Vec<wire::EventAnalogue> = drain_edges(&snap, &event_pos, EdgeKind::Symmetric(atlas_graph_types::edge::SymRelationId::Analogue))
        .into_iter()
        .filter_map(|entry| match entry.node {
            Position::Node(id) => atlas_graph::legacy::event_from_node(&id, &snap, &graph.chronology.chrono).map(|other| {
                let provenance = graph
                    .analogue_provenance(&e.id, &other.id)
                    .filter(|p| !p.trim().is_empty())
                    .ok_or_else(|| ApiError::internal(&format!("analogue row {} <-> {} has no provenance to attribute it to", e.id, other.id)))?;
                Ok(wire::EventAnalogue { id: other.id.clone(), title: other.label.clone(), provenance })
            }),
            Position::Edge(_) => None,
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(Json(wire::EventDetail {
        id: e.id.clone(),
        title: e.label.clone(),
        kind: e.kind.clone(),
        when,
        places,
        witnesses,
        robertson_section: e.robertson_section.clone(),
        acts_section: e.acts_section.clone(),
        atlas_section: e.atlas_section.clone(),
        kjv_superscription: e.kjv_superscription.clone(),
        ref_note: e.ref_note.clone(),
        mentioned_in,
        analogues,
        provenance: snap
            .node(&atlas_graph::event_world::event_node_id(&e.id))
            .map(|n| n.provenance)
            .filter(|p| !p.trim().is_empty())
            .ok_or_else(|| ApiError::internal(&format!("event {} has no node to attribute it to", e.id)))?,
        witnesses_provenance: graph.attests_provenance(&e.id),
        mentions_provenance: graph.event_mentions_provenance(&e.id),
    }))
}

pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(narrative_event_positions))
        .routes(routes!(event))
}
