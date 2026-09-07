//! Batch NODE-1: books and chapters become nodes (owner orders, verbatim,
//! `progress.md` 2026-09-07: "chapters and books are nodes, though.
//! chapters are nodes and their frontier basically is the verses and event
//! containers they contain, as well as previous/next chapter navigation
//! and whatever is associated on the map side, etc... they're not special
//! cases they just have their own presentation."; "first thing I want you
//! to do is make sure books and chapters are nodes."; "if that's not in
//! code, it needs to be").
//!
//! Mints one `Container` node per Bible book (66 over the real canon) and
//! per chapter (1,189), plus the Bible-corpus `Contains<BibleTag>` rows
//! (chapter ⊃ its verses -- ~31,102 loci over the real canon), following
//! `concord_adapter.rs`'s own container-minting idiom EXACTLY (the
//! precedent the NODE-1 brief names): `ContainerNodeId::new` with a
//! stable, internal, never-displayed raw id; the DISPLAY name is the
//! node's own `NodePayload::Container.title` -- "names are refs, not
//! identity" (`kjv_adapter::dot_ref`'s own doc comment names this same
//! discipline).
//!
//! NODE IDENTITY: a book container's raw id is `"bible-book-{CODE}"` and a
//! chapter container's is `"bible-chapter-{CODE}-{chapter}"`, where
//! `{CODE}` is the CANONICAL `atlas_core::canon::BOOKS` code (`GEN`,
//! `1SA`, ...) -- the same stable book-identity convention every dot-ref
//! in this app already rides -- mirroring Concord's own
//! `"concord-doc-{key}"` / `"concord-art-{key}-{article}"` grammar one for
//! one. TITLES come from `atlas_core::canon::BOOKS[..].name` -- the SAME
//! table `atlas_core::refs::BookId::name()` reads, i.e. the display name
//! `ChapterOut.book` already serves the reader (one naming truth, no new
//! name table).
//!
//! CONTAINMENT SHAPE, disclosed: `Contains<C>`'s `content` is a
//! `LocusSet<C>` -- a FLAT set of TEXT loci ("no container-of-containers
//! nesting in the type", `concord_adapter.rs`'s own TWO-TIER doc comment)
//! -- so "book ⊃ its chapters" is structurally unrepresentable as an
//! authored `Contains` row. Chapter ⊃ verses IS authored as rows (the
//! artifact's own `contains_bible` table, serialized as of NODE-1);
//! book ⊃ chapter membership and the chapter/book Succession chains are
//! DERIVED index entries instead -- see
//! [`add_derived_membership_and_succession`] below and its own doc
//! comment for the full reasoning.

use std::collections::BTreeSet;

use atlas_graph_types::edge::Contains;
use atlas_graph_types::id::{AnyNodeId, ContainerNodeId, NodeKind};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{BibleTag, Locus, LocusSet, VerseRef};

use crate::pipeline::BuildCtx;

#[derive(Debug, Clone, Copy, Default)]
pub struct BibleContainerStats {
    pub books: usize,
    pub chapters: usize,
    /// Total verse loci across every chapter `Contains` row (31,102 over
    /// the real canon -- one locus per KJV verse).
    pub verse_loci: usize,
}

/// The Container node id for one Bible book -- see module doc comment for
/// the grammar. `code` must be the CANONICAL `atlas_core::canon::BOOKS`
/// code, not a source alias (the caller resolves; see `normalize`).
pub fn book_container_id(code: &str) -> ContainerNodeId {
    ContainerNodeId::new(format!("bible-book-{code}"))
}

/// The Container node id for one Bible chapter -- same grammar note as
/// `book_container_id`.
pub fn chapter_container_id(code: &str, chapter: u16) -> ContainerNodeId {
    ContainerNodeId::new(format!("bible-chapter-{code}-{chapter}"))
}

/// The inverse of `book_container_id` -- the canonical book index, `None`
/// for anything not shaped like one of this adapter's own book ids
/// (mirrors `kjv_adapter::decode_text_unit` exactly: wrong kind or a
/// malformed/unknown raw string is `None`, never a panic).
pub fn decode_book_container(id: &AnyNodeId) -> Option<u8> {
    if id.kind != NodeKind::Container {
        return None;
    }
    let code = id.raw.strip_prefix("bible-book-")?;
    atlas_core::canon::resolve_alias(code).map(|b| b.0)
}

