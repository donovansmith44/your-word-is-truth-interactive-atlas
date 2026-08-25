//! Wire-identity conversions for the two generic graph endpoints (design
//! doc §5; M-A brief requirement 4): a human-inspectable, round-trippable
//! STRING form for `AnyNodeId` and `EdgeKind`, kept entirely at this DTO
//! layer -- graph-types' own `AnyNodeId`/`EdgeKind` shapes are untouched;
//! this module only converts.
//!
//! Node ids: `"text-unit:{BOOK}.{chapter}.{verse}"` (e.g.
//! `"text-unit:JHN.3.16"`) for the only node kind M-A materializes.
//! Internally a TextUnit's `AnyNodeId.raw` is a numeric-book-index string
//! (`atlas_graph::kjv_adapter`'s own adapter convention, chosen for a
//! stable identity independent of any one citation scheme -- "names are
//! refs, not identity," design doc §9b); this layer is where that internal
//! identity is dressed up into the human/dot-ref form the REST of this
//! app's wire already uses everywhere (`ChapterOut.ref`, `/api/chapter/{cref}`,
//! ...), and back.
//!
//! BATCH M-B id grammar (extends M-A's, per the brief's own requirement 4):
//! Event/Narrative/Anchor/Place-stub ids are the curated string ids
//! themselves (`ab_ur`, `theo-157`, `conquest`, `solomon-crowned`,
//! `jericho`, ...) -- already stable, unique, human-legible, with no
//! numeric re-encoding needed (unlike TextUnit's book-index scheme, which
//! exists only because `VerseRef` has no string form of its own). The wire
//! form is `"{Kind:?}:{raw}"` (e.g. `"Event:ab_ur"`, `"Anchor:solomon-crowned"`)
//! -- this is NOT a new convention: `encode_node_id`'s own pre-existing
//! generic fallback (`other => format!("{other:?}:{}", id.raw)`) already
//! PRODUCES exactly this shape for any non-TextUnit kind, unmodified since
//! M-A. `decode_node_id` below adds the matching arms so the four kinds
//! this batch newly materializes complete that round trip -- the encode
//! half needed no change at all.
//!
//! Edge kinds: the wire `kind` string IS `EdgeKind::label()` (already a
//! stable, human-legible string graph-types computes from its own relation
//! manifest -- `"cites"`/`"cited-by"`/...); `parse_edge_kind` is its total
//! inverse, built by scanning `RelationId::ALL`/`SymRelationId::ALL` (the
//! SAME manifest, so a new relation never needs a second hand-written
//! table here).

use atlas_graph_types::edge::{Direction, EdgeKind, RelationId, SymRelationId};
use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::store::GraphQuery;

/// Encodes any node id this batch's graph can produce. Only `NodeKind::TextUnit`
/// is ever actually built in M-A; the fallback keeps this function total
/// (never a panic) for whichever kind materializes next (M-B/M-C), at the
/// cost of a less pretty wire string until THAT batch's own DTO work lands.
pub fn encode_node_id(id: &AnyNodeId) -> String {
    match id.kind {
        // CORP-2a: `NodeKind::TextUnit` is now shared by TWO corpora (the
        // scouting memo's own "ConcordRef locus grammar joins graph_wire,
        // one decode arm, the P precedent" -- realized here, in BOTH
        // halves of the round trip, not just decode). Bible tried first
        // (the overwhelmingly common case, unchanged cost); a Concord id
        // (`kjv_adapter::decode_text_unit` returns `None` for a
        // "concord/..." raw string, since it checks the "bible/" prefix)
        // falls to the Concord decode, which produces the wire form
        // `"text-unit:BoC {part}.{article}.{paragraph}"` -- reusing
        // `ConcordTag::cite()`'s own citation format verbatim
        // (graph-types' text.rs), not a new one invented here.
        NodeKind::TextUnit => match atlas_graph::kjv_adapter::decode_text_unit(id) {
            Some((book, chapter, verse)) => format!("text-unit:{}", atlas_graph::kjv_adapter::dot_ref(book, chapter, verse)),
            None => match atlas_graph::concord_adapter::decode_text_unit(id) {
                Some((part, article, paragraph)) => format!("text-unit:BoC {part}.{article}.{paragraph}"),
                None => format!("text-unit:{}", id.raw),
            },
        },
        other => format!("{other:?}:{}", id.raw),
    }
}

