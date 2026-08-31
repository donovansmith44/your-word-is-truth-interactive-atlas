//! Batch CORP-2a: the Book of Concord corpus adapter -- turns
//! `atlas_etl::concord::ConcordCorpus` (the parsed HTML) into one
//! TextUnit node per paragraph, the document/article `Contains<
//! ConcordTag>` containers (decision 3), the "concord" reading spine
//! (canonical document order, decision 3), and the SC-overlap
//! `CatechismLink` rows (decision 4) -- mirrors `kjv_adapter.rs`'s own
//! "TextUnit + reading spine" shape for NORMALIZE and `catechism_
//! adapter.rs`'s own "curated cross-reference -> symmetric relation rows"
//! shape for MERGE/ALIAS.
//!
//! NODE IDENTITY: a TextUnit's `AnyNodeId.raw` is
//! `"concord/{part}.{article}.{paragraph}"` -- EXACTLY what `graph_types::
//! graph::Graph::build_indexes`'s own (private) `text_node` helper
//! produces from a `TextRef::Concord(ConcordRef)` (required for `cites`/
//! `confesses`/`catechism-link`/... BiIndexes, built from `TextLocus`
//! endpoints, to resolve to the SAME node ids this adapter inserts into
//! `graph.nodes` -- the same requirement `kjv_adapter.rs`'s own doc
//! comment states for the Bible corpus). A CONTAINER's own id is
//! `"concord-doc-{key}"` (one per document) or `"concord-art-{key}-
//! {article}"` (one per article) -- stable, internal, never displayed;
//! the container's own DISPLAY name is its `NodePayload::Container.title`
//! (a document's own canonical title, or an article's own source-given
//! title) -- "names are refs, not identity" (`kjv_adapter.rs`'s own `dot_
//! ref` doc comment names this same discipline).
//!
//! TWO-TIER CONTAINMENT (decision 3: "contains_concord rows build the
//! document/article containers"): `Contains<C>` holds a FLAT `LocusSet`
//! (no container-of-containers nesting in the type), so a document
//! container's own `content` is the UNION of every one of its articles'
//! own paragraphs, alongside each article's own, narrower `Contains` row
//! -- both rows populate `graph.contains_concord`, both explorable
//! through the SAME generic "contains"/"member-of" port (graph-types'
//! `build_indexes` now lowers `contains_concord` -- see that crate's own
//! `graph.rs` doc comment on the loop this batch added).

use std::collections::BTreeSet;

use atlas_etl::concord::{ConcordCorpus, ScOverlapRow};
use atlas_graph_types::edge::{CatechismLink, Contains};
use atlas_graph_types::graph::ReadingSpine;
use atlas_graph_types::id::{AnyNodeId, CatechismItemId, ContainerNodeId, NodeKind};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{ConcordRef, ConcordTag, Locus, LocusSet, TextLocus, TranslationId};

use crate::pipeline::BuildCtx;

pub const CONCORD_CORPUS: &str = "concord";
/// The canonical rendering layer for the whole Concord corpus (decision 3:
/// "canonical rendering = Bente-Dau English") -- one translation, unlike
/// the Bible corpus's many; a distinct key from `kjv_adapter::
/// KJV_TRANSLATION` on purpose (this is NOT the King James Version).
pub const CONCORD_TRANSLATION: &str = "bente-dau";

/// The parsed corpus + the curated SC-overlap alignment, bundled so
/// `BuildCtx` threads ONE new `Option<&ConcordBundle>` field (mirrors
/// `BuildCtx.brainfuel: Option<&BrainFuelCorpus>`'s own "absent == an
/// honestly empty build, not a placeholder" precedent) rather than two
/// independently-optional ones that could disagree about whether Concord
/// data is present at all.
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
    /// A curated SC-overlap row whose own `item` names no real
    /// CatechismItem node in THIS build (`merge_alias`'s own doc comment
    /// -- disclosed, never a hard failure).
    pub sc_overlap_unmatched_items: usize,
    /// A curated SC-overlap row whose own paragraph names no real Concord
    /// TextUnit node in THIS build.
    pub sc_overlap_unmatched_paragraphs: usize,
}

/// The TextUnit node id for one Concord paragraph position -- see module
/// doc comment for the format (must match `graph_types::graph::text_node`
/// exactly).
pub fn text_unit_id(part: u8, article: u16, paragraph: u16) -> AnyNodeId {
    AnyNodeId { kind: NodeKind::TextUnit, raw: format!("concord/{part}.{article}.{paragraph}") }
}

