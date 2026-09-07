//! Batch NODE-1 real-data law: books and chapters are NODES over the real
//! committed corpus (owner orders, `progress.md` 2026-09-07 -- see
//! `bible_container_adapter.rs`'s own module doc comment for the verbatim
//! charter). Pins the real canon's own container population (66 books,
//! 1,189 chapters, 31,102 verse loci across the chapter `Contains` rows)
//! and walks the owner's own named frontiers -- Members (what a container
//! holds) and prev/next navigation (Succession, ACROSS book boundaries:
//! GEN.50 -> EXO.1, the disclosed boundary choice) -- through the SAME
//! generic edge port every other relation already rides.
//!
//! Same per-file real-graph construction `frontier_falsifiability.rs` /
//! `kretzmann_adapter_real_data.rs` already establish (deliberately
//! duplicated rather than shared -- each `_real_data.rs` file in this
//! crate builds its own real graph; see those files' own headers), PLUS
//! the NODE-1 derived-edges step (`add_derived_membership_and_succession`),
//! since the membership/succession assertions below walk derived entries.

use std::path::Path;

use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::explore::{EdgeQuery, Explorable, PositionRef};
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;

fn real_atlas_data() -> atlas_core::data::AtlasData {
    static CACHED: std::sync::OnceLock<atlas_core::data::AtlasData> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile")
                .data
        })
        .clone()
}

fn real_graph() -> &'static atlas_graph_types::graph::Graph {
    static GRAPH: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
        let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
        let atlas = real_atlas_data();
        let (mut graph, ..) = atlas_graph::build::build_graph_from_sources_with_eras(&kjv_json, &xrefs_tsv, &atlas, &atlas.eras)
            .expect("the real committed sources must build");
        graph.build_indexes();
        atlas_graph::bible_container_adapter::add_derived_membership_and_succession(&mut graph);
        graph
    })
}

fn chapter(code: &str, n: u16) -> Position {
    Position::Node(atlas_graph::bible_container_adapter::chapter_container_id(code, n).erase())
}

fn book(code: &str) -> Position {
    Position::Node(atlas_graph::bible_container_adapter::book_container_id(code).erase())
}

fn edges(g: &atlas_graph_types::graph::Graph, p: &Position, kind: EdgeKind, limit: usize) -> Vec<Position> {
    PositionRef(p.clone()).edges(g, &EdgeQuery { kind, cursor: None, limit }).entries.into_iter().map(|e| e.node).collect()
}

const CONTAINS: EdgeKind = EdgeKind::Directed(RelationId::Contains, Direction::Forward);
const MEMBER_OF: EdgeKind = EdgeKind::Directed(RelationId::Contains, Direction::Inverse);
const FOLLOWS: EdgeKind = EdgeKind::Directed(RelationId::Succession, Direction::Forward);
const PRECEDES: EdgeKind = EdgeKind::Directed(RelationId::Succession, Direction::Inverse);

/// The real canon's container population, pinned: 66 books + 1,189
/// chapters, every one a real `Container` node; 1,189 chapter `Contains`
/// rows carrying 31,102 verse loci (one per KJV verse).
#[test]
fn the_real_canon_mints_66_books_and_1189_chapters_with_31102_verse_loci() {
    let g = real_graph();
    let mut books = 0usize;
    let mut chapters = 0usize;
    for id in g.nodes.keys() {
        if atlas_graph::bible_container_adapter::decode_book_container(id).is_some() {
            books += 1;
        } else if atlas_graph::bible_container_adapter::decode_chapter_container(id).is_some() {
            chapters += 1;
        }
    }
    assert_eq!(books, 66);
    assert_eq!(chapters, 1189);
    assert_eq!(g.contains_bible.len(), 1189, "one Contains row per chapter");
    let loci: usize = g.contains_bible.iter().map(|r| r.content.0.len()).sum();
    assert_eq!(loci, 31_102, "one verse locus per KJV verse");
}

