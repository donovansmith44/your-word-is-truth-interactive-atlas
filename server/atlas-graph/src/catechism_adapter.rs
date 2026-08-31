//! Batch M-C, controller decision 2: the catechism adapter -- "catechism
//! links (locus <-> CatechismItem; legacy vocabulary maps at the
//! boundary)". Source: `ctx.atlas.catechism` (`AtlasData.catechism`, a
//! `Vec<CatechismPart>` -- kept standing this batch: the RICH item content
//! (explanation/where_written/part_title/question titles) stays served by
//! the existing `/api/catechism/item/{id}` endpoint, unmigrated; only the
//! item's own label and its locus links join the graph, per this batch's
//! own disclosed scope).
//!
//! LEGACY VOCABULARY MAPPED AT THE BOUNDARY: two curated citation shapes
//! collapse into ONE symmetric `catechism-link` row each -- an item-level
//! embedded citation (`CatechismItem.verses`, Luther's own wording, no
//! question context) and a question-level citation
//! (`CatechismQuestion.verses`, Batch F2's own per-topic mapping). Neither
//! curated file is re-parsed or re-authored; both already live on the SAME
//! `AtlasData.catechism` this adapter reads. The distinction itself (WHICH
//! question a citation came from) does not survive onto the graph row --
//! disclosed, not silent: `catechism-link` is a flat, symmetric, locus <->
//! item relation (design doc §4's own edge-kind table), with no room for a
//! question tag; the rich, question-aware view stays on the existing
//! bespoke endpoint, which reads `AtlasData.catechism` directly and never
//! loses that distinction. A verse cited by the SAME item under two
//! different questions collapses to ONE graph row (deduped by (locus,
//! item) below) -- the graph's own question is "does this locus connect to
//! this item," not "under how many questions."

use std::collections::BTreeSet;

use atlas_graph_types::edge::CatechismLink;
use atlas_graph_types::id::{CatechismItemId, CatechismPartId, CatechismTopicId};
use atlas_graph_types::edge::{CatechismMembership, CatechismPartLink};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::TextLocus;

use crate::pipeline::BuildCtx;

#[derive(Debug, Clone, Copy, Default)]
pub struct CatechismAdapterStats {
    pub items: usize,
    pub link_rows: usize,
    /// SVEB-1: topical grouping nodes emitted, and how many link rows
    /// carry one. Reported so the batch can show the recovery against the
    /// old flat count rather than assert it.
    pub topics: usize,
    pub topic_link_rows: usize,
    /// PARTS-1: chief-part nodes, their item-membership rows, and the
    /// part-level citation rows the seven deferred `resources/*.yaml`
    /// files contribute.
    pub parts: usize,
    pub membership_rows: usize,
    pub part_link_rows: usize,
    /// Of `parts`, how many this app materializes rather than
    /// `catechism.toml` defining -- carried on the node itself, never
    /// inferred downstream from the id.
    pub noncurated_parts: usize,
}

pub fn catechism_item_node_id(id: &str) -> atlas_graph_types::id::AnyNodeId {
    CatechismItemId::new(id.to_string()).erase()
}

/// SVEB-1: a topic's stable id, scoped to its own item.
///
/// Scoping matters: the 574 groupings carry only 562 DISTINCT titles, so a
/// title alone collides -- two different items can both gather a topic
/// called "Fear, Love, and Trust in God", and those are genuinely two
/// groupings with two different verse sets, not one shared node. The
/// ORDINAL is part of the key too, because an item may legitimately carry
/// the same title twice from two mapping sources.
pub fn catechism_topic_node_id(item_id: &str, ordinal: usize) -> CatechismTopicId {
    CatechismTopicId::new(format!("{item_id}#{ordinal}"))
}

/// PARTS-1: a chief part's node id -- the curated part id verbatim, the
/// same shape `catechism_item_node_id` uses, since part ids and item ids
/// are already disjoint namespaces in `catechism.toml`.
pub fn catechism_part_node_id(id: &str) -> CatechismPartId {
    CatechismPartId::new(id.to_string())
}

