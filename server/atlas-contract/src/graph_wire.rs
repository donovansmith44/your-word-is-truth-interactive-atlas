//! The wire form of a node id: a round-trippable string a caller hands back.

use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::node::Node;
use atlas_graph_types::store::GraphQuery;

use crate::wire::{EdgeRef, NodeRef, PositionRef};

pub fn encode_node_id(id: &AnyNodeId) -> String {
    match id.kind {
        NodeKind::TextUnit => match atlas_graph::kjv_adapter::decode_text_unit(id) {
            Some((book, chapter, verse)) => format!("text-unit:{}", atlas_graph::kjv_adapter::dot_ref(book, chapter, verse)),
            None => match atlas_graph::concord_adapter::decode_text_unit(id) {
                Some((part, article, paragraph)) => format!("text-unit:BoC {part}.{article}.{paragraph}"),
                None => format!("text-unit:{}", id.raw),
            },
        },
        // The fallback keeps this total for a kind with no prettier wire form,
        // rather than leaving one unreachable through the generic endpoints.
        other => format!("{other:?}:{}", id.raw),
    }
}

pub fn decode_node_id(s: &str) -> Option<AnyNodeId> {
    let (kind, rest) = s.split_once(':')?;
    if rest.is_empty() {
        return None;
    }
    match kind {
        "text-unit" => {
            // A Bible dot-ref never starts "BoC ", so the prefix decides the
            // corpus unambiguously and the Bible parse below needs no guard.
            if let Some(concord_rest) = rest.strip_prefix("BoC ") {
                let mut parts = concord_rest.split('.');
                let part: u8 = parts.next()?.parse().ok()?;
                let article: u16 = parts.next()?.parse().ok()?;
                let paragraph: u16 = parts.next()?.parse().ok()?;
                if parts.next().is_some() {
                    return None;
                }
                return Some(atlas_graph::concord_adapter::text_unit_id(part, article, paragraph));
            }
            let vid = atlas_core::refs::VerseId::parse_canonical(rest).ok()?;
            Some(atlas_graph::kjv_adapter::verse_node_id(vid.book.0, vid.chapter, vid.verse))
        }
        "Event" => Some(AnyNodeId { kind: NodeKind::Event, raw: rest.to_string() }),
        "Narrative" => Some(AnyNodeId { kind: NodeKind::Narrative, raw: rest.to_string() }),
        "Anchor" => Some(AnyNodeId { kind: NodeKind::Anchor, raw: rest.to_string() }),
        "Place" => Some(AnyNodeId { kind: NodeKind::Place, raw: rest.to_string() }),
        "Era" => Some(AnyNodeId { kind: NodeKind::Era, raw: rest.to_string() }),
        "Polity" => Some(AnyNodeId { kind: NodeKind::Polity, raw: rest.to_string() }),
        "CatechismItem" => Some(AnyNodeId { kind: NodeKind::CatechismItem, raw: rest.to_string() }),
        "Person" => Some(AnyNodeId { kind: NodeKind::Person, raw: rest.to_string() }),
        "Translation" => Some(AnyNodeId { kind: NodeKind::Translation, raw: rest.to_string() }),
        "CommentaryItem" => Some(AnyNodeId { kind: NodeKind::CommentaryItem, raw: rest.to_string() }),
        "Container" => Some(AnyNodeId { kind: NodeKind::Container, raw: rest.to_string() }),
        "LexiconEntry" => Some(AnyNodeId { kind: NodeKind::LexiconEntry, raw: rest.to_string() }),
        "Map" => Some(AnyNodeId { kind: NodeKind::Map, raw: rest.to_string() }),
        _ => None,
    }
}

pub fn describe_node(id: &AnyNodeId, query: &dyn GraphQuery) -> String {
    text_unit_label(id).unwrap_or_else(|| node_label(id, query.node(id)))
}

pub fn describe_nodes(ids: &BTreeSet<AnyNodeId>, query: &dyn GraphQuery) -> BTreeMap<AnyNodeId, NodeRef> {
    let read: Vec<AnyNodeId> = ids.iter().filter(|id| text_unit_label(id).is_none()).cloned().collect();
    let mut nodes: BTreeMap<AnyNodeId, Option<Node>> = read.iter().cloned().zip(query.nodes(&read)).collect();
    ids.iter()
        .map(|id| {
            let label = text_unit_label(id).unwrap_or_else(|| node_label(id, nodes.remove(id).flatten()));
            (id.clone(), labelled(id, label))
        })
        .collect()
}