/// A chapter node resolves like any other node, and its label is the
/// reader's own display name (canon::BOOKS name + chapter).
#[test]
fn chapter_and_book_nodes_carry_reader_display_titles() {
    let g = real_graph();
    let jhn3 = g.node(&atlas_graph::bible_container_adapter::chapter_container_id("JHN", 3).erase()).expect("JHN 3 container node");
    match &jhn3.payload {
        atlas_graph_types::node::NodePayload::Container { title } => assert_eq!(title, "John 3"),
        other => panic!("expected Container payload, got {other:?}"),
    }
    let sng = g.node(&atlas_graph::bible_container_adapter::book_container_id("SNG").erase()).expect("SNG book container node");
    match &sng.payload {
        atlas_graph_types::node::NodePayload::Container { title } => assert_eq!(title, "Song of Solomon"),
        other => panic!("expected Container payload, got {other:?}"),
    }
}

/// The owner's charter, walked: a chapter's frontier IS its verses (the
/// Members capability's `Contains`-forward edge), through the same
/// generic port.
#[test]
fn a_chapter_contains_its_verses_and_a_verse_is_a_member_of_its_chapter() {
    let g = real_graph();
    let gen1 = edges(g, &chapter("GEN", 1), CONTAINS, 100);
    assert_eq!(gen1.len(), 31, "Genesis 1 has 31 verses");
    assert_eq!(gen1[0], Position::Node(atlas_graph::kjv_adapter::verse_node_id(0, 1, 1)), "the SAME TextUnit node the KJV adapter minted");

    let psa119 = edges(g, &chapter("PSA", 119), CONTAINS, 200);
    assert_eq!(psa119.len(), 176, "Psalm 119 has 176 verses");

    // Inverse: JHN.3.16's own member-of frontier names John 3.
    let jhn316 = Position::Node(atlas_graph::kjv_adapter::verse_node_id(42, 3, 16));
    let back = edges(g, &jhn316, MEMBER_OF, 10);
    assert_eq!(back, vec![chapter("JHN", 3)]);
}

/// Book membership, both directions (derived -- see the adapter's doc
/// comment for why book -> chapter cannot be an authored row).
#[test]
fn a_book_contains_its_chapters_and_a_chapter_is_a_member_of_its_book() {
    let g = real_graph();
    let gen = edges(g, &book("GEN"), CONTAINS, 100);
    assert_eq!(gen.len(), 50, "Genesis has 50 chapters");
    assert_eq!(gen[0], chapter("GEN", 1));
    assert_eq!(gen[49], chapter("GEN", 50));

    let psa = edges(g, &book("PSA"), CONTAINS, 200);
    assert_eq!(psa.len(), 150, "Psalms has 150 chapters");

    let back = edges(g, &chapter("GEN", 50), MEMBER_OF, 10);
    assert_eq!(back, vec![book("GEN")]);
}

/// The owner's "previous/next chapter navigation" as graph edges, and the
/// disclosed boundary choice: canon order ACROSS book boundaries.
#[test]
fn chapter_succession_navigates_within_and_across_book_boundaries() {
    let g = real_graph();
    // Within-book.
    assert_eq!(edges(g, &chapter("GEN", 1), FOLLOWS, 10), vec![chapter("GEN", 2)]);
    // The brief's own named boundary: GEN.50 -> EXO.1.
    assert_eq!(edges(g, &chapter("GEN", 50), FOLLOWS, 10), vec![chapter("EXO", 1)]);
    // The testaments' own seam: MAL.4 -> MAT.1 (canon order, no special case).
    assert_eq!(edges(g, &chapter("MAL", 4), FOLLOWS, 10), vec![chapter("MAT", 1)]);
    // Inverse reading: EXO.1 precedes-in names GEN.50 back.
    assert_eq!(edges(g, &chapter("EXO", 1), PRECEDES, 10), vec![chapter("GEN", 50)]);
    // Honest ends: GEN.1 has no predecessor, REV.22 no successor.
    assert_eq!(edges(g, &chapter("GEN", 1), PRECEDES, 10), Vec::<Position>::new());
    assert_eq!(edges(g, &chapter("REV", 22), FOLLOWS, 10), Vec::<Position>::new());
}

/// Book succession in canon order, with honest ends.
#[test]
fn book_succession_chains_the_canon() {
    let g = real_graph();
    assert_eq!(edges(g, &book("GEN"), FOLLOWS, 10), vec![book("EXO")]);
    assert_eq!(edges(g, &book("MAL"), FOLLOWS, 10), vec![book("MAT")]);
    assert_eq!(edges(g, &book("GEN"), PRECEDES, 10), Vec::<Position>::new());
    assert_eq!(edges(g, &book("REV"), FOLLOWS, 10), Vec::<Position>::new());
}
