mod common;

#[test]
fn justified_by_grand_total_over_the_real_compiled_data_is_pinned() {
    let inputs = common::PipelineInputs::read();

    let ctx = inputs.run();

    println!("EDGE-1a/JB-1 justified-by grand total (real compiled data): {}", ctx.justified_by_count);
    assert_eq!(
        ctx.justified_by_count, 76,
        "justified-by grand total regressed or grew over the real committed data -- if this is a genuine data/curated change (a new DatedBy/fulfills/typology/named_after row, or a new ground on an existing row), update this pin in the SAME commit"
    );
}
