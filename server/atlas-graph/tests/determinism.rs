use std::path::Path;

fn real_sources() -> (String, String, atlas_core::data::AtlasData, Vec<atlas_core::data::Era>, atlas_etl::brainfuel::BrainFuelCorpus) {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let raw_dir = data_dir.join("raw");
    let curated_dir = data_dir.join("curated");

    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    let atlas = atlas_etl::compile::compile(&raw_dir, &curated_dir)
        .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
        .data;
    let eras = atlas.eras.clone();
    let brainfuel = atlas_etl::brainfuel::read_all(&raw_dir.join("brain-fuel-bible"))
        .expect("data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step first");

    (kjv_json, xrefs_tsv, atlas, eras, brainfuel)
}

fn build_and_dump(kjv_json: &str, xrefs_tsv: &str, atlas: &atlas_core::data::AtlasData, eras: &[atlas_core::data::Era], brainfuel: &atlas_etl::brainfuel::BrainFuelCorpus) -> (String, Vec<Vec<u8>>) {
    let (mut graph, _stats, _event_world_stats, chrono) =
        atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel(kjv_json, xrefs_tsv, atlas, eras, Some(brainfuel)).expect("the real committed sources must build");
    graph.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut graph);
    let sources_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled/sources.json");
    let sources: atlas_core::sources::SourcesDocument = serde_json::from_str(&std::fs::read_to_string(sources_path).expect("sources.json")).expect("sources.json parses");
    let extras = atlas_graph::sqlite::extras::compute(&graph, &chrono, &std::collections::HashMap::new(), atlas, &sources, &[]).expect("the fold");
    extras.attach(&mut graph);
    let root = atlas_graph_types::sections::version_root(&graph).hex();
    let dumps = atlas_graph_types::sections::Section::SHIPPED.iter().map(|s| atlas_graph_types::sections::logical_dump_section(&graph, *s)).collect();
    (root, dumps)
}

#[test]
fn the_shipped_dumps_and_root_are_deterministic_across_independent_builds() {
    let (kjv_json_a, xrefs_tsv_a, atlas_a, eras_a, brainfuel_a) = real_sources();
    let (kjv_json_b, xrefs_tsv_b, atlas_b, eras_b, brainfuel_b) = real_sources();
    assert_eq!(kjv_json_a, kjv_json_b, "raw KJV source bytes must be read identically (sanity check on the test's own inputs)");
    assert_eq!(xrefs_tsv_a, xrefs_tsv_b, "raw xrefs source bytes must be read identically (sanity check on the test's own inputs)");

    let (root_a, dumps_a) = build_and_dump(&kjv_json_a, &xrefs_tsv_a, &atlas_a, &eras_a, &brainfuel_a);
    let (root_b, dumps_b) = build_and_dump(&kjv_json_b, &xrefs_tsv_b, &atlas_b, &eras_b, &brainfuel_b);

    for (i, (a, b)) in dumps_a.iter().zip(&dumps_b).enumerate() {
        assert_eq!(a.len(), b.len(), "section {i}: two independent builds from identical sources produced different dump lengths -- a real non-determinism, not a rounding artifact");
        assert_eq!(a, b, "section {i}: two independent builds from identical sources produced different logical dumps -- the shipped sections must be byte-deterministic (same sources -> same dumps -> same root); find the HashMap-ordered (or otherwise non-deterministic) iteration this build introduced and fix it at the source, the same discipline event_world::populate_dated_by already establishes");
    }
    assert_eq!(root_a, root_b, "same dumps, same root");
}
