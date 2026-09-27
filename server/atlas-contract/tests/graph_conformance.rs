use std::collections::HashSet;
use std::path::Path;

use atlas_graph::build::{build_graph_from_sources, build_graph_from_sources_with_eras_and_brainfuel};
use atlas_graph_types::store::{assert_answers_match, GraphPublisher, GraphStore, MemStore};

fn real_kjv_slice(book_names: &[&str]) -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let raw = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist (committed real data)");
    let mut root: serde_json::Value = serde_json::from_str(&raw).expect("data/raw/kjv.json must be valid JSON");
    let wanted: HashSet<&str> = book_names.iter().copied().collect();
    let books = root.get_mut("books").and_then(|b| b.as_array_mut()).expect("kjv.json must have a books array");
    books.retain(|b| b.get("name").and_then(|n| n.as_str()).is_some_and(|n| wanted.contains(n)));
    assert_eq!(books.len(), book_names.len(), "every requested book name must exist verbatim in the real kjv.json");
    serde_json::to_string(&root).unwrap()
}

fn real_xrefs() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist (committed real data)")
}

#[test]
fn the_real_kjv_derived_graph_is_admitted_the_in_memory_store_answers_match_the_model_exactly() {
    let kjv_json = real_kjv_slice(&["Ruth"]);
    let xrefs_tsv = real_xrefs();
    let atlas = atlas_graph::event_world::empty_atlas();

    let (model, model_stats, ..) = build_graph_from_sources(&kjv_json, &xrefs_tsv, &atlas).expect("the real Ruth slice must parse");
    assert_eq!(model_stats.kjv_verses, 85, "Ruth has 85 verses in the real KJV text");

    let (for_store, ..) = build_graph_from_sources(&kjv_json, &xrefs_tsv, &atlas).expect("the real Ruth slice must parse a second time identically");
    let mut store = MemStore::default();
    let version = store.publish(for_store);
    let snapshot = store.open(version).expect("the just-published version must be open-able");

    assert_answers_match(&snapshot, &model);
}

#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn the_full_real_graph_is_admitted_the_in_memory_store_answers_match_the_model_exactly() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist (committed real data)");
    let xrefs_tsv =
        std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist (committed real data)");

    let curated = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/curated");
    let atlas = atlas_etl::compile::compile(&dir, &curated).expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify").data;
    let brainfuel = atlas_etl::brainfuel::read_all(&dir.join("brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step first");

    let (model, model_stats, model_ew_stats, _) =
        build_graph_from_sources_with_eras_and_brainfuel(&kjv_json, &xrefs_tsv, &atlas, &[], Some(&brainfuel)).expect("the real KJV source must parse");
    assert_eq!(model_stats.kjv_verses, 31_102, "the real KJV text is 31,102 verses");
    assert!(model_ew_stats.dated_events >= 450, "expected the real compiled event set to carry well over 450 dated events, got {}", model_ew_stats.dated_events);

    let (for_store, ..) =
        build_graph_from_sources_with_eras_and_brainfuel(&kjv_json, &xrefs_tsv, &atlas, &[], Some(&brainfuel)).expect("the real KJV source must parse a second time identically");
    let mut store = MemStore::default();
    let version = store.publish(for_store);
    let snapshot = store.open(version).expect("the just-published version must be open-able");

    const CONFORMANCE_CEILING: std::time::Duration = std::time::Duration::from_secs(60);
    let match_start = std::time::Instant::now();
    assert_answers_match(&snapshot, &model);
    let elapsed = match_start.elapsed();

    eprintln!(
        "M-B/M-C FULL-SCALE CONFORMANCE: {} text units, {} cites edges, {} events ({} dated), {} narratives, {} anchors, {} attests, {} succession rows, {} located-at rows, {} dated-by rows -- assert_answers_match wall time: {:?} (ceiling {:?})",
        model_stats.kjv_verses,
        model_stats.cites_rows,
        model_ew_stats.events,
        model_ew_stats.dated_events,
        model_ew_stats.narratives,
        model_ew_stats.anchors,
        model_ew_stats.attests_rows,
        model_ew_stats.succession_rows,
        model_ew_stats.located_at_rows,
        model_ew_stats.dated_by_rows,
        elapsed,
        CONFORMANCE_CEILING,
    );
    assert!(
        elapsed <= CONFORMANCE_CEILING,
        "full-graph assert_answers_match took {elapsed:?}, exceeding the committed {CONFORMANCE_CEILING:?} ceiling (controller decision 6) -- this is a red build, not a hope"
    );
}
