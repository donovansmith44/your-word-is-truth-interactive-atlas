//! M-C2 requirement 1: the decisive-title law re-homed as a graph query
//! (`atlas_graph::heading::build_heading_index`) -- red-then-green over
//! the batch's own two NAMED real-data cases (batch-mc2-brief.md), each
//! already independently pinned by an atlas-core unit test
//! (`AtlasData::heading_collision_tests`, `server/atlas-core/src/data.rs`)
//! and/or a live Playwright regression (`tests/ux/w3-passages.spec.ts`) --
//! this file proves the SAME two outcomes hold when computed straight off
//! the graph's own `Event` nodes, never `AtlasData`.
//!
//! JHN.12.1 (CONTRACT.md's own DECISIVE-CONTAINER MODEL example): `jm_bethany`
//! (a bare `jesus-ministry` narrative leg, heading-worthy only via the
//! "existing-title freebie" rule -- layer 0) and `pw_bethany` (a REAL
//! curated container, this batch's own flagship 3-witness passion-week leg
//! -- layer 1) both anchor JHN.12.1; the LAYER tier must decide the
//! collision outright, `pw_bethany` winning.
//!
//! PSA.53(.1) (`data/curated/passages/psalms.toml`'s own header note, Batch
//! W3's "Psalm 14 <-> Psalm 53" parallel): `psa_014` carries TWO witnesses
//! (self PSA.14 + parallel PSA.53), so its own heading-anchor set spans
//! BOTH psalms' own opening verses under ONE shared, both-true title --
//! proves multi-witness anchoring (one event, N heading anchors) survives
//! the re-homing, not just single-anchor collision resolution.

use std::path::Path;

use atlas_core::data::AtlasData;
use atlas_graph::build::build_graph_from_sources_with_eras;
use atlas_graph::heading::build_heading_index;

fn real_graph() -> atlas_graph_types::graph::Graph {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let raw_dir = data_dir.join("raw");
    let compiled_dir = data_dir.join("compiled");
    let curated_dir = data_dir.join("curated");

    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    let atlas = AtlasData::load(&compiled_dir).expect("data/compiled must exist (committed real data)").finish();
    let eras_toml = std::fs::read_to_string(curated_dir.join("eras.toml")).expect("data/curated/eras.toml must exist");
    let eras = atlas_etl::curated::parse_eras(&eras_toml).expect("eras.toml must parse");

    let (graph, ..) = build_graph_from_sources_with_eras(&kjv_json, &xrefs_tsv, &atlas, &eras).expect("the real committed sources must build");
    graph
}

#[test]
fn named_case_jhn_12_1_the_real_container_beats_the_freebie() {
    let graph = real_graph();
    let index = build_heading_index(&graph);

    let heading = index.get("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
    assert_eq!(heading.event_id, "pw_bethany", "the real, witness-bearing container must win over the bare narrative-leg freebie (jm_bethany)");
    assert_ne!(heading.event_id, "jm_bethany", "the freebie must not win the collision");
}

#[test]
fn named_case_psa_53_1_the_shared_fool_incipit_container_anchors_both_psalms() {
    let graph = real_graph();
    let index = build_heading_index(&graph);

    let psa53 = index.get("PSA.53.1").expect("PSA.53.1 must anchor a heading -- psa_014's own second (parallel) witness");
    assert_eq!(psa53.event_id, "psa_014");
    assert_eq!(psa53.title, "The fool hath said in his heart, There is no God.");
    assert_eq!(psa53.kind, "general");

    // The SAME container also anchors its own psalm's own verse 1 -- one
    // event, two heading anchors (multi-witness anchoring, not a
    // single-anchor coincidence).
    let psa14 = index.get("PSA.14.1").expect("PSA.14.1 must anchor a heading -- psa_014's own first (self) witness");
    assert_eq!(psa14.event_id, "psa_014");
    assert_eq!(psa14.title, psa53.title, "one shared container, one shared title, true at both anchors (fix round 1, batch-w3-review.md Important-1)");
}