/// The inverse of `encode_node_id` for the one kind this batch resolves
/// (`text-unit:...`). `None` for anything else -- a syntactically odd or
/// unsupported-kind id is a 400 `bad_ref` at the handler, never a panic.
pub fn decode_node_id(s: &str) -> Option<AnyNodeId> {
    let (kind, rest) = s.split_once(':')?;
    if rest.is_empty() {
        return None;
    }
    match kind {
        // CORP-2a: try the Concord "BoC {part}.{article}.{paragraph}"
        // form first (a cheap, unambiguous prefix check -- Bible dot-refs
        // never start "BoC "), then fall through to the existing Bible
        // parse unchanged. See `encode_node_id`'s own matching doc
        // comment for the full round-trip picture.
        "text-unit" => {
            if let Some(concord_rest) = rest.strip_prefix("BoC ") {
                let mut parts = concord_rest.split('.');
                let part: u8 = parts.next()?.parse().ok()?;
                let article: u16 = parts.next()?.parse().ok()?;
                let paragraph: u16 = parts.next()?.parse().ok()?;
                if parts.next().is_some() {
                    return None; // trailing garbage -- not one of this adapter's ids
                }
                return Some(atlas_graph::concord_adapter::text_unit_id(part, article, paragraph));
            }
            let vid = atlas_core::refs::VerseId::parse_canonical(rest).ok()?;
            Some(atlas_graph::kjv_adapter::verse_node_id(vid.book.0, vid.chapter, vid.verse))
        }
        // Batch M-B: the four newly-materialized kinds, completing the
        // round trip `encode_node_id`'s own pre-existing generic fallback
        // (`{Kind:?}:{raw}`) already produces for them (module doc
        // comment above) -- `raw` is the curated id verbatim, no
        // re-encoding.
        "Event" => Some(AnyNodeId { kind: NodeKind::Event, raw: rest.to_string() }),
        "Narrative" => Some(AnyNodeId { kind: NodeKind::Narrative, raw: rest.to_string() }),
        "Anchor" => Some(AnyNodeId { kind: NodeKind::Anchor, raw: rest.to_string() }),
        "Place" => Some(AnyNodeId { kind: NodeKind::Place, raw: rest.to_string() }),
        // Batch M-C: three more newly-materialized kinds (era_adapter/
        // polity_adapter/catechism_adapter), same round-trip completion.
        "Era" => Some(AnyNodeId { kind: NodeKind::Era, raw: rest.to_string() }),
        "Polity" => Some(AnyNodeId { kind: NodeKind::Polity, raw: rest.to_string() }),
        "CatechismItem" => Some(AnyNodeId { kind: NodeKind::CatechismItem, raw: rest.to_string() }),
        // Batch P (the extensibility proof): the ONE line this batch adds
        // to this file -- completing the round trip `encode_node_id`'s own
        // pre-existing generic fallback ALREADY produces for Person
        // ("Person:aaron_1"), the identical one-arm pattern every prior
        // node-kind batch added here (M-B's four, M-C's three). Nothing
        // else in `graph_handlers.rs`/`store.rs`/`explore.rs` needed a
        // change for the two generic endpoints to serve Person nodes.
        "Person" => Some(AnyNodeId { kind: NodeKind::Person, raw: rest.to_string() }),
        // Batch CORP-1a: same one-arm round-trip completion for the six
        // Translation nodes this batch newly authors (`brainfuel_adapter.rs`)
        // -- `encode_node_id`'s own pre-existing generic fallback already
        // produces "Translation:latin_vulgate" etc.; this is what makes a
        // rendering's own TranslationId resolve to something actually
        // reachable through the existing generic `/api/node/{id}` endpoint
        // (controller decision 6), not just internally present. NOT a new
        // endpoint, NOT a client change -- the identical pattern every
        // prior node-kind batch (M-B/M-C/P) added here.
        "Translation" => Some(AnyNodeId { kind: NodeKind::Translation, raw: rest.to_string() }),
        _ => None,
    }
}

/// Total inverse of `EdgeKind::label()`, built from graph-types' own
/// relation manifest (`RelationId::ALL`/`SymRelationId::ALL`) rather than a
/// hand-duplicated match -- an added relation can never drift out of sync
/// with what this function accepts.
pub fn parse_edge_kind(label: &str) -> Option<EdgeKind> {
    for r in RelationId::ALL {
        if r.forward_label() == label {
            return Some(EdgeKind::Directed(*r, Direction::Forward));
        }
        if r.inverse_label() == label {
            return Some(EdgeKind::Directed(*r, Direction::Inverse));
        }
    }
    for s in SymRelationId::ALL {
        if s.label() == label {
            return Some(EdgeKind::Symmetric(*s));
        }
    }
    None
}

