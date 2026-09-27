use atlas_graph_types::{Direction, EdgeKind, NodeKind, RelationId, SymRelationId};

const DECLARED_DIRECTED_RELATIONS: usize = 20;
const DECLARED_SYMMETRIC_RELATIONS: usize = 6;
const DECLARED_EDGE_KINDS: usize = 2 * DECLARED_DIRECTED_RELATIONS + DECLARED_SYMMETRIC_RELATIONS;

#[test]
fn every_name_round_trips_through_from_name() {
    // Arrange
    let kinds = NodeKind::ALL;
    // Act
    let back: Vec<Option<NodeKind>> = kinds.iter().map(|k| NodeKind::from_name(k.name())).collect();
    // Assert
    assert_eq!(back, kinds.iter().map(|k| Some(*k)).collect::<Vec<_>>());
}

#[test]
fn names_are_the_debug_names_the_wire_already_carries() {
    // Arrange
    let kinds = NodeKind::ALL;
    // Act
    let names: Vec<&str> = kinds.iter().map(|k| k.name()).collect();
    let debug_names: Vec<String> = kinds.iter().map(|k| format!("{k:?}")).collect();
    // Assert
    assert_eq!(names, debug_names);
}

#[test]
fn from_name_rejects_an_undeclared_kind() {
    // Arrange
    let name = "Verse";
    // Act
    let kind = NodeKind::from_name(name);
    // Assert
    assert_eq!(kind, None);
}

#[test]
fn all_lists_forward_then_inverse_for_every_relation_then_every_symmetric() {
    // Arrange
    let expected: Vec<EdgeKind> = RelationId::ALL
        .iter()
        .flat_map(|r| [EdgeKind::Directed(*r, Direction::Forward), EdgeKind::Directed(*r, Direction::Inverse)])
        .chain(SymRelationId::ALL.iter().map(|s| EdgeKind::Symmetric(*s)))
        .collect();
    // Act
    let all: Vec<EdgeKind> = EdgeKind::all().collect();
    // Assert
    assert_eq!(all, expected);
    assert_eq!(all.len(), DECLARED_EDGE_KINDS);
}

#[test]
fn labels_are_distinct() {
    // Arrange
    let labels: Vec<&str> = EdgeKind::labels().collect();
    // Act
    let mut deduped = labels.clone();
    deduped.sort_unstable();
    deduped.dedup();
    // Assert
    assert_eq!(deduped.len(), labels.len());
}

#[test]
fn every_label_round_trips_through_from_label() {
    // Arrange
    let kinds: Vec<EdgeKind> = EdgeKind::all().collect();
    // Act
    let back: Vec<Option<EdgeKind>> = kinds.iter().map(|k| EdgeKind::from_label(k.label())).collect();
    // Assert
    assert_eq!(back, kinds.iter().map(|k| Some(*k)).collect::<Vec<_>>());
}

#[test]
fn from_label_rejects_an_undeclared_label() {
    // Arrange
    let label = "cited";
    // Act
    let kind = EdgeKind::from_label(label);
    // Assert
    assert_eq!(kind, None);
}