/// The inverse of `chapter_container_id` -- `(canonical book index,
/// chapter)`, `None` for anything not shaped like one of this adapter's
/// own chapter ids.
pub fn decode_chapter_container(id: &AnyNodeId) -> Option<(u8, u16)> {
    if id.kind != NodeKind::Container {
        return None;
    }
    let rest = id.raw.strip_prefix("bible-chapter-")?;
    // The code itself never contains '-' (BOOKS codes are 3 alphanumerics),
    // so the LAST '-' separates code from chapter.
    let (code, chapter) = rest.rsplit_once('-')?;
    let chapter: u16 = chapter.parse().ok()?;
    let book = atlas_core::canon::resolve_alias(code)?.0;
    Some((book, chapter))
}

/// Pipeline-facing NORMALIZE entry point: walks `ctx.kjv_canon` into one
/// Container node per book and per chapter, plus one Bible-corpus
/// `Contains` row per chapter (its own verses) -- self-contained (reads
/// only the canon + verse map, no other pass's output), the SAME
/// NORMALIZE-eligibility `kjv_adapter::normalize`/`concord_adapter::
/// normalize` already have. A verse counted by the canon but absent from
/// the parsed verse map is skipped, never fabricated (the SAME
/// out-of-canon tolerance `kjv_adapter::ordered_verses_from_canon`
/// documents); a chapter whose verses are ALL absent still gets its
/// Container node (the canon structure is real) but no empty `Contains`
/// row.
pub fn normalize(ctx: &mut BuildCtx) -> anyhow::Result<BibleContainerStats> {
    let mut stats = BibleContainerStats::default();
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
                // The SAME key format `kjv_adapter::ordered_verses_from_canon`
                // reads -- the source's own code, so a partial fixture skips
                // exactly the verses that adapter skips.
                let key = format!("{}.{}.{}", book.code, chapter, v);
                if ctx.kjv_verses.contains_key(&key) {
                    content.insert(Locus::whole(VerseRef { book: book_index, chapter, verse: v }));
                }
            }
            stats.verse_loci += content.len();
            if !content.is_empty() {
                ctx.graph.contains_bible.push(Contains {
                    container: chapter_container,
                    content: LocusSet(content),
                    provenance: ProvenanceId::from("kjv"),
                    justification: Default::default(),
                });
            }
        }
    }
    Ok(stats)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DerivedContainerEdges {
    /// book -> chapter `Contains` entries (forward; "member-of" inverse).
    pub book_chapter_membership: usize,
    /// chapter -> next chapter `Succession` entries, canon order, ACROSS
    /// book boundaries (GEN.50 -> EXO.1 -- see the doc comment below).
    pub chapter_succession: usize,
    /// book -> next book `Succession` entries, canon order.
    pub book_succession: usize,
}

