//! One `Container` node per Bible book and per chapter, with declared rows for chapter contains
//! verses, book contains chapter (one pairwise row per child) and the canon succession steps. A book
//! container's raw id is `bible-book-{CODE}`, a chapter's `bible-chapter-{CODE}-{chapter}`.

use std::collections::BTreeSet;

use atlas_graph_types::edge::{CanonSuccession, ContainerContent, Contains};
use atlas_graph_types::id::{AnyNodeId, ContainerNodeId, NodeKind};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{BibleTag, Locus, LocusSet, VerseRef};

use crate::pipeline::BuildCtx;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BibleContainerStats {
    pub books: usize,
    pub chapters: usize,
    /// Total verse loci across the chapter `Contains` rows: one locus per KJV verse.
    pub verse_loci: usize,
    /// book contains chapter `Contains` rows, one per child rather than one list.
    pub book_chapter_rows: usize,
    /// chapter -> next-chapter `CanonSuccession` rows, in canon order across book boundaries.
    pub chapter_steps: usize,
    pub book_steps: usize,
}

/// The Container node id for one Bible book. `code` must be the CANONICAL `atlas_core::canon::BOOKS`
/// code, never a source alias; the caller resolves it first.
pub fn book_container_id(code: &str) -> ContainerNodeId {
    ContainerNodeId::new(format!("bible-book-{code}"))
}

/// The Container node id for one Bible chapter; `code` must be canonical here too.
pub fn chapter_container_id(code: &str, chapter: u16) -> ContainerNodeId {
    ContainerNodeId::new(format!("bible-chapter-{code}-{chapter}"))
}

/// An exact match against `BOOKS[..].code`, deliberately not `canon::resolve_alias`: the minters only
/// ever emit canonical codes, so decoding must be their strict inverse and not a normalising parse.
fn canonical_book_index(code: &str) -> Option<u8> {
    atlas_core::canon::BOOKS.iter().position(|b| b.code == code).map(|i| i as u8)
}

/// The inverse of `book_container_id`: `None` for anything not shaped like one of this adapter's own
/// book ids, a non-canonical alias spelling included, never a panic.
pub fn decode_book_container(id: &AnyNodeId) -> Option<u8> {
    if id.kind != NodeKind::Container {
        return None;
    }
    let code = id.raw.strip_prefix("bible-book-")?;
    canonical_book_index(code)
}

/// The inverse of `chapter_container_id`, with the same strictness as `decode_book_container`.
pub fn decode_chapter_container(id: &AnyNodeId) -> Option<(u8, u16)> {
    if id.kind != NodeKind::Container {
        return None;
    }
    let rest = id.raw.strip_prefix("bible-chapter-")?;
    // The code itself never contains '-' (BOOKS codes are 3 alphanumerics),
    // so the LAST '-' separates code from chapter.
    let (code, chapter) = rest.rsplit_once('-')?;
    let chapter: u16 = chapter.parse().ok()?;
    let book = canonical_book_index(code)?;
    Some((book, chapter))
}