/// The inverse of `text_unit_id` -- `None` for anything not shaped like
/// one of this adapter's own ids, never a panic (mirrors `kjv_adapter::
/// decode_text_unit` exactly).
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

/// Pipeline-facing NORMALIZE entry point: walks `ctx.concord`'s own parsed
/// corpus into one TextUnit node per paragraph, one Container node +
/// `Contains<ConcordTag>` row per document AND per article, and the
/// "concord" reading spine, in canonical (part, article, paragraph)
/// order -- mirrors `kjv_adapter::normalize`'s own "nodes + the spine,
/// self-contained, no cross-adapter dependency" NORMALIZE-eligibility
/// (module doc comment's own "TWO-TIER CONTAINMENT"). Absent `ctx.concord`
/// (every test fixture that doesn't supply real Concord data, the SAME
/// "honestly empty, not a placeholder" treatment `ctx.brainfuel`/`ctx.eras`
/// already get) is a true no-op.
pub fn normalize(ctx: &mut BuildCtx) -> ConcordAdapterStats {
    let mut stats = ConcordAdapterStats::default();
    let Some(bundle) = ctx.concord else {
        return stats;
    };
    let mut order: Vec<AnyNodeId> = Vec::new();

    for doc in &bundle.corpus.documents {
        stats.documents += 1;
        let mut doc_content: BTreeSet<Locus<ConcordTag>> = BTreeSet::new();

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

                let locus = Locus::whole(ConcordRef { part: doc.part, article: article.article, paragraph: p.paragraph });
                art_content.insert(locus.clone());
                doc_content.insert(locus);
            }

            let art_container = article_container_id(doc.key, article.article);
            ctx.graph.nodes.insert(
                art_container.erase(),
                Node { id: art_container.erase(), payload: NodePayload::Container { title: article.title.clone() }, provenance: "concord".to_string() },
            );
            ctx.graph.contains_concord.push(Contains {
                container: art_container,
                content: LocusSet(art_content),
                provenance: ProvenanceId::from("concord"),
                justification: Default::default(),
            });
        }

        let doc_container = doc_container_id(doc.key);
        ctx.graph.nodes.insert(
            doc_container.erase(),
            Node { id: doc_container.erase(), payload: NodePayload::Container { title: doc.title.to_string() }, provenance: "concord".to_string() },
        );
        ctx.graph.contains_concord.push(Contains {
            container: doc_container,
            content: LocusSet(doc_content),
            provenance: ProvenanceId::from("concord"),
            justification: Default::default(),
        });
    }

    ctx.graph.reading.insert(CONCORD_CORPUS, ReadingSpine { order });
    stats
}