/// NODE-1's derived container edges -- "previous/next chapter navigation"
/// (owner's words) made queryable graph edges, plus book ⊃ chapter
/// membership. A REAL POST-PROCESSING STEP over the already-built graph,
/// NOT a graph-types change -- the exact `event_world::add_justified_by`
/// precedent (its own doc comment: "merges ... into `graph.indexes` using
/// graph-types' own public `entry_id`/`BiIndex` primitives"), and called
/// at the SAME sites, immediately after it. Must run AFTER
/// `graph.build_indexes()` (that call REBUILDS `graph.indexes` from the
/// row tables from scratch, so anything merged in beforehand is wiped).
///
/// WHY DERIVED, NOT AUTHORED ROWS (disclosed, the NODE-1 report carries
/// the full version):
/// - `Succession`'s row type is `chain: Vec<EventId>` -- event-typed by
///   construction. A chapter/book chain is structurally unrepresentable
///   without a graph-types shape change, which NODE-1's own brief forbids
///   ("graph-types crate UNTOUCHED"). The chapter/book chains are also a
///   pure, deterministic function of the canon (no curation, no
///   justification to carry): exactly the "computed relations with no
///   authored-row counterpart" class `pipeline.rs`'s own DERIVE doc
///   comment names.
/// - `Contains`' `content` is a flat `LocusSet` of TEXT loci -- a book
///   row cannot name its chapter CONTAINERS (see the module doc comment).
///   Book ⊃ chapter membership is likewise a pure function of the ids.
///
/// Both edge families are therefore derived from the graph's OWN
/// Container nodes (minted by `normalize` above from the committed
/// canon), so BOTH load paths -- from-sources and from-artifact -- derive
/// the identical entries from the identical nodes with no second data
/// source, and content-addressed `entry_id`s keep the result
/// deterministic across recompiles (the same determinism law every edge
/// family obeys).
///
/// SUCCESSION BOUNDARY CHOICE (the brief orders its disclosure): chapter
/// chains run in canon order ACROSS book boundaries -- GEN.50's
/// "follows-in" names EXO.1 -- because the owner's "previous/next chapter
/// navigation" is the READER's next chapter, and the reader's next
/// chapter after a book's last is the next book's first (the reading
/// spine's own order at chapter granularity). Books chain book -> next
/// book in the same canon order.
pub fn add_derived_membership_and_succession(graph: &mut atlas_graph_types::graph::Graph) -> DerivedContainerEdges {
    use atlas_graph_types::edge::{at, BiIndex, RelationId as R};
    use atlas_graph_types::explore::EdgeMeta as M;
    use atlas_graph_types::id::Position;

    // Collect this graph's own Bible containers, keyed for canon order.
    // `graph.nodes` is a BTreeMap but its STRING order is not canon order
    // ("bible-chapter-GEN-10" < "bible-chapter-GEN-2"), so sort by the
    // decoded numeric keys.
    let mut books: Vec<(u8, AnyNodeId)> = Vec::new();
    let mut chapters: Vec<((u8, u16), AnyNodeId)> = Vec::new();
    for id in graph.nodes.keys() {
        if let Some(b) = decode_book_container(id) {
            books.push((b, id.clone()));
        } else if let Some(bc) = decode_chapter_container(id) {
            chapters.push((bc, id.clone()));
        }
    }
    books.sort_by_key(|(b, _)| *b);
    chapters.sort_by_key(|(bc, _)| *bc);

    let mut contains_pairs: Vec<(Position, Position, M)> = Vec::new();
    for ((book_index, _), chapter_id) in &chapters {
        if let Some((_, book_id)) = books.iter().find(|(b, _)| b == book_index) {
            contains_pairs.push((at(book_id), at(chapter_id), M::None));
        }
    }

    let mut succession_pairs: Vec<(Position, Position, M)> = Vec::new();
    let mut chapter_succession = 0usize;
    for w in chapters.windows(2) {
        succession_pairs.push((at(&w[0].1), at(&w[1].1), M::None));
        chapter_succession += 1;
    }
    let mut book_succession = 0usize;
    for w in books.windows(2) {
        succession_pairs.push((at(&w[0].1), at(&w[1].1), M::None));
        book_succession += 1;
    }

    let out = DerivedContainerEdges {
        book_chapter_membership: contains_pairs.len(),
        chapter_succession,
        book_succession,
    };

    // Merge into the row-lowered indexes. Every key these entries land on
    // is a CONTAINER position no row-lowered entry of the same relation +
    // direction shares (chapter `Contains` rows put chapters in `fwd` and
    // verses in `inv`; these pairs put books in `fwd` and chapters in
    // `inv` -- fully disjoint key/direction sets; Succession's row-lowered
    // keys are all Event positions), so this merge never interleaves with
    // -- or reorders -- an existing entry list.
    for (rel, pairs) in [(R::Contains, contains_pairs), (R::Succession, succession_pairs)] {
        if pairs.is_empty() {
            continue;
        }
        let built = BiIndex::build(rel, &pairs);
        let target = graph.indexes.entry(rel).or_default();
        for (k, v) in built.fwd {
            target.fwd.entry(k).or_default().extend(v);
        }
        for (k, v) in built.inv {
            target.inv.entry(k).or_default().extend(v);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{Canon, CanonBook};
    use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
    use atlas_graph_types::explore::{EdgeQuery, Explorable, PositionRef};
    use atlas_graph_types::id::Position;
    use std::collections::HashMap;

    /// Two books with a real canon shape: GEN (2 chapters) and EXO (1) --
    /// enough to exercise within-book AND across-book succession plus both
    /// membership directions, without the real 31,102-verse canon.
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
        assert_eq!((stats.books, stats.chapters, stats.verse_loci), (2, 3, 7));
        let mut graph = std::mem::take(&mut ctx.graph);
        graph.build_indexes();
        add_derived_membership_and_succession(&mut graph);
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
        // Wrong kinds / malformed raws are None, never a panic.
        assert_eq!(decode_book_container(&AnyNodeId { kind: NodeKind::TextUnit, raw: "bible-book-GEN".into() }), None);
        assert_eq!(decode_chapter_container(&AnyNodeId { kind: NodeKind::Container, raw: "bible-chapter-XXX-1".into() }), None);
        assert_eq!(decode_chapter_container(&AnyNodeId { kind: NodeKind::Container, raw: "bible-chapter-GEN-".into() }), None);
        // Concord's own containers are NOT this adapter's (disjoint grammars).
        assert_eq!(decode_book_container(&AnyNodeId { kind: NodeKind::Container, raw: "concord-doc-small-catechism".into() }), None);
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

        // Inverse: a verse's own 'member-of' frontier names its chapter.
        let inverse = EdgeKind::Directed(RelationId::Contains, Direction::Inverse);
        let back = PositionRef(Position::Node(crate::kjv_adapter::verse_node_id(0, 2, 1)))
            .edges(&graph, &EdgeQuery { kind: inverse, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.entries[0].node, Position::Node(chapter_container_id("GEN", 2).erase()));
    }

    #[test]
    fn book_membership_is_derived_both_directions() {
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
    fn chapter_succession_crosses_the_book_boundary_in_canon_order() {
        let graph = built_ctx_graph();
        let follows = EdgeKind::Directed(RelationId::Succession, Direction::Forward);

        // Within-book: GEN 1 -> GEN 2.
        let p1 = PositionRef(Position::Node(chapter_container_id("GEN", 1).erase()))
            .edges(&graph, &EdgeQuery { kind: follows, cursor: None, limit: 10 });
        assert_eq!(p1.entries.len(), 1);
        assert_eq!(p1.entries[0].node, Position::Node(chapter_container_id("GEN", 2).erase()));

        // ACROSS the boundary: GEN 2 (this fixture's own GEN.50) -> EXO 1.
        let p2 = PositionRef(Position::Node(chapter_container_id("GEN", 2).erase()))
            .edges(&graph, &EdgeQuery { kind: follows, cursor: None, limit: 10 });
        assert_eq!(p2.entries.len(), 1);
        assert_eq!(p2.entries[0].node, Position::Node(chapter_container_id("EXO", 1).erase()));

        // Inverse reading: EXO 1 'precedes-in' names GEN 2 back.
        let precedes = EdgeKind::Directed(RelationId::Succession, Direction::Inverse);
        let back = PositionRef(Position::Node(chapter_container_id("EXO", 1).erase()))
            .edges(&graph, &EdgeQuery { kind: precedes, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.entries[0].node, Position::Node(chapter_container_id("GEN", 2).erase()));

        // The last chapter of the last book has no successor (honest end).
        let last = PositionRef(Position::Node(chapter_container_id("EXO", 1).erase()))
            .edges(&graph, &EdgeQuery { kind: follows, cursor: None, limit: 10 });
        assert_eq!(last.entries.len(), 0);
    }

    #[test]
    fn book_succession_chains_in_canon_order() {
        let graph = built_ctx_graph();
        let follows = EdgeKind::Directed(RelationId::Succession, Direction::Forward);
        let p = PositionRef(Position::Node(book_container_id("GEN").erase()))
            .edges(&graph, &EdgeQuery { kind: follows, cursor: None, limit: 10 });
        assert_eq!(p.entries.len(), 1);
        assert_eq!(p.entries[0].node, Position::Node(book_container_id("EXO").erase()));
    }

    #[test]
    fn derived_edge_counts_are_reported_honestly() {
        let canon = tiny_canon();
        let verses = tiny_verses();
        let atlas = crate::event_world::empty_atlas();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        crate::kjv_adapter::normalize(&mut ctx).unwrap();
        normalize(&mut ctx).unwrap();
        let mut graph = std::mem::take(&mut ctx.graph);
        graph.build_indexes();
        let derived = add_derived_membership_and_succession(&mut graph);
        assert_eq!(
            derived,
            DerivedContainerEdges { book_chapter_membership: 3, chapter_succession: 2, book_succession: 1 }
        );
    }

    #[test]
    fn an_empty_canon_is_a_true_no_op() {
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = normalize(&mut ctx).unwrap();
        assert_eq!((stats.books, stats.chapters, stats.verse_loci), (0, 0, 0));
        assert!(ctx.graph.contains_bible.is_empty());
        let mut graph = std::mem::take(&mut ctx.graph);
        graph.build_indexes();
        let derived = add_derived_membership_and_succession(&mut graph);
        assert_eq!(derived, DerivedContainerEdges::default());
    }

    #[test]
    fn absent_verses_are_skipped_never_fabricated() {
        // A canon that counts a verse the parsed verse map does not carry
        // (GEN.1 claims 3 verses; only 2 exist) -- the row holds 2 loci.
        let canon = Canon { books: vec![CanonBook { code: "GEN".into(), name: "Genesis".into(), chapters: vec![3] }] };
        let mut verses = HashMap::new();
        verses.insert("GEN.1.1".to_string(), "v1".to_string());
        verses.insert("GEN.1.2".to_string(), "v2".to_string());
        let atlas = crate::event_world::empty_atlas();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = normalize(&mut ctx).unwrap();
        assert_eq!(stats.verse_loci, 2);
        assert_eq!(ctx.graph.contains_bible.len(), 1);
        assert_eq!(ctx.graph.contains_bible[0].content.0.len(), 2);
    }
}
