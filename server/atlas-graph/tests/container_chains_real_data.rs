mod common;

use common::OptionalCorpora;

use atlas_graph_types::adjacency::{Adjacent, Cursor, EdgeQuery, PositionRef};
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnyNodeId, Position};
use atlas_graph_types::text::{BibleTag, ConcordTag};

const CONTAINS: EdgeKind = EdgeKind::Directed(RelationId::Contains, Direction::Forward);
const FOLLOWS: EdgeKind = EdgeKind::Directed(RelationId::Succession, Direction::Forward);
const PRECEDES: EdgeKind = EdgeKind::Directed(RelationId::Succession, Direction::Inverse);
const WHOLE_LEVEL: usize = 4096;

fn real_graph() -> &'static Graph {
    static GRAPH: std::sync::OnceLock<Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| common::indexed_raw_graph(OptionalCorpora { kretzmann: false, red_letter: false }))
}

#[test]
fn each_level_of_each_corpus_is_one_chain_in_reading_order() {
    // Arrange
    let g = real_graph();
    let roots = [atlas_graph::corpus_root::corpus_root_id::<BibleTag>().erase(), atlas_graph::corpus_root::corpus_root_id::<ConcordTag>().erase()];
    // Act
    let mut broken: Vec<(String, usize, usize)> = Vec::new();
    for root in &roots {
        let books = neighbours(g, root, CONTAINS);
        let chapters: Vec<AnyNodeId> = books.iter().flat_map(|book| neighbours(g, book, CONTAINS)).collect();
        for (level, containers) in [("book", books), ("chapter", chapters)] {
            let walked = walk(g, &containers[0]);
            if walked != containers {
                broken.push((format!("{} {level}", root.raw), walked.len(), containers.len()));
            }
        }
    }
    // Assert
    assert_eq!(broken, Vec::new());
}

#[test]
fn next_and_prev_are_inverse_at_every_level_of_every_corpus() {
    // Arrange
    let g = real_graph();
    let roots = [atlas_graph::corpus_root::corpus_root_id::<BibleTag>().erase(), atlas_graph::corpus_root::corpus_root_id::<ConcordTag>().erase()];
    // Act
    let mut not_inverse: Vec<String> = Vec::new();
    for root in &roots {
        let books = neighbours(g, root, CONTAINS);
        let chapters: Vec<AnyNodeId> = books.iter().flat_map(|book| neighbours(g, book, CONTAINS)).collect();
        for container in books.iter().chain(&chapters) {
            for next in neighbours(g, container, FOLLOWS) {
                if neighbours(g, &next, PRECEDES) != vec![container.clone()] {
                    not_inverse.push(container.raw.clone());
                }
            }
        }
    }
    // Assert
    assert_eq!(not_inverse, Vec::<String>::new());
}

fn walk(g: &Graph, first: &AnyNodeId) -> Vec<AnyNodeId> {
    let mut walked = vec![first.clone()];
    while let [next] = neighbours(g, walked.last().expect("a walk starts somewhere"), FOLLOWS).as_slice() {
        if walked.contains(next) {
            break;
        }
        walked.push(next.clone());
    }
    walked
}

fn neighbours(g: &Graph, at: &AnyNodeId, kind: EdgeKind) -> Vec<AnyNodeId> {
    PositionRef(Position::Node(at.clone()))
        .edges(g, &EdgeQuery { kind, cursor: Cursor::FIRST, limit: WHOLE_LEVEL })
        .entries
        .into_iter()
        .filter_map(|entry| match entry.node {
            Position::Node(id) => Some(id),
            Position::Edge(_) => None,
        })
        .collect()
}
