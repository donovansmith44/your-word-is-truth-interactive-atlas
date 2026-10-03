//! The wire form of a node id: a round-trippable string a caller hands back.

use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::edge::EdgeId;
use atlas_graph_types::id::{AnyNodeId, Position};
use atlas_graph_types::store::GraphQuery;

use crate::error::ApiError;
use crate::wire::{EdgeRef, NodeRef, PositionRef};

use atlas_core::identity::{ConcordReference, NodeId, UnreferencedUnit};
use atlas_graph_types::canon::ids::any_node_id_str;

pub fn decode_node_id(s: &str) -> Option<AnyNodeId> {
    let (kind, rest) = s.split_once(':')?;
    if rest.is_empty() {
        return None;
    }
    if kind == NodeId::TEXT_UNIT {
        return match rest.parse::<ConcordReference>() {
            Ok(paragraph) => Some(atlas_graph::concord_adapter::text_unit_id(paragraph.part, paragraph.article, paragraph.paragraph)),
            Err(_) => atlas_core::refs::VerseId::parse_canonical(rest).ok().map(|verse| atlas_graph::kjv_adapter::verse_node_id(verse.book.0, verse.chapter, verse.verse)),
        };
    }
    NodeId::addressable().find(|addressable| addressable.name() == kind).map(|kind| AnyNodeId { kind, raw: rest.to_string() })
}

pub fn describe_node(id: &AnyNodeId, query: &dyn GraphQuery) -> Result<String, ApiError> {
    let at = Position::Node(id.clone());
    labelled_positions(std::slice::from_ref(&at), query).map(|mut labels| labels.remove(0))
}

pub fn describe_nodes(ids: &BTreeSet<AnyNodeId>, query: &dyn GraphQuery) -> Result<BTreeMap<AnyNodeId, NodeRef>, ApiError> {
    let ids: Vec<AnyNodeId> = ids.iter().cloned().collect();
    let at: Vec<Position> = ids.iter().map(|id| Position::Node(id.clone())).collect();
    let labels = labelled_positions(&at, query)?;
    let wire_ids = NodeId::encoded(&ids, query)?;
    Ok(ids.into_iter().zip(wire_ids).zip(labels).map(|((id, wire_id), label)| (id.clone(), NodeRef { id: wire_id, kind: id.kind, label })).collect())
}

pub fn describe_positions(at: &[Position], query: &dyn GraphQuery) -> Result<Vec<PositionRef>, ApiError> {
    let labels = labelled_positions(at, query)?;
    let nodes: Vec<AnyNodeId> = at.iter().filter_map(|position| match position {
        Position::Node(id) => Some(id.clone()),
        Position::Edge(_) => None,
    }).collect();
    let mut wire_ids = NodeId::encoded(&nodes, query)?.into_iter();
    at.iter()
        .zip(labels)
        .map(|(position, label)| match position {
            Position::Node(id) => {
                let wire_id = wire_ids.next().ok_or_else(|| ApiError::internal(&format!("{} was asked for and no wire id was encoded for it", named(position))))?;
                Ok(PositionRef::Node { node: NodeRef { id: wire_id, kind: id.kind, label } })
            }
            Position::Edge(id) => edge_ref(id, label).map(|edge| PositionRef::Edge { edge }),
        })
        .collect()
}

pub fn describe_position(at: &Position, query: &dyn GraphQuery) -> Result<PositionRef, ApiError> {
    describe_positions(std::slice::from_ref(at), query).map(|mut described| described.remove(0))
}

pub fn edge_ref(id: &EdgeId, label: String) -> Result<EdgeRef, ApiError> {
    let kind = id.recorded_kind().ok_or_else(|| ApiError::internal(&format!("the edge id {} names no relation", id.0)))?;
    Ok(EdgeRef { id: id.clone(), kind, label })
}

pub fn labelled_positions(at: &[Position], query: &dyn GraphQuery) -> Result<Vec<String>, ApiError> {
    at.iter()
        .zip(query.labels(at))
        .map(|(position, label)| label.ok_or_else(|| ApiError::internal(&format!("{} is held but no label is compiled for it", named(position)))))
        .collect()
}

