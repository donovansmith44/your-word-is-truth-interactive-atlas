#![cfg(all(feature = "serde", feature = "openapi"))]

use atlas_graph_types::{EdgeKind, NodeKind};
use utoipa::PartialSchema;

const DECLARED_NODE_KINDS: usize = 15;
const DECLARED_EDGE_KINDS: usize = 46;
const NODE_KIND_DESCRIPTION: &str = "What kind of thing one node of this atlas stands for.";
const EDGE_KIND_DESCRIPTION: &str = "A relation between two nodes, named in the direction it is travelled: the label one frontier of a node is asked for by.";

#[test]
fn serialize_emits_the_name_and_the_label() {
    // Arrange
    let kind = NodeKind::Container;
    let edge = EdgeKind::from_label("member-of").unwrap();
    // Act
    let kind_json = serde_json::to_string(&kind).unwrap();
    let edge_json = serde_json::to_string(&edge).unwrap();
    // Assert
    assert_eq!(kind_json, "\"Container\"");
    assert_eq!(edge_json, "\"member-of\"");
}

#[test]
fn a_node_kind_reads_back_from_the_name_it_was_written_as() {
    // Arrange
    let every_kind = NodeKind::ALL;
    // Act
    let json = serde_json::to_string(&every_kind).unwrap();
    let back: Vec<NodeKind> = serde_json::from_str(&json).unwrap();
    // Assert
    assert_eq!(back, every_kind.to_vec());
}

#[test]
fn an_edge_kind_reads_back_from_the_label_it_was_written_as() {
    // Arrange
    let every_kind: Vec<EdgeKind> = EdgeKind::all().collect();
    // Act
    let json = serde_json::to_string(&every_kind).unwrap();
    let back: Vec<EdgeKind> = serde_json::from_str(&json).unwrap();
    // Assert
    assert_eq!(back, every_kind);
}

#[test]
fn a_node_kind_that_names_no_kind_is_refused_with_the_kinds_that_would_have_done() {
    // Arrange
    let undeclared = "\"Verse\"";
    // Act
    let refusal = serde_json::from_str::<NodeKind>(undeclared).unwrap_err().to_string();
    // Assert
    assert_eq!(refusal, format!("unknown variant `Verse`, expected one of {} at line 1 column 7", NodeKind::ALL.map(|kind| format!("`{}`", kind.name())).join(", ")));
}

#[test]
fn an_edge_kind_that_names_no_relation_is_refused_by_the_label_it_was_asked_with() {
    // Arrange
    let undeclared = "\"cited\"";
    // Act
    let refusal = serde_json::from_str::<EdgeKind>(undeclared).unwrap_err().to_string();
    // Assert
    assert_eq!(refusal, "unknown relation label `cited` at line 1 column 7");
}

#[test]
fn node_kind_schema_is_the_closed_enum_of_names() {
    // Arrange
    let expected: Vec<&str> = NodeKind::ALL.iter().map(|k| k.name()).collect();
    // Act
    let schema = serde_json::to_value(NodeKind::schema()).unwrap();
    // Assert
    assert_eq!(expected.len(), DECLARED_NODE_KINDS);
    assert_eq!(schema, serde_json::json!({ "type": "string", "description": NODE_KIND_DESCRIPTION, "enum": expected }));
}

#[test]
fn edge_kind_schema_is_the_closed_enum_of_labels() {
    // Arrange
    let expected: Vec<&str> = EdgeKind::labels().collect();
    // Act
    let schema = serde_json::to_value(EdgeKind::schema()).unwrap();
    // Assert
    assert_eq!(expected.len(), DECLARED_EDGE_KINDS);
    assert_eq!(schema, serde_json::json!({ "type": "string", "description": EDGE_KIND_DESCRIPTION, "enum": expected }));
}

#[test]
fn schema_component_names_are_the_type_names() {
    // Arrange
    use utoipa::ToSchema;
    // Act
    let node = <NodeKind as ToSchema>::name();
    let edge = <EdgeKind as ToSchema>::name();
    // Assert
    assert_eq!(node, "NodeKind");
    assert_eq!(edge, "EdgeKind");
}
