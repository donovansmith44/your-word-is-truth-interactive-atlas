mod common;

use atlas_core::data::{AtlasData, Canon};
use atlas_graph::pipeline::{self, BuildCtx};

fn build_real_ctx<'a>(kjv_json: &'a str, xrefs_tsv: &'a str, atlas: &'a AtlasData, canon: &'a Canon, verses: &'a std::collections::HashMap<String, String>) -> BuildCtx<'a> {
    let mut ctx = BuildCtx::new(canon, verses, Some(kjv_json), xrefs_tsv, atlas);
    pipeline::run_pipeline(&mut ctx, &pipeline::pipeline()).expect("the real committed sources must build cleanly through the full pipeline");
    ctx
}

#[test]
fn justified_by_grand_total_over_the_real_compiled_data_is_pinned() {
    let kjv_json = common::kjv_json();
    let xrefs_tsv = common::cross_references_tsv();
    let atlas = common::real_atlas();
    let (canon, verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");

    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, atlas, &canon, &verses);

    println!("EDGE-1a/JB-1 justified-by grand total (real compiled data): {}", ctx.justified_by_count);
    assert_eq!(
        ctx.justified_by_count, 76,
        "justified-by grand total regressed or grew over the real committed data -- if this is a genuine data/curated change (a new DatedBy/fulfills/typology/named_after row, or a new ground on an existing row), update this pin in the SAME commit"
    );
}
