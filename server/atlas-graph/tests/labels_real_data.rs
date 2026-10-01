mod common;

use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnyNodeId, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;

use common::{committed_graph, committed_service};

const LABELS_PER_READ: usize = 5000;

fn served_labels(at: &[Position]) -> Vec<(Position, String)> {
    let snapshot = committed_service().snapshot();
    at.chunks(LABELS_PER_READ).flat_map(|chunk| chunk.iter().cloned().zip(snapshot.labels(chunk)).filter_map(|(position, label)| label.map(|label| (position, label))).collect::<Vec<_>>()).collect()
}

fn every_position(graph: &Graph) -> Vec<Position> {
    graph.nodes.keys().cloned().map(Position::Node).chain(graph.edges_by_id.keys().cloned().map(Position::Edge)).collect()
}

fn spelled_as_words(text: &str) -> String {
    text.to_ascii_lowercase().replace([' ', '_'], "-")
}

fn carries_its_ids_disambiguator(id: &AnyNodeId, label: &str) -> bool {
    atlas_core::history::strip_disambiguation_suffix(label) != label && spelled_as_words(label) == spelled_as_words(&id.raw)
}

fn carries_an_id_shape(label: &str) -> bool {
    label.contains("::") || label.split_whitespace().any(|word| word.contains('_'))
}

#[test]
fn no_served_label_carries_a_disambiguation_suffix_or_an_id_shape() {
    // Arrange
    let graph = committed_graph();
    let positions = every_position(graph);

    // Act
    let labels = served_labels(&positions);
    let offenders: Vec<(Position, String)> = labels
        .iter()
        .filter(|(position, label)| carries_an_id_shape(label) || matches!(position, Position::Node(id) if carries_its_ids_disambiguator(id, label)))
        .take(20)
        .cloned()
        .collect();

    // Assert
    assert_eq!((labels.len(), offenders), (positions.len(), Vec::new()));
}

#[test]
fn every_places_label_is_its_compiled_default_display_name() {
    // Arrange
    let places: Vec<Position> = committed_graph().nodes.values().filter(|node| matches!(node.payload, NodePayload::Place { .. })).map(|node| Position::Node(node.id.clone())).collect();

    // Act
    let disagreeing: Vec<(Position, String, Option<String>)> = served_labels(&places)
        .into_iter()
        .filter_map(|(position, label)| {
            let Position::Node(id) = &position else { return None };
            let display_name = committed_service().geography.place_default(id).map(|default| default.display_name.clone());
            (display_name.as_deref() != Some(label.as_str())).then_some((position, label, display_name))
        })
        .take(20)
        .collect();

    // Assert
    assert_eq!((places.is_empty(), disagreeing), (false, Vec::new()));
}

#[test]
fn a_label_spelling_its_ids_disambiguator_or_a_code_path_is_an_offender_and_a_reader_facing_one_is_not() {
    // Arrange
    let achzib = atlas_graph_types::id::PlaceId::new("achzib-1").erase();
    let chapter = atlas_graph_types::id::ContainerNodeId::new("bible-chapter-1CH-1").erase();

    // Act
    let judged = (
        carries_its_ids_disambiguator(&achzib, "Achzib 1"),
        carries_its_ids_disambiguator(&achzib, "Achzib"),
        carries_its_ids_disambiguator(&chapter, "1 Chronicles 1"),
        carries_an_id_shape("See atlas_core::chronology::ANCHOR_DEFERRALS"),
        carries_an_id_shape("ret_babylon still ships"),
        carries_an_id_shape("Adoni-bezek"),
    );

    // Assert
    assert_eq!(judged, (true, false, false, true, true, false));
}
