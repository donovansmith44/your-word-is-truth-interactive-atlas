mod common;

use atlas_core::time::TimeRange;
use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::node::NodePayload;

use common::{committed_graph, committed_service};

fn nodes_of(payload: fn(&NodePayload) -> bool) -> Vec<(AnyNodeId, NodePayload)> {
    committed_graph().nodes.values().filter(|node| payload(&node.payload)).map(|node| (node.id.clone(), node.payload.clone())).collect()
}

#[test]
fn every_polity_has_one_compiled_reign_spanning_its_eras() {
    // Arrange
    let polities = nodes_of(|payload| matches!(payload, NodePayload::Polity { .. }));

    // Act
    let unreigned: Vec<(AnyNodeId, Option<TimeRange>)> = polities
        .iter()
        .filter_map(|(id, payload)| {
            let NodePayload::Polity { eras, .. } = payload else { return None };
            let span = TimeRange { from_year: eras.iter().map(|era| era.from_year).min()?, to_year: eras.iter().map(|era| era.to_year).max()? };
            let compiled = committed_service().geography.reign_of(id);
            (compiled != Some(span)).then(|| (id.clone(), compiled))
        })
        .collect();

    // Assert
    assert_eq!((polities.is_empty(), unreigned), (false, Vec::new()));
}

#[test]
fn every_place_has_its_default_name_compiled() {
    // Arrange
    let places = nodes_of(|payload| matches!(payload, NodePayload::Place { .. }));

    // Act
    let unnamed: Vec<&AnyNodeId> = places.iter().map(|(id, _)| id).filter(|id| committed_service().geography.place_default(id).is_none()).collect();

    // Assert
    assert_eq!((places.is_empty(), unnamed), (false, Vec::<&AnyNodeId>::new()));
}
