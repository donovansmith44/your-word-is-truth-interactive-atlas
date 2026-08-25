//! EDGE1A-m2 (batch-polish1-brief.md item 3, "count guard"): pins the
//! `justified-by` GRAND TOTAL over the real committed data. EDGE-1a's own
//! `event_world::add_justified_by` -- and the generalization its JB-1
//! rider landed (`graph.fulfills`/`graph.typology`/`graph.named_after`,
//! alongside the pre-existing `graph.dated_by` loop) -- is already
//! MECHANISM-tested (`event_world.rs`'s own
//! `justified_by_wires_fulfills_typology_and_named_after_rows_own_grounds`
//! unit test, hand-built graph, proves each relation's own row wires a
//! `justified-by` edge to the right ground) and, separately, HTTP-tested
//! over one real row per new relation
//! (`atlas-server/tests/graph_api.rs`'s own justified-by/justifies
//! coverage). Neither pins the AGGREGATE count `add_justified_by` returns
//! over the real committed graph -- batch-edge1a-report.md's own
//! "+59 new justified-by pairs on top of DatedBy's pre-existing count" is
//! a delta, not an asserted total. This file adds that missing total,
//! read directly off a real build (read-back discipline: the exact number
//! below came from a real `cargo test` run over the real committed data,
//! not computed from report arithmetic).
//!
//! Real committed `data/raw` + `data/curated`, the SAME `atlas_etl::
//! compile::compile`-backed pattern `description_real_data.rs`/
//! `narrative_real_data.rs`/`artifact_conformance.rs`/`version_root_
//! regression.rs` already establish in this crate (duplicated helper, per
//! `exports_laws.rs`'s own documented convention -- every real-data
//! integration test file in this crate keeps its own copy). This never
//! touches `data/compiled/` -- `atlas_etl::compile::compile` returns an
//! in-memory `AtlasData`, and `pipeline::run_pipeline` below builds an
//! in-memory `Graph` only; nothing here writes an artifact or moves the
//! version root.

use std::path::Path;

use atlas_core::data::{AtlasData, Canon};
use atlas_graph::pipeline::{self, BuildCtx};

fn real_atlas_data() -> AtlasData {
    static CACHED: std::sync::OnceLock<AtlasData> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
                .data
        })
        .clone()
}

/// Builds the full real graph via the pipeline directly (the same "why not
/// `build::build_graph_from_sources`" reasoning `description_real_data.rs`
/// already documents: only the direct `BuildCtx` exposes
/// `ctx.justified_by_count`, `pipeline::IndexPass`'s own captured return
/// value of `event_world::add_justified_by`, alongside the built
/// `ctx.graph`).
fn build_real_ctx<'a>(kjv_json: &'a str, xrefs_tsv: &'a str, atlas: &'a AtlasData, canon: &'a Canon, verses: &'a std::collections::HashMap<String, String>) -> BuildCtx<'a> {
    let mut ctx = BuildCtx::new(canon, verses, Some(kjv_json), xrefs_tsv, atlas);
    pipeline::run_pipeline(&mut ctx, &pipeline::pipeline()).expect("the real committed sources must build cleanly through the full pipeline");
    ctx
}

/// The count guard itself. `add_justified_by` returns the GRAND TOTAL
/// `pairs.len()` across all four relations (`dated_by` pre-existing +
/// EDGE-1a's own `fulfills`/`typology`/`named_after` rider), one entry per
/// (row, ground) pair -- exact equality, not a floor, matching this
/// workspace's own convention for a fixed, disclosed real-data total
/// (`peoples_real_data.rs`'s own `people_group_total` law is the same
/// shape): a real future data/curated change to any of the four
/// justification-carrying relations is real, deliberate content, updated
/// here in that same commit, not silently absorbed.
#[test]
fn justified_by_grand_total_over_the_real_compiled_data_is_pinned() {
    let raw_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    let atlas = real_atlas_data();
    let (canon, verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");

    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

    println!("EDGE-1a/JB-1 justified-by grand total (real compiled data): {}", ctx.justified_by_count);
    // Read back from a real run (read-back discipline), not computed from
    // report arithmetic: 76 = DatedBy's own 17 pre-existing pairs +
    // EDGE-1a's own JB-1 rider's disclosed 59 (24 fulfills x1 ground + 16
    // typology x1 ground + 18 named_after x1-2 grounds), reconciling
    // exactly with batch-edge1a-report.md's own delta.
    assert_eq!(
        ctx.justified_by_count, 76,
        "justified-by grand total regressed or grew over the real committed data -- if this is a genuine data/curated change (a new DatedBy/fulfills/typology/named_after row, or a new ground on an existing row), update this pin in the SAME commit"
    );
}
