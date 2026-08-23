//! Batch M-C, controller decision 3 (the compiler pipeline contract):
//! "the existing M-A/M-B build logic restructures INTO passes --
//! behavior-identical, proven by the graph version root being UNCHANGED
//! for identical sources (content addressing is your regression harness)."
//!
//! `GraphVersion` (`atlas_graph_types::store::GraphVersion`) is a content
//! hash over every node's id+payload -- two builds of "the same" graph
//! content are equal iff their roots are equal (design doc §9b). This test
//! pins the version root over the REAL committed sources (`data/raw/kjv.json`
//! + `data/raw/xrefs/cross_references.txt` + the real compiled `AtlasData`)
//! to an EXACT, hardcoded hex value, captured BEFORE the pipeline
//! restructuring (commit history: this file was added and its expected
//! value captured against the pre-pipeline `build_graph_from_sources`/
//! `event_world::populate`/`Graph::build_indexes`/`add_justified_by` call
//! chain, then the pipeline commit landed on top with this test unchanged
//! and still green) -- so a divergence here means the restructuring changed
//! what gets built, not just how it's built, which is exactly the failure
//! mode "passes as data" is supposed to make impossible to introduce
//! silently.
//!
//! NOTE: this baseline value moves whenever a LATER M-C commit adds real
//! data to the graph (new adapters -- places/eras/polities/catechism/
//! mentions all change node/edge content, which is supposed to change the
//! version root; that is the root doing its job). Each such commit updates
//! the constant below directly in the same commit that changes what the
//! graph contains, with a one-line note of why, so this file stays a live
//! regression harness rather than a one-time snapshot.

use std::path::Path;

use atlas_graph::GraphService;
use atlas_graph_types::store::GraphSnapshot as _;

fn real_atlas_data() -> atlas_core::data::AtlasData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled");
    atlas_core::data::AtlasData::load(&dir).expect("data/compiled must exist (committed real data)").finish()
}

#[test]
fn version_root_matches_the_captured_pre_pipeline_baseline() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv =
        std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    let atlas = real_atlas_data();

    let svc = GraphService::from_sources(&kjv_json, &xrefs_tsv, &atlas).expect("the real committed sources must build");
    let hex = atlas_graph::version_hex(svc.snapshot().version());

    // Captured once, before the M-C pipeline restructuring (controller
    // decision 3's own regression harness) -- update alongside a commit
    // that deliberately changes graph content, never silently.
    assert_eq!(
        hex, EXPECTED_VERSION_HEX,
        "graph version root diverged from the captured baseline -- if this build genuinely changed graph \
         content on purpose, update EXPECTED_VERSION_HEX in this same commit with a one-line reason; if not, \
         this is exactly the regression this test exists to catch"
    );
}

// Captured 2026-08-22, pre-pipeline-restructuring, base commit ddb4b73
// (EdgeMeta fix + EventWorld narrative retirement -- no adapter/graph
// content change), over the real committed data/raw + data/compiled:
// "4ad17fd0f68660f7".
//
// MOVED (deliberately -- controller decision 2, new adapters): the place/
// polity/catechism adapters now build real payload (Place gains lat/lon +
// KJV aliases, was `canonical` only; Polity/CatechismItem nodes newly
// exist) and new rows (named/mentions/catechism-link) over the SAME real
// `data/compiled/*.json` this test's own `real_atlas_data()` loads --
// this test itself calls the plain (eras-less) `GraphService::from_sources`,
// so eras stay absent from THIS particular root; the place/polity/
// catechism content change alone is what moved it: "ea2276b34f44d29f".
//
// MOVED AGAIN (deliberately -- map migration, controller decision 7):
// `PolityEraPayload.transition`/`.fall` widened from a collapsed display
// string to the fully structured `PolityDeltaPayload { event, verses,
// ref_note }` (graph-types/src/node.rs), so `/api/polities`'s own wire
// response (`PolityDeltaOut`) reconstructs losslessly from the graph
// payload alone instead of a lossy formatted summary -- real content
// change, not a bug.
const EXPECTED_VERSION_HEX: &str = "2003993f12d099ab";
