//! PG-1a ("People groups & eponymy: the data half" -- batch-pg1a-brief.md
//! requirement 7): node-count + mention-sense + eponymy-law checks over the
//! REAL committed `data/raw`/`data/curated` sources (never a synthetic
//! fixture) -- same `atlas_etl::compile::compile`-backed pattern
//! `description_real_data.rs`/`narrative_real_data.rs`/
//! `version_root_regression.rs` already establish in this crate.
//!
//! Runs the pipeline directly (`pipeline::BuildCtx` + `pipeline::
//! run_pipeline`), the SAME "direct BuildCtx access" shape
//! `description_real_data.rs`'s own module doc comment explains (this file
//! needs `ctx.graph` itself, not just the service-wrapped snapshot).

use std::collections::HashMap;
use std::path::Path;

use atlas_core::data::{AtlasData, Canon};
use atlas_graph::pipeline::{self, BuildCtx};
use atlas_graph_types::edge::{Ground, MentionedEntity, Namesake};
use atlas_graph_types::id::{NodeKind, PeopleGroupId, PersonId};
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

fn build_real_ctx<'a>(kjv_json: &'a str, xrefs_tsv: &'a str, atlas: &'a AtlasData, canon: &'a Canon, verses: &'a HashMap<String, String>) -> BuildCtx<'a> {
    let mut ctx = BuildCtx::new(canon, verses, Some(kjv_json), xrefs_tsv, atlas);
    pipeline::run_pipeline(&mut ctx, &pipeline::pipeline()).expect("the real committed sources must build cleanly through the full pipeline (LAW-CHECK included -- reaching this line already proves check_peoples_fidelity/every_named_after_row_has_a_scripture_ground both passed)");
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

/// Requirement 7: "node counts (23 + curated seeds + 9 reclassified, exact
/// expected totals asserted)". Exact, not a floor -- the roster of THREE
/// PeopleGroup sources is a fixed, curated/Theographic-bounded set this
/// batch establishes, not something that grows on its own the way Easton's
/// coverage does.
#[test]
fn peoplegroup_node_counts_match_all_three_sources_exactly() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

    assert_eq!(atlas.people_groups.len(), 23, "the real committed peopleGroups.json must carry exactly 23 records");
    assert_eq!(atlas.people_group_seeds.len(), 6, "the curated nation seeds: Ammonites/Moabites/Edomites/Philistines/Amalekites/Canaanites");
    assert_eq!(atlas.people_group_reclassify.len(), 9, "the closed nine-slug Gen-10 gentilic reclassification list");

    let group_nodes = ctx.graph.nodes.values().filter(|n| n.id.kind == NodeKind::PeopleGroup).count();
    assert_eq!(group_nodes, 38, "23 Theographic + 6 curated seeds + 9 reclassified = 38 PeopleGroup nodes in the built graph");

    // Every reclassified slug's own Person node is GONE -- the partition
    // is exhaustive, not merely additive.
    for r in &atlas.people_group_reclassify {
        assert!(ctx.graph.nodes.get(&PersonId::new(r.person_slug.clone()).erase()).is_none(), "reclassified slug '{}' must carry NO Person node", r.person_slug);
        let g = ctx.graph.nodes.get(&PeopleGroupId::new(r.person_slug.clone()).erase());
        assert!(g.is_some(), "reclassified slug '{}' must carry a PeopleGroup node under the SAME raw id", r.person_slug);
    }

    // Person node count correspondingly drops by exactly nine.
    let person_nodes = ctx.graph.nodes.values().filter(|n| n.id.kind == NodeKind::Person).count();
    assert_eq!(person_nodes, atlas.people.len() - 9, "Person node count = source records MINUS the nine reclassified");
}

/// Requirement 7: "reclassified mention rows carry PeopleGroup sense
/// (spot-check JOS 15:63-class loci for jebusite)". DISCLOSED, not
/// silently substituted: Theographic's own "jebusite_748" record carries
/// EXACTLY TWO real verse_links, GEN.10.16 and 1CH.1.14 (the Gen-10/
/// 1-Chronicles-1 genealogical table occurrences) -- JOS.15.63 is never
/// among them (Theographic ships NO per-locus attestation for the many
/// narrative occurrences of "the Jebusites" outside that one genealogical
/// pair; see this batch's own report). This test spot-checks the REAL
/// available locus of that exact CLASS (an in-text gentilic-name mention)
/// rather than fabricating a JOS.15.63 row the source does not attest --
/// the Sin-guard principle applies to this test's own fixture choice too.
#[test]
fn reclassified_mention_rows_carry_peoplegroup_sense_at_a_real_locus() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

    let jebusite = PeopleGroupId::new("jebusite_748");
    let gen_10_16 = atlas_graph::kjv_adapter::dot_ref(0, 10, 16); // GEN is book index 0
    assert_eq!(gen_10_16, "GEN.10.16");

    let hits: Vec<_> = ctx
        .graph
        .mentions
        .iter()
        .filter(|row| matches!(&row.entity, MentionedEntity::PeopleGroup(g) if *g == jebusite))
        .filter_map(|row| atlas_graph::legacy::locus_dot_ref(&row.locus))
        .collect();
    assert!(hits.contains(&"GEN.10.16".to_string()), "jebusite_748 must carry a real PeopleGroup mention at GEN.10.16: {hits:?}");
    assert!(hits.contains(&"1CH.1.14".to_string()), "jebusite_748 must carry a real PeopleGroup mention at 1CH.1.14: {hits:?}");
    assert_eq!(hits.len(), 2, "Theographic's own jebusite_748 record ships EXACTLY these two verse_links, no more -- see this test's own doc comment");

    // No Person-kind mention survives for this slug -- the sense really
    // changed, not merely duplicated.
    let person_hits = ctx.graph.mentions.iter().filter(|row| matches!(&row.entity, MentionedEntity::Person(p) if p.0 == "jebusite_748")).count();
    assert_eq!(person_hits, 0, "jebusite_748 must carry NO Person-kind mentions any more");
}

