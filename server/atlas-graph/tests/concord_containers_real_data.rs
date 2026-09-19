//! D3 (owner, 2026-09-15): "new abstractions that are implementations of our
//! container abstraction to group things in the BoC in an analogous way to how
//! we did the Bible." The Bible's shape: book ⊃ chapter as Container rows (one
//! per child), chapter ⊃ verses as Loci. This test pins the same shape for the
//! Concord: document ⊃ article as Container rows, article ⊃ paragraphs as Loci.
//! Before this batch the document tier was a FLAT union of paragraph loci
//! (concord_adapter.rs's own module comment disclosed it).

use std::path::Path;

use atlas_graph::concord_adapter::ConcordBundle;
use atlas_graph_types::edge::ContainerContent;
use atlas_graph_types::id::{AnyNodeId, NodeKind};

fn real_atlas_data() -> atlas_core::data::AtlasData {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated")).expect("data/raw + data/curated must compile").data
}

/// The real graph with the Concord corpus (the same build the sibling
/// `concord_sc_overlap_real_data.rs` uses).
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
        let bundle = ConcordBundle { corpus: concord_corpus, sc_overlap };
        let (mut graph, ..) = atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord(&kjv_json, &xrefs_tsv, &atlas, &atlas.eras, Some(&brainfuel), Some(&bundle))
            .expect("the real committed sources must build");
        graph.build_indexes();
        graph
    })
}

#[test]
fn every_concord_document_contains_its_articles_as_container_rows() {
    let g = real_graph();
    let doc_rows: Vec<_> = g.contains_concord.iter().filter(|r| r.container.0.starts_with("concord-doc-")).collect();
    assert!(!doc_rows.is_empty(), "no document-level Contains rows at all");
    for r in &doc_rows {
        match &r.content {
            ContainerContent::Container(child) => {
                assert!(child.0.starts_with("concord-art-"), "document {} contains a non-article container {}", r.container.0, child.0)
            }
            ContainerContent::Loci(_) => panic!("document {} still has a flat Loci row -- the Bible shape is Container rows only", r.container.0),
        }
    }
    println!("D3 CONCORD CONTAINERS: {} document -> article rows", doc_rows.len());
}

#[test]
fn every_concord_article_container_is_contained_by_exactly_one_document() {
    let g = real_graph();
    let articles: Vec<&AnyNodeId> = g.nodes.keys().filter(|id| id.kind == NodeKind::Container && id.raw.starts_with("concord-art-")).collect();
    assert!(!articles.is_empty());
    for a in articles {
        let parents = g.contains_concord.iter().filter(|r| matches!(&r.content, ContainerContent::Container(c) if c.0 == a.raw)).count();
        assert_eq!(parents, 1, "article {} has {parents} document parents", a.raw);
    }
}

#[test]
fn article_containers_keep_their_paragraph_loci_and_the_forest_law_holds() {
    let g = real_graph();
    let mut n = 0;
    for r in g.contains_concord.iter().filter(|r| r.container.0.starts_with("concord-art-")) {
        n += 1;
        assert!(matches!(r.content, ContainerContent::Loci(_)), "article {} must contain paragraph loci", r.container.0);
    }
    assert!(n > 0);
    atlas_graph::law_check::container_containment_is_a_forest(g).expect("documents are roots, articles single-parent");
}
