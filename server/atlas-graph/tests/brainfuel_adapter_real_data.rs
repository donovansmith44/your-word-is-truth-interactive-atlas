//! CORP-1a fix round 1 (F3): pins `brainfuel_adapter::normalize`'s own
//! `BrainFuelAdapterStats` against the real vendored data -- the counts
//! this batch's own report already verified three independent ways
//! (Python sweep, `atlas_etl::brainfuel` unit test, `atlas-graph-compile`'s
//! own console output), now asserted directly against the GRAPH adapter's
//! own return value too, not just the parser's.
//!
//! Deliberately does NOT drive the full six-stage pipeline (`pipeline::
//! pipeline()`/`atlas_etl::compile::compile`) -- `brainfuel_adapter::
//! normalize` only ever reads `ctx.graph.nodes` (built by
//! `kjv_adapter::normalize`) and `ctx.brainfuel`, so calling exactly those
//! two NORMALIZE-stage calls directly, over a real-but-minimal `BuildCtx`
//! (an empty `AtlasData`, no xrefs), proves the same real-data merge
//! behavior at a fraction of the cost of a full ETL compile -- no
//! `data/curated/` parse, no event-world/Theographic processing.

use std::path::Path;

use atlas_graph::brainfuel_adapter;
use atlas_graph::kjv_adapter;
use atlas_graph::pipeline::BuildCtx;

const NO_XREFS: &str = "From Verse\tTo Verse\tVotes\t#comment\n";

#[test]
fn merge_stats_over_the_real_vendored_data_match_the_verified_per_edition_totals() {
    let raw_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let (canon, verses) = atlas_etl::kjv::parse(&kjv_json).expect("the real kjv.json must parse");
    let brainfuel = atlas_etl::brainfuel::read_all(&raw_dir.join("brain-fuel-bible"))
        .expect("data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step first");
    let atlas = atlas_graph::event_world::empty_atlas();

    let mut ctx = BuildCtx::with_eras_and_brainfuel(&canon, &verses, None, NO_XREFS, &atlas, &[], Some(&brainfuel));
    kjv_adapter::normalize(&mut ctx).expect("the real KJV canon/verses must normalize into TextUnit nodes");
    let stats = brainfuel_adapter::normalize(&mut ctx);

    // Sum of this batch's own report table (data/raw/README.md +
    // LICENSES.md + batch-corp1a-report.md all carry the identical
    // breakdown): 31,092 (Vulgate) + 23,145 (WLC) + 23,132 (Douay-Rheims)
    // + 31,102 (Biblia 1776) + 31,099 (Karl XII) + 7,957 (Greek TR).
    assert_eq!(stats.renderings_merged, 147_527, "one (edition, verse) rendering pair merged per Present outcome -- must equal the sum of the six per-edition present counts");
    assert_eq!(stats.rows_with_no_matching_text_unit, 0, "every brain-fuel verse row must resolve to a real KJV TextUnit node -- the two skeletons are fully aligned, zero orphans");
    assert_eq!(stats.translation_nodes, 6, "one Translation node per ingested edition, never for KJV itself");
}
