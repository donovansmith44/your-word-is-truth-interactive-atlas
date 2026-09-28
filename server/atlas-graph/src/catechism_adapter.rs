//! `CatechismItem` nodes and the symmetric `catechism-link` rows. Both curated citation shapes,
//! item-level and question-level, collapse into one flat locus <-> item row deduped by
//! (locus, item): which question cited a verse does not survive onto the row.

use std::collections::BTreeSet;

use atlas_graph_types::edge::CatechismLink;
use atlas_graph_types::id::CatechismItemId;
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::TextLocus;

use crate::pipeline::BuildCtx;

#[derive(Debug, Clone, Copy, Default)]
pub struct CatechismAdapterStats {
    pub items: usize,
    pub link_rows: usize,
}

pub fn catechism_item_node_id(id: &str) -> atlas_graph_types::id::AnyNodeId {
    CatechismItemId::new(id.to_string()).erase()
}

fn verse_locus(vref: &str) -> Option<TextLocus> {
    let vid = atlas_core::refs::VerseId::parse_canonical(vref).ok()?;
    let vr = atlas_graph_types::text::VerseRef { book: vid.book.0, chapter: vid.chapter, verse: vid.verse };
    Some(TextLocus::from(atlas_graph_types::text::BibleLocus::whole(vr)))
}

/// One node per catechism item, across every part.
pub fn normalize(ctx: &mut BuildCtx) -> CatechismAdapterStats {
    let mut stats = CatechismAdapterStats::default();
    for part in &ctx.atlas.catechism {
        for item in &part.items {
            let node = Node {
                id: CatechismItemId::new(item.id.clone()).erase(),
                payload: NodePayload::CatechismItem { label: item.name.clone() },
                provenance: "curated-catechism".to_string(),
            };
            ctx.graph.nodes.insert(node.id.clone(), node);
            stats.items += 1;
        }
    }
    stats
}

pub fn merge_alias(ctx: &mut BuildCtx) -> CatechismAdapterStats {
    let mut stats = CatechismAdapterStats::default();
    let mut seen: BTreeSet<(TextLocus, CatechismItemId)> = BTreeSet::new();

    for part in &ctx.atlas.catechism {
        for item in &part.items {
            let item_id = CatechismItemId::new(item.id.clone());
            let mut refs: Vec<&str> = item.verses.iter().map(String::as_str).collect();
            for q in &item.questions {
                refs.extend(q.verses.iter().map(String::as_str));
            }
            for vref in refs {
                let Some(locus) = verse_locus(vref) else { continue };
                if !seen.insert((locus.clone(), item_id.clone())) {
                    continue;
                }
                ctx.graph.catechism.push(CatechismLink {
                    locus,
                    item: item_id.clone(),
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
        }];
        d
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

    #[test]
    fn item_and_question_citations_dedupe_to_one_row_per_locus() {
        let atlas = atlas_with_catechism();
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        normalize(&mut ctx);
        let stats = merge_alias(&mut ctx);
        assert_eq!(stats.link_rows, 2, "EXO.20.3 (deduped) + 1CO.10.14");
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
        assert_eq!(page.entries.len(), 2, "the item's own frontier lists both linked loci");

        let vid = atlas_core::refs::VerseId::parse_canonical("EXO.20.3").unwrap();
        let verse_node_id = crate::kjv_adapter::verse_node_id(vid.book.0, vid.chapter, vid.verse);
        let verse_pos = Position::Node(verse_node_id);
        let verse_page = PositionRef(verse_pos.clone()).edges(&ctx.graph, &atlas_graph_types::explore::EdgeQuery { kind, cursor: None, limit: 10 });
        assert_eq!(verse_page.entries.len(), 1);
        assert_eq!(verse_page.entries[0].node, item_pos, "querying from the verse's own end must return the item");

        let from_item_entry = page.entries.iter().find(|e| e.node == verse_pos).expect("the item's own page must list this verse");
        assert_eq!(from_item_entry.edge, verse_page.entries[0].edge, "the SAME edge id, from either end -- the bijection witness");
    }
}
