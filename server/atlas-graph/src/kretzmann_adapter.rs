//! Batch KRETZ-1: the Kretzmann corpus adapter -- turns `atlas_etl::
//! kretzmann::KretzmannCorpus` (the parsed HTML, LEMMA-EXCISED per
//! decision 2) into ONE `Source` node for the work itself, one
//! `CommentaryItem` node per unit (owner-ruled ANNOTATION shape: "a
//! comprehensive commentary without the verses interleaved into it"), and
//! one `CommentsOn` row per unit -> its own Bible locus range ("indexed so
//! that each verse mapped bit of commentary is mapped to the appropriate
//! verse in our graph"). Mirrors `concord_adapter.rs`'s own "parsed corpus
//! -> nodes + rows, self-contained, no cross-adapter dependency"
//! NORMALIZE-eligibility shape -- this adapter reads nothing any OTHER
//! adapter builds, so it runs in NORMALIZE, not MERGE/ALIAS.
//!
//! NODE IDENTITY: a `CommentaryItem`'s own id is exactly `KretzUnit.id`
//! (`"kretzmann/{book}.{chapter}.{ordinal}"`, `kretzmann.rs`'s own doc
//! comment) -- internal, stable within one parse, never displayed (the
//! node's own `card()` view reads `heading`, per `NodePayload::
//! CommentaryItem`'s own doc comment in `graph-types/src/node.rs`).
//!
//! GROUNDING: every `CommentsOn` row's own `Justification` grounds in the
//! unit's OWN locus range (`fulfillment_adapter.rs`'s own `scripture_
//! ground` precedent: "Scripture frequently SELF-ATTESTS these rows") --
//! decision 4's own "justification grounded in the lemma's own locus."
//! PROVENANCE is a per-page locator (`"kretzmann/{slug}/{chapter}"`,
//! decision 4) -- resolvable straight back to the vendored file this row's
//! own content came from.

use std::collections::BTreeSet;

use atlas_etl::kretzmann::{KretzmannCorpus, KretzUnit, BOOKS};
use atlas_graph_types::edge::{CommentsOn, Ground, Justification};
use atlas_graph_types::id::{AnyNodeId, CommentaryItemId, SourceId};
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{BibleLocusRange, Locus, VerseRef};

use crate::pipeline::BuildCtx;

/// The commentary work's own `Source` node id/label -- one node for the
/// whole work (decision 4: "one Source node for the work"), never one per
/// book/chapter.
pub const KRETZMANN_SOURCE_ID: &str = "kretzmann-popular-commentary";
pub const KRETZMANN_SOURCE_LABEL: &str = "Kretzmann, Popular Commentary of the Bible, CPH 1921-1924";
const KRETZMANN_PROVENANCE_KIND: &str = "kretzmann";

#[derive(Debug, Clone, Copy, Default)]
pub struct KretzmannAdapterStats {
    pub source_nodes: usize,
    pub items: usize,
    pub comments_on: usize,
    /// A unit whose own range failed to construct (`verse_from > verse_to`
    /// -- never true by the parser's own construction, but checked rather
    /// than assumed, the same defensive discipline `fulfillment_adapter::
    /// ground_range` already establishes for ITS curated ranges).
    pub inverted_range_dropped: usize,
}

/// The `CommentaryItem` node id for one parsed unit -- `KretzUnit.id` IS
/// the raw id (module doc comment's own "NODE IDENTITY").
fn commentary_item_id(unit: &KretzUnit) -> CommentaryItemId {
    CommentaryItemId::new(unit.id.clone())
}

fn book_slug(book_index: u8) -> &'static str {
    BOOKS.iter().find(|b| b.book_index == book_index).map(|b| b.slug).unwrap_or("unknown")
}

fn scripture_ground(range: &BibleLocusRange) -> BTreeSet<Ground> {
    let mut grounds = BTreeSet::new();
    grounds.insert(Ground::Scripture(range.clone()));
    grounds
}

