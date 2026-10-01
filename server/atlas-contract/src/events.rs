use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;

use atlas_core::data::{AtlasData, Event};
use atlas_core::history::resolve_display_name;
use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;

use crate::error::{ApiError, NoRefusals};
use crate::reading::drain_edges;
use crate::wire;

/// Where one event sits in time: its neighbours in each narrative it is a leg of, and its neighbours in the whole chronology.
///
/// `{id}` is an event id handed back by another response; an id naming no event
/// is `not_found`. An event that is a leg of no narrative answers an empty
/// `narrative` list, and one with no date carries no `timeline` at all.
#[utoipa::path(get, path = "/api/narrative/event/{id}", params(("id" = String, Path)), responses((status = 200, body = wire::NarrativeEventPositions), NoRefusals), tag = "events")]
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

/// One event in full: its title and date, where it happened, the passages that narrate it, the verses that only mention it, and the events whose accounts resemble it.
///
/// `{id}` is an event id handed back by another response; an id naming no event
/// is `not_found`. A titled passage with no date carries no `when`.
#[utoipa::path(get, path = "/api/event/{id}", params(("id" = String, Path)), responses((status = 200, body = wire::EventPage), NoRefusals), tag = "events")]
pub async fn event(State(data): State<Arc<AtlasData>>, State(graph): State<Arc<GraphService>>, Path(id): Path<String>) -> Result<Json<wire::EventPage>, ApiError> {
    let snap = graph.snapshot();
    let e: Event = atlas_graph::legacy::event_from_node(&atlas_graph::event_world::event_node_id(&id), &snap, &graph.chronology.chrono).ok_or_else(|| ApiError::not_found("event"))?;
    let e: &Event = &e;

    let window = e.date();
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

    Ok(Json(wire::EventPage {
        id: e.id.clone(),
        title: e.label.clone(),
        kind: e.kind,
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
