//! EDGE-1a ("Prophecy & typology: the seed data" -- batch-edge1a-brief.md
//! decision 5): row-count + Scripture-ground-law + locus-validity checks
//! over the REAL committed `data/curated/fulfillments.toml`/`typology.toml`
//! -- same `atlas_etl::compile::compile`-backed pattern
//! `peoples_real_data.rs`/`description_real_data.rs` already establish in
//! this crate.

use std::collections::HashMap;
use std::path::Path;

use atlas_core::data::{AtlasData, Canon};
use atlas_graph::pipeline::{self, BuildCtx};
use atlas_graph_types::edge::Ground;
use atlas_graph_types::text::BibleLocusRange;

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

fn build_real_ctx<'a>(kjv_json: &'a str, xrefs_tsv: &'a str, atlas: &'a AtlasData, canon: &'a Canon, verses: &'a HashMap<String, String>) -> BuildCtx<'a> {
    let mut ctx = BuildCtx::new(canon, verses, Some(kjv_json), xrefs_tsv, atlas);
    pipeline::run_pipeline(&mut ctx, &pipeline::pipeline()).expect(
        "the real committed sources must build cleanly through the full pipeline (LAW-CHECK included -- reaching this line already proves every_fulfillment_row_has_a_scripture_ground/every_typology_row_has_a_scripture_ground both passed)",
    );
    ctx
}

fn real_ctx_pieces() -> (AtlasData, Canon, HashMap<String, String>, String, String) {
    let raw_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    let atlas = real_atlas_data();
    let (canon, verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
    (atlas, canon, verses, kjv_json, xrefs_tsv)
}

/// Decision 5: "row-count assertions (exact seeded totals)".
#[test]
fn exact_seeded_row_counts() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    assert_eq!(atlas.fulfillment_seeds.len(), 24, "data/curated/fulfillments.toml must author exactly 24 [[fulfillment]] rows");
    assert_eq!(atlas.typology_seeds.len(), 16, "data/curated/typology.toml must author exactly 16 [[typology]] rows");

    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);
    assert_eq!(ctx.graph.fulfills.len(), 24, "every curated fulfillment row's own locus must parse -- zero runtime omissions expected");
    assert_eq!(ctx.graph.typology.len(), 16, "every curated typology row's own locus must parse -- zero runtime omissions expected");
}

/// Decision 5: "every-row-has-Scripture-ground law test (same shape as
/// PG-1a's)" -- both re-derived fresh (the dedicated law function, proving
/// it over the real table independently of LAW-CHECK already having run)
/// and via a direct inline loop.
#[test]
fn every_fulfillment_and_typology_row_has_a_scripture_ground() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

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

/// Decision 5: "locus-validity test (every BibleLocusRange resolves to
/// real KJV verses -- no typo'd books/chapters)". Every range in this
/// batch's own curated data is WITHIN a single book/chapter (verified at
/// authoring time) -- this test checks EVERY verse the range spans, not
/// just its two endpoints, over the REAL compiled KJV verse map.
#[test]
fn every_locus_in_every_row_resolves_to_a_real_kjv_verse() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

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
        assert_range_is_real(&verses, &row.prophecy, &format!("fulfillment row #{i} prophecy"));
        assert_range_is_real(&verses, &row.fulfillment, &format!("fulfillment row #{i} fulfillment"));
    }
    for (i, row) in ctx.graph.typology.iter().enumerate() {
        assert_range_is_real(&verses, &row.type_passage, &format!("typology row #{i} type_passage"));
        assert_range_is_real(&verses, &row.antitype_passage, &format!("typology row #{i} antitype_passage"));
    }
}

/// Spot-checks the brief's own worked example end to end over the real
/// build: ISA 7:14 -> MAT 1:22-23, self-attesting ground, real quote text.
#[test]
fn spot_check_isaiah_7_14_fulfilled_in_matthew_1_22_23() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

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

/// Spot-checks a real Typology row: Melchizedek, GEN 14:18-20 -> HEB 7:1-17.
#[test]
fn spot_check_melchizedek_typology_row() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

    let row = ctx.graph.typology.iter().find(|r| r.note.as_deref() == Some("Melchizedek")).expect("a Melchizedek typology row must exist");
    assert_eq!(atlas_graph::kjv_adapter::dot_ref(row.type_passage.from.unit.book, row.type_passage.from.unit.chapter, row.type_passage.from.unit.verse), "GEN.14.18");
    assert_eq!(atlas_graph::kjv_adapter::dot_ref(row.type_passage.to.unit.book, row.type_passage.to.unit.chapter, row.type_passage.to.unit.verse), "GEN.14.20");
    assert_eq!(atlas_graph::kjv_adapter::dot_ref(row.antitype_passage.from.unit.book, row.antitype_passage.from.unit.chapter, row.antitype_passage.from.unit.verse), "HEB.7.1");
    assert_eq!(atlas_graph::kjv_adapter::dot_ref(row.antitype_passage.to.unit.book, row.antitype_passage.to.unit.chapter, row.antitype_passage.to.unit.verse), "HEB.7.17");
}

/// Brief decision 1b's own "may appear in both tables" case: the passover
/// lamb's EXO.12.46 -> JHN.19.36 pairing is BOTH a fulfillment row AND a
/// typology row.
#[test]
fn the_passover_lamb_exo_12_46_jhn_19_36_appears_in_both_tables() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

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

/// Version-root regression companion (decision 2): these rows are EDGES,
/// never nodes -- `graph.nodes` must be untouched by this batch. The
/// dedicated `tests/version_root_regression.rs` harness is the real
/// acceptance gate; this is a fast, local sanity companion living
/// alongside the rest of this file's own real-data checks.
#[test]
fn fulfillment_and_typology_rows_add_zero_nodes() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);
    assert!(ctx.graph.fulfills.len() > 0 && ctx.graph.typology.len() > 0, "fixture sanity: real rows exist");
    // Every fulfillment/typology endpoint resolves to an ALREADY-EXISTING
    // TextUnit node (the KJV adapter's own, built earlier in NORMALIZE) --
    // never a new node kind or a new node of an existing kind.
    for row in &ctx.graph.fulfills {
        let from_id = atlas_graph_types::id::AnyNodeId { kind: atlas_graph_types::id::NodeKind::TextUnit, raw: format!("bible/{}.{}.{}", row.prophecy.from.unit.book, row.prophecy.from.unit.chapter, row.prophecy.from.unit.verse) };
        assert!(ctx.graph.nodes.contains_key(&from_id), "prophecy endpoint must resolve to an existing TextUnit node");
    }
}
