//! Batch M-C, controller decision 4: the serialized artifact as GraphStore
//! IMPLEMENTATION #2, admitted over the FULL real graph -- "implementation
//! #2 passes the same law as #1" (design doc §9a's two-implementation
//! honesty). ADMISSION (compile-time only -- the slow, ~15-40s
//! `assert_answers_match` pass, per M-B's own report; never run at server
//! startup) and the LOAD-TIME PERFORMANCE LAW (controller decision 6:
//! start-to-listening <= 3s release; measured here as "read bytes off
//! disk -> decode -> to_graph -> build_indexes -> add_justified_by," the
//! same sequence `atlas-server`'s own real startup performs when loading
//! from an artifact file instead of building from raw) are BOTH proven
//! here, over the real committed sources, as committed failing-test laws.

use std::path::Path;
use std::time::Instant;

use atlas_graph::GraphService;

/// The committed law (controller decision 6): serialized-artifact LOAD
/// start-to-listening <= 3s release, on this machine. Exceeding this is a
/// red build, not a hope.
const LOAD_CEILING: std::time::Duration = std::time::Duration::from_secs(3);

// M-C2 DELETION EVENT: `AtlasData::load`'s own five retiring-file reads
// return empty now -- `atlas_etl::compile::compile` is this crate's own
// real-data source from here on (already a normal dependency). Its own
// `AtlasData.eras` is already populated (compile() parses eras.toml
// internally), so `real_eras` below just reads it off, rather than a
// second, independent re-parse. Cached so both real-graph tests in this
// file share one compile.
fn real_atlas_data() -> atlas_core::data::AtlasData {
    static CACHED: std::sync::OnceLock<atlas_core::data::AtlasData> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
                .data
        })
        .clone()
}

fn real_eras() -> Vec<atlas_core::data::Era> {
    real_atlas_data().eras.clone()
}

#[test]
fn serialized_artifact_is_admitted_and_loads_under_the_committed_ceiling() {
    let raw_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    let atlas = real_atlas_data();
    let eras = real_eras();

    // Implementation #1: build from raw, as `GraphService::build` does at
    // real startup (before this batch's artifact path exists).
    let built = atlas_graph::build::build_graph_from_sources_with_eras(&kjv_json, &xrefs_tsv, &atlas, &eras).expect("the real committed sources must build");
    let (model_graph, stats, event_world_stats, chrono) = built;
    let chronology = atlas_graph::Chronology::from_derivation(chrono);
    println!(
        "M-C ARTIFACT SOURCE GRAPH: {} text units, {} cites edges, {} events ({} dated), {} places, {} narratives, {} anchors",
        stats.kjv_verses, stats.cites_rows, event_world_stats.events, event_world_stats.dated_events, event_world_stats.places, event_world_stats.narratives, event_world_stats.anchors,
    );

    // Dump + encode + write to a temp file -- NOT timed (this is the
    // compile step's own one-time cost, not the server's startup cost).
    let dump = atlas_graph::artifact::dump(&model_graph, &chronology, &stats, &event_world_stats).expect("dump must succeed over the real full graph");
    let tmp = std::env::temp_dir().join(format!("atlas-graph-artifact-conformance-{}.bin", std::process::id()));
    atlas_graph::artifact::write_file(&dump, &tmp).expect("writing the artifact file must succeed");
    let file_len = std::fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);

    // THE TIMED LOAD (controller decision 6): read bytes -> decode ->
    // to_graph -> build_indexes -> add_justified_by -- exactly what
    // `atlas-server`'s own startup performs when loading from an artifact.
    let load_start = Instant::now();
    let loaded_dump = atlas_graph::artifact::read_file(&tmp).expect("reading the artifact file must succeed");
    let (mut loaded_graph, _loaded_stats, _loaded_ews, _loaded_chronology) = atlas_graph::artifact::to_service_parts(loaded_dump).expect("to_service_parts must succeed");
    loaded_graph.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut loaded_graph);
    let load_elapsed = load_start.elapsed();

    let _ = std::fs::remove_file(&tmp);

    println!("M-C ARTIFACT LOAD: {file_len} bytes, load time {load_elapsed:?} (ceiling {LOAD_CEILING:?})");
    assert!(
        load_elapsed <= LOAD_CEILING,
        "serialized-artifact load took {load_elapsed:?}, exceeding the committed {LOAD_CEILING:?} ceiling (controller decision 6) -- this is a red build, not a hope"
    );

    // ADMISSION (design §9a: "implementation #2 passes the same law as
    // #1"), over the FULL graph, not timed against the load ceiling --
    // this is the compile-time-only gate the real compile-step binary
    // (bin/compile_graph.rs) also runs before it will write an artifact
    // file at all.
    let admit_start = Instant::now();
    atlas_graph_types::store::assert_answers_match(&loaded_graph, &model_graph);
    println!("M-C ARTIFACT ADMISSION (assert_answers_match, full graph): {:?}", admit_start.elapsed());
}

/// A GraphService can be constructed directly from a written artifact file
/// -- the real server-facing path (`GraphService::from_artifact`), not
/// just the lower-level `artifact::to_graph`.
#[test]
fn graph_service_from_artifact_serves_the_same_answers_as_from_sources() {
    let raw_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).unwrap();
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt")).unwrap();
    let atlas = real_atlas_data();
    let eras = real_eras();

    let from_sources = GraphService::from_sources_with_eras(&kjv_json, &xrefs_tsv, &atlas, &eras).unwrap();

    let (model_graph, stats, event_world_stats, chrono) = atlas_graph::build::build_graph_from_sources_with_eras(&kjv_json, &xrefs_tsv, &atlas, &eras).unwrap();
    let chronology = atlas_graph::Chronology::from_derivation(chrono);
    let dump = atlas_graph::artifact::dump(&model_graph, &chronology, &stats, &event_world_stats).unwrap();
    let tmp = std::env::temp_dir().join(format!("atlas-graph-service-from-artifact-{}.bin", std::process::id()));
    atlas_graph::artifact::write_file(&dump, &tmp).unwrap();

    let from_artifact = GraphService::from_artifact(&tmp).expect("GraphService::from_artifact must succeed");
    let _ = std::fs::remove_file(&tmp);

    assert_eq!(from_sources.version(), from_artifact.version(), "the same content must produce the same version root regardless of which implementation built it");

    let jn316 = atlas_graph::kjv_adapter::verse_node_id(42, 3, 16);
    let snap_a = from_sources.snapshot();
    let snap_b = from_artifact.snapshot();
    assert_eq!(atlas_graph::window::render(&snap_a, &jn316), atlas_graph::window::render(&snap_b, &jn316));
    assert_eq!(atlas_graph::window::render(&snap_a, &jn316).as_deref(), Some("For God so loved the world, that he gave his only begotten Son, that whosoever believeth in him should not perish, but have everlasting life."));
}
