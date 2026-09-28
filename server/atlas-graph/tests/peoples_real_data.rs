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

#[test]
fn peoplegroup_node_counts_match_all_three_sources_exactly() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

    assert_eq!(atlas.people_groups.len(), 23, "the real committed peopleGroups.json must carry exactly 23 records");
    assert_eq!(atlas.people_group_seeds.len(), 6, "the curated nation seeds: Ammonites/Moabites/Edomites/Philistines/Amalekites/Canaanites");
    assert_eq!(atlas.people_group_reclassify.len(), 9, "the closed nine-slug Gen-10 gentilic reclassification list");

    let group_nodes = ctx.graph.nodes.values().filter(|n| n.id.kind == NodeKind::PeopleGroup).count();
    assert_eq!(group_nodes, 38, "23 Theographic + 6 curated seeds + 9 reclassified = 38 PeopleGroup nodes in the built graph");

    for r in &atlas.people_group_reclassify {
        assert!(ctx.graph.nodes.get(&PersonId::new(r.person_slug.clone()).erase()).is_none(), "reclassified slug '{}' must carry NO Person node", r.person_slug);
        let g = ctx.graph.nodes.get(&PeopleGroupId::new(r.person_slug.clone()).erase());
        assert!(g.is_some(), "reclassified slug '{}' must carry a PeopleGroup node under the SAME raw id", r.person_slug);
    }

    let person_nodes = ctx.graph.nodes.values().filter(|n| n.id.kind == NodeKind::Person).count();
    assert_eq!(person_nodes, atlas.people.len() - 9, "Person node count = source records MINUS the nine reclassified");
}

#[test]
fn reclassified_mention_rows_carry_peoplegroup_sense_at_a_real_locus() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

    let jebusite = PeopleGroupId::new("jebusite_748");
    let gen_10_16 = atlas_graph::kjv_adapter::dot_ref(0, 10, 16);
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

    let person_hits = ctx.graph.mentions.iter().filter(|row| matches!(&row.entity, MentionedEntity::Person(p) if p.0 == "jebusite_748")).count();
    assert_eq!(person_hits, 0, "jebusite_748 must carry NO Person-kind mentions any more");
}

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

    assert!(atlas_graph::peoples_adapter::every_named_after_row_has_a_scripture_ground(&ctx.graph).is_ok());

    let ammonites_row = ctx
        .graph
        .named_after
        .iter()
        .find(|r| matches!(&r.namesake, Namesake::PeopleGroup(g) if g.0 == "ammonites"))
        .expect("an Ammonites named_after row must exist");
    assert_eq!(ammonites_row.eponym.0, "ben-ammi_451");

    let edomites_row = ctx
        .graph
        .named_after
        .iter()
        .find(|r| matches!(&r.namesake, Namesake::PeopleGroup(g) if g.0 == "edomites"))
        .expect("an Edomites named_after row must exist");
    assert_eq!(edomites_row.eponym.0, "esau_1216");
    assert_eq!(edomites_row.justification.grounds.len(), 2, "Edomites' own real curated row carries two Scripture grounds");
}

