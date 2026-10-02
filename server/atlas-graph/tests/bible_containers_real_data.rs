mod common;

use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::adjacency::{EdgeQuery, Adjacent, PositionRef};
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;

fn real_graph() -> &'static atlas_graph_types::graph::Graph {
    static GRAPH: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| common::kjv_and_atlas_build(&common::real_atlas().eras).0)
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

#[test]
fn the_real_canon_declares_every_container_row() {
    use atlas_graph_types::edge::ContainerContent;
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

    let mut loci_rows = 0usize;
    let mut child_rows = 0usize;
    let mut loci = 0usize;
    for r in &g.contains_bible {
        match &r.content {
            ContainerContent::Loci(set) => {
                loci_rows += 1;
                loci += set.0.len();
            }
            ContainerContent::Container(_) => child_rows += 1,
        }
    }
    assert_eq!(loci_rows, 1189, "one Loci row per chapter");
    assert_eq!(loci, 31_102, "one verse locus per KJV verse");
    assert_eq!(child_rows, 1189 + books, "one book ⊃ chapter row per chapter and one root ⊃ book row per book -- an edge per child, never a list");
    assert_eq!(g.contains_bible.len(), 2378 + books);

    assert_eq!(g.canon_succession.len(), 1253, "1,188 chapter steps + 65 book steps, pairwise");
}

#[test]
fn container_containment_is_a_forest_over_the_real_canon() {
    let g = real_graph();
    atlas_graph::law_check::container_containment_is_a_forest(g).expect("the canon's book ⊃ chapter rows must form a forest");
}

#[test]
fn served_indexes_are_a_pure_function_of_the_declared_rows() {
    let g = real_graph();
    atlas_graph::law_check::indexes_derive_exactly_from_rows(g).expect("indexes must derive exactly from rows");
}

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

#[test]
fn a_chapter_contains_its_verses_and_a_verse_is_a_member_of_its_chapter() {
    let g = real_graph();
    let gen1 = edges(g, &chapter("GEN", 1), CONTAINS, 100);
    assert_eq!(gen1.len(), 31, "Genesis 1 has 31 verses");
    assert_eq!(gen1[0], Position::Node(atlas_graph::kjv_adapter::verse_node_id(0, 1, 1)), "the SAME TextUnit node the KJV adapter minted");

    let psa119 = edges(g, &chapter("PSA", 119), CONTAINS, 200);
    assert_eq!(psa119.len(), 176, "Psalm 119 has 176 verses");

    let jhn316 = Position::Node(atlas_graph::kjv_adapter::verse_node_id(42, 3, 16));
    let back = edges(g, &jhn316, MEMBER_OF, 10);
    assert_eq!(back, vec![chapter("JHN", 3)]);
}

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

#[test]
fn chapter_succession_navigates_within_and_across_book_boundaries() {
    let g = real_graph();
    assert_eq!(edges(g, &chapter("GEN", 1), FOLLOWS, 10), vec![chapter("GEN", 2)]);
    assert_eq!(edges(g, &chapter("GEN", 50), FOLLOWS, 10), vec![chapter("EXO", 1)]);
    assert_eq!(edges(g, &chapter("MAL", 4), FOLLOWS, 10), vec![chapter("MAT", 1)]);
    assert_eq!(edges(g, &chapter("EXO", 1), PRECEDES, 10), vec![chapter("GEN", 50)]);
    assert_eq!(edges(g, &chapter("GEN", 1), PRECEDES, 10), Vec::<Position>::new());
    assert_eq!(edges(g, &chapter("REV", 22), FOLLOWS, 10), Vec::<Position>::new());
}

#[test]
fn book_succession_chains_the_canon() {
    let g = real_graph();
    assert_eq!(edges(g, &book("GEN"), FOLLOWS, 10), vec![book("EXO")]);
    assert_eq!(edges(g, &book("MAL"), FOLLOWS, 10), vec![book("MAT")]);
    assert_eq!(edges(g, &book("GEN"), PRECEDES, 10), Vec::<Position>::new());
    assert_eq!(edges(g, &book("REV"), FOLLOWS, 10), Vec::<Position>::new());
}
