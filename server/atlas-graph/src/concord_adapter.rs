//! The Book of Concord corpus: one TextUnit node per paragraph, whose raw id is
//! `concord/{part}.{article}.{paragraph}` -- the spelling `Graph::build_indexes` derives from a
//! `TextRef::Concord`. A container's id is `concord-doc-{key}` or `concord-art-{key}-{article}`.

use std::collections::BTreeSet;

use atlas_etl::concord::{ConcordCorpus, ScOverlapRow};
use atlas_graph_types::edge::{CatechismLink, ContainerContent, Contains};
use atlas_graph_types::graph::ReadingSpine;
use atlas_graph_types::id::{AnyNodeId, CatechismItemId, ContainerNodeId, NodeKind};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{ConcordRef, ConcordTag, Locus, LocusSet, TextLocus, TranslationId};

use crate::pipeline::BuildCtx;

pub const CONCORD_CORPUS: &str = "concord";
/// The canonical rendering layer for the whole Concord corpus: one translation, unlike the Bible
/// corpus's many, and a key deliberately distinct from the KJV's -- this is not that translation.
pub const CONCORD_TRANSLATION: &str = "bente-dau";

/// The parsed corpus and the curated SC-overlap alignment, bundled so one optional field threads
/// through the build rather than two that could disagree about whether Concord data is present.
pub struct ConcordBundle {
    pub corpus: ConcordCorpus,
    pub sc_overlap: Vec<ScOverlapRow>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ConcordAdapterStats {
    pub documents: usize,
    pub articles: usize,
    pub paragraphs: usize,
    pub sc_overlap_links: usize,
    /// A curated SC-overlap row whose `item` names no real CatechismItem node in THIS build:
    /// disclosed, never a hard failure.
    pub sc_overlap_unmatched_items: usize,
    /// A curated SC-overlap row whose own paragraph names no real Concord
    /// TextUnit node in THIS build.
    pub sc_overlap_unmatched_paragraphs: usize,
}

pub fn text_unit_id(part: u8, article: u16, paragraph: u16) -> AnyNodeId {
    AnyNodeId { kind: NodeKind::TextUnit, raw: format!("concord/{part}.{article}.{paragraph}") }
}

pub fn decode_text_unit(id: &AnyNodeId) -> Option<(u8, u16, u16)> {
    if id.kind != NodeKind::TextUnit {
        return None;
    }
    let rest = id.raw.strip_prefix("concord/")?;
    let mut parts = rest.split('.');
    let part: u8 = parts.next()?.parse().ok()?;
    let article: u16 = parts.next()?.parse().ok()?;
    let paragraph: u16 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((part, article, paragraph))
}

fn doc_container_id(key: &str) -> ContainerNodeId {
    ContainerNodeId::new(format!("concord-doc-{key}"))
}

fn article_container_id(key: &str, article: u16) -> ContainerNodeId {
    ContainerNodeId::new(format!("concord-art-{key}-{article}"))
}

/// Absent `ctx.concord` is a true no-op: no nodes, no rows, no spine.
pub fn normalize(ctx: &mut BuildCtx) -> ConcordAdapterStats {
    let mut stats = ConcordAdapterStats::default();
    let Some(bundle) = ctx.concord else {
        return stats;
    };
    let mut order: Vec<AnyNodeId> = Vec::new();

    for doc in &bundle.corpus.documents {
        stats.documents += 1;
        let doc_container = doc_container_id(doc.key);

        for article in &doc.articles {
            stats.articles += 1;
            let mut art_content: BTreeSet<Locus<ConcordTag>> = BTreeSet::new();

            for p in &article.paragraphs {
                stats.paragraphs += 1;
                let unit_id = text_unit_id(doc.part, article.article, p.paragraph);
                let mut renderings = atlas_graph_types::text::LayerMap::new();
                renderings.insert(TranslationId(CONCORD_TRANSLATION.to_string()), p.text.clone());
                ctx.graph.nodes.insert(
                    unit_id.clone(),
                    Node { id: unit_id.clone(), payload: NodePayload::TextUnit { corpus: CONCORD_CORPUS, renderings }, provenance: "concord".to_string() },
                );
                order.push(unit_id);

                art_content.insert(Locus::whole(ConcordRef { part: doc.part, article: article.article, paragraph: p.paragraph }));
            }

            let art_container = article_container_id(doc.key, article.article);
            ctx.graph.nodes.insert(
                art_container.erase(),
                Node { id: art_container.erase(), payload: NodePayload::Container { title: article.title.clone() }, provenance: "concord".to_string() },
            );
            ctx.graph.contains_concord.push(Contains {
                container: art_container.clone(),
                content: ContainerContent::Loci(LocusSet(art_content)),
                provenance: ProvenanceId::from("concord"),
                justification: Default::default(),
            });
            // Document contains article as a Container row, the book-contains-chapter shape: one row per
            // article, in article order, which is load-bearing -- contents trees and `member-of` pages read it.
            // A paragraph is reachable through its article, as a verse through its chapter.
            ctx.graph.contains_concord.push(Contains {
                container: doc_container.clone(),
                content: ContainerContent::Container(art_container),
                provenance: ProvenanceId::from("concord"),
                justification: Default::default(),
            });
        }

        ctx.graph.nodes.insert(
            doc_container.erase(),
            Node { id: doc_container.erase(), payload: NodePayload::Container { title: doc.title.to_string() }, provenance: "concord".to_string() },
        );
    }

    ctx.graph.reading.insert(CONCORD_CORPUS, ReadingSpine { order });
    stats
}

/// Runs in MERGE/ALIAS rather than NORMALIZE because it cross-references the CatechismItem nodes
/// that pass built. The small-catechism document's own `part` is looked up from the parsed corpus
/// rather than hardcoded, so a renumbered corpus still targets the right document.
pub fn merge_alias(ctx: &mut BuildCtx) -> ConcordAdapterStats {
    let mut stats = ConcordAdapterStats::default();
    let Some(bundle) = ctx.concord else {
        return stats;
    };
    let Some(sc_part) = bundle.corpus.documents.iter().find(|d| d.key == "small-catechism").map(|d| d.part) else {
        return stats;
    };
    for row in &bundle.sc_overlap {
        let item_id = CatechismItemId::new(row.item.clone());
        // A curated row whose item names no node in THIS build is skipped and counted rather than
        // emitted: a caller can supply a real Concord bundle over an empty `AtlasData`, and an edge
        // naming a missing node would fail the referential-integrity law hard.
        if !ctx.graph.nodes.contains_key(&item_id.erase()) {
            stats.sc_overlap_unmatched_items += 1;
            continue;
        }
        for &paragraph in &row.paragraphs {
            let unit_id = text_unit_id(sc_part, row.article, paragraph);
            if !ctx.graph.nodes.contains_key(&unit_id) {
                stats.sc_overlap_unmatched_paragraphs += 1;
                continue;
            }
            let locus: TextLocus = Locus::<ConcordTag>::whole(ConcordRef { part: sc_part, article: row.article, paragraph }).into();
            ctx.graph.catechism.push(CatechismLink { locus, item: item_id.clone(), provenance: ProvenanceId::from("concord-sc-overlap"), justification: Default::default() });
            stats.sc_overlap_links += 1;
        }
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, CatechismItem, CatechismPart};
    use atlas_etl::concord::{ConcordArticle, ConcordDocument, ConcordParagraph, ConcordStats};
    use atlas_graph_types::edge::{Direction, EdgeKind, RelationId, SymRelationId};
    use atlas_graph_types::explore::{Explorable, PositionRef};
    use atlas_graph_types::id::Position;
    use atlas_graph_types::store::GraphQuery;
    use std::collections::HashMap;

    fn tiny_corpus() -> ConcordCorpus {
        ConcordCorpus {
            documents: vec![
                ConcordDocument {
                    part: 3,
                    key: "augsburg-confession",
                    title: "The Augsburg Confession",
                    articles: vec![ConcordArticle {
                        article: 4,
                        slug: "/augsburg-confession/of-justification/".into(),
                        title: "Article IV. Of Justification.".into(),
                        paragraphs: vec![
                            ConcordParagraph { paragraph: 1, source_label: "1".into(), text: "Also they teach that men cannot be justified before God by their own strength.".into() },
                            ConcordParagraph { paragraph: 2, source_label: "2".into(), text: "This faith God imputes for righteousness in His sight.".into() },
                        ],
                    }],
                },
                ConcordDocument {
                    part: 7,
                    key: "small-catechism",
                    title: "The Small Catechism",
                    articles: vec![ConcordArticle {
                        article: 2,
                        slug: "/small-catechism/ten-commandments/".into(),
                        title: "The Ten Commandments".into(),
                        paragraphs: vec![ConcordParagraph {
                            paragraph: 1,
                            source_label: "1/1b".into(),
                            text: "Thou shalt have no other gods. What does this mean? \u{2013}Answer: We should fear, love, and trust in God above all things.".into(),
                        }],
                    }],
                },
            ],
            stats: ConcordStats::default(),
        }
    }

    fn sc_overlap_rows() -> Vec<ScOverlapRow> {
        vec![ScOverlapRow { item: "commandment-1".into(), article: 2, paragraphs: vec![1] }]
    }

    fn atlas_with_first_commandment() -> AtlasData {
        let mut d = AtlasData::new(Canon { books: vec![] }, vec![], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        d.catechism = vec![CatechismPart {
            id: "ten-commandments".into(),
            title: "The Ten Commandments".into(),
            items: vec![CatechismItem {
                id: "commandment-1".into(),
                name: "The First Commandment".into(),
                text: Some("Thou shalt have no other gods.".into()),
                explanation_heading: "What does this mean?".into(),
                explanation: "We should fear, love, and trust in God above all things.".into(),
                where_written: None,
                verses: vec![],
                ref_note: None,
                questions: vec![],
            }],
        }];
        d
    }

    fn ctx_with_concord<'a>(canon: &'a Canon, verses: &'a HashMap<String, String>, atlas: &'a AtlasData, bundle: &'a ConcordBundle) -> BuildCtx<'a> {
        let mut ctx = BuildCtx::new(canon, verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", atlas);
        ctx.concord = Some(bundle);
        ctx
    }

    #[test]
    fn normalize_builds_one_text_unit_per_paragraph_in_canonical_spine_order() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let bundle = ConcordBundle { corpus: tiny_corpus(), sc_overlap: vec![] };
        let mut ctx = ctx_with_concord(&canon, &verses, &atlas, &bundle);

        let stats = normalize(&mut ctx);
        assert_eq!(stats.documents, 2);
        assert_eq!(stats.articles, 2);
        assert_eq!(stats.paragraphs, 3);

        let spine = ctx.graph.reading.get(CONCORD_CORPUS).expect("concord reading spine must exist");
        assert_eq!(spine.order.len(), 3);
        let decoded: Vec<_> = spine.order.iter().map(|id| decode_text_unit(id).unwrap()).collect();
        assert_eq!(decoded, vec![(3, 4, 1), (3, 4, 2), (7, 2, 1)]);
    }

    #[test]
    fn normalize_renders_the_canonical_bente_dau_layer_verbatim() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let bundle = ConcordBundle { corpus: tiny_corpus(), sc_overlap: vec![] };
        let mut ctx = ctx_with_concord(&canon, &verses, &atlas, &bundle);
        normalize(&mut ctx);

        let id = text_unit_id(3, 4, 2);
        let node = ctx.graph.node(&id).unwrap();
        match &node.payload {
            NodePayload::TextUnit { corpus, renderings } => {
                assert_eq!(*corpus, CONCORD_CORPUS);
                assert_eq!(renderings.get(&TranslationId(CONCORD_TRANSLATION.to_string())).map(String::as_str), Some("This faith God imputes for righteousness in His sight."));
            }
            other => panic!("expected TextUnit, got {other:?}"),
        }
    }

    #[test]
    fn normalize_builds_document_and_article_containers_queryable_through_the_generic_port() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let bundle = ConcordBundle { corpus: tiny_corpus(), sc_overlap: vec![] };
        let mut ctx = ctx_with_concord(&canon, &verses, &atlas, &bundle);
        normalize(&mut ctx);
        ctx.graph.build_indexes();

        let doc_container = doc_container_id("augsburg-confession");
        let art_container = article_container_id("augsburg-confession", 4);
        let forward = EdgeKind::Directed(RelationId::Contains, Direction::Forward);
        let page = PositionRef(Position::Node(doc_container.erase())).edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind: forward, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 1, "the document container's own frontier lists its one article container");
        assert_eq!(page.entries[0].node, Position::Node(art_container.erase()));