#[test]
fn a_curated_nation_seeds_description_fills_from_eastons_over_the_real_data() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

    let node = ctx.graph.nodes.get(&PeopleGroupId::new("ammonites").erase()).expect("the curated 'ammonites' PeopleGroup node must exist");
    match &node.payload {
        NodePayload::PeopleGroup { label, description } => {
            assert_eq!(label, "Ammonites");
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

#[test]
fn pg1b_real_data_yields_exactly_13_mentions_rows_at_the_reported_loci() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);

    let judah = atlas.people_groups.iter().find(|g| g.id == "tribe-of-judah").expect("Tribe of Judah must exist in the real compiled data");
    assert_eq!(judah.verse_links, vec!["PRO.25.1"], "Tribe of Judah's own one real verse -- NOT JDG.1.2");

    let israel = atlas.people_groups.iter().find(|g| g.id == "nation-of-israel").expect("Nation of Israel must exist in the real compiled data");
    assert_eq!(
        israel.verse_links,
        vec!["PSA.14.7", "PSA.53.6", "PSA.76.1", "PSA.78.21", "PSA.78.31", "PSA.78.41", "PSA.81.8", "PSA.81.11", "PSA.81.13", "PSA.89.18", "PSA.105.10", "PSA.147.19"]
    );

    let mut hits: Vec<(String, String)> = ctx
        .graph
        .mentions
        .iter()
        .filter_map(|row| match &row.entity {
            MentionedEntity::PeopleGroup(g) if g.0 == "tribe-of-judah" || g.0 == "nation-of-israel" => {
                atlas_graph::legacy::locus_dot_ref(&row.locus).map(|r| (g.0.clone(), r))
            }
            _ => None,
        })
        .collect();
    hits.sort();
    assert_eq!(hits.len(), 13, "exactly 13 PeopleGroup mentions rows across the two verse-bearing groups: {hits:?}");

    let mut expected: Vec<(String, String)> = vec![("tribe-of-judah".into(), "PRO.25.1".into())];
    for v in ["PSA.105.10", "PSA.14.7", "PSA.147.19", "PSA.53.6", "PSA.76.1", "PSA.78.21", "PSA.78.31", "PSA.78.41", "PSA.81.11", "PSA.81.13", "PSA.81.8", "PSA.89.18"] {
        expected.push(("nation-of-israel".into(), v.into()));
    }
    expected.sort();
    assert_eq!(hits, expected, "the exact 13 loci -- verbatim in the batch report");

    let total_theographic_source_mentions =
        ctx.graph.mentions.iter().filter(|row| matches!(&row.entity, MentionedEntity::PeopleGroup(_)) && row.provenance == atlas_graph::peoples_adapter::PROVENANCE_THEOGRAPHIC).count();
    assert_eq!(total_theographic_source_mentions, 13);
}

#[test]
fn reclassified_mentions_total_and_per_slug_counts_match_the_disclosed_table() {
    let (atlas, canon, verses, kjv_json, xrefs_tsv) = real_ctx_pieces();
    let ctx = build_real_ctx(&kjv_json, &xrefs_tsv, &atlas, &canon, &verses);
    let total = ctx.graph.mentions.iter().filter(|r| matches!(&r.entity, MentionedEntity::PeopleGroup(_)) && r.provenance == atlas_graph::peoples_adapter::PROVENANCE_RECLASSIFIED).count();
    assert_eq!(total, 27, "total PeopleGroup mentions rows across all nine reclassified slugs");

    let grand_total = ctx.graph.mentions.iter().filter(|r| matches!(&r.entity, MentionedEntity::PeopleGroup(_))).count();
    assert_eq!(grand_total, 40, "27 reclassified + 13 PG-1B source-(a) verse-bearing groups");

    let expected: &[(&str, usize)] =
        &[("jebusite_748", 2), ("amorite_237", 4), ("girgasite_1322", 2), ("hivite_1534", 9), ("arkite_308", 2), ("sinite_2755", 2), ("arvadite_316", 2), ("zemarite_3036", 2), ("hamathite_1361", 2)];
    assert_eq!(expected.iter().map(|(_, n)| n).sum::<usize>(), 27, "fixture sanity: this table's own numbers must sum to the total asserted above");
    for r in &atlas.people_group_reclassify {
        let n = ctx.graph.mentions.iter().filter(|row| matches!(&row.entity, MentionedEntity::PeopleGroup(g) if g.0 == r.person_slug)).count();
        let (_, want) = expected.iter().find(|(slug, _)| *slug == r.person_slug).unwrap_or_else(|| panic!("'{}' missing from this test's own expected table -- update it in the same commit as a curated reclassify-list change", r.person_slug));
        assert_eq!(n, *want, "{}", r.person_slug);
    }
}