/// Pipeline-facing NORMALIZE entry point: the Kretzmann `Source` node +
/// one `CommentaryItem` node and one `CommentsOn` row per unit, in
/// document order. Absent `ctx.kretzmann` (every test fixture that doesn't
/// supply real Kretzmann data) is a true no-op -- the SAME "absent ==
/// honestly empty, not a placeholder" treatment `ctx.concord`/`ctx.
/// brainfuel` already get (`concord_adapter.rs`'s own module doc comment).
pub fn normalize(ctx: &mut BuildCtx) -> KretzmannAdapterStats {
    let mut stats = KretzmannAdapterStats::default();
    let Some(corpus): Option<&KretzmannCorpus> = ctx.kretzmann else {
        return stats;
    };

    let source_id = SourceId::new(KRETZMANN_SOURCE_ID.to_string());
    let source_node_id: AnyNodeId = source_id.erase();
    ctx.graph.nodes.insert(
        source_node_id.clone(),
        Node { id: source_node_id, payload: NodePayload::Source { label: KRETZMANN_SOURCE_LABEL.to_string() }, provenance: KRETZMANN_PROVENANCE_KIND.to_string() },
    );
    stats.source_nodes = 1;

    for chapter in &corpus.chapters {
        let slug = book_slug(chapter.book_index);
        let page_provenance = format!("{KRETZMANN_PROVENANCE_KIND}/{slug}/{}", chapter.chapter);

        for unit in &chapter.units {
            let item_id = commentary_item_id(unit);
            let node_id = item_id.erase();
            ctx.graph.nodes.insert(
                node_id.clone(),
                Node {
                    id: node_id,
                    payload: NodePayload::CommentaryItem { work: source_id.clone(), heading: unit.heading.clone(), text: unit.text.clone() },
                    provenance: page_provenance.clone(),
                },
            );
            stats.items += 1;

            let from = Locus::whole(VerseRef { book: unit.book_index, chapter: unit.chapter, verse: unit.verse_from });
            let to = Locus::whole(VerseRef { book: unit.book_index, chapter: unit.chapter, verse: unit.verse_to });
            let Ok(range) = BibleLocusRange::new(from, to) else {
                stats.inverted_range_dropped += 1;
                continue;
            };
            let grounds = scripture_ground(&range);
            ctx.graph.comments_on.push(CommentsOn {
                item: item_id,
                on: range,
                provenance: page_provenance.clone(),
                justification: Justification { text: None, grounds },
            });
            stats.comments_on += 1;
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_etl::kretzmann::{ChapterStats, ParsedChapter, UnitKind};
    use atlas_core::data::Canon;
    use atlas_graph_types::id::NodeKind;
    use std::collections::HashMap;

    fn tiny_corpus() -> KretzmannCorpus {
        KretzmannCorpus {
            chapters: vec![ParsedChapter {
                book_index: 0,
                chapter: 1,
                units: vec![
                    KretzUnit {
                        id: "kretzmann/0.1.0".to_string(),
                        book_index: 0,
                        chapter: 1,
                        verse_from: 1,
                        verse_to: 1,
                        kind: UnitKind::Verse,
                        heading: Some("The Creation of the World.".to_string()),
                        text: "In the beginning, cp. John 1, 1.".to_string(),
                    },
                    KretzUnit {
                        id: "kretzmann/0.1.1".to_string(),
                        book_index: 0,
                        chapter: 1,
                        verse_from: 2,
                        verse_to: 2,
                        kind: UnitKind::Verse,
                        heading: Some("The Creation of the World.".to_string()),
                        text: "The material substance.".to_string(),
                    },
                ],
                fragments: vec![],
                stats: ChapterStats::default(),
            }],
            stats: Default::default(),
        }
    }

    fn ctx_with_kretzmann<'a>(canon: &'a Canon, verses: &'a HashMap<String, String>, atlas: &'a atlas_core::data::AtlasData, corpus: &'a KretzmannCorpus) -> BuildCtx<'a> {
        let mut ctx = BuildCtx::new(canon, verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", atlas);
        ctx.kretzmann = Some(corpus);
        ctx
    }

    #[test]
    fn normalize_builds_one_source_node_and_one_commentary_item_per_unit() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let corpus = tiny_corpus();
        let mut ctx = ctx_with_kretzmann(&canon, &verses, &atlas, &corpus);

        let stats = normalize(&mut ctx);
        assert_eq!(stats.source_nodes, 1);
        assert_eq!(stats.items, 2);
        assert_eq!(stats.comments_on, 2);
        assert_eq!(stats.inverted_range_dropped, 0);

        let source_node = ctx.graph.nodes.get(&SourceId::new(KRETZMANN_SOURCE_ID.to_string()).erase()).expect("the Source node must exist");
        assert_eq!(source_node.id.kind, NodeKind::Source);
        match &source_node.payload {
            NodePayload::Source { label } => assert_eq!(label, KRETZMANN_SOURCE_LABEL),
            other => panic!("expected Source payload, got {other:?}"),
        }

        let item = ctx.graph.nodes.get(&CommentaryItemId::new("kretzmann/0.1.1".to_string()).erase()).expect("the second unit's own node must exist");
        assert_eq!(item.id.kind, NodeKind::CommentaryItem);
        match &item.payload {
            NodePayload::CommentaryItem { work, heading, text } => {
                assert_eq!(work.0, KRETZMANN_SOURCE_ID);
                assert_eq!(heading.as_deref(), Some("The Creation of the World."));
                assert_eq!(text, "The material substance.");
            }
            other => panic!("expected CommentaryItem payload, got {other:?}"),
        }
    }

    #[test]
    fn comments_on_rows_carry_the_units_own_verse_range_and_self_grounding() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let corpus = tiny_corpus();
        let mut ctx = ctx_with_kretzmann(&canon, &verses, &atlas, &corpus);
        normalize(&mut ctx);

        let row = ctx.graph.comments_on.iter().find(|r| r.item.0 == "kretzmann/0.1.1").expect("a comments_on row for the second unit must exist");
        assert_eq!(row.on.from.unit, VerseRef { book: 0, chapter: 1, verse: 2 });
        assert_eq!(row.on.to.unit, VerseRef { book: 0, chapter: 1, verse: 2 });
        assert_eq!(row.provenance, "kretzmann/genesis/1");
        assert!(row.justification.grounds.contains(&Ground::Scripture(row.on.clone())), "the row's own justification grounds in its own locus (decision 4)");
    }

    #[test]
    fn absent_kretzmann_bundle_is_a_true_no_op() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        assert!(ctx.kretzmann.is_none());
        let stats = normalize(&mut ctx);
        assert_eq!(stats.items, 0);
        assert_eq!(stats.source_nodes, 0);
        assert!(ctx.graph.comments_on.is_empty());
        assert!(!ctx.graph.nodes.keys().any(|id| id.kind == NodeKind::CommentaryItem));
    }
}
