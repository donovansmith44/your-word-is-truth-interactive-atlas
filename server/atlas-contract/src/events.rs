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
    // OVERLAY-1 Task 5: the existence check and the label are ONE node
    // fetch now -- the label comes straight off the Event node's own
    // payload, so this handler needs no materialised event collection at
    // all (it replaces `data.event_by_id(&id).label`, which the deleted
    // overlay used to populate).
    let Some(node) = snap.node(&event_id) else {
        return Err(ApiError::not_found("event"));
    };
    let event_label = match node.payload {
        NodePayload::Event { label, .. } => label,
        _ => String::new(),
    };
    let event_pos = Position::Node(event_id);

    // "follows-in" (Forward) is THIS event's own following-event page;
    // "precedes-in" (Inverse) is its own prior-event page -- see
    // `graph_types::graph::Graph::build_indexes`'s own Succession pairing
    // (subject = the earlier leg, object = the later leg; `fwd` reads
    // subject -> object, `inv` reads object -> subject).
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
    // THE ONE PRESENTATION SOURCE for this whole handler (OVERLAY-1-HOTFIX-1).
    //
    // `atlas_core::narrative::adjacent_event` -- the shared builder that turns
    // an event id into {label, places, verse_groups} for every `prior`,
    // `following` and `timeline.*` below -- used to take `&AtlasData` and
    // resolve through `AtlasData.events`. OVERLAY-1 Task 5 left that vec
    // permanently EMPTY on every serving path (the boot-time overlay that
    // filled it is gone), so all three fields came back blank for every event
    // in the real atlas, while `tests/api.rs`'s fixture test -- whose
    // `demo_fixture()` hand-fills `events` -- stayed green. Three Playwright
    // specs were the only gate that caught it.
    //
    // It takes the SceneSource now, and this is the same object
    // `map::scene_time`/`scene_scripture` compose the map from
    // (materialised from the port with `finish()`'s merges and sort replayed),
    // so an adjacent event's own label/places/verse_groups are LITERALLY the
    // map arrow endpoint's own -- the ONE-GRAPH property
    // `atlas_core::narrative`'s own header states, now true on the serving
    // path too.
    //
    // Fix round 1 (review I-1): "true by construction" is what the first
    // version of this comment said, and it was not. `impl SceneSource for
    // AtlasData` exists, so a future edit CAN hand this function an
    // `AtlasData` again (`adjacent_event(&*data, ..)` compiles) and get the
    // same empty answer back. Two standing laws are the real guard --
    // `tests/no_legacy_event_reads.rs` (no serving source reads the emptied
    // collections or their derived accessors) and
    // `atlas-core/tests/no_atlas_data_in_public_signatures.rs` (no new public
    // fn in atlas-core takes an `AtlasData` at all) -- and the cure is
    // ETL-INPUT-1, deleting the three fields.
    let src = graph.scene_source(&data);

    // A narrative whose `legs` names exactly ONE event (a real, if rare,
    // shape -- e.g. `demo_fixture()`'s own "patriarchs-demo") produces NO
    // `Succession` row pair at all: `chain.windows(2)` on a one-element
    // chain is empty by construction (a doubly-linked list of one node has
    // no links), so its membership is genuinely invisible to the port's
    // EdgeMeta-tagged succession pages -- a real, structural gap in the
    // `succession` relation's own shape (it communicates SEQUENCE, not bare
    // membership), not a bug in this batch's port-based rewrite. Solo-leg
    // narratives are enumerated directly off the narrative list (a small,
    // in-memory scan -- narrative counts stay in the tens, never paged) so
    // this event's own membership in one is never silently dropped; every
    // narrative reached this way that DOES have a real prior/following
    // still gets it from the port entries above.
    // OVERLAY-1 Task 5: that list is `graph.scene_source(&data)`'s own,
    // materialised from `gs.narrative_ids` + `gs.narrative_legs` through
    // `legacy::narrative_from_node` (and post-`apply_event_merges`, so a leg
    // naming an absorbed event is already repointed) -- the exact content
    // and order the deleted `AtlasData.narratives` carried.
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
                prior: prior.and_then(|pid| atlas_core::narrative::adjacent_event(src, &pid)).map(Into::into),
                following: following.and_then(|pid| atlas_core::narrative::adjacent_event(src, &pid)).map(Into::into),
            }
        })
        .collect();

    // Batch HOTFIX-4 requirement 1's own "global chronological PRIOR/
    // FOLLOWING" half. TRAV-1 (controller decision 2, "the graph serves
    // it"): temporal-adjacency IS a materialized graph edge now
    // (`RelationId`'s symmetric sibling `SymRelationId::TemporalAdjacency`,
    // TRAV-1's crate patch) -- this reads `GraphService::temporal_neighbors`,
    // built once from the real `temporal_adjacency` rows' own honest
    // `earlier`/`later` ends (service.rs's own doc comment), never
    // re-derived from a position index. `None` (field omitted) for a
    // general-kind passage, by construction -- a general-kind event never
    // gets a `ChronologyDerivation` entry at all (`derive_chronology`
    // filters to `kind == "event"`), so it never gets a `temporal_adjacency`
    // row either, hence absent from `temporal_neighbors` exactly like it
    // was absent from the old `timeline_index`.
    // DB-3: through the port (`GraphService::temporal_neighbors_of`):
    // membership from the chronology order, adjacency from the
    // temporal-adjacency edges, direction from that order.
    let timeline = graph.temporal_neighbors_of(&id).map(|(prior, following)| wire::TimelinePosition {
        prior: prior.as_deref().and_then(|pid| atlas_core::narrative::adjacent_event(src, pid)).map(Into::into),
        following: following.as_deref().and_then(|pid| atlas_core::narrative::adjacent_event(src, pid)).map(Into::into),
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
    // M-C2 (definitive surface list): reconstructed from the graph's own
    // Event node (`NodePayload::Event`'s own M-C2 widening carries every
    // field this handler needs) instead of `data.event_by_id`.
    // `place_history_for`/`place_name_alias_for` below stay on `AtlasData`
    // deliberately -- `place-history.json`/`place-names-kjv.json` are not
    // this batch's deletion target, unaffected by the migration.
    let snap = graph.snapshot();
    let e: Event = atlas_graph::legacy::event_from_node(&atlas_graph::event_world::event_node_id(&id), &snap, &graph.chronology.chrono).ok_or_else(|| ApiError::not_found("event"))?;
    let e: &Event = &e;

    // Batch E3: resolved name (period-history- and KJV-alias-aware), not the
    // bare Theographic default -- this is the "PARALLEL ACCOUNTS place
    // lines" surface (an EVENT node's own `event-places`/`event-place-{id}`
    // rows render right alongside its PARALLEL ACCOUNTS witness section).
    // Window = this event's own `e.when`, gated on `e.kind == "event"` --
    // Fix round 1 (I-1): a `kind != "event"` ("general") passage's `e.when`
    // is `TimeRange::undated()` (the WHOLE atlas span, [-4004,100] -- see
    // its own doc comment), not an out-of-range sentinel, so passing it as a
    // real window trivially intersects every curated period-name range and
    // lets `resolve_display_name` spuriously pick a period name (or an
    // arbitrary one among several) for a passage that structurally has no
    // date at all. Mirrors the SAME kind-gate this handler already applies
    // to the wire `when` field below (the SAME "no real window here"
    // reasoning `reading::chapter`/`compose_scripture_scene` already use)
    // -- computed once here, reused for both the places resolution and
    // `when`, so the two can never drift apart again.
    let window = if e.kind == "event" { Some(e.when) } else { None };
    let places = e
        .places
        .iter()
        .filter_map(|pid| atlas_graph::legacy::place_from_node(&atlas_graph::event_world::place_stub_node_id(pid), &snap))
        .map(|p| wire::EventPlace {
            id: p.id.clone(),
            name: resolve_display_name(&p.name, data.place_history_for(&p.id), window, data.place_name_alias_for(&p.id)),
        })
        .collect();
    let witnesses = atlas_core::scene::witnesses_for(e).into_iter().map(wire::EventWitness::from).collect();
    // Batch T2: never surface the undated() sentinel to the wire for a
    // general-kind passage -- see EventDetail's own doc comment.
    let when = window;

    // ATTEST-1: the two new frontier sections, both read straight off the
    // graph's own indexes through the SAME generic `drain_edges` walk
    // every other relation in this crate's handlers uses -- no second path, no
    // re-derivation from AtlasData.
    let event_pos = Position::Node(atlas_graph::event_world::event_node_id(&e.id));
    // (L3) "Mentioned in": `Mentions` INVERSE, event -> the text units
    // that reference it. Rendered as canonical verse ids so the client
    // can hand them straight back to `/api/verse/{sref}`; sorted for a
    // stable reading order (the index's own order is insertion order,
    // which is curated-file order, not canonical order).
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
    // (L4) "Similar Accounts": the SYMMETRIC `Analogue` relation, walked
    // from this end. Titles come from the neighbour's own Event node, so
    // the client never needs a second fetch just to label the row.
    let analogues: Vec<wire::EventAnalogue> = drain_edges(&snap, &event_pos, EdgeKind::Symmetric(atlas_graph_types::edge::SymRelationId::Analogue))
        .into_iter()
        .filter_map(|entry| match entry.node {
            Position::Node(id) => atlas_graph::legacy::event_from_node(&id, &snap, &graph.chronology.chrono).map(|other| {
                // Batch PROV-1: THIS ROW's own provenance, looked up by the
                // pair it joins. `analogue_for_pair` is symmetric (both
                // orderings are stored), so walking the relation from
                // either end resolves the same single row.
                //
                // FIX ROUND 1 (review H-1, HIGH): this used to be
                // `.unwrap_or_default()`, defended by the same false claim
                // corrected at `reading::verse`'s own `VerseEvent` row -- the client filtered
                // the blank into silence rather than shouting about it. The
                // miss IS unreachable for a pair we just WALKED an Analogue
                // edge to reach, which is exactly why a 500 costs nothing
                // and makes the fail-loud claim TRUE. The failure it now
                // catches is the real one the review named: an id
                // normalization or `EventId` alias change that leaves the
                // walked edge and the row key disagreeing.
                // DB-3: `row_provenance` through the port
                // (`GraphService::analogue_provenance`); the fail-loud
                // claim is unchanged.
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
        // Batch PROV-1: the event's own node provenance. `event_from_node`
        // above already proved the node exists (it reconstructed `e` from
        // it), so this second read cannot legitimately miss -- fail-loud
        // rather than defaulted, same reasoning as `reading::verse`.
        // FIX ROUND 1 (review H-1): `.filter` added for the same reason as
        // `reading::verse`'s -- a node whose provenance is a blank string
        // used to pass the absence guard and reach the wire as "".
        provenance: snap
            .node(&atlas_graph::event_world::event_node_id(&e.id))
            .map(|n| n.provenance)
            .filter(|p| !p.trim().is_empty())
            .ok_or_else(|| ApiError::internal(&format!("event {} has no node to attribute it to", e.id)))?,
        // DB-3: both through `GraphQuery::row_provenance`, one lookup per
        // walked edge (`GraphService::{attests_provenance,
        // event_mentions_provenance}`), instead of the retired load-time
        // per-event maps.
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