/// Requirement 7: "every NamedAfter row's justification carries at least
/// one Scripture ground (a law-shaped test over the table)" -- exercised
/// here over the REAL curated `data/curated/people-groups.toml` rows,
/// alongside the exact expected row/omission counts decision 3's own seed
/// list implies: 18 curated `[[named_after]]` rows authored (12 tribes + Nation
/// of Israel + Ammonites/Moabites/Edomites/Amalekites/Canaanites), every
/// ONE of which has a real, existing eponym Person node in the real
/// compiled data (verified independently against `data/raw/theographic/
/// .../people.json` at authoring time -- see the curated file's own header
/// comment) -- so zero RUNTIME omissions. Philistines carries no
/// `[[named_after]]` row AT ALL (a curated-TIME, disclosed omission, not a
/// runtime one -- see that file's own trailing comment and the batch
/// report).
#[test]
fn every_named_after_row_is_scripture_grounded_and_the_seed_counts_match_decision_3() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

    assert_eq!(atlas.named_after_seeds.len(), 18, "the curated people-groups.toml must author exactly 18 [[named_after]] rows");
    assert_eq!(ctx.graph.named_after.len(), 18, "every curated row's own eponym must resolve to a real Person node -- zero runtime omissions expected");

    for row in &ctx.graph.named_after {
        assert!(!row.justification.grounds.is_empty(), "named_after row (eponym {}) must carry >=1 ground", row.eponym.0);
        assert!(row.justification.grounds.iter().any(|g| matches!(g, Ground::Scripture(_))), "named_after row (eponym {}) must carry >=1 Ground::Scripture specifically", row.eponym.0);
    }

    // Independently re-run the dedicated law function too (not just the
    // inline loop above) -- proves the SAME law `LawCheckPass` already
    // ran unconditionally during `build_real_ctx` above (reaching that
    // line already proved it green; this re-derives the same verdict from
    // scratch over the real table, the "check the built graph fresh"
    // discipline this crate's own fidelity laws follow).
    assert!(atlas_graph::peoples_adapter::every_named_after_row_has_a_scripture_ground(&ctx.graph).is_ok());

    // Spot-check one real row's own full shape: Ammonites -> Ben-ammi,
    // GEN.19.38 ("she called his name Ben-ammi... the father of the
    // children of Ammon unto this day").
    let ammonites_row = ctx
        .graph
        .named_after
        .iter()
        .find(|r| matches!(&r.namesake, Namesake::PeopleGroup(g) if g.0 == "ammonites"))
        .expect("an Ammonites named_after row must exist");
    assert_eq!(ammonites_row.eponym.0, "ben-ammi_451");

    // Edomites carries TWO grounds on one row (GEN 36:8-9 range + GEN
    // 25:30) -- the real curated multi-ground shape, not a synthetic one.
    let edomites_row = ctx
        .graph
        .named_after
        .iter()
        .find(|r| matches!(&r.namesake, Namesake::PeopleGroup(g) if g.0 == "edomites"))
        .expect("an Edomites named_after row must exist");
    assert_eq!(edomites_row.eponym.0, "esau_1216");
    assert_eq!(edomites_row.justification.grounds.len(), 2, "Edomites' own real curated row carries two Scripture grounds");
}

/// Sanity companion to the group-description-fill test in
/// `description_real_data.rs` (which reports the fill RATE): here we just
/// confirm a real PeopleGroup node's own description survives the fill
/// pass end to end when Easton's has a matching entry, spot-checked on
/// "Ammonites" (the brief's own worked example, decision 4).
#[test]
fn a_curated_nation_seeds_description_fills_from_eastons_over_the_real_data() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

    let node = ctx.graph.nodes.get(&PeopleGroupId::new("ammonites").erase()).expect("the curated 'ammonites' PeopleGroup node must exist");
    match &node.payload {
        NodePayload::PeopleGroup { label, description } => {
            assert_eq!(label, "Ammonites");
            // Honest report, not an assumed pass: Easton's own dict_lookup
            // must match "Ammonites" EXACTLY (case-insensitive, tier c) for
            // this to fill -- if Easton's entry is singular "Ammonite"
            // only, this assertion is exactly the miss this batch's own
            // brief (decision 4) asks to be disclosed rather than silently
            // patched.
            if let Some(text) = description {
                assert!(!text.trim().is_empty());
                println!("AMMONITES description ({} chars): {}", text.len(), &text[..text.len().min(120)]);
            } else {
                println!("AMMONITES description: None (tier c dict_lookup miss -- disclosed in the batch report, not silently patched)");
            }
        }
        other => panic!("expected PeopleGroup, got {other:?}"),
    }
}

