#![cfg(all(feature = "serde", feature = "openapi"))]

use atlas_graph_types::{EdgeKind, NodeKind};
use utoipa::PartialSchema;

const DECLARED_NODE_KINDS: usize = 15;
const DECLARED_EDGE_KINDS: usize = 46;

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
fn node_kind_schema_is_the_closed_enum_of_names() {
    // Arrange
    let expected: Vec<&str> = NodeKind::ALL.iter().map(|k| k.name()).collect();
    // Act
    let schema = serde_json::to_value(NodeKind::schema()).unwrap();
    // Assert
    assert_eq!(expected.len(), DECLARED_NODE_KINDS);
    assert_eq!(schema, serde_json::json!({ "type": "string", "enum": expected }));
}

#[test]
fn edge_kind_schema_is_the_closed_enum_of_labels() {
    // Arrange
    let expected: Vec<&str> = EdgeKind::labels().collect();
    // Act
    let schema = serde_json::to_value(EdgeKind::schema()).unwrap();
    // Assert
    assert_eq!(expected.len(), DECLARED_EDGE_KINDS);
    assert_eq!(schema, serde_json::json!({ "type": "string", "enum": expected }));
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