fn text_unit_label(id: &AnyNodeId) -> Option<String> {
    if id.kind != NodeKind::TextUnit {
        return None;
    }
    if let Some((book, chapter, verse)) = atlas_graph::kjv_adapter::decode_text_unit(id) {
        return Some(atlas_graph::kjv_adapter::dot_ref(book, chapter, verse));
    }
    if let Some((part, article, paragraph)) = atlas_graph::concord_adapter::decode_text_unit(id) {
        use atlas_graph_types::text::Corpus;
        return Some(atlas_graph_types::text::ConcordTag::cite(&atlas_graph_types::text::ConcordRef { part, article, paragraph }));
    }
    Some("text unit".to_string())
}

fn node_label(id: &AnyNodeId, node: Option<Node>) -> String {
    node.map(|n| atlas_graph_types::node::card(&n).label).unwrap_or_else(|| id.kind.name().to_string())
}

pub fn describe_position(pos: &Position, query: &dyn GraphQuery) -> PositionRef {
    match pos {
        Position::Node(id) => PositionRef::Node { node: node_ref(id, query) },
        Position::Edge(eid) => PositionRef::Edge { edge: EdgeRef { id: eid.0.clone() } },
    }
}

pub fn node_ref(id: &AnyNodeId, query: &dyn GraphQuery) -> NodeRef {
    labelled(id, describe_node(id, query))
}

fn labelled(id: &AnyNodeId, label: String) -> NodeRef {
    NodeRef { id: encode_node_id(id), kind: id.kind, label }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A_DATING: &str = "DatedBy:00ff";

    #[test]
    fn a_position_is_described_as_the_node_or_the_edge_it_names() {
        // Arrange
        let graph = atlas_graph_types::graph::Graph::default();
        let verse = decode_node_id("text-unit:JHN.3.16").unwrap();
        let dating = atlas_graph_types::edge::EdgeId(A_DATING.to_string());
        // Act
        let described = [describe_position(&Position::Node(verse), &graph), describe_position(&Position::Edge(dating), &graph)];
        // Assert
        assert_eq!(
            described,
            [
                PositionRef::Node { node: NodeRef { id: "text-unit:JHN.3.16".to_string(), kind: NodeKind::TextUnit, label: "JHN.3.16".to_string() } },
                PositionRef::Edge { edge: EdgeRef { id: A_DATING.to_string() } },
            ]
        );
    }

    #[test]
    fn event_narrative_anchor_place_ids_round_trip_through_the_wire_form() {
        for (kind, raw, expected_wire) in [
            (NodeKind::Event, "ab_ur", "Event:ab_ur"),
            (NodeKind::Narrative, "conquest", "Narrative:conquest"),
            (NodeKind::Anchor, "solomon-crowned", "Anchor:solomon-crowned"),
            (NodeKind::Place, "jericho", "Place:jericho"),
            (NodeKind::Era, "patriarchs", "Era:patriarchs"),
            (NodeKind::Polity, "egypt", "Polity:egypt"),
            (NodeKind::CatechismItem, "first-commandment", "CatechismItem:first-commandment"),
            (NodeKind::Person, "aaron_1", "Person:aaron_1"),
            (NodeKind::Translation, "latin_vulgate", "Translation:latin_vulgate"),
            (NodeKind::CommentaryItem, "kretzmann/0.1.0", "CommentaryItem:kretzmann/0.1.0"),
            (NodeKind::Container, "bible-book-GEN", "Container:bible-book-GEN"),
            (NodeKind::Container, "bible-chapter-GEN-1", "Container:bible-chapter-GEN-1"),
            (NodeKind::Container, "concord-doc-small-catechism", "Container:concord-doc-small-catechism"),
            (NodeKind::LexiconEntry, "H430", "LexiconEntry:H430"),
            (NodeKind::Map, "era-primeval", "Map:era-primeval"),
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
        let id = atlas_graph::kjv_adapter::verse_node_id(42, 3, 16);
        let wire = encode_node_id(&id);
        assert_eq!(wire, "text-unit:JHN.3.16");
        assert_eq!(decode_node_id(&wire), Some(id));
    }
}
