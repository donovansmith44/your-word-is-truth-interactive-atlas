mod common;

use std::collections::HashMap;

use atlas_graph_types::edge::Ground;
use atlas_graph_types::text::BibleLocusRange;

#[test]
fn exact_seeded_row_counts() {
    let atlas = common::real_atlas();
    let inputs = common::PipelineInputs::read();
    assert_eq!(atlas.fulfillment_seeds.len(), 24, "data/curated/fulfillments.toml must author exactly 24 [[fulfillment]] rows");
    assert_eq!(atlas.typology_seeds.len(), 16, "data/curated/typology.toml must author exactly 16 [[typology]] rows");

    let ctx = inputs.run();
    assert_eq!(ctx.graph.fulfills.len(), 24, "every curated fulfillment row's own locus must parse -- zero runtime omissions expected");
    assert_eq!(ctx.graph.typology.len(), 16, "every curated typology row's own locus must parse -- zero runtime omissions expected");
}

#[test]
fn every_fulfillment_and_typology_row_has_a_scripture_ground() {
    let inputs = common::PipelineInputs::read();
    let ctx = inputs.run();

    assert!(atlas_graph::fulfillment_adapter::every_fulfillment_row_has_a_scripture_ground(&ctx.graph).is_ok());
    assert!(atlas_graph::fulfillment_adapter::every_typology_row_has_a_scripture_ground(&ctx.graph).is_ok());

    for row in &ctx.graph.fulfills {
        assert_eq!(row.justification.grounds.len(), 1, "every fulfillment row carries exactly one ground: the fulfillment passage self-attesting");
        assert!(matches!(row.justification.grounds.iter().next().unwrap(), Ground::Scripture(r) if *r == row.fulfillment));
        assert!(row.justification.text.is_some(), "every fulfillment row carries a real formula quote");
    }
    for row in &ctx.graph.typology {
        assert_eq!(row.justification.grounds.len(), 1, "every typology row carries exactly one ground: the antitype passage self-attesting");
        assert!(matches!(row.justification.grounds.iter().next().unwrap(), Ground::Scripture(r) if *r == row.antitype_passage));
        assert!(row.justification.text.is_some(), "every typology row carries a real grounding quote");
        assert!(row.note.is_some(), "every typology row carries a real figure note");
    }
}

#[test]
fn every_locus_in_every_row_resolves_to_a_real_kjv_verse() {
    let inputs = common::PipelineInputs::read();
    let ctx = inputs.run();

    fn assert_range_is_real(verses: &HashMap<String, String>, range: &BibleLocusRange, label: &str) {
        let from = &range.from.unit;
        let to = &range.to.unit;
        assert_eq!(from.book, to.book, "{label}: range must stay within one book (this batch's own curated data never crosses a book boundary)");
        assert_eq!(from.chapter, to.chapter, "{label}: range must stay within one chapter (this batch's own curated data never crosses a chapter boundary)");
        assert!(from.verse <= to.verse, "{label}: inverted range");
        for v in from.verse..=to.verse {
            let dot = atlas_graph::kjv_adapter::dot_ref(from.book, from.chapter, v);
            assert!(verses.contains_key(&dot), "{label}: {dot} does not resolve to a real KJV verse");
        }
    }

    for (i, row) in ctx.graph.fulfills.iter().enumerate() {
        assert_range_is_real(&inputs.verses, &row.prophecy, &format!("fulfillment row #{i} prophecy"));
        assert_range_is_real(&inputs.verses, &row.fulfillment, &format!("fulfillment row #{i} fulfillment"));
    }
    for (i, row) in ctx.graph.typology.iter().enumerate() {
        assert_range_is_real(&inputs.verses, &row.type_passage, &format!("typology row #{i} type_passage"));
        assert_range_is_real(&inputs.verses, &row.antitype_passage, &format!("typology row #{i} antitype_passage"));
    }
}

