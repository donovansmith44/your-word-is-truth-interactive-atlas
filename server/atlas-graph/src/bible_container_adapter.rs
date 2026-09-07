//! Batch NODE-1: books and chapters become nodes (owner orders, verbatim,
//! `progress.md` 2026-09-07: "chapters and books are nodes, though.
//! chapters are nodes and their frontier basically is the verses and event
//! containers they contain, as well as previous/next chapter navigation
//! and whatever is associated on the map side, etc... they're not special
//! cases they just have their own presentation."; "first thing I want you
//! to do is make sure books and chapters are nodes."; "if that's not in
//! code, it needs to be").
//!
//! NODE1-ROWS-1 (fix round 1; owner ruling verbatim: "derived index
//! entries, not authored edge rows yeah not good. we have to declare
//! edges. no special cases."): EVERY container edge this adapter is
//! responsible for is now a DECLARED, artifact-serialized row --
//!   - chapter ⊃ its verses: one `Contains<BibleTag>` row per chapter,
//!     `ContainerContent::Loci` (1,189 rows / 31,102 loci over the real
//!     canon);
//!   - book ⊃ chapter: one `Contains<BibleTag>` row PER CHILD,
//!     `ContainerContent::Container` (1,189 rows) -- an edge, not a list
//!     (owner correction at sign-off: "edges are PAIRWISE rows");
//!   - previous/next navigation: one `CanonSuccession` row per canon
//!     step, PAIRWISE -- 1,188 chapter steps (ACROSS book boundaries:
//!     GEN.50 -> EXO.1, MAL.4 -> MAT.1 -- the reader's own next chapter)
//!     and 65 book steps.
//! The original batch's post-index derived-entry merge
//! (`add_derived_membership_and_succession`) is DELETED: indexes derive
//! FROM rows in `Graph::build_indexes`, never the reverse (which also
//! dissolves review findings M-2/M-3 structurally -- there is no merge
//! left to be non-idempotent and no second derivation step to forget at
//! a call site). The container-containment rows are gated by
//! `law_check::container_containment_is_a_forest` (acyclicity +
//! single-parent, fail-loud at build time) per the owner's recursion
//! addendum ("are nodes all recursively defined? they should be.").
//!
//! Mints one `Container` node per Bible book (66 over the real canon) and
//! per chapter (1,189), following `concord_adapter.rs`'s own
//! container-minting idiom (the NODE-1 brief's named precedent):
//! `ContainerNodeId::new` with a stable, internal, never-displayed raw
//! id; the DISPLAY name is the node's own `NodePayload::Container.title`
//! -- "names are refs, not identity" (`kjv_adapter::dot_ref`'s own doc
//! comment names this same discipline).
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
//! name table). The decoders below are the STRICT inverse of the minters
//! (fix round 1, review L-5): only the canonical code decodes -- an
//! OSIS/full-name alias spelling this adapter can never mint is rejected,
//! so decode∘encode is a real round trip, not a normalizing parse.
//!
//! CANON-ORDER AUTHORITY (fix round 1, review M-6): the order behind
//! every `CanonSuccession` row -- "what comes after Malachi 4" -- is
//! `atlas_core::canon::BOOKS`' own array order; see THAT module's doc
//! comment (added the same fix round) for what the order is and where it
//! comes from.

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
    /// Total verse loci across the chapter `Contains` rows (31,102 over
    /// the real canon -- one locus per KJV verse).
    pub verse_loci: usize,
    /// book ⊃ chapter `Contains` rows (`ContainerContent::Container`,
    /// one per child -- NODE1-ROWS-1).
    pub book_chapter_rows: usize,
    /// chapter -> next-chapter `CanonSuccession` rows (canon order,
    /// across book boundaries).
    pub chapter_steps: usize,
    /// book -> next-book `CanonSuccession` rows (canon order).
    pub book_steps: usize,
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

/// The canonical book index for a CANONICAL code, and only that -- an
/// exact match against `BOOKS[..].code`, deliberately NOT
/// `canon::resolve_alias` (fix round 1, review L-5: the minters only ever
/// emit canonical codes, so the decoders must not accept OSIS/full-name
/// alias spellings -- decode must be encode's strict inverse).
fn canonical_book_index(code: &str) -> Option<u8> {
    atlas_core::canon::BOOKS.iter().position(|b| b.code == code).map(|i| i as u8)
}

/// The inverse of `book_container_id` -- the canonical book index, `None`
/// for anything not shaped like one of this adapter's own book ids
/// (wrong kind, malformed raw, or a non-canonical alias spelling this
/// adapter never mints), never a panic.
pub fn decode_book_container(id: &AnyNodeId) -> Option<u8> {
    if id.kind != NodeKind::Container {
        return None;
    }
    let code = id.raw.strip_prefix("bible-book-")?;
    canonical_book_index(code)
}

