//! Batch CORP-1a ("brain-fuel editions: the ingestion half"): the graph
//! half of `atlas_etl::brainfuel` -- merges each of the six ingested
//! editions' own renderings onto the ALREADY-BUILT KJV `TextUnit` nodes
//! (sweep F1: one node per skeleton position, all layer renderings as
//! payload) and authors one `Translation` node per ingested edition, so a
//! rendering's own `TranslationId` resolves to a real node (batch brief
//! controller decision 6). Runs in NORMALIZE, immediately after
//! `kjv_adapter::normalize` (module doc comment on `pipeline.rs`'s own
//! `NormalizePass`) -- a real, disclosed in-stage ordering dependency: this
//! adapter MUTATES the TextUnit nodes `kjv_adapter::normalize` just
//! inserted, rather than creating new ones, so those nodes must already
//! exist.
//!
//! No pre-existing KJV `Translation` node was found anywhere in this
//! codebase to "follow the pattern of" (searched exhaustively -- disclosed
//! in this batch's own report): `NodePayload::Translation`/`NodeKind::
//! Translation` exist in `graph-types` but had never once been
//! instantiated before this batch. This adapter follows the closest real
//! precedent instead -- the general "adapter constructs a `Node` directly
//! and inserts it into `ctx.graph.nodes`" shape every simple label-payload
//! NORMALIZE adapter already uses (e.g. `catechism_adapter::normalize`'s
//! own `NodePayload::CatechismItem { label }` nodes) -- and authors nodes
//! ONLY for the SIX editions this batch actually ingests (controller
//! decision 6: "one node per ingested edition"), not for KJV itself (the
//! pre-existing canonical layer, not "ingested" by this batch).
//!
//! A rendering's `TranslationId` and its own `Translation` node's
//! `AnyNodeId.raw` deliberately share ONE string space (both are exactly
//! `atlas_etl::brainfuel::EDITIONS`'s own slugs, e.g. `"latin_vulgate"`) --
//! literally the same "which translation" fact, viewed from the payload
//! side and the node-identity side; no separate mapping table exists or is
//! needed between them.

use atlas_graph_types::id::TranslationNodeId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::TranslationId;

use crate::pipeline::BuildCtx;

/// One `(edition slug, display label)` row per ingested edition -- the
/// label is the source repo's own `data/editions.json` `name` field,
/// carried verbatim (provenance-respecting: this app did not invent these
/// names). Order here is immaterial (nodes are inserted into `ctx.graph.
/// nodes`, a `BTreeMap`, so iteration order at query time is always by id)
/// but matches `atlas_etl::brainfuel::EDITIONS`'s own declared order.
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
    /// Must be 0 over real data -- a brain-fuel verse row whose own (book,
    /// chapter, verse) position has no matching KJV `TextUnit` node would
    /// mean the two skeletons disagree; asserted `0` by this batch's own
    /// real-data test (`brainfuel_layers.rs`), not just hoped.
    pub rows_with_no_matching_text_unit: usize,
}

/// Pipeline-facing NORMALIZE entry point (`pipeline::NormalizePass`, called
/// immediately after `kjv_adapter::normalize` -- module doc comment). A
/// no-op, honestly (zero Translation nodes, zero merges), when `ctx.
/// brainfuel` is `None` -- every test fixture that doesn't wire real
/// brain-fuel data simply doesn't get multilingual content, exactly the
/// same "no data, no nodes" discipline every other adapter in this crate
/// already follows.
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

        // No KJV Translation node is authored this batch (module doc
        // comment: not "ingested" by CORP-1a, no pre-existing pattern to
        // follow -- disclosed scoping decision, not an oversight).
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
