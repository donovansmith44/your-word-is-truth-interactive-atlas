//! Merges each ingested edition's rendering onto the ALREADY-BUILT KJV `TextUnit` nodes, so this
//! pass must run after `kjv_adapter::normalize`: it mutates those nodes instead of creating them.
//! A rendering's `TranslationId` and its `Translation` node's raw id are one string space of slugs.

use atlas_graph_types::id::TranslationNodeId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::TranslationId;

use crate::pipeline::BuildCtx;

/// One `(edition slug, display label)` row per ingested edition; the label is the source repo's own
/// name, carried verbatim. Order is immaterial -- nodes are keyed by id.
const EDITION_LABELS: &[(&str, &str)] = &[
    ("latin_vulgate", "Clementine Vulgate"),
    ("hebrew_masoretic", "Westminster Leningrad Codex"),
    ("douay_rheims", "Douay-Rheims (Challoner)"),
    ("finnish_biblia", "Biblia 1776"),
    ("swedish_karl_xii", "Karl XII:s Bibel (1703)"),
    ("greek_textus_receptus", "Greek Textus Receptus"),
];

#[derive(Debug, Clone, Copy, Default)]
pub struct BrainFuelAdapterStats {
    pub translation_nodes: usize,
    pub renderings_merged: usize,
    /// A brain-fuel verse row whose own position has no matching KJV `TextUnit` node: anything but
    /// zero means the two skeletons disagree.
    pub rows_with_no_matching_text_unit: usize,
}