/// The inverse of `chapter_container_id` -- `(canonical book index,
/// chapter)`, `None` for anything not shaped like one of this adapter's
/// own chapter ids (same strictness note as `decode_book_container`).
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

/// Pipeline-facing NORMALIZE entry point: walks `ctx.kjv_canon` into one
/// Container node per book and per chapter, plus the DECLARED rows
/// (NODE1-ROWS-1 -- see the module doc comment for the full inventory):
/// chapter ⊃ verses (`Loci`), book ⊃ chapter (`Container`, one row per
/// child), and the pairwise `CanonSuccession` steps for chapters (across
/// book boundaries, canon order) and books. Self-contained (reads only
/// the canon + verse map, no other pass's output), the SAME
/// NORMALIZE-eligibility `kjv_adapter::normalize`/`concord_adapter::
/// normalize` already have. A verse counted by the canon but absent from
/// the parsed verse map is skipped, never fabricated (the SAME
/// out-of-canon tolerance `kjv_adapter::ordered_verses_from_canon`
/// documents); a chapter whose verses are ALL absent still gets its
/// Container node, its book ⊃ chapter row, and its succession steps (the
/// canon structure is real) but no empty `Loci` row.
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
                    container: chapter_container.clone(),
                    content: ContainerContent::Loci(LocusSet(content)),
                    provenance: ProvenanceId::from("kjv"),
                    justification: Default::default(),
                });
            }

            // NODE1-ROWS-1: book ⊃ chapter is a DECLARED edge -- one row
            // per child container, never a list.
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

    // NODE1-ROWS-1: the pairwise canon steps. Chapter steps run in canon
    // order ACROSS book boundaries -- GEN.50 -> EXO.1, MAL.4 -> MAT.1 --
    // because the owner's "previous/next chapter navigation" is the
    // READER's next chapter, and the reader's next chapter after a
    // book's last is the next book's first (the reading spine's own
    // order at chapter granularity). `all_chapters`/`all_books` are
    // already in canon order (the canon walk above), so `windows(2)` IS
    // the step list -- no re-sort, no derivation from ids.
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
        // Wrong kinds / malformed raws are None, never a panic.
        assert_eq!(decode_book_container(&AnyNodeId { kind: NodeKind::TextUnit, raw: "bible-book-GEN".into() }), None);
        assert_eq!(decode_chapter_container(&AnyNodeId { kind: NodeKind::Container, raw: "bible-chapter-XXX-1".into() }), None);
        assert_eq!(decode_chapter_container(&AnyNodeId { kind: NodeKind::Container, raw: "bible-chapter-GEN-".into() }), None);
        // Concord's own containers are NOT this adapter's (disjoint grammars).
        assert_eq!(decode_book_container(&AnyNodeId { kind: NodeKind::Container, raw: "concord-doc-small-catechism".into() }), None);
    }

    /// Fix round 1 (review L-5): the decoders are the STRICT inverse of
    /// the minters -- alias spellings the minter can never emit (OSIS,
    /// full name, case variants) are rejected, not normalized.
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

        // Inverse: a verse's own 'member-of' frontier names its chapter.
        let inverse = EdgeKind::Directed(RelationId::Contains, Direction::Inverse);
        let back = PositionRef(Position::Node(crate::kjv_adapter::verse_node_id(0, 2, 1)))
            .edges(&graph, &EdgeQuery { kind: inverse, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1);
        assert_eq!(back.entries[0].node, Position::Node(chapter_container_id("GEN", 2).erase()));
    }

    /// NODE1-ROWS-1: book membership rides DECLARED `ContainerContent::
    /// Container` rows (one per child), lowered by `Graph::build_indexes`
    /// like every other row -- no post-index derivation step exists any
    /// more.
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

    /// NODE1-ROWS-1: succession rides DECLARED pairwise `CanonSuccession`
    /// rows; the boundary choice (canon order ACROSS book boundaries)
    /// is now row data, still pinned here.
    #[test]
    fn chapter_succession_rows_cross_the_book_boundary_in_canon_order() {
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
        // A canon that counts a verse the parsed verse map does not carry
        // (GEN.1 claims 3 verses; only 2 exist) -- the Loci row holds 2.
        let canon = Canon { books: vec![CanonBook { code: "GEN".into(), name: "Genesis".into(), chapters: vec![3] }] };
        let mut verses = HashMap::new();
        verses.insert("GEN.1.1".to_string(), "v1".to_string());
        verses.insert("GEN.1.2".to_string(), "v2".to_string());
        let atlas = crate::event_world::empty_atlas();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = normalize(&mut ctx).unwrap();
        assert_eq!(stats.verse_loci, 2);
        // Two rows: the chapter's Loci row (2 loci) + the book ⊃ chapter row.
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
