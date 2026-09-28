use std::path::Path;

use atlas_graph::GraphService;
use atlas_graph_types::store::GraphSnapshot as _;

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

#[test]
fn version_root_matches_the_captured_pre_pipeline_baseline() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv =
        std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    let atlas = real_atlas_data();
    let brainfuel = atlas_etl::brainfuel::read_all(&dir.join("brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step first");
    let concord_corpus = atlas_etl::concord::read_all(&dir.join("concord")).expect("data/raw/concord must exist -- run data/fetch-raw.ps1 first");
    let sc_overlap_text = std::fs::read_to_string(dir.parent().unwrap().join("curated/concord-sc-overlap.toml")).expect("data/curated/concord-sc-overlap.toml must exist");
    let sc_overlap = atlas_etl::concord::parse_sc_overlap(&sc_overlap_text).expect("concord-sc-overlap.toml must parse");
    let concord_bundle = atlas_graph::concord_adapter::ConcordBundle { corpus: concord_corpus, sc_overlap };
    let (_, kjv_verses_for_kretzmann) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
    let kretzmann_corpus =
        atlas_etl::kretzmann::read_all(&dir.join("kretzmann"), &kjv_verses_for_kretzmann).expect("data/raw/kretzmann must exist -- run data/fetch-raw.ps1 first");

    let svc = GraphService::from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann(&kjv_json, &xrefs_tsv, &atlas, &[], Some(&brainfuel), Some(&concord_bundle), Some(&kretzmann_corpus))
        .expect("the real committed sources must build");
    let hex = atlas_graph::version_hex(svc.snapshot().version());

    assert_eq!(
        hex, EXPECTED_VERSION_HEX,
        "graph version root diverged from the captured baseline -- if this build genuinely changed graph \
         content on purpose, update EXPECTED_VERSION_HEX in this same commit with a one-line reason; if not, \
         this is exactly the regression this test exists to catch"
    );
}

const EXPECTED_VERSION_HEX: &str = "11c50986e095373f3e1bd6ac2218ed28";
