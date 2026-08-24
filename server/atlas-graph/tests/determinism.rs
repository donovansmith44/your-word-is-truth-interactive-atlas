//! M-C2 requirement 3: "the pre-existing serialization non-determinism
//! (sorted-maps class) dies with the old writers OR is fixed where any
//! writer survives -- the serialized graph artifact must be byte-
//! deterministic (same sources -> same bytes -> same root; add the law
//! if not already asserted)."
//!
//! THE OLD NON-DETERMINISM (M-C report's own Concern 2, confirmed
//! genuinely pre-existing by the M-C review): `verses-kjv.json`/
//! `cross-refs.json` were `HashMap`-keyed JSON writes, whose per-process
//! randomized iteration order produced a different (content-identical)
//! byte layout on every `atlas-etl` run. Both files -- and the writers
//! that produced them -- are deleted this batch (batch-mc2-report.md's
//! own deletion inventory); the class of bug dies with them, not merely
//! by coincidence: every row table the SERIALIZED GRAPH ARTIFACT dumps
//! (`artifact.rs::dump`) is sourced from `graph-types::graph::Graph`'s own
//! `Vec`/`BTreeMap` fields (never a `HashMap`), themselves built by
//! iterating deterministically-ordered sources (curated TOML file lists,
//! sorted-by-filename; `atlas_core::data::AtlasData`'s own `Vec` fields).
//!
//! This test is the LAW, not the argument: builds the full real graph
//! artifact TWICE, independently, from the identical real committed
//! sources, and asserts the encoded BYTES are identical -- stronger than
//! comparing version roots (a content hash over node id+payload alone;
//! this compares the entire dumped row-table byte stream, including
//! order) and stronger than `assert_answers_match` (which proves semantic
//! equivalence, not byte identity). A future change that reintroduces a
//! `HashMap`-ordered iteration anywhere in the row-table build path fails
//! this test loud, not silently, regardless of whether it happens to
//! "look" deterministic on any one run.
//!
//! Real-data cost, disclosed: builds the ~344k-cites-edge graph TWICE, now
//! including all six of CORP-1a's own brain-fuel editions each time (two
//! independent `atlas_etl::compile::compile` + `atlas_etl::brainfuel::
//! read_all` + full pipeline builds + dump + encode) -- measured directly,
//! standalone, twice, on this machine: 89.70s and 89.79s (CORP-1a fix
//! round 1, F1; previously ~5-10s each / ~10-20s total, before this
//! batch's six editions joined every build this test performs). A THIRD
//! measurement, inside a full `cargo test --workspace` run (this crate's
//! own real per-CI-commit condition, other test binaries contending for
//! the same CPU cores): 92.60s -- confirms the "a slower CI machine could
//! plausibly cross 90s" concern below is not theoretical; it already
//! crossed the notional mark once, under ordinary shared-machine load, on
//! THIS machine. Still run once, not per-CI-commit-multiplied, same
//! tradeoff this crate's other real-data tests already make -- but
//! disclosed honestly as a real, order-of-magnitude cost increase, not
//! silently absorbed: this is now this crate's single most expensive
//! test, and sits close enough to a
//! notional 90s budget that a slower CI machine could plausibly cross it
//! (a controller call, not silently re-budgeted here).

use std::path::Path;

fn real_sources() -> (String, String, atlas_core::data::AtlasData, Vec<atlas_core::data::Era>, atlas_etl::brainfuel::BrainFuelCorpus) {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let raw_dir = data_dir.join("raw");
    let curated_dir = data_dir.join("curated");

    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    // M-C2 DELETION EVENT: `AtlasData::load`'s own five retiring-file
    // reads return empty now -- `atlas_etl::compile::compile` is this
    // test's real-data source (a real raw+curated compile is exactly
    // the right ground truth for a determinism law anyway: two
    // independent CALLS to `real_sources` -- via `build_and_encode`,
    // once each, below -- prove the WHOLE pipeline, ETL compile
    // included, is deterministic, not just the graph-build half).
    let atlas = atlas_etl::compile::compile(&raw_dir, &curated_dir)
        .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
        .data;
    let eras = atlas.eras.clone();
    // CORP-1a fix round 1 (F1): real vendored brain-fuel data joins this
    // determinism proof too -- otherwise this test would only ever prove
    // determinism for a graph `atlas-graph-compile` no longer produces
    // (its own real sequence merges six brain-fuel editions; see
    // `bins/compile_graph.rs`). Read independently on each of the two
    // `real_sources()` calls below, same "prove the WHOLE pipeline
    // deterministic" reasoning as the KJV/curated compile above.
    let brainfuel = atlas_etl::brainfuel::read_all(&raw_dir.join("brain-fuel-bible"))
        .expect("data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step first");

    (kjv_json, xrefs_tsv, atlas, eras, brainfuel)
}

/// Builds the graph from scratch and dumps+encodes it to bytes -- the
/// exact sequence `atlas-graph-compile` performs before writing `graph.bin`.
fn build_and_encode(kjv_json: &str, xrefs_tsv: &str, atlas: &atlas_core::data::AtlasData, eras: &[atlas_core::data::Era], brainfuel: &atlas_etl::brainfuel::BrainFuelCorpus) -> Vec<u8> {
    let (graph, stats, event_world_stats, chrono) =
        atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel(kjv_json, xrefs_tsv, atlas, eras, Some(brainfuel)).expect("the real committed sources must build");
    let chronology = atlas_graph::Chronology::from_derivation(chrono);
    let dump = atlas_graph::artifact::dump(&graph, &chronology, &stats, &event_world_stats).expect("dump must succeed over the real full graph");
    atlas_graph::artifact::encode(&dump).expect("encode must succeed")
}

#[test]
fn serialized_artifact_bytes_are_deterministic_across_independent_builds() {
    // `real_sources()` called TWICE, independently -- not once with the
    // result reused -- so this proves the WHOLE pipeline deterministic,
    // `atlas_etl::compile::compile` (raw+curated parse/merge/validate,
    // including any `HashMap`-keyed intermediate step, e.g.
    // `catechism_map::merge_questions_into_parts`) included, not just
    // the graph-build half over an already-fixed `AtlasData`.
    let (kjv_json_a, xrefs_tsv_a, atlas_a, eras_a, brainfuel_a) = real_sources();
    let (kjv_json_b, xrefs_tsv_b, atlas_b, eras_b, brainfuel_b) = real_sources();
    assert_eq!(kjv_json_a, kjv_json_b, "raw KJV source bytes must be read identically (sanity check on the test's own inputs)");
    assert_eq!(xrefs_tsv_a, xrefs_tsv_b, "raw xrefs source bytes must be read identically (sanity check on the test's own inputs)");

    let bytes_a = build_and_encode(&kjv_json_a, &xrefs_tsv_a, &atlas_a, &eras_a, &brainfuel_a);
    let bytes_b = build_and_encode(&kjv_json_b, &xrefs_tsv_b, &atlas_b, &eras_b, &brainfuel_b);

    assert_eq!(bytes_a.len(), bytes_b.len(), "two independent builds from identical sources produced different byte lengths -- a real non-determinism, not a rounding artifact");
    assert_eq!(
        bytes_a, bytes_b,
        "two independent builds from identical sources produced different bytes -- the serialized graph artifact must be byte-deterministic (same sources -> same bytes -> same root); find the HashMap-ordered (or otherwise non-deterministic) iteration this build introduced and fix it at the source, the same discipline event_world::populate_dated_by/artifact.rs's own chrono_order already establish"
    );
}
