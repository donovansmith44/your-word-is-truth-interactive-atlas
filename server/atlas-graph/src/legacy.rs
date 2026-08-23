//! M-C2: reconstructs small `atlas_core::data` structs directly from graph
//! queries -- the SAME per-entity building block shared by (a) the
//! handlers migrated onto the graph this batch (`handlers::place`/
//! `event`/`verse`/`narratives`, which read a single node/verse and reuse
//! `atlas_core`'s own EXISTING, already-tested presentation functions --
//! `scene::to_scene_event`/`witnesses_for`, `history::
//! resolve_display_name`, ... -- against the reconstructed struct, never
//! re-deriving display logic atlas_core already owns) and (b) `atlas_data_
//! overlay` below, the server's own default-startup-path bridge that keeps
//! every NOT-yet-migrated surface (scene.rs's own map composition,
//! `handlers::chapter`'s place-mention half, `handlers::catechism_item`'s
//! proof-verse text, `narrative_event_positions`'s residual `adjacent_
//! event` calls) working unchanged once the five retiring compiled JSON
//! files are gone.
//!
//! Every function here reads ONLY through `atlas_graph_types::store::
//! GraphQuery` (the port) -- no raw `Graph` field reach -- so it works
//! identically against the from-sources build and the loaded artifact.

use std::collections::HashMap;

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

/// Reconstructs the full `verse -> KJV text` map by walking the bible
/// reading spine end to end (the port's own `reading_window`, the SAME
/// primitive the reader's own windowed queries use) -- 200,000 is a
/// generous margin over the real ~31,102-verse KJV (same "wide, documented
/// margin" class as `GraphService::MAX_CHAPTER_SPAN_PROBE`), not a magic
/// exact count.
pub fn verses_from_graph(q: &impl GraphQuery) -> HashMap<String, String> {
    const GENEROUS_SPINE_LENGTH: usize = 200_000;
    let mut out = HashMap::new();
    for id in q.reading_window(crate::kjv_adapter::BIBLE_CORPUS, 0, GENEROUS_SPINE_LENGTH) {
        let Some((b, c, v)) = crate::kjv_adapter::decode_text_unit(&id) else { continue };
        if let Some(text) = crate::window::render(q, &id) {
            out.insert(crate::kjv_adapter::dot_ref(b, c, v), text);
        }
    }
    out
}

/// The five `AtlasData` fields the deletion event retires from `AtlasData::
/// load`'s own file-backed loaders, reconstructed instead -- `cross_refs`
/// is deliberately ABSENT (this struct's own doc comment on `atlas_data_
/// overlay` has the reason).
#[derive(Debug, Clone, Default)]
pub struct LegacyAtlasFields {
    pub events: Vec<Event>,
    pub places: Vec<Place>,
    pub narratives: Vec<Narrative>,
    pub verses: HashMap<String, String>,
}

/// M-C2: keeps every surface NOT in this batch's own definitive migration
/// scope (scene.rs's map composition; `handlers::chapter`'s place-mention
/// half; `handlers::catechism_item`'s proof-verse text; `narrative_event_
/// positions`'s residual `atlas_core::narrative::adjacent_event` calls)
/// working, unchanged, once `places.json`/`events.json`/`narratives.json`/
/// `verses-kjv.json` are deleted and their `AtlasData::load` reads retire.
///
/// Called ONCE, on the server's own DEFAULT startup path, immediately
/// after `GraphService::from_artifact` -- never per-request, never on the
/// `--build-from-raw` dev-fallback path (which has no graph yet to
/// reconstruct FROM; it builds `AtlasData` from raw+curated sources
/// directly, via `atlas_etl::compile::compile`). DISCLOSED GAP (found
/// while landing the M-C2 FIX above, not new to it): no dedicated test
/// file independently checks this function's own output against real
/// `atlas_etl::compile::compile` data end to end -- the only verification
/// on record is the manual live-curl comparison this batch's own commit
/// messages describe (both startup paths served identical results on a
/// real scratch-port server). Its own composed pieces (`event_from_node`/
/// `place_from_node`/`narrative_from_node`/`verses_from_graph`) each have
/// real unit coverage individually; the WHOLE-function, both-paths-agree
/// property does not. Fast: an in-memory
/// pass over the graph's own `event_ids`/`place_ids`/`narrative_ids`
/// companions (thousands of entries, no file I/O) plus one walk of the
/// reading spine -- this is what keeps the artifact LOAD-TIME ceiling
/// (<=3s, a committed law) meaningful for the server's own real total
/// startup, not just the graph's own isolated load step.
pub fn atlas_data_overlay(gs: &crate::service::GraphService) -> LegacyAtlasFields {
    let snap = gs.snapshot();

    let events: Vec<Event> = gs.event_ids.iter().filter_map(|id| event_from_node(id, &snap, &gs.chronology.chrono)).collect();
    let places: Vec<Place> = gs.place_ids.iter().filter_map(|id| place_from_node(id, &snap)).collect();
    let empty_legs: Vec<String> = Vec::new();
    let narratives: Vec<Narrative> = gs
        .narrative_ids
        .iter()
        .filter_map(|id| narrative_from_node(id, &snap, gs.narrative_legs.get(&id.raw).unwrap_or(&empty_legs)))
        .collect();
    // M-C2 FIX: `gs.verse_text` (this service's own precomputed companion,
    // added so `handlers::verse`/`handlers::xrefs` can read verse-preview
    // text through the graph instead of `AtlasData`) is built from this
    // SAME `verses_from_graph(&snap)` call -- reused here, not
    // recomputed, so the full ~31,102-verse reading-spine walk happens
    // once per startup, not twice.
    let verses = gs.verse_text.clone();

    LegacyAtlasFields { events, places, narratives, verses }
}
