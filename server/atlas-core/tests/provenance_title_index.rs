use atlas_core::sources::{DuplicateProvenance, ProvenanceTitles};

const GENERATED_IDS: usize = 32;

fn rows(ids: usize) -> Vec<(String, String)> {
    (0..ids).map(|i| (format!("provenance-{i}"), format!("Source {i}"))).collect()
}

#[test]
fn a_title_index_built_from_rows_with_a_repeated_id_is_refused_naming_that_id() {
    // Arrange
    let conflicting: Vec<Vec<(String, String)>> = (0..GENERATED_IDS)
        .map(|i| {
            let mut with_duplicate = rows(GENERATED_IDS);
            with_duplicate.push((format!("provenance-{i}"), "Another source".to_string()));
            with_duplicate
        })
        .collect();

    // Act
    let built: Vec<Result<ProvenanceTitles, DuplicateProvenance>> = conflicting.into_iter().map(ProvenanceTitles::from_rows).collect();

    // Assert
    let expected: Vec<Result<ProvenanceTitles, DuplicateProvenance>> = (0..GENERATED_IDS).map(|i| Err(DuplicateProvenance { id: format!("provenance-{i}") })).collect();
    assert_eq!(built, expected);
}

#[test]
fn a_title_index_built_from_rows_with_distinct_ids_holds_every_row() {
    // Arrange
    let distinct = rows(GENERATED_IDS);

    // Act
    let built = ProvenanceTitles::from_rows(distinct.clone()).unwrap();

    // Assert
    let mut expected = distinct;
    expected.sort();
    assert_eq!(built.rows().map(|(id, title)| (id.to_string(), title.to_string())).collect::<Vec<_>>(), expected);
}
