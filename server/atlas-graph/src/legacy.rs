//! Per-entity reconstruction: small `atlas_core::data` structs built from graph queries. Every
//! function here reads ONLY through `atlas_graph_types::store::GraphQuery`, never a raw `Graph`
//! field, so it behaves identically against a from-sources build and a loaded snapshot.

use atlas_core::data::{Event, EventWitness, Narrative, Place};
use atlas_core::time::TimeRange;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::explore::{EdgeEntry, EdgeQuery};
use atlas_graph_types::id::{AnyNodeId, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::{TextLocus, TextRef};

/// Drains every page of one edge kind at one position. Duplicated from atlas-server rather than
/// shared: that copy is private there, and this crate must not depend on atlas-server.
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

/// `places` comes from `located-at` forward edges in row order, so `places[0]` is still the true
/// anchor place. `from_year` comes from `chrono.resolved`, while the curated `to_year`/`order_key`
/// come from `chrono.source_meta`; an undated event falls back to `undated()`, never a made-up date.
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

/// `verse_links` comes from `mentioned-in` INVERSE edges -- every TextUnit mentioning this place --
/// decoded back to canonical dot-refs.
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

/// `legs` is handed in, the `succession` row's own order-preserved chain, rather than duplicated
/// onto the payload; an empty slice is a narrative with no legs, a lawful chain rather than a
/// missing one.
pub fn narrative_from_node(id: &AnyNodeId, q: &impl GraphQuery, legs: &[String]) -> Option<Narrative> {
    let node = q.node(id)?;
    let NodePayload::Narrative { label, color } = node.payload else { return None };
    Some(Narrative { id: id.raw.clone(), name: label, color, legs: legs.to_vec() })
}

/// A Bible-corpus `TextLocus`'s canonical dot-ref; `None` for a Concord locus or a sub-verse span,
/// neither of which has one.
pub fn locus_dot_ref(l: &TextLocus) -> Option<String> {
    match &l.at {
        TextRef::Bible(v) => Some(crate::kjv_adapter::dot_ref(v.book, v.chapter, v.verse)),
        TextRef::Concord(_) => None,
    }
}
