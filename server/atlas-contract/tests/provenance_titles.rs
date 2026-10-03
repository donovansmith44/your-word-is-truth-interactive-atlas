use std::path::Path;

const TITLED_PROVENANCE: &str = "#/components/schemas/Provenance";

#[test]
fn every_provenance_the_published_contract_serves_is_titled() {
    // Arrange
    let document = serde_json::to_value(atlas_contract::document::openapi()).unwrap();
    let schemas = document["components"]["schemas"].as_object().unwrap();

    // Act
    let served: Vec<String> = schemas
        .iter()
        .flat_map(|(schema, body)| {
            body["properties"].as_object().into_iter().flatten().filter(|(name, _)| name.ends_with("provenance")).map(move |(name, property)| (format!("{schema}.{name}"), property))
        })
        .filter(|(_, property)| !property.to_string().contains(TITLED_PROVENANCE))
        .map(|(at, _)| at)
        .collect();

    // Assert
    assert_eq!(served, Vec::<String>::new());
}

#[test]
fn every_provenance_the_committed_artifact_carries_has_its_source_title() {
    // Arrange
    let compiled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled");
    let (graph, data, _) = atlas_contract::load::load_all(&compiled).unwrap();
    let carried: Vec<String> = graph.provenance.families().into_iter().flat_map(|family| graph.provenance.by_family(family)).collect();

    // Act
    let untitled: Vec<&String> = carried.iter().filter(|id| data.provenance_titles.title_of(id).is_none()).collect();

    // Assert
    assert_eq!((carried.is_empty(), untitled), (false, Vec::<&String>::new()));
}
