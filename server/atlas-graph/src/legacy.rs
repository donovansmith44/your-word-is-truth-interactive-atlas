//! Per-entity RECONSTRUCTION: small `atlas_core::data` structs built
//! directly from graph queries. Three builders --
//! `event_from_node`/`place_from_node`/`narrative_from_node` -- and the
//! `drain` paging helper they share. That is the whole of this module now.
//!
//! WHO CALLS THEM (OVERLAY-1 Task 5 -- the list is short and deliberate):
//!   * `atlas_server::handlers::event`/`place`/`narratives`/`verse`, which
//!     read ONE node (or one node's own frontier) per request and reuse
//!     `atlas_core`'s own existing, already-tested presentation functions
//!     (`scene::to_scene_event`/`witnesses_for`,
//!     `history::resolve_display_name`, ...) against the reconstructed
//!     struct, never re-deriving display logic `atlas_core` already owns;
//!   * `crate::scene_source::GraphSceneSource::build`, which runs all three
//!     over `gs.event_ids`/`place_ids`/`narrative_ids` ONCE, at load, to
//!     materialise the map scene's own data.
//!
//! `locus_dot_ref` is the module's fifth item: a `TextLocus` -> dot-ref
//! helper `GraphService::assemble` and `person_adapter` share with the
//! builders above.
//!
//! WHAT IS GONE: `atlas_data_overlay` and `LegacyAtlasFields`, the
//! server's own default-startup-path bridge that used to assign
//! reconstructed `events`/`places`/`narratives` (and, before OVERLAY-1
//! Task 2, a whole-spine `verses` clone) back onto `AtlasData` so every
//! not-yet-migrated surface kept working after the five compiled JSON files
//! retired. Every one of those surfaces now reads the graph directly, or
//! reads `GraphSceneSource`; `AtlasData`'s `events`/`places`/`narratives`
//! are never written on ANY serving path, which is the memory result
//! OVERLAY-1 exists for (db1-plan.md §3.4 measured that bridge, plus the
//! artifact load it followed, as the boot's dominant cost). The proof that
//! the cut-over changed nothing served is
//! `atlas-server/tests/scene_byte_identity.rs`: its 25 pinned response
//! hashes are composed from `GraphSceneSource` and are the values captured
//! against the overlay path.
//!
//! Every function here reads ONLY through `atlas_graph_types::store::
//! GraphQuery` (the port) -- no raw `Graph` field reach -- so it works
//! identically against the from-sources build and the loaded artifact.

