mod common;

use atlas_graph::brainfuel_adapter;
use atlas_graph::kjv_adapter;
use atlas_graph::pipeline::BuildCtx;

const NO_XREFS: &str = "From Verse\tTo Verse\tVotes\t#comment\n";

#[test]
fn merge_stats_over_the_real_vendored_data_match_the_verified_per_edition_totals() {
    let (canon, verses) = atlas_etl::kjv::parse(&common::kjv_json()).expect("the real kjv.json must parse");
    let brainfuel = common::brainfuel_corpus();
    let atlas = atlas_graph::event_world::empty_atlas();

    let mut ctx = BuildCtx::with_eras_and_brainfuel(&canon, &verses, None, NO_XREFS, &atlas, &[], Some(&brainfuel));
    kjv_adapter::normalize(&mut ctx).expect("the real KJV canon/verses must normalize into TextUnit nodes");
    let stats = brainfuel_adapter::normalize(&mut ctx);

    assert_eq!(stats.renderings_merged, 147_527, "one (edition, verse) rendering pair merged per Present outcome -- must equal the sum of the six per-edition present counts");
    assert_eq!(stats.rows_with_no_matching_text_unit, 0, "every brain-fuel verse row must resolve to a real KJV TextUnit node -- the two skeletons are fully aligned, zero orphans");
    assert_eq!(stats.translation_nodes, 6, "one Translation node per ingested edition, never for KJV itself");
}