/// A short, human-legible label for a node -- the citation string for a
/// TextUnit (e.g. `"JHN.3.16"`), or a generic fallback for any other kind.
/// Deliberately NOT `graph_types::node::card()`'s own placeholder label
/// (`"text unit (bible)"`) -- that function is documented as a skeleton
/// stand-in awaiting real `CorpusScheme::cite`-driven citation strings
/// (types doc §6); this is this DTO layer's own such computation, an
/// EXTENSION (new conversion), not a change to graph-types' shipped `card()`.
pub fn describe_node(id: &AnyNodeId, query: &dyn GraphQuery) -> (String, String) {
    match id.kind {
        NodeKind::TextUnit => {
            if let Some((book, chapter, verse)) = atlas_graph::kjv_adapter::decode_text_unit(id) {
                return (atlas_graph::kjv_adapter::dot_ref(book, chapter, verse), "TextUnit".to_string());
            }
            // CORP-2a: the Concord sibling -- `ConcordTag::cite`'s own
            // citation format (graph-types' text.rs), reused verbatim
            // rather than a third hand-written "BoC ..." string.
            if let Some((part, article, paragraph)) = atlas_graph::concord_adapter::decode_text_unit(id) {
                use atlas_graph_types::text::Corpus;
                return (atlas_graph_types::text::ConcordTag::cite(&atlas_graph_types::text::ConcordRef { part, article, paragraph }), "TextUnit".to_string());
            }
            ("text unit".to_string(), "TextUnit".to_string())
        }
        _ => {
            // Not TextUnit (M-A never materializes another kind): fall back
            // to graph-types' own `card()` view assembly rather than
            // re-deriving its match here -- one label computation, reused.
            // Node lookup goes through THE PORT (design doc §9a; fix round
            // 1, C1) -- `GraphQuery::node`, never a direct field reach.
            let label =
                query.node(id).map(|n| atlas_graph_types::node::card(&n).label).unwrap_or_else(|| format!("{:?}", id.kind));
            (label, format!("{:?}", id.kind))
        }
    }
}

/// A `Position` (a node or an edge -- edges take focus too, design doc §0)
/// rendered as a wire node-reference: `(id, kind, label)`. Total over both
/// variants, never a panic -- M-A's own `edges()` calls only ever surface
/// `Position::Node` today (no edge-as-position query exists yet), but the
/// type is `Position` so this stays honest about the full shape.
pub fn describe_position(pos: &Position, query: &dyn GraphQuery) -> (String, String, String) {
    match pos {
        Position::Node(id) => {
            let (label, kind) = describe_node(id, query);
            (encode_node_id(id), kind, label)
        }
        Position::Edge(eid) => (format!("edge:{}", eid.0), "Edge".to_string(), eid.0.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_narrative_anchor_place_ids_round_trip_through_the_wire_form() {
        for (kind, raw, expected_wire) in [
            (NodeKind::Event, "ab_ur", "Event:ab_ur"),
            (NodeKind::Narrative, "conquest", "Narrative:conquest"),
            (NodeKind::Anchor, "solomon-crowned", "Anchor:solomon-crowned"),
            (NodeKind::Place, "jericho", "Place:jericho"),
            // Batch M-C.
            (NodeKind::Era, "patriarchs", "Era:patriarchs"),
            (NodeKind::Polity, "egypt", "Polity:egypt"),
            (NodeKind::CatechismItem, "first-commandment", "CatechismItem:first-commandment"),
            // Batch P.
            (NodeKind::Person, "aaron_1", "Person:aaron_1"),
            // Batch CORP-1a.
            (NodeKind::Translation, "latin_vulgate", "Translation:latin_vulgate"),
        ] {
            let id = AnyNodeId { kind, raw: raw.to_string() };
            let wire = encode_node_id(&id);
            assert_eq!(wire, expected_wire, "encode_node_id's own pre-existing generic fallback must already produce this shape");
            assert_eq!(decode_node_id(&wire), Some(id), "decode must be encode's exact inverse for every M-B/M-C kind");
        }
    }

    #[test]
    fn decode_node_id_rejects_an_empty_raw_id() {
        assert_eq!(decode_node_id("Event:"), None);
        assert_eq!(decode_node_id("not-even-a-colon-pair"), None);
    }

    #[test]
    fn text_unit_id_round_trips_through_the_wire_form() {
        let id = atlas_graph::kjv_adapter::verse_node_id(42, 3, 16); // JHN is index 42
        let wire = encode_node_id(&id);
        assert_eq!(wire, "text-unit:JHN.3.16");
        assert_eq!(decode_node_id(&wire), Some(id));
    }

    #[test]
    fn edge_kind_labels_round_trip_for_every_relation() {
        for r in RelationId::ALL {
            for dir in [Direction::Forward, Direction::Inverse] {
                let k = EdgeKind::Directed(*r, dir);
                assert_eq!(parse_edge_kind(k.label()), Some(k));
            }
        }
        for s in SymRelationId::ALL {
            let k = EdgeKind::Symmetric(*s);
            assert_eq!(parse_edge_kind(k.label()), Some(k));
        }
        assert_eq!(parse_edge_kind("not-a-real-kind"), None);
    }
}
