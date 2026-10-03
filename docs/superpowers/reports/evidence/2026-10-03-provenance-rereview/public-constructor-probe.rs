use review_title_core::sources::ProvenanceTitles;
fn main() {
    let mut results=Vec::new();
    for seed in 0..32_u32 {
        let id=format!("provenance-{seed}");
        let last=format!("Other {seed}");
        let index:ProvenanceTitles=vec![(id.clone(),format!("Title {seed}")),(id.clone(),last.clone())].into_iter().collect();
        let whole:Vec<(String,String)>=index.rows().map(|(id,title)|(id.to_string(),title.to_string())).collect();
        assert_eq!(whole,vec![(id,last)]);
        results.push(whole);
    }
    println!("Separate crate, exported production FromIterator API: all 32 conflicting duplicate constructions compiled and completed. Whole outputs: {results:?}");
    let actual:ProvenanceTitles=vec![("kjv".to_string(),"The King James Version".to_string()),("kjv".to_string(),"OpenBible.info Geocoding".to_string())].into_iter().collect();
    let whole:Vec<(String,String)>=actual.rows().map(|(id,title)|(id.to_string(),title.to_string())).collect();
    assert_eq!(whole,vec![("kjv".to_string(),"OpenBible.info Geocoding".to_string())]);
    println!("Complete representative output: {whole:?}; no ETL dependency or AdmittedSources value used.");
}