/// One Container node per book and per chapter, plus the declared containment and succession rows.
/// A verse the canon counts but the parsed verse map lacks is skipped, never fabricated; a chapter
/// whose verses are all absent still gets its node and its rows, but no empty `Loci` row.
pub fn normalize(ctx: &mut BuildCtx) -> anyhow::Result<BibleContainerStats> {
    let mut stats = BibleContainerStats::default();
    let mut all_books: Vec<ContainerNodeId> = Vec::new();
    let mut all_chapters: Vec<ContainerNodeId> = Vec::new();

    for book in &ctx.kjv_canon.books {
        let book_index = atlas_core::canon::resolve_alias(&book.code).map(|id| id.0).ok_or_else(|| {
            anyhow::anyhow!(
                "book code '{}' does not resolve to a canonical book (unreachable: the canon was already validated)",
                book.code
            )
        })?;
        let info = &atlas_core::canon::BOOKS[book_index as usize];

        let book_container = book_container_id(info.code);
        ctx.graph.nodes.insert(
            book_container.erase(),
            Node {
                id: book_container.erase(),
                payload: NodePayload::Container { title: info.name.to_string() },
                provenance: "kjv".to_string(),
            },
        );
        stats.books += 1;

        for (chapter_index, &verse_count) in book.chapters.iter().enumerate() {
            let chapter = (chapter_index + 1) as u16;
            let chapter_container = chapter_container_id(info.code, chapter);
            ctx.graph.nodes.insert(
                chapter_container.erase(),
                Node {
                    id: chapter_container.erase(),
                    payload: NodePayload::Container { title: format!("{} {}", info.name, chapter) },
                    provenance: "kjv".to_string(),
                },
            );
            stats.chapters += 1;

            let mut content: BTreeSet<Locus<BibleTag>> = BTreeSet::new();
            for v in 1..=verse_count {
                // The SAME key format `kjv_adapter::ordered_verses_from_canon` reads -- the source's
                // own code -- so a partial fixture skips exactly the verses that adapter skips.
                let key = format!("{}.{}.{}", book.code, chapter, v);
                if ctx.kjv_verses.contains_key(&key) {
                    content.insert(Locus::whole(VerseRef { book: book_index, chapter, verse: v }));
                }
            }
            stats.verse_loci += content.len();
            if !content.is_empty() {
                ctx.graph.contains_bible.push(Contains {
                    container: chapter_container.clone(),
                    content: ContainerContent::Loci(LocusSet(content)),
                    provenance: ProvenanceId::from("kjv"),
                    justification: Default::default(),
                });
            }

            ctx.graph.contains_bible.push(Contains {
                container: book_container.clone(),
                content: ContainerContent::Container(chapter_container.clone()),
                provenance: ProvenanceId::from("kjv"),
                justification: Default::default(),
            });
            stats.book_chapter_rows += 1;

            all_chapters.push(chapter_container);
        }

        all_books.push(book_container);
    }

    // `all_chapters`/`all_books` come out of the canon walk already in canon order, so `windows(2)`
    // IS the step list: no re-sort, and nothing derived from ids.
    for w in all_chapters.windows(2) {
        ctx.graph.canon_succession.push(CanonSuccession {
            prior: w[0].clone(),
            next: w[1].clone(),
            provenance: ProvenanceId::from("kjv"),
            justification: Default::default(),
        });
        stats.chapter_steps += 1;
    }
    for w in all_books.windows(2) {
        ctx.graph.canon_succession.push(CanonSuccession {
            prior: w[0].clone(),
            next: w[1].clone(),
            provenance: ProvenanceId::from("kjv"),
            justification: Default::default(),
        });
        stats.book_steps += 1;
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{Canon, CanonBook};
    use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
    use atlas_graph_types::explore::{EdgeQuery, Explorable, PositionRef};
    use atlas_graph_types::id::Position;
    use std::collections::HashMap;

    fn tiny_canon() -> Canon {
        Canon {
            books: vec![
                CanonBook { code: "GEN".into(), name: "Genesis".into(), chapters: vec![2, 3] },
                CanonBook { code: "EXO".into(), name: "Exodus".into(), chapters: vec![2] },
            ],
        }
    }

    fn tiny_verses() -> HashMap<String, String> {
        let mut m = HashMap::new();
        for key in ["GEN.1.1", "GEN.1.2", "GEN.2.1", "GEN.2.2", "GEN.2.3", "EXO.1.1", "EXO.1.2"] {
            m.insert(key.to_string(), format!("text of {key}"));
        }
        m
    }

    fn built_ctx_graph() -> atlas_graph_types::graph::Graph {
        let canon = tiny_canon();
        let verses = tiny_verses();
        let atlas = crate::event_world::empty_atlas();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        crate::kjv_adapter::normalize(&mut ctx).unwrap();
        let stats = normalize(&mut ctx).unwrap();
        assert_eq!(
            stats,
            BibleContainerStats { books: 2, chapters: 3, verse_loci: 7, book_chapter_rows: 3, chapter_steps: 2, book_steps: 1 }
        );
        let mut graph = std::mem::take(&mut ctx.graph);
        graph.build_indexes();
        graph
    }

    #[test]
    fn container_ids_round_trip_through_their_decoders() {
        let b = book_container_id("GEN").erase();
        assert_eq!(b.raw, "bible-book-GEN");
        assert_eq!(decode_book_container(&b), Some(0));
        let c = chapter_container_id("1SA", 12).erase();
        assert_eq!(c.raw, "bible-chapter-1SA-12");
        assert_eq!(decode_chapter_container(&c), Some((8, 12)));
        assert_eq!(decode_book_container(&AnyNodeId { kind: NodeKind::TextUnit, raw: "bible-book-GEN".into() }), None);
        assert_eq!(decode_chapter_container(&AnyNodeId { kind: NodeKind::Container, raw: "bible-chapter-XXX-1".into() }), None);
        assert_eq!(decode_chapter_container(&AnyNodeId { kind: NodeKind::Container, raw: "bible-chapter-GEN-".into() }), None);
        assert_eq!(decode_book_container(&AnyNodeId { kind: NodeKind::Container, raw: "concord-doc-small-catechism".into() }), None);
    }

    #[test]
    fn decoders_reject_alias_spellings_the_minter_never_emits() {
        for raw in ["bible-book-Genesis", "bible-book-Gen", "bible-book-gen"] {
            assert_eq!(
                decode_book_container(&AnyNodeId { kind: NodeKind::Container, raw: raw.into() }),
                None,
                "{raw} is not a mintable id and must not decode"
            );
        }
        assert_eq!(decode_chapter_container(&AnyNodeId { kind: NodeKind::Container, raw: "bible-chapter-1Sam-12".into() }), None);
        assert_eq!(decode_chapter_container(&AnyNodeId { kind: NodeKind::Container, raw: "bible-chapter-Genesis-1".into() }), None);
    }

    #[test]
    fn normalize_titles_come_from_the_canonical_books_table() {
        let graph = built_ctx_graph();
        let book = graph.nodes.get(&book_container_id("GEN").erase()).expect("book container node");
        match &book.payload {
            NodePayload::Container { title } => assert_eq!(title, "Genesis"),
            other => panic!("expected Container, got {other:?}"),
        }
        let ch = graph.nodes.get(&chapter_container_id("GEN", 2).erase()).expect("chapter container node");
        match &ch.payload {
            NodePayload::Container { title } => assert_eq!(title, "Genesis 2"),
            other => panic!("expected Container, got {other:?}"),
        }
    }

    #[test]
    fn chapter_contains_rows_reach_their_verses_through_the_generic_port() {
        let graph = built_ctx_graph();
        let forward = EdgeKind::Directed(RelationId::Contains, Direction::Forward);
        let page = PositionRef(Position::Node(chapter_container_id("GEN", 2).erase()))
            .edges(&graph, &EdgeQuery { kind: forward, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 3, "GEN 2's own frontier lists its three verses");
        assert_eq!(
            page.entries[0].node,
            Position::Node(crate::kjv_adapter::verse_node_id(0, 2, 1)),
            "the SAME TextUnit node id the KJV adapter minted -- one identity, both adapters"
        );

        let inverse = EdgeKind::Directed(RelationId::Contains, Direction::Inverse);
        let back = PositionRef(Position::Node(crate::kjv_adapter::verse_node_id(0, 2, 1)))
            .edges(&graph, &EdgeQuery { kind: inverse, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.entries[0].node, Position::Node(chapter_container_id("GEN", 2).erase()));
    }

    #[test]
    fn book_membership_rows_serve_both_directions() {
        let graph = built_ctx_graph();
        let forward = EdgeKind::Directed(RelationId::Contains, Direction::Forward);
        let page = PositionRef(Position::Node(book_container_id("GEN").erase()))
            .edges(&graph, &EdgeQuery { kind: forward, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 2, "Genesis' own Members frontier lists its two chapters");
        assert_eq!(page.entries[0].node, Position::Node(chapter_container_id("GEN", 1).erase()));
        assert_eq!(page.entries[1].node, Position::Node(chapter_container_id("GEN", 2).erase()));

        let inverse = EdgeKind::Directed(RelationId::Contains, Direction::Inverse);
        let back = PositionRef(Position::Node(chapter_container_id("GEN", 1).erase()))
            .edges(&graph, &EdgeQuery { kind: inverse, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1, "a chapter is a member of exactly its book");
        assert_eq!(back.entries[0].node, Position::Node(book_container_id("GEN").erase()));
    }

    #[test]
    fn chapter_succession_rows_cross_the_book_boundary_in_canon_order() {
        let graph = built_ctx_graph();
        let follows = EdgeKind::Directed(RelationId::Succession, Direction::Forward);

        let p1 = PositionRef(Position::Node(chapter_container_id("GEN", 1).erase()))
            .edges(&graph, &EdgeQuery { kind: follows, cursor: None, limit: 10 });
        assert_eq!(p1.entries.len(), 1);
        assert_eq!(p1.entries[0].node, Position::Node(chapter_container_id("GEN", 2).erase()));

        let p2 = PositionRef(Position::Node(chapter_container_id("GEN", 2).erase()))
            .edges(&graph, &EdgeQuery { kind: follows, cursor: None, limit: 10 });
        assert_eq!(p2.entries.len(), 1);
        assert_eq!(p2.entries[0].node, Position::Node(chapter_container_id("EXO", 1).erase()));

        let precedes = EdgeKind::Directed(RelationId::Succession, Direction::Inverse);
        let back = PositionRef(Position::Node(chapter_container_id("EXO", 1).erase()))
            .edges(&graph, &EdgeQuery { kind: precedes, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.entries[0].node, Position::Node(chapter_container_id("GEN", 2).erase()));

        let last = PositionRef(Position::Node(chapter_container_id("EXO", 1).erase()))
            .edges(&graph, &EdgeQuery { kind: follows, cursor: None, limit: 10 });
        assert_eq!(last.entries.len(), 0);
    }

    #[test]
    fn book_succession_rows_chain_in_canon_order() {
        let graph = built_ctx_graph();
        let follows = EdgeKind::Directed(RelationId::Succession, Direction::Forward);
        let p = PositionRef(Position::Node(book_container_id("GEN").erase()))
            .edges(&graph, &EdgeQuery { kind: follows, cursor: None, limit: 10 });
        assert_eq!(p.entries.len(), 1);
        assert_eq!(p.entries[0].node, Position::Node(book_container_id("EXO").erase()));
    }

    #[test]
    fn an_empty_canon_is_a_true_no_op() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = normalize(&mut ctx).unwrap();
        assert_eq!(stats, BibleContainerStats::default());
        assert!(ctx.graph.contains_bible.is_empty());
        assert!(ctx.graph.canon_succession.is_empty());
    }

    #[test]
    fn absent_verses_are_skipped_never_fabricated() {
        let canon = Canon { books: vec![CanonBook { code: "GEN".into(), name: "Genesis".into(), chapters: vec![3] }] };
        let mut verses = HashMap::new();
        verses.insert("GEN.1.1".to_string(), "v1".to_string());
        verses.insert("GEN.1.2".to_string(), "v2".to_string());
        let atlas = crate::event_world::empty_atlas();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = normalize(&mut ctx).unwrap();
        assert_eq!(stats.verse_loci, 2);
        assert_eq!(ctx.graph.contains_bible.len(), 2);
        let loci_rows: Vec<_> = ctx
            .graph
            .contains_bible
            .iter()
            .filter_map(|r| match &r.content {
                ContainerContent::Loci(set) => Some(set.0.len()),
                ContainerContent::Container(_) => None,
            })
            .collect();
        assert_eq!(loci_rows, vec![2]);
    }
}
