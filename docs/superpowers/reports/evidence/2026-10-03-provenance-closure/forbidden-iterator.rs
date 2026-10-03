use review_title_core::sources::ProvenanceTitles;
fn main() {
    let _: ProvenanceTitles = vec![("id".to_string(), "One".to_string()), ("id".to_string(), "Two".to_string())].into_iter().collect();
}