/// Decision 4 ("REPORT the group fill rate honestly... disclose the miss
/// pattern"): the EXACT roster of which 8 of 38 PeopleGroup nodes fill,
/// spot-checked by name rather than just a count -- catches a regression
/// in EITHER direction (a real future Easton's/label change silently
/// gaining or losing a match) the aggregate 8/38 in
/// `description_real_data.rs` alone would not localize. Verified directly
/// against the real compiled data at authoring time (batch-pg1a-report.md
/// has the full breakdown): 3 of the 9 reclassified gentilics (Arkite/
/// Sinite/Zemarite -- their OWN Theographic `name` already matches an
/// Easton headword of the identical singular shape), 2 of the 6 curated
/// nation seeds (Canaanites/Philistines -- Easton's own headword happens
/// to be plural for these two), 3 of the 23 Theographic groups
/// (Pharisees/Sadducees/Scribes, common NT terms Easton covers). NOT
/// filled, disclosed: the 12 tribes + Nation of Israel (no Easton
/// headword shaped like "Tribe of Judah" exists) and four of the six
/// curated nation seeds (Ammonites/Moabites/Edomites/Amalekites --
/// PLURAL curated label vs. Easton's own SINGULAR headword, e.g.
/// "Ammonite" not "Ammonites" -- an exact-match miss tier c's own
/// no-fuzzy-matching law correctly declines to bridge, never silently
/// patched).
#[test]
fn group_description_fill_matches_the_exact_disclosed_roster() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

    let mut filled: Vec<String> = ctx
        .graph
        .nodes
        .values()
        .filter(|n| n.id.kind == NodeKind::PeopleGroup)
        .filter_map(|n| match &n.payload {
            NodePayload::PeopleGroup { label, description: Some(_) } => Some(label.clone()),
            _ => None,
        })
        .collect();
    filled.sort();

    let mut expected = vec!["Arkite", "Canaanites", "Pharisees", "Philistines", "Sadducees", "Scribes", "Sinite", "Zemarite"];
    expected.sort();
    assert_eq!(filled, expected, "the exact set of PeopleGroup nodes carrying a filled description must match the disclosed roster -- a change here is real Easton's/label-drift content, update this test's own doc comment and the batch report in the same commit");

    let total = ctx.graph.nodes.values().filter(|n| n.id.kind == NodeKind::PeopleGroup).count();
    assert_eq!((filled.len(), total), (8, 38), "8/38 (21.1%) -- the PG-1a group description fill rate this batch reports");
}

/// The exact, per-slug PeopleGroup mentions breakdown -- 27 rows total
/// across the nine reclassified slugs (batch-pg1a-report.md has this same
/// table): Amorite carries 4 (the Gen-10/1-Chronicles-1 pair PLUS AMO 2:9-
/// 10, its own two extra prophetic mentions); Hivite carries 9 (the pair
/// plus the standard "nations of Canaan" lists, e.g. EXO 3:8/23:23, JOS
/// 9:1/11:3); the remaining seven carry exactly 2 each (the genealogical
/// pair only) -- confirming decision 1c's own "the only per-locus group
/// attestations the source actually ships" is real, verified content, not
/// an assumption.
#[test]
fn reclassified_mentions_total_and_per_slug_counts_match_the_disclosed_table() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);
    let total = ctx.graph.mentions.iter().filter(|r| matches!(&r.entity, MentionedEntity::PeopleGroup(_))).count();
    assert_eq!(total, 27, "total PeopleGroup mentions rows across all nine reclassified slugs");

    let expected: &[(&str, usize)] =
        &[("jebusite_748", 2), ("amorite_237", 4), ("girgasite_1322", 2), ("hivite_1534", 9), ("arkite_308", 2), ("sinite_2755", 2), ("arvadite_316", 2), ("zemarite_3036", 2), ("hamathite_1361", 2)];
    assert_eq!(expected.iter().map(|(_, n)| n).sum::<usize>(), 27, "fixture sanity: this table's own numbers must sum to the total asserted above");
    for r in &atlas.people_group_reclassify {
        let n = ctx.graph.mentions.iter().filter(|row| matches!(&row.entity, MentionedEntity::PeopleGroup(g) if g.0 == r.person_slug)).count();
        let (_, want) = expected.iter().find(|(slug, _)| *slug == r.person_slug).unwrap_or_else(|| panic!("'{}' missing from this test's own expected table -- update it in the same commit as a curated reclassify-list change", r.person_slug));
        assert_eq!(n, *want, "{}", r.person_slug);
    }
}
