//! ENT-1a ("Easton's descriptions: the data half" -- batch-ent1a-brief.md
//! requirement 6): the real-data spot check + honest fill-rate report,
//! over the REAL committed `data/raw`/`data/curated` sources (never a
//! synthetic fixture) -- same `atlas_etl::compile::compile`-backed pattern
//! `narrative_real_data.rs`/`artifact_conformance.rs`/
//! `version_root_regression.rs` already establish in this crate.
//!
//! Runs the pipeline directly (`pipeline::BuildCtx` + `pipeline::
//! run_pipeline`, both re-exported/`pub mod`), not the higher-level
//! `build::build_graph_from_sources` wrapper, for exactly one reason: only
//! the direct `BuildCtx` gives this test access to `ctx.description_stats`
//! (`description_adapter::fill_descriptions`'s own captured return value)
//! alongside the built `ctx.graph` itself, so the per-tier breakdown and
//! the spot-checked nodes come from the SAME single build, not two.

use std::collections::HashMap;
use std::path::Path;

use atlas_core::data::{AtlasData, Canon};
use atlas_graph::pipeline::{self, BuildCtx};
use atlas_graph_types::id::{PersonId, PlaceId};
use atlas_graph_types::node::NodePayload;

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

/// Builds the full real graph via the pipeline directly (module doc
/// comment above has the "why not build::build_graph_from_sources"
/// reasoning), returning the built context so callers can read both
/// `ctx.graph` (for spot checks) and `ctx.description_stats` (for the
/// fill-rate report) from the ONE build.
fn build_real_ctx<'a>(kjv_json: &'a str, xrefs_tsv: &'a str, atlas: &'a AtlasData, canon: &'a Canon, verses: &'a HashMap<String, String>) -> BuildCtx<'a> {
    let mut ctx = BuildCtx::new(canon, verses, Some(kjv_json), xrefs_tsv, atlas);
    pipeline::run_pipeline(&mut ctx, &pipeline::pipeline()).expect("the real committed sources must build cleanly through the full pipeline");
    ctx
}

#[test]
fn hebron_and_moses_carry_non_empty_descriptions_over_the_real_compiled_data() {
    let raw_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    let atlas = real_atlas_data();
    let (canon, verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");

    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

    let hebron_id = PlaceId::new("hebron").erase();
    let hebron = ctx.graph.nodes.get(&hebron_id).expect("a compiled Place node with id 'hebron' must exist over the real geo data");
    let hebron_desc = match &hebron.payload {
        NodePayload::Place { canonical, description, .. } => {
            assert_eq!(canonical, "Hebron");
            description.clone()
        }
        other => panic!("expected NodePayload::Place for 'hebron', got {other:?}"),
    };
    assert!(hebron_desc.as_deref().is_some_and(|s| !s.trim().is_empty()), "Hebron must carry a non-empty description over the real compiled data, got {hebron_desc:?}");
    println!("HEBRON description ({} chars): {}", hebron_desc.as_ref().unwrap().len(), &hebron_desc.as_ref().unwrap()[..hebron_desc.as_ref().unwrap().len().min(120)]);

    let moses_id = PersonId::new("moses_2108").erase();
    let moses = ctx.graph.nodes.get(&moses_id).expect("a compiled Person node with id 'moses_2108' must exist over the real Theographic people data");
    let moses_desc = match &moses.payload {
        NodePayload::Person { label, description, .. } => {
            assert_eq!(label, "Moses");
            description.clone()
        }
        other => panic!("expected NodePayload::Person for 'moses_1', got {other:?}"),
    };
    assert!(moses_desc.as_deref().is_some_and(|s| !s.trim().is_empty()), "Moses must carry a non-empty description over the real compiled data, got {moses_desc:?}");
    println!("MOSES description ({} chars): {}", moses_desc.as_ref().unwrap().len(), &moses_desc.as_ref().unwrap()[..moses_desc.as_ref().unwrap().len().min(120)]);
}

#[test]
fn description_fill_rates_over_the_real_compiled_data_are_reported_honestly() {
    let raw_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    let atlas = real_atlas_data();
    let (canon, verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");

    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);
    let s = &ctx.description_stats;

    println!("ENT-1a DESCRIPTION FILL RATES (real compiled data):");
    println!(
        "  persons: {}/{} filled ({:.1}%) -- tier a {}, tier b {}, tier c {}",
        s.person_filled(),
        s.person_total,
        100.0 * s.person_filled() as f64 / s.person_total.max(1) as f64,
        s.person_tier_a,
        s.person_tier_b,
        s.person_tier_c
    );
    println!(
        "  places:  {}/{} filled ({:.1}%) -- tier b {}, tier c {}",
        s.place_filled(),
        s.place_total,
        100.0 * s.place_filled() as f64 / s.place_total.max(1) as f64,
        s.place_tier_b,
        s.place_tier_c
    );
    println!(
        "  people-groups: {}/{} filled ({:.1}%) -- tier c {}",
        s.people_group_filled(),
        s.people_group_total,
        100.0 * s.people_group_filled() as f64 / s.people_group_total.max(1) as f64,
        s.people_group_tier_c
    );

    // Sanity floors, not brittle exact-equality (real Theographic/Easton's
    // data can grow -- the SAME "n >= floor" discipline
    // `narrative_real_data.rs`'s own `global_timeline_real_compiled_data_
    // has_well_over_450_dated_events` already uses): a real regression
    // (the pass silently stops running, or a future refactor breaks the
    // matcher) would collapse these to zero; a genuine data-refresh
    // improving coverage should never make this test red.
    assert_eq!(s.person_total, atlas.people.len(), "every compiled person must be counted exactly once");
    assert!(s.person_filled() >= 2000, "person fill count regressed below a sane floor: {} of {}", s.person_filled(), s.person_total);
    assert!(s.place_filled() >= 600, "place fill count regressed below a sane floor: {} of {}", s.place_filled(), s.place_total);
    // No PeopleGroup nodes exist yet as of ENT-1a (PG-1a, the group/eponymy
    // data batch, has not landed) -- a lawful, honest zero, not a gap in
    // this pass: `description_adapter`'s own unit tests
    // (`people_group_fills_from_tier_c_only`) already prove the arm works
    // once real PeopleGroup nodes exist.
    assert_eq!(s.people_group_total, 0, "no PeopleGroup nodes exist yet (PG-1a not landed) -- update this once PG-1a ships real group nodes");
}