/// A true no-op -- no Translation nodes, no merges -- when `ctx.brainfuel` is `None`.
pub fn normalize(ctx: &mut BuildCtx) -> BrainFuelAdapterStats {
    let mut stats = BrainFuelAdapterStats::default();
    let Some(corpus) = ctx.brainfuel else {
        return stats;
    };

    for &(edition, label) in EDITION_LABELS {
        let node = Node {
            id: TranslationNodeId::new(edition.to_string()).erase(),
            payload: NodePayload::Translation { label: label.to_string() },
            provenance: "brainfuel".to_string(),
        };
        ctx.graph.nodes.insert(node.id.clone(), node);
        stats.translation_nodes += 1;
    }

    for row in &corpus.rows {
        let id = crate::kjv_adapter::verse_node_id(row.book.0, row.chapter, row.verse);
        let Some(node) = ctx.graph.nodes.get_mut(&id) else {
            stats.rows_with_no_matching_text_unit += 1;
            continue;
        };
        let NodePayload::TextUnit { renderings, .. } = &mut node.payload else {
            stats.rows_with_no_matching_text_unit += 1;
            continue;
        };
        for (translation, text) in &row.renderings {
            renderings.insert(TranslationId((*translation).to_string()), text.clone());
            stats.renderings_merged += 1;
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, CanonBook};
    use atlas_core::refs::BookId;
    use atlas_graph_types::id::NodeKind;
    use atlas_graph_types::store::GraphQuery;
    use std::collections::HashMap;

    fn gen_ctx_fixture() -> (Canon, HashMap<String, String>, AtlasData) {
        let canon = Canon { books: vec![CanonBook { code: "GEN".into(), name: "Genesis".into(), chapters: vec![1] }] };
        let mut verses = HashMap::new();
        verses.insert("GEN.1.1".to_string(), "In the beginning God created the heaven and the earth.".to_string());
        (canon, verses, crate::event_world::empty_atlas())
    }

    fn corpus_with_one_rendering() -> atlas_etl::brainfuel::BrainFuelCorpus {
        atlas_etl::brainfuel::BrainFuelCorpus {
            rows: vec![atlas_etl::brainfuel::VerseRow {
                book: BookId(0),
                chapter: 1,
                verse: 1,
                king_james: Some("their kjv text".to_string()),
                renderings: vec![("latin_vulgate", "In principio...".to_string()), ("hebrew_masoretic", "בְּרֵאשִׁית".to_string())],
            }],
            stats: atlas_etl::brainfuel::ParseStats::default(),
        }
    }

    #[test]
    fn none_brainfuel_is_a_true_no_op() {
        let (canon, verses, atlas) = gen_ctx_fixture();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        crate::kjv_adapter::normalize(&mut ctx).unwrap();
        let stats = normalize(&mut ctx);
        assert_eq!(stats.translation_nodes, 0);
        assert_eq!(stats.renderings_merged, 0);
        assert!(ctx.graph.nodes.values().all(|n| n.id.kind != NodeKind::Translation), "no Translation nodes without real brainfuel data");
    }

    #[test]
    fn merges_renderings_onto_the_existing_kjv_text_unit_node() {
        let (canon, verses, atlas) = gen_ctx_fixture();
        let corpus = corpus_with_one_rendering();
        let mut ctx = BuildCtx::with_eras_and_brainfuel(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas, &[], Some(&corpus));
        crate::kjv_adapter::normalize(&mut ctx).unwrap();
        let text_units_before = ctx.graph.nodes.len();
        let stats = normalize(&mut ctx);
        assert_eq!(stats.renderings_merged, 2);
        assert_eq!(ctx.graph.nodes.len(), text_units_before + 6, "6 new Translation nodes, zero new TextUnit nodes (merged, not duplicated)");

        let id = crate::kjv_adapter::verse_node_id(0, 1, 1);
        let node = ctx.graph.node(&id).expect("the verse node must still exist");
        let NodePayload::TextUnit { renderings, .. } = &node.payload else { panic!("not a TextUnit") };
        assert_eq!(renderings.get(&TranslationId("latin_vulgate".to_string())).map(String::as_str), Some("In principio..."));
        assert_eq!(renderings.get(&TranslationId("hebrew_masoretic".to_string())).map(String::as_str), Some("בְּרֵאשִׁית"));
        assert_eq!(renderings.get(&TranslationId("kjv".to_string())).map(String::as_str), Some("In the beginning God created the heaven and the earth."), "the canonical KJV layer must be untouched");
        assert!(node.provenance == "kjv", "merging renderings must not overwrite the node's own KJV provenance");
    }

    #[test]
    fn authors_one_translation_node_per_ingested_edition_resolving_the_translation_id() {
        let (canon, verses, atlas) = gen_ctx_fixture();
        let corpus = corpus_with_one_rendering();
        let mut ctx = BuildCtx::with_eras_and_brainfuel(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas, &[], Some(&corpus));
        crate::kjv_adapter::normalize(&mut ctx).unwrap();
        let stats = normalize(&mut ctx);
        assert_eq!(stats.translation_nodes, 6);

        let vulgate_id = TranslationNodeId::new("latin_vulgate".to_string()).erase();
        let node = ctx.graph.node(&vulgate_id).expect("a rendering's own TranslationId must resolve to a real Translation node");
        assert!(matches!(&node.payload, NodePayload::Translation { label } if label == "Clementine Vulgate"));
        assert_eq!(node.id.kind, NodeKind::Translation);

        let kjv_translation_id = TranslationNodeId::new("kjv".to_string()).erase();
        assert!(ctx.graph.node(&kjv_translation_id).is_none());
    }

    #[test]
    fn a_row_with_no_matching_text_unit_is_counted_not_panicked() {
        let (canon, verses, atlas) = gen_ctx_fixture();
        let corpus = atlas_etl::brainfuel::BrainFuelCorpus {
            rows: vec![atlas_etl::brainfuel::VerseRow {
                book: BookId(0),
                chapter: 99,
                verse: 99,
                king_james: None,
                renderings: vec![("latin_vulgate", "orphan".to_string())],
            }],
            stats: atlas_etl::brainfuel::ParseStats::default(),
        };
        let mut ctx = BuildCtx::with_eras_and_brainfuel(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas, &[], Some(&corpus));
        crate::kjv_adapter::normalize(&mut ctx).unwrap();
        let stats = normalize(&mut ctx);
        assert_eq!(stats.rows_with_no_matching_text_unit, 1);
        assert_eq!(stats.renderings_merged, 0);
    }
}