pub fn node_ref(id: &AnyNodeId, query: &dyn GraphQuery) -> Result<NodeRef, ApiError> {
    let label = describe_node(id, query)?;
    Ok(atlas_graph::node_ref::node_ref(id, label, query)?)
}

fn named(at: &Position) -> String {
    match at {
        Position::Node(id) => any_node_id_str(id),
        Position::Edge(id) => id.0.clone(),
    }
}

impl From<UnreferencedUnit> for ApiError {
    fn from(unreferenced: UnreferencedUnit) -> Self {
        ApiError::internal(&format!("{} is a text unit and no reference is compiled for it", any_node_id_str(&unreferenced.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::id::NodeKind;

    const A_DATING: &str = "DatedBy:00ff";
    const A_DATING_LABEL: &str = "Solomon crowned · Dated by · 970 BC";
    const VERSE_LABEL: &str = "JHN.3.16";
    const VERSE_REFERENCE: &str = "JHN.3.16";

    #[test]
    fn a_position_is_described_as_the_node_or_the_edge_it_names_by_its_compiled_label() {
        // Arrange
        let verse = decode_node_id("text-unit:JHN.3.16").unwrap();
        let dating = atlas_graph_types::edge::EdgeId(A_DATING.to_string());
        let mut graph = atlas_graph_types::graph::Graph::default();
        graph.labels.insert(Position::Node(verse.clone()), VERSE_LABEL.to_string());
        graph.references.insert(verse.clone(), VERSE_REFERENCE.to_string());
        graph.labels.insert(Position::Edge(dating.clone()), A_DATING_LABEL.to_string());
        // Act
        let described = describe_positions(&[Position::Node(verse), Position::Edge(dating)], &graph).unwrap();
        // Assert
        assert_eq!(
            described,
            vec![
                PositionRef::Node { node: NodeRef { id: NodeId::asked("text-unit:JHN.3.16"), kind: NodeKind::TextUnit, label: VERSE_LABEL.to_string() } },
                PositionRef::Edge {
                    edge: EdgeRef {
                        id: atlas_graph_types::edge::EdgeId(A_DATING.to_string()),
                        kind: atlas_graph_types::edge::EdgeKind::Directed(atlas_graph_types::edge::RelationId::DatedBy, atlas_graph_types::edge::Direction::Forward),
                        label: A_DATING_LABEL.to_string(),
                    }
                },
            ]
        );
    }

    #[test]
    fn a_held_position_with_no_compiled_label_is_an_internal_defect_naming_it() {
        // Arrange
        let graph = atlas_graph_types::graph::Graph::default();
        let verse = decode_node_id("text-unit:JHN.3.16").unwrap();
        // Act
        let refused = describe_position(&Position::Node(verse), &graph).unwrap_err();
        // Assert
        assert_eq!((refused.code, refused.message), (crate::error::ErrorCode::Internal, "TextUnit:bible/42.3.16 is held but no label is compiled for it".to_string()));
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
            let wire = NodeId::encoded_one(&id, &atlas_graph_types::graph::Graph::default()).unwrap().to_string();
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
        let mut graph = atlas_graph_types::graph::Graph::default();
        graph.references.insert(id.clone(), VERSE_REFERENCE.to_string());
        let wire = NodeId::encoded_one(&id, &graph).unwrap().to_string();
        assert_eq!(wire, "text-unit:JHN.3.16");
        assert_eq!(decode_node_id(&wire), Some(id));
    }

    #[test]
    fn a_text_unit_with_no_compiled_reference_is_an_internal_defect_naming_it() {
        // Arrange
        let verse = atlas_graph::kjv_adapter::verse_node_id(42, 3, 16);
        let mut graph = atlas_graph_types::graph::Graph::default();
        graph.labels.insert(Position::Node(verse.clone()), VERSE_LABEL.to_string());

        // Act
        let refused = node_ref(&verse, &graph).unwrap_err();

        // Assert
        assert_eq!((refused.code, refused.message), (crate::error::ErrorCode::Internal, "TextUnit:bible/42.3.16 is a text unit and no reference is compiled for it".to_string()));
    }
}