        let art_page = PositionRef(Position::Node(art_container.erase())).edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind: forward, cursor: None, limit: 10 });
        assert_eq!(art_page.entries.len(), 2, "the article container's own frontier lists both of its paragraphs");

        let p1 = text_unit_id(3, 4, 1);
        let inverse = EdgeKind::Directed(RelationId::Contains, Direction::Inverse);
        let back = PositionRef(Position::Node(p1)).edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind: inverse, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1, "paragraph 1 is a member of its article container only");
        assert_eq!(back.entries[0].node, Position::Node(art_container.erase()));
        let up = PositionRef(Position::Node(art_container.erase())).edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind: inverse, cursor: None, limit: 10 });
        assert_eq!(up.entries.len(), 1);
        assert_eq!(up.entries[0].node, Position::Node(doc_container.erase()));
    }

    #[test]
    fn merge_alias_builds_sc_overlap_catechism_links_queryable_symmetrically() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = atlas_with_first_commandment();
        let bundle = ConcordBundle { corpus: tiny_corpus(), sc_overlap: sc_overlap_rows() };
        let mut ctx = ctx_with_concord(&canon, &verses, &atlas, &bundle);

        crate::catechism_adapter::normalize(&mut ctx);
        normalize(&mut ctx);
        let stats = merge_alias(&mut ctx);
        assert_eq!(stats.sc_overlap_links, 1);
        ctx.graph.build_indexes();

        let item_pos = Position::Node(crate::catechism_adapter::catechism_item_node_id("commandment-1"));
        let kind = EdgeKind::Symmetric(SymRelationId::CatechismLink);
        let page = PositionRef(item_pos.clone()).edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 1, "the First Commandment's own CatechismItem reaches its Concord home");

        let concord_locus = Position::Node(text_unit_id(7, 2, 1));
        assert_eq!(page.entries[0].node, concord_locus, "linked to the Ten Commandments article's own paragraph 1 -- the First Commandment");

        let from_locus = PositionRef(concord_locus).edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind, cursor: None, limit: 10 });
        assert_eq!(from_locus.entries.len(), 1);
        assert_eq!(from_locus.entries[0].node, item_pos);
        assert_eq!(from_locus.entries[0].edge, page.entries[0].edge);
    }

    #[test]
    fn merge_alias_skips_and_counts_sc_overlap_rows_over_an_empty_atlas_never_panics_or_dangles() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let bundle = ConcordBundle { corpus: tiny_corpus(), sc_overlap: sc_overlap_rows() };
        let mut ctx = ctx_with_concord(&canon, &verses, &atlas, &bundle);
        crate::catechism_adapter::normalize(&mut ctx);
        normalize(&mut ctx);
        let stats = merge_alias(&mut ctx);

        assert_eq!(stats.sc_overlap_links, 0);
        assert_eq!(stats.sc_overlap_unmatched_items, 1, "the one curated row (commandment-1) is disclosed, not silently dropped or panicked on");
        assert!(ctx.graph.catechism.is_empty(), "no dangling CatechismLink row was authored");

        ctx.graph.build_indexes();
        crate::law_check::every_authored_edge_resolves(&ctx.graph).expect("no row this adapter authors may dangle, even over a partial-fixture build");
    }

    #[test]
    fn absent_concord_bundle_is_a_true_no_op() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        assert!(ctx.concord.is_none(), "every OTHER test fixture's own BuildCtx::new gets an honestly absent bundle");
        let n_stats = normalize(&mut ctx);
        let m_stats = merge_alias(&mut ctx);
        assert_eq!(n_stats.paragraphs, 0);
        assert_eq!(m_stats.sc_overlap_links, 0);
        assert!(ctx.graph.reading.get(CONCORD_CORPUS).is_none());
    }
}