/// PARTS-1: a PART-level topic's id. Distinct namespace from the
/// item-level one (`item#ordinal`): a part and an item could otherwise
/// share an id string and collide. The `part:` prefix keeps them apart.
pub fn catechism_part_topic_node_id(part_id: &str, ordinal: usize) -> CatechismTopicId {
    CatechismTopicId::new(format!("part:{part_id}#{ordinal}"))
}

fn verse_locus(vref: &str) -> Option<TextLocus> {
    let vid = atlas_core::refs::VerseId::parse_canonical(vref).ok()?;
    let vr = atlas_graph_types::text::VerseRef { book: vid.book.0, chapter: vid.chapter, verse: vid.verse };
    Some(TextLocus::from(atlas_graph_types::text::BibleLocus::whole(vr)))
}

/// Pipeline-facing NORMALIZE entry point (`pipeline::NormalizePass`): one
/// node per catechism item, across every part.
pub fn normalize(ctx: &mut BuildCtx) -> CatechismAdapterStats {
    let mut stats = CatechismAdapterStats::default();
    for part in &ctx.atlas.catechism {
        // PARTS-1: the chief part itself is a node now, so part-level
        // content has an owner and the reader has a rung to stand on
        // between "the whole catechism" and a single numbered item.
        let part_node = Node {
            id: catechism_part_node_id(&part.id).erase(),
            payload: NodePayload::CatechismPart { label: part.title.clone(), curated: part.curated },
            provenance: "curated-catechism".to_string(),
        };
        ctx.graph.nodes.insert(part_node.id.clone(), part_node);
        stats.parts += 1;
        if !part.curated {
            stats.noncurated_parts += 1;
        }

        // PARTS-1: part-level topical groupings -- the seven files Batch
        // F2 deferred. Same node kind as the item-level ones; only the
        // owner differs.
        for (ordinal, q) in part.questions.iter().enumerate() {
            let topic = Node {
                id: catechism_part_topic_node_id(&part.id, ordinal).erase(),
                payload: NodePayload::CatechismTopic { label: q.title.clone(), source: q.source.clone() },
                provenance: "curated-catechism".to_string(),
            };
            ctx.graph.nodes.insert(topic.id.clone(), topic);
            stats.topics += 1;
        }

        for item in &part.items {
            let node = Node {
                id: CatechismItemId::new(item.id.clone()).erase(),
                payload: NodePayload::CatechismItem { label: item.name.clone() },
                provenance: "curated-catechism".to_string(),
            };
            ctx.graph.nodes.insert(node.id.clone(), node);
            stats.items += 1;

            // SVEB-1: one node per topical grouping, in curated order.
            for (ordinal, q) in item.questions.iter().enumerate() {
                let topic = Node {
                    id: catechism_topic_node_id(&item.id, ordinal).erase(),
                    payload: NodePayload::CatechismTopic { label: q.title.clone(), source: q.source.clone() },
                    provenance: "curated-catechism".to_string(),
                };
                ctx.graph.nodes.insert(topic.id.clone(), topic);
                stats.topics += 1;
            }
        }
    }
    stats
}

