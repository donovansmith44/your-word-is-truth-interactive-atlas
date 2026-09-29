mod common;

use common::OptionalCorpora;

use atlas_graph_types::edge::ContainerContent;
use atlas_graph_types::id::{AnyNodeId, NodeKind};

fn real_graph() -> &'static atlas_graph_types::graph::Graph {
    static GRAPH: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| common::indexed_raw_graph(OptionalCorpora { kretzmann: false, red_letter: false }))
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
