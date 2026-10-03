use review_title_core::sources::{DuplicateProvenance, ProvenanceTitles};

const GENERATED_IDS: usize = 32;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ordered: Vec<(String, String)> = (0..GENERATED_IDS).map(|i| (format!("p-{i:02}"), format!("Title {i}"))).collect();
    let mut descending = ordered.clone();
    descending.reverse();
    let distinct = ProvenanceTitles::from_rows(descending)?;
    let actual: Vec<(String, String)> = distinct.rows().map(|(id, title)| (id.into(), title.into())).collect();
    assert_eq!(actual, ordered);
    let empty = ProvenanceTitles::from_rows(Vec::<(String, String)>::new())?;
    assert_eq!(empty, ProvenanceTitles::default());
    assert_eq!(empty.rows().collect::<Vec<_>>(), Vec::<(&str, &str)>::new());
    let mut refused = 0;
    for (id, title) in &ordered {
        for duplicate_title in [title.clone(), "Conflicting title".into()] {
            for at in 0..=ordered.len() {
                let mut rows = ordered.clone();
                rows.insert(at, (id.clone(), duplicate_title.clone()));
                assert_eq!(ProvenanceTitles::from_rows(rows), Err(DuplicateProvenance { id: id.clone() }));
                refused += 1;
            }
        }
        for locator in 0..GENERATED_IDS {
            assert_eq!(distinct.title_of(&format!("{id}/piece-{locator}/nested")), Some(title.as_str()));
        }
    }
    let error = DuplicateProvenance { id: "p-00".into() };
    assert_eq!(error.to_string(), "the provenance p-00 is titled more than once");
    assert_eq!(refused, GENERATED_IDS * 2 * (GENERATED_IDS + 1));
    println!("{refused} duplicate cases refused with the exact conflicting id, including identical/conflicting titles at every insertion position.");
    println!("The complete ordered 32-row result, empty/default result, clone and 1024 nested locator lookups pass across a separately compiled public consumer.");
    assert_eq!(distinct.clone(), distinct);
    Ok(())
}