/// Pipeline-facing MERGE/ALIAS entry point: lowers the curated SC-overlap
/// alignment (`ctx.concord.sc_overlap`, decision 4) into `catechism-link`
/// rows -- runs in MERGE/ALIAS (not NORMALIZE) because it cross-references
/// `catechism_adapter::normalize`'s own CatechismItem nodes, the SAME
/// "crosses an adapter boundary -> MERGE/ALIAS" reasoning `catechism_
/// adapter::merge_alias` itself documents (`pipeline.rs`'s own stage
/// ordering guarantees `catechism_adapter::normalize` already ran, since
/// NormalizePass completes in full before MergeAliasPass starts). The
/// small-catechism document's own `part` is looked up from `ctx.concord`'s
/// own parsed corpus rather than hardcoded -- if the Concord corpus's own
/// vendored/parsed shape ever changed part numbers, this adapter would
/// still target the RIGHT document, not a stale constant.
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
        // Defensive existence check (decision 4: "unmatched items/paras
        // disclosed"), not force-fit: a caller that supplies a real
        // Concord bundle over a PARTIAL/empty `AtlasData` (no real
        // catechism.toml content -- e.g. a test fixture scoped to
        // something else entirely, `window_law.rs`'s own real, live-
        // caught case) would otherwise emit a `CatechismLink` row naming
        // a CatechismItem node that plain does not exist in THIS build,
        // failing the pipeline's own generic referential-integrity law
        // (`law_check::every_authored_edge_resolves`) hard -- skipped and
        // counted instead, the SAME "a row naming no real node is
        // skipped, not panicked on" discipline `peoples_adapter.rs`'s own
        // reclassify-row handling already establishes. Over the REAL
        // committed `catechism.toml` (every real caller -- `GraphService::
        // build`/`bins/compile_graph.rs`), every one of these 33 curated
        // rows' own item id DOES resolve (hand-verified, decision 4) --
        // this guard exists for build-time robustness, not because a real
        // mismatch is expected.
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
            ctx.graph.catechism.push(CatechismLink { locus, item: item_id.clone(), topic: None, provenance: ProvenanceId::from("concord-sc-overlap"), justification: Default::default() });
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
        // Two documents, mirroring the real shape closely enough to
        // exercise both container tiers + the spine, without needing the
        // real 3,827-paragraph corpus in a unit test.
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
            questions: vec![],
            curated: true,
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
        // Augsburg Confession (part 3) before Small Catechism (part 7) --
        // canonical part order, document order preserved within a part.
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

        // The Augsburg Confession's own DOCUMENT container holds both of
        // ITS paragraphs (both articles collapse into one document in
        // this fixture's own single-article-per-document shape, so
        // document == article content here; the article container proves
        // the narrower tier separately below).
        let doc_container = doc_container_id("augsburg-confession");
        let forward = EdgeKind::Directed(RelationId::Contains, Direction::Forward);
        let page = PositionRef(Position::Node(doc_container.erase())).edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind: forward, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 2, "the document container's own frontier lists both of its paragraphs");

        let art_container = article_container_id("augsburg-confession", 4);
        let art_page = PositionRef(Position::Node(art_container.erase())).edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind: forward, cursor: None, limit: 10 });
        assert_eq!(art_page.entries.len(), 2, "the article container's own frontier ALSO lists both (this fixture's one article holds its document's only paragraphs)");

        // Inverse: a paragraph's own 'member-of' frontier names its
        // container(s) back -- both the article and the document contain
        // it, so BOTH must appear (two separate Contains rows, decision
        // 3's own "document/article containers", plural).
        let p1 = text_unit_id(3, 4, 1);
        let inverse = EdgeKind::Directed(RelationId::Contains, Direction::Inverse);
        let back = PositionRef(Position::Node(p1)).edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind: inverse, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 2, "paragraph 1 is a member of BOTH its article container and its document container");
    }

    #[test]
    fn merge_alias_builds_sc_overlap_catechism_links_queryable_symmetrically() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = atlas_with_first_commandment();
        let bundle = ConcordBundle { corpus: tiny_corpus(), sc_overlap: sc_overlap_rows() };
        let mut ctx = ctx_with_concord(&canon, &verses, &atlas, &bundle);

        // MERGE/ALIAS depends on NORMALIZE having already run (module doc
        // comment) -- both catechism_adapter's own CatechismItem nodes and
        // this adapter's own Concord TextUnits must exist first.
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

        // Symmetric: querying from the Concord paragraph's own end returns
        // the SAME item under the SAME edge id (the bijection witness).
        let from_locus = PositionRef(concord_locus).edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind, cursor: None, limit: 10 });
        assert_eq!(from_locus.entries.len(), 1);
        assert_eq!(from_locus.entries[0].node, item_pos);
        assert_eq!(from_locus.entries[0].edge, page.entries[0].edge);
    }

    /// A real, live-caught case (`window_law.rs`'s own real build):
    /// `GraphService::build` threads a real Concord bundle unconditionally
    /// (it reads `raw_dir/concord/` whenever present, independent of what
    /// the CALLER's own `atlas` parameter carries), so a caller supplying
    /// an EMPTY `AtlasData` (no real `catechism.toml` content -- a
    /// perfectly legitimate fixture for a test scoped to something else
    /// entirely) must never hard-fail the whole build just because the
    /// SC-overlap table's own curated item ids don't resolve.
    #[test]
    fn merge_alias_skips_and_counts_sc_overlap_rows_over_an_empty_atlas_never_panics_or_dangles() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas(); // no CatechismItem nodes will exist
        let bundle = ConcordBundle { corpus: tiny_corpus(), sc_overlap: sc_overlap_rows() };
        let mut ctx = ctx_with_concord(&canon, &verses, &atlas, &bundle);
        crate::catechism_adapter::normalize(&mut ctx); // builds zero items -- atlas.catechism is empty
        normalize(&mut ctx);
        let stats = merge_alias(&mut ctx);

        assert_eq!(stats.sc_overlap_links, 0);
        assert_eq!(stats.sc_overlap_unmatched_items, 1, "the one curated row (commandment-1) is disclosed, not silently dropped or panicked on");
        assert!(ctx.graph.catechism.is_empty(), "no dangling CatechismLink row was authored");

        // The generic referential-integrity law this real bug tripped
        // must now pass clean over exactly this shape.
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