#[test]
fn spot_check_isaiah_7_14_fulfilled_in_matthew_1_22_23() {
    let inputs = common::PipelineInputs::read();
    let ctx = inputs.run();

    let row = ctx
        .graph
        .fulfills
        .iter()
        .find(|r| atlas_graph::kjv_adapter::dot_ref(r.prophecy.from.unit.book, r.prophecy.from.unit.chapter, r.prophecy.from.unit.verse) == "ISA.7.14")
        .expect("an ISA.7.14 fulfillment row must exist");
    assert_eq!(atlas_graph::kjv_adapter::dot_ref(row.fulfillment.from.unit.book, row.fulfillment.from.unit.chapter, row.fulfillment.from.unit.verse), "MAT.1.22");
    assert_eq!(atlas_graph::kjv_adapter::dot_ref(row.fulfillment.to.unit.book, row.fulfillment.to.unit.chapter, row.fulfillment.to.unit.verse), "MAT.1.23");
    assert!(row.justification.text.as_deref().unwrap().contains("virgin shall be with child"));
}

#[test]
fn spot_check_melchizedek_typology_row() {
    let inputs = common::PipelineInputs::read();
    let ctx = inputs.run();

    let row = ctx.graph.typology.iter().find(|r| r.note.as_deref() == Some("Melchizedek")).expect("a Melchizedek typology row must exist");
    assert_eq!(atlas_graph::kjv_adapter::dot_ref(row.type_passage.from.unit.book, row.type_passage.from.unit.chapter, row.type_passage.from.unit.verse), "GEN.14.18");
    assert_eq!(atlas_graph::kjv_adapter::dot_ref(row.type_passage.to.unit.book, row.type_passage.to.unit.chapter, row.type_passage.to.unit.verse), "GEN.14.20");
    assert_eq!(atlas_graph::kjv_adapter::dot_ref(row.antitype_passage.from.unit.book, row.antitype_passage.from.unit.chapter, row.antitype_passage.from.unit.verse), "HEB.7.1");
    assert_eq!(atlas_graph::kjv_adapter::dot_ref(row.antitype_passage.to.unit.book, row.antitype_passage.to.unit.chapter, row.antitype_passage.to.unit.verse), "HEB.7.17");
}

#[test]
fn the_passover_lamb_exo_12_46_jhn_19_36_appears_in_both_tables() {
    let inputs = common::PipelineInputs::read();
    let ctx = inputs.run();

    let as_fulfillment = ctx
        .graph
        .fulfills
        .iter()
        .any(|r| atlas_graph::kjv_adapter::dot_ref(r.prophecy.from.unit.book, r.prophecy.from.unit.chapter, r.prophecy.from.unit.verse) == "EXO.12.46" && atlas_graph::kjv_adapter::dot_ref(r.fulfillment.from.unit.book, r.fulfillment.from.unit.chapter, r.fulfillment.from.unit.verse) == "JHN.19.36");
    assert!(as_fulfillment, "EXO.12.46 -> JHN.19.36 must appear in the fulfillments table");

    let as_typology = ctx.graph.typology.iter().any(|r| {
        atlas_graph::kjv_adapter::dot_ref(r.type_passage.from.unit.book, r.type_passage.from.unit.chapter, r.type_passage.from.unit.verse) == "EXO.12.46"
            && atlas_graph::kjv_adapter::dot_ref(r.antitype_passage.from.unit.book, r.antitype_passage.from.unit.chapter, r.antitype_passage.from.unit.verse) == "JHN.19.36"
            && r.note.as_deref() == Some("the passover lamb")
    });
    assert!(as_typology, "EXO.12.46 -> JHN.19.36 must ALSO appear in the typology table, noted 'the passover lamb'");
}

#[test]
fn fulfillment_and_typology_rows_add_zero_nodes() {
    let inputs = common::PipelineInputs::read();
    let ctx = inputs.run();
    assert!(ctx.graph.fulfills.len() > 0 && ctx.graph.typology.len() > 0, "fixture sanity: real rows exist");
    for row in &ctx.graph.fulfills {
        let from_id = atlas_graph_types::id::AnyNodeId { kind: atlas_graph_types::id::NodeKind::TextUnit, raw: format!("bible/{}.{}.{}", row.prophecy.from.unit.book, row.prophecy.from.unit.chapter, row.prophecy.from.unit.verse) };
        assert!(ctx.graph.nodes.contains_key(&from_id), "prophecy endpoint must resolve to an existing TextUnit node");
    }
}