/// Pipeline-facing MERGE/ALIAS entry point (`pipeline::MergeAliasPass`):
/// lowers both citation shapes into `catechism-link` rows, deduped by
/// (locus, item) -- see this module's own doc comment.
pub fn merge_alias(ctx: &mut BuildCtx) -> CatechismAdapterStats {
    let mut stats = CatechismAdapterStats::default();
    // SVEB-1: the dedup key gained the topic. Under the old (locus, item)
    // key, a verse cited by one item under three different topics kept ONE
    // row and silently dropped two; 740 curated citations died that way.
    // Keyed by (locus, item, topic) each survives as its own row, and the
    // bare item-level citation (topic None) still cannot duplicate itself.
    let mut seen: BTreeSet<(TextLocus, CatechismItemId, Option<CatechismTopicId>)> = BTreeSet::new();

    // PARTS-1: the part-level rows carry the SAME three multiplicity laws
    // the item-level ones do -- see graph.rs's index lowering.
    let mut part_seen: BTreeSet<(TextLocus, CatechismPartId, Option<CatechismTopicId>)> = BTreeSet::new();

    for part in &ctx.atlas.catechism {
        let part_id = catechism_part_node_id(&part.id);

        // Which items this part contains -- structure, not citation, so it
        // is its own row kind rather than a catechism-link with a null
        // locus.
        for item in &part.items {
            ctx.graph.catechism_membership.push(CatechismMembership {
                part: part_id.clone(),
                item: CatechismItemId::new(item.id.clone()),
                provenance: ProvenanceId::from("curated-catechism"),
            });
            stats.membership_rows += 1;
        }

        for (ordinal, q) in part.questions.iter().enumerate() {
            let topic_id = catechism_part_topic_node_id(&part.id, ordinal);
            for vref in &q.verses {
                let Some(locus) = verse_locus(vref) else { continue };
                if !part_seen.insert((locus.clone(), part_id.clone(), Some(topic_id.clone()))) {
                    continue;
                }
                ctx.graph.catechism_part.push(CatechismPartLink {
                    locus,
                    part: part_id.clone(),
                    topic: Some(topic_id.clone()),
                    provenance: ProvenanceId::from("curated-catechism"),
                    justification: Default::default(),
                });
                stats.part_link_rows += 1;
            }
        }

        for item in &part.items {
            let item_id = CatechismItemId::new(item.id.clone());
            // (verse ref, owning topic) -- Luther's own embedded citations
            // first, with no topic, then each grouping's own verses tagged
            // with the topic that gathered them.
            let mut refs: Vec<(&str, Option<CatechismTopicId>)> =
                item.verses.iter().map(|v| (v.as_str(), None)).collect();
            for (ordinal, q) in item.questions.iter().enumerate() {
                let topic_id = catechism_topic_node_id(&item.id, ordinal);
                refs.extend(q.verses.iter().map(|v| (v.as_str(), Some(topic_id.clone()))));
            }
            for (vref, topic) in refs {
                let Some(locus) = verse_locus(vref) else { continue };
                if !seen.insert((locus.clone(), item_id.clone(), topic.clone())) {
                    continue;
                }
                if topic.is_some() {
                    stats.topic_link_rows += 1;
                }
                ctx.graph.catechism.push(CatechismLink {
                    locus,
                    item: item_id.clone(),
                    topic,
                    provenance: ProvenanceId::from("curated-catechism"),
                    justification: Default::default(),
                });
                stats.link_rows += 1;
            }
        }
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, CatechismItem, CatechismPart, CatechismQuestion};
    use atlas_graph_types::edge::EdgeKind;
    use atlas_graph_types::explore::{Explorable, PositionRef};
    use atlas_graph_types::id::Position;
    use atlas_graph_types::store::GraphQuery;
    use std::collections::HashMap;

    fn atlas_with_catechism() -> AtlasData {
        let mut d = AtlasData::new(Canon { books: vec![] }, vec![], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        d.catechism = vec![CatechismPart {
            id: "ten-commandments".into(),
            title: "The Ten Commandments".into(),
            items: vec![CatechismItem {
                id: "first-commandment".into(),
                name: "The First Commandment".into(),
                text: Some("Thou shalt have no other gods.".into()),
                explanation_heading: "What does this mean?".into(),
                explanation: "We should fear, love, and trust in God above all things.".into(),
                where_written: None,
                verses: vec!["EXO.20.3".into()],
                ref_note: None,
                questions: vec![CatechismQuestion { title: "Idolatry".into(), verses: vec!["EXO.20.3".into(), "1CO.10.14".into()], source: "test".into() }],
            }],
            questions: vec![],
            curated: true,
        }];
        d
    }

    /// PARTS-1: the same part, plus a PART-level grouping citing one verse
    /// the item already cites (EXO.20.3) and one it does not (DEU.6.5) --
    /// so a part row can be told apart from an item row. Its own fixture,
    /// deliberately: `atlas_with_catechism` pins item-level laws, and
    /// those should not have to account for part content.
    fn atlas_with_part_level_questions() -> AtlasData {
        let mut d = atlas_with_catechism();
        d.catechism[0].questions = vec![CatechismQuestion {
            title: "The Law as a Whole".into(),
            verses: vec!["EXO.20.3".into(), "DEU.6.5".into()],
            source: "test".into(),
        }];
        d
    }

    /// PARTS-1: a part carrying only part-level questions -- the Daily
    /// Prayers / Table of Duties shape.
    fn atlas_with_itemless_part() -> AtlasData {
        let mut d = AtlasData::new(Canon { books: vec![] }, vec![], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        d.catechism = vec![CatechismPart {
            id: "daily-prayers".into(),
            title: "Daily Prayers".into(),
            items: vec![],
            questions: vec![CatechismQuestion { title: "Morning Prayer".into(), verses: vec!["PSA.5.3".into()], source: "test".into() }],
            curated: false,
        }];
        d
    }

    #[test]
    fn a_part_is_a_node_carrying_whether_it_is_one_luther_numbered() {
        let atlas = atlas_with_itemless_part();
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.parts, 1);
        assert_eq!(stats.noncurated_parts, 1, "Daily Prayers is materialized here, not defined by catechism.toml");
        // Zero items, but the part still exists as a node -- the whole
        // point of the kind: part-level content has an owner.
        assert_eq!(stats.items, 0);
        assert_eq!(stats.topics, 1);

        let id = catechism_part_node_id("daily-prayers").erase();
        match &ctx.graph.nodes.get(&id).expect("part node").payload {
            NodePayload::CatechismPart { label, curated } => {
                assert_eq!(label, "Daily Prayers");
                assert!(!curated, "`curated` rides on the node -- never inferred downstream from the id");
            }
            other => panic!("wrong payload: {other:?}"),
        }
    }

    #[test]
    fn part_level_citations_do_not_collapse_into_the_items_own_rows() {
        let atlas = atlas_with_part_level_questions();
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        normalize(&mut ctx);
        let stats = merge_alias(&mut ctx);

        // EXO.20.3 is cited by the ITEM (bare and under a topic) AND by
        // the PART. Those are three different claims about three
        // different owners; the part row lives in its own table and
        // cannot be deduped away by the item's seen-set.
        assert_eq!(stats.part_link_rows, 2, "EXO.20.3 and DEU.6.5, each once under the part's grouping");
        assert_eq!(stats.membership_rows, 1);
        assert!(ctx
            .graph
            .catechism_part
            .iter()
            .all(|r| r.part.0 == "ten-commandments" && r.topic.is_some()));
        assert!(
            ctx.graph.catechism.iter().any(|r| r.item.0 == "first-commandment"),
            "the item's own rows are untouched by the part pass"
        );
    }

    #[test]
    fn part_and_item_topic_ids_cannot_collide() {
        // A part id and an item id could be the same string; without the
        // `part:` prefix both topics would hash to one node and one
        // grouping's verses would silently vanish into the other's.
        assert_ne!(catechism_part_topic_node_id("x", 0).0, catechism_topic_node_id("x", 0).0);
    }

    #[test]
    fn one_node_per_item_labeled_by_its_own_name() {
        let atlas = atlas_with_catechism();
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.items, 1);
        let node = ctx.graph.node(&catechism_item_node_id("first-commandment")).unwrap();
        assert!(matches!(node.payload, NodePayload::CatechismItem { label } if label == "The First Commandment"));
    }

    /// SVEB-1: this test's own law CHANGED, deliberately, and the rename
    /// says so.
    ///
    /// It used to assert `dedupe_to_one_row_per_locus`: EXO.20.3 cited both
    /// item-level (Luther's own embedded citation) and question-level (the
    /// topic "Idolatry") collapsed to ONE row. That collapse is exactly the
    /// bug this batch exists to fix -- across the real data it discarded 740
    /// curated citations and lost the topic on every survivor. The two
    /// citations are genuinely different provenance, so they are now two
    /// rows, keyed by (locus, item, topic).
    #[test]
    fn citations_are_one_row_per_locus_item_and_topic() {
        let atlas = atlas_with_catechism();
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        // Topic NODES are counted by NORMALIZE; link ROWS by MERGE/ALIAS.
        let normalize_stats = normalize(&mut ctx);
        let stats = merge_alias(&mut ctx);
        assert_eq!(normalize_stats.topics, 1, "the question grouping is a node now");
        // EXO.20.3 item-level (topic None) + EXO.20.3 under "Idolatry"
        // (topic Some) + 1CO.10.14 under "Idolatry". Three rows.
        assert_eq!(stats.link_rows, 3, "EXO.20.3 bare + EXO.20.3 under a topic + 1CO.10.14");
        // Two of the three carry a topic; the bare item-level one does not.
        assert_eq!(stats.topic_link_rows, 2);
    }

    #[test]
    fn catechism_link_is_queryable_symmetrically_through_the_generic_port() {
        let atlas = atlas_with_catechism();
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        normalize(&mut ctx);
        merge_alias(&mut ctx);
        ctx.graph.build_indexes();

        let item_pos = Position::Node(catechism_item_node_id("first-commandment"));
        let kind = EdgeKind::Symmetric(atlas_graph_types::edge::SymRelationId::CatechismLink);
        let page = PositionRef(item_pos.clone())
            .edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind, cursor: None, limit: 10 });
        // SVEB-1: 2 loci + the 1 topic node that now sits between the item
        // and its question-level citations. The topic is reachable from the
        // item precisely so "what does this item gather" is one hop.
        // PARTS-1: +1 -- the item also reaches its own chief part, which is
        // what makes "go back to the part I came from" a single hop rather
        // than a lookup the caller has to do out of band. A deliberate
        // widening of this law, not a silenced assertion.
        assert_eq!(page.entries.len(), 4, "the item's frontier lists both loci, its topic, AND its part");
        assert!(
            page.entries.iter().any(|e| matches!(&e.node, Position::Node(n) if n.raw == "ten-commandments")),
            "the fourth entry is the part: {:?}",
            page.entries.iter().map(|e| &e.node).collect::<Vec<_>>()
        );

        // Query from the OTHER end (a linked verse) -- symmetric means the
        // SAME edge id comes back either way (the bijection witness,
        // symmetric case).
        let vid = atlas_core::refs::VerseId::parse_canonical("EXO.20.3").unwrap();
        let verse_node_id = crate::kjv_adapter::verse_node_id(vid.book.0, vid.chapter, vid.verse);
        let verse_pos = Position::Node(verse_node_id);
        let verse_page = PositionRef(verse_pos.clone()).edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind, cursor: None, limit: 10 });
        // SVEB-1: the verse reaches its ITEM and its TOPIC -- "which topics
        // does this verse serve" is the reverse direction this batch added,
        // so a bare `entries[0]` is no longer a safe way to name the item.
        assert_eq!(verse_page.entries.len(), 2, "the verse reaches its item and its topic");
        let verse_to_item = verse_page
            .entries
            .iter()
            .find(|e| e.node == item_pos)
            .expect("querying from the verse's own end must return the item");

        let from_item_entry = page.entries.iter().find(|e| e.node == verse_pos).expect("the item's own page must list this verse");
        assert_eq!(from_item_entry.edge, verse_to_item.edge, "the SAME edge id, from either end -- the bijection witness");
    }
}
