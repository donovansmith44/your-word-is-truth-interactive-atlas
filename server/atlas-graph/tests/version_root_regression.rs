mod common;

use std::path::Path;

use common::{OptionalCorpora, RawSources};

use atlas_graph::sqlite::manifest::{raw_manifest_path, raw_provenance, read_manifest, recorded_raw_root, RawProvenance};
use atlas_graph::sqlite::source::SectionLayout;
use atlas_graph_types::store::GraphSnapshot as _;

#[test]
fn version_root_matches_the_captured_pre_pipeline_baseline() {
    let svc = RawSources::read(OptionalCorpora { kretzmann: true, red_letter: false }).build_service(&[]);
    let hex = atlas_graph::version_hex(svc.snapshot().version());

    assert_eq!(
        hex,
        EXPECTED_VERSION_HEX,
        "graph version root diverged from the captured baseline -- if this build genuinely changed graph \
         content on purpose, update EXPECTED_VERSION_HEX in this same commit with a one-line reason; if not, \
         this is exactly the regression this test exists to catch. {}",
        raw_provenance_of_the_committed_artifact(&common::compiled_dir())
    );
}

fn raw_provenance_of_the_committed_artifact(compiled: &Path) -> RawProvenance {
    let layout = SectionLayout::under(compiled);
    let compiled_from = read_manifest(&layout.manifest_path()).expect("data/compiled/manifest.toml must verify").raw_root;
    let recorded = recorded_raw_root(&raw_manifest_path(&layout)).expect("data/raw/MANIFEST.toml must be readable when present");
    raw_provenance(compiled_from, recorded)
}

const EXPECTED_VERSION_HEX: &str = "336a866118a1e341a277965aeb115e2c";