use atlas_core::data::{Event, EventWitness, Narrative, Place};
use atlas_core::time::TimeRange;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::explore::{EdgeEntry, EdgeQuery};
use atlas_graph_types::id::{AnyNodeId, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::{TextLocus, TextRef};

/// Drains every page of one edge kind at one position -- same shape as
/// `atlas_server::handlers::drain_edges` (duplicated, not shared: that one
/// is atlas-server-private, and this crate must not depend on atlas-server).
fn drain(q: &impl GraphQuery, p: &Position, kind: EdgeKind) -> Vec<EdgeEntry> {
    let mut cursor = None;
    let mut out = Vec::new();
    loop {
        let page = q.edges(p, &EdgeQuery { kind, cursor, limit: 500 });
        out.extend(page.entries);
        match page.next {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }
    out
}

/// Reconstructs one `atlas_core::data::Event` from its graph node --
/// `witnesses`/`kind`/`robertson_section`/etc. come straight off the
/// widened `NodePayload::Event` payload (`event_world::event_node`'s own
/// doc comment); `places` comes from `located-at` FORWARD edges, in
/// ORDER (the graph's own `BiIndex` preserves row insertion order, and
/// `event_world::populate_nodes_and_direct_rows` pushes `located_at` rows
/// in `Event.places`'s own original order -- `places[0]` stays the true
/// anchor place, load-bearing for `scene::build_arrows`'s own arrow
/// endpoints and `handlers::event`'s implicit "first place" conventions)
/// -- never duplicated onto the payload (the located-at relation is the
/// single source).
///
/// M-D3 (owner ruling R1 propagation): `from_year`/`to_year`/`order_key` no
/// longer ride the payload at all (chronology lives solely in `dated_by`
/// placements now) -- `chrono` (`event_world::ChronologyDerivation`, the
/// SAME derivation `GraphService.chronology` publishes on every startup
/// path, from-sources AND from-artifact) is the single remaining source:
/// `from_year` comes from `chrono.resolved` (the timeline's own authority);
/// the genuine `to_year`/`order_key` come from the SEPARATE `chrono.
/// source_meta` companion, never `resolved.date.to`/`.seq` (see
/// `SourceEventMeta`'s own doc comment for why a faithful ORDERING
/// substitute -- `seq` -- is not the same thing as the literal curated
/// VALUE this reconstruction must reproduce exactly, a real gap
/// `overlay_equivalence.rs` caught). A general-kind (undated) event has no
/// entry in either map -- falls back to `TimeRange::undated()`'s own
/// whole-atlas sentinel, order_key 0, the SAME "deliberately undated" idiom
/// every other general-kind call site in this workspace already uses,
/// never a fabricated date.
pub fn event_from_node(id: &AnyNodeId, q: &impl GraphQuery, chrono: &crate::event_world::ChronologyDerivation) -> Option<Event> {
    let node = q.node(id)?;
    let NodePayload::Event { label, kind, verses, witnesses, robertson_section, acts_section, atlas_section, kjv_superscription, ref_note } = node.payload else {
        return None;
    };

    let places: Vec<String> = drain(q, &Position::Node(id.clone()), EdgeKind::Directed(RelationId::LocatedAt, Direction::Forward))
        .into_iter()
        .filter_map(|e| match e.node {
            Position::Node(pid) => Some(pid.raw),
            Position::Edge(_) => None,
        })
        .collect();

    let witnesses: Vec<EventWitness> = witnesses
        .into_iter()
        .map(|w| EventWitness { book: w.book, translations: w.translations.into_iter().collect(), ref_note: w.ref_note, robertson_section: w.robertson_section })
        .collect();

    let (from_year, to_year, order_key) = match chrono.resolved.get(&id.raw) {
        Some(rp) => {
            let from_year = rp.date.from.year.get();
            let meta = chrono.source_meta.get(&id.raw).copied().unwrap_or(crate::event_world::SourceEventMeta { to_year: from_year, order_key: 0 });
            (from_year, meta.to_year, meta.order_key)
        }
        None => (TimeRange::undated().from_year, TimeRange::undated().to_year, 0),
    };

    Some(Event {
        id: id.raw.clone(),
        label,
        when: TimeRange { from_year, to_year },
        places,
        verses,
        kind,
        witnesses,
        robertson_section,
        acts_section,
        atlas_section,
        kjv_superscription,
        ref_note,
        order_key,
    })
}

/// Reconstructs one `atlas_core::data::Place` from its graph node --
/// `verse_links` comes from `mentioned-in` INVERSE edges (every TextUnit
/// mentioning this place), decoded back to canonical dot-refs via the SAME
/// `kjv_adapter::decode_text_unit`/`dot_ref` pair every other verse-id
/// round trip in this crate already uses.
pub fn place_from_node(id: &AnyNodeId, q: &impl GraphQuery) -> Option<Place> {
    let node = q.node(id)?;
    let NodePayload::Place { canonical, lat, lon, .. } = node.payload else { return None };

    let verse_links: Vec<String> = drain(q, &Position::Node(id.clone()), EdgeKind::Directed(RelationId::Mentions, Direction::Inverse))
        .into_iter()
        .filter_map(|e| match e.node {
            Position::Node(tid) => crate::kjv_adapter::decode_text_unit(&tid).map(|(b, c, v)| crate::kjv_adapter::dot_ref(b, c, v)),
            Position::Edge(_) => None,
        })
        .collect();

    Some(Place { id: id.raw.clone(), name: canonical, lat, lon, verse_links })
}

/// Layering cleanup (batch-finalp2-brief.md ticket 10; origin: batch-
/// corp1-review.md S-3/placement note): `node_description` RELOCATED to
/// `atlas-server::graph_handlers` -- its only two callers (`graph_handlers::
/// node_card`, `handlers::place`) both already lived in that crate, so this
/// was a clean move-only diff (grep-confirmed: nothing outside atlas-server
/// ever called `atlas_graph::legacy::node_description`), not a functional
/// change. See `atlas-server/src/graph_handlers.rs`'s own `node_description`
/// doc comment for the byte-identical body and the full history
/// (ENT-1a/CORP-1b) this doc comment used to carry.
///
/// Reconstructs one `atlas_core::data::Narrative` from its graph node --
/// `legs` is handed in (the `succession` relation's own row `chain`,
/// order-preserved) rather than duplicated onto the payload (`NodePayload::
/// Narrative`'s own doc comment); callers pass `GraphService::
/// narrative_legs`'s own per-narrative entry (empty for a narrative with no
/// legs at all -- a lawful, honest empty chain, not a missing one).
pub fn narrative_from_node(id: &AnyNodeId, q: &impl GraphQuery, legs: &[String]) -> Option<Narrative> {
    let node = q.node(id)?;
    let NodePayload::Narrative { label, color } = node.payload else { return None };
    Some(Narrative { id: id.raw.clone(), name: label, color, legs: legs.to_vec() })
}

/// A Bible-corpus `TextLocus`'s own canonical dot-ref (e.g. `"JHN.3.16"`),
/// via the SAME `kjv_adapter::dot_ref` every other verse-id round trip in
/// this crate uses -- `None` for a Concord locus (reserved, unpopulated
/// today) or a sub-verse span (not yet materialized).
pub fn locus_dot_ref(l: &TextLocus) -> Option<String> {
    match &l.at {
        TextRef::Bible(v) => Some(crate::kjv_adapter::dot_ref(v.book, v.chapter, v.verse)),
        TextRef::Concord(_) => None,
    }
}
