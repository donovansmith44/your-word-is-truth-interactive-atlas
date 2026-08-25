//! Batch CORP-2a, decision 4 (SC-OVERLAP) + decision 9 ("SC-overlap link
//! count + spot-checks... e.g. the First Commandment's paragraph links
//! its CatechismItem"): builds the REAL graph (real `catechism.toml` +
//! real vendored Concord data + the real curated `concord-sc-overlap.toml`)
//! and asserts the aggregate link count, zero unmatched rows, and the
//! First Commandment spot-check -- over real data, not a synthetic
//! fixture (`concord_adapter.rs`'s own unit tests already cover the
//! mechanism in isolation; this is the "does the curated table actually
//! agree with BOTH real sources" proof).

use std::path::Path;

use atlas_graph::concord_adapter::ConcordBundle;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId, SymRelationId};
use atlas_graph_types::explore::{EdgeQuery, Explorable, PositionRef};
use atlas_graph_types::id::Position;
use atlas_graph_types::text::TextRef;

fn real_atlas_data() -> atlas_core::data::AtlasData {
    static CACHED: std::sync::OnceLock<atlas_core::data::AtlasData> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile")
                .data
        })
        .clone()
}

fn real_graph() -> &'static atlas_graph_types::graph::Graph {
    static GRAPH: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
        let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
        let atlas = real_atlas_data();
        let brainfuel = atlas_etl::brainfuel::read_all(&dir.join("brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist");
        let concord_corpus = atlas_etl::concord::read_all(&dir.join("concord")).expect("data/raw/concord must exist -- run data/fetch-raw.ps1 first");
        let sc_overlap_text = std::fs::read_to_string(dir.parent().unwrap().join("curated/concord-sc-overlap.toml")).expect("data/curated/concord-sc-overlap.toml must exist");
        let sc_overlap = atlas_etl::concord::parse_sc_overlap(&sc_overlap_text).expect("concord-sc-overlap.toml must parse");
        let curated_row_count = sc_overlap.len();
        assert_eq!(curated_row_count, 33, "the curated table's own [[link]] entry count -- one per CatechismItem, before paragraph-list expansion");
        let bundle = ConcordBundle { corpus: concord_corpus, sc_overlap };

        let (mut graph, ..) = atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord(&kjv_json, &xrefs_tsv, &atlas, &atlas.eras, Some(&brainfuel), Some(&bundle))
            .expect("the real committed sources must build");
        graph.build_indexes();
        graph
    })
}

/// Every `CatechismLink` row whose own locus is CONCORD-corpus (as
/// opposed to the pre-existing Bible-locus links `catechism_adapter.rs`
/// authors) -- isolates this batch's own contribution from the shared
/// `graph.catechism` table.
fn concord_locus_links(graph: &atlas_graph_types::graph::Graph) -> Vec<&atlas_graph_types::edge::CatechismLink> {
    graph.catechism.iter().filter(|l| matches!(l.locus.at, TextRef::Concord(_))).collect()
}

#[test]
fn sc_overlap_expands_to_37_catechism_link_rows_with_zero_unmatched() {
    let graph = real_graph();
    let concord_links = concord_locus_links(graph);
    assert_eq!(concord_links.len(), 37, "11 commandments + 3 creed + 9 lord's-prayer + 7 baptism + 2 confession + 5 altar (decision 4's own hand-verified alignment table)");

    // Zero unmatched (both directions) over the REAL data -- the curated
    // table's own 33 rows all resolve against the real catechism.toml AND
    // the real parsed Concord corpus (concord_adapter::merge_alias's own
    // defensive skip-and-count path, verified NEVER triggered here: if it
    // had, `concord_links.len()` above would be less than 37).
}

#[test]
fn the_first_commandment_catechism_item_reaches_its_concord_home_over_real_data() {
    let graph = real_graph();
    let item_pos = Position::Node(atlas_graph::catechism_adapter::catechism_item_node_id("commandment-1"));
    let kind = EdgeKind::Symmetric(SymRelationId::CatechismLink);
    // Fully paginate (the First Commandment's own pre-existing Bible-locus
    // links, from `catechism-mapping.toml`'s own topic verses, F2, turn
    // out to number in the hundreds -- a fixed single-page limit silently
    // truncated before reaching this batch's own appended-last Concord
    // row, a real lesson from this test's own first failing draft).
    let mut all_entries = Vec::new();
    let mut cursor = None;
    loop {
        let page = PositionRef(item_pos.clone()).edges(graph, &EdgeQuery { kind, cursor, limit: 200 });
        all_entries.extend(page.entries);
        match page.next {
            Some(n) => cursor = Some(n),
            None => break,
        }
    }
    assert!(!all_entries.is_empty(), "the First Commandment must be reachable through catechism-link at all");

    // Decision 4: "after this batch a catechism item is reachable from
    // BOTH its Bible grounds and its Concord home" -- both are real; this
    // spot-check isolates the Concord one specifically.
    let concord_pos = Position::Node(atlas_graph::concord_adapter::text_unit_id(7, 2, 1));
    let concord_entries: Vec<_> = all_entries.iter().filter(|e| e.node == concord_pos).collect();
    assert_eq!(concord_entries.len(), 1, "exactly one link to Small Catechism (part 7), Ten Commandments (article 2), paragraph 1 -- among {} total (Bible + Concord)", all_entries.len());

    // The paragraph's own rendered text is the real, bookofconcord.org-
    // sourced First Commandment (byte-verbatim, quoted in the batch
    // report).
    let node = graph.nodes.get(&atlas_graph::concord_adapter::text_unit_id(7, 2, 1)).unwrap();
    match &node.payload {
        atlas_graph_types::node::NodePayload::TextUnit { renderings, .. } => {
            let text = renderings.get(&atlas_graph_types::text::TranslationId("bente-dau".to_string())).unwrap();
            assert_eq!(text, "Thou shalt have no other gods. What does this mean? \u{2013}Answer: We should fear, love, and trust in God above all things.");
        }
        other => panic!("expected TextUnit, got {other:?}"),
    }
}

#[test]
fn augsburg_confession_article_iv_container_holds_exactly_its_three_paragraphs() {
    // Decision 3 spot-check: the document/article containers are real and
    // queryable over the real corpus, not just the synthetic
    // `concord_adapter.rs` unit-test fixture.
    let graph = real_graph();
    let article_container = atlas_graph_types::id::ContainerNodeId::new("concord-art-augsburg-confession-5");
    let forward = EdgeKind::Directed(RelationId::Contains, Direction::Forward);
    let page = PositionRef(Position::Node(article_container.erase())).edges(graph, &EdgeQuery { kind: forward, cursor: None, limit: 10 });
    assert_eq!(page.entries.len(), 3, "Augsburg Confession Article IV (Of Justification) has exactly 3 real paragraphs");
}
