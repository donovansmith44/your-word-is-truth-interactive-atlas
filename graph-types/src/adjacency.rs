
use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::edge::{dual, Direction, EdgeId, EdgeKind};
use crate::graph::Graph;
use crate::id::Position;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeQuery {
    pub kind: EdgeKind,
    pub cursor: Option<usize>,
    pub limit: usize,
}

/// An entry can carry the fact that belongs to it -- a succession entry its narrative, a
/// citation entry its rank. The SAME meta is visible from both directions of a row, so the
/// bijection between the two projections extends to meta.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeMeta {
    None,
    Narrative(crate::id::NarrativeId),
    Votes(u32),
    Parentage(crate::edge::Parentage),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeEntry {
    pub edge: EdgeId,
    pub node: Position,
    pub meta: EdgeMeta,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgePage {
    pub kind: EdgeKind,
    pub entries: Vec<EdgeEntry>,
    pub previous: Option<usize>,
    pub next: Option<usize>,
}

pub type EdgeSummary = BTreeMap<EdgeKind, usize>;

/// One page of node ids of one kind, in id byte order. `next` is `Some(cursor + ids.len())`
/// exactly when more remain, including at `limit = 0`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodePage {
    pub ids: Vec<crate::id::AnyNodeId>,
    pub next: Option<usize>,
}

/// The target node is already fetched, and absent when the target is itself an edge. No
/// `PartialEq`: a node carries `f64` payload fields, so comparison goes through its debug form.
#[derive(Clone, Debug)]
pub struct EdgeEntryWithNode {
    pub entry: EdgeEntry,
    pub node: Option<crate::node::Node>,
}

#[derive(Clone, Debug)]
pub struct EdgePageWithNodes {
    pub kind: EdgeKind,
    pub entries: Vec<EdgeEntryWithNode>,
    pub next: Option<usize>,
}

pub trait Adjacent {
    fn edge_summary(&self, g: &Graph) -> EdgeSummary;
    fn edges(&self, g: &Graph, q: &EdgeQuery) -> EdgePage;
}

/// The generic position handle every surface consumes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PositionRef(pub Position);

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Adjacency {
    rows: Vec<EdgeEntry>,
    edges: Vec<u32>,
}

impl Adjacency {
    pub fn of_rows(rows: Vec<EdgeEntry>) -> Adjacency {
        let edges = first_row_of_each_edge(&rows);
        Adjacency { rows, edges }
    }

    pub fn append(&mut self, mut later: Adjacency) {
        self.rows.append(&mut later.rows);
        self.edges = first_row_of_each_edge(&self.rows);
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn edges(&self) -> impl Iterator<Item = &EdgeEntry> + '_ {
        self.edges.iter().map(move |&ord| &self.rows[ord as usize])
    }

    pub fn page(&self, q: &EdgeQuery) -> EdgePage {
        let from = self.edges.partition_point(|&ord| (ord as usize) < q.cursor.unwrap_or(0));
        let entries = self.edges[from..].iter().take(q.limit).map(|&ord| self.rows[ord as usize].clone()).collect();
        let next = self.edges.get(from.saturating_add(q.limit)).map(|&ord| ord as usize);
        let before = from.saturating_sub(q.limit);
        let previous = (before > 0 && q.limit > 0).then(|| self.edges[before] as usize);
        EdgePage { kind: q.kind, entries, previous, next }
    }
}

fn first_row_of_each_edge(rows: &[EdgeEntry]) -> Vec<u32> {
    let mut seen = HashSet::with_capacity(rows.len());
    rows.iter().enumerate().filter(|(_, row)| seen.insert(&row.edge)).map(|(ord, _)| ord as u32).collect()
}

fn adjacency_at<'g>(g: &'g Graph, p: &Position, kind: EdgeKind) -> Option<&'g Adjacency> {
    match kind {
        EdgeKind::Directed(rel, dir) => g.indexes.get(&rel).and_then(|ix| match dir {
            Direction::Forward => ix.fwd.get(p),
            Direction::Inverse => ix.inv.get(p),
        }),
        EdgeKind::Symmetric(rel) => g.symmetric_indexes.get(&rel).and_then(|ix| ix.fwd.get(p)),
    }
}

fn edge_count_at(g: &Graph, p: &Position, kind: EdgeKind) -> usize {
    adjacency_at(g, p, kind).map_or(0, Adjacency::edge_count)
}

impl Adjacent for PositionRef {
    fn edge_summary(&self, g: &Graph) -> EdgeSummary {
        let mut out = EdgeSummary::new();
        for rel in crate::edge::RelationId::ALL {
            for dir in [Direction::Forward, Direction::Inverse] {
                let k = EdgeKind::Directed(*rel, dir);
                let n = edge_count_at(g, &self.0, k);
                if n > 0 {
                    out.insert(k, n);
                }
            }
        }
        for rel in crate::edge::SymRelationId::ALL {
            let k = EdgeKind::Symmetric(*rel);
            let n = edge_count_at(g, &self.0, k);
            if n > 0 {
                out.insert(k, n);
            }
        }
        out
    }

    fn edges(&self, g: &Graph, q: &EdgeQuery) -> EdgePage {
        adjacency_at(g, &self.0, q.kind).map_or_else(|| EdgePage { kind: q.kind, entries: Vec::new(), previous: None, next: None }, |adjacency| adjacency.page(q))
    }
}

/// Set semantics: a position is arrived at once. Edges never dedup -- they live in pages, each
/// carrying its own id.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Holdings(pub BTreeSet<Position>);

impl Holdings {
    pub fn focus(p: Position) -> Holdings {
        let mut s = BTreeSet::new();
        s.insert(p);
        Holdings(s)
    }

    /// bind: follow-and-pool a continuation at every held position.
    pub fn bind(&self, f: impl Fn(&Position) -> Holdings) -> Holdings {
        let mut out = BTreeSet::new();
        for p in &self.0 {
            out.extend(f(p).0);
        }
        Holdings(out)
    }

    pub fn step(&self, g: &Graph, k: EdgeKind) -> Holdings {
        self.bind(|p| {
            Holdings(
                adjacency_at(g, p, k)
                    .into_iter()
                    .flat_map(Adjacency::edges)
                    .map(|e| e.node.clone())
                    .collect(),
            )
        })
    }

    pub fn steps(&self, g: &Graph, path: &[EdgeKind]) -> Holdings {
        path.iter().fold(self.clone(), |h, k| h.step(g, *k))
    }
}

/// Traversing forward and then asking the target for its inverse entry finds the same edge id.
pub fn inverse_entry_ids(g: &Graph, from: &Position, kind: EdgeKind) -> Vec<(EdgeId, EdgeId)> {
    let dk = dual(kind);
    adjacency_at(g, from, kind)
        .into_iter()
        .flat_map(Adjacency::edges)
        .flat_map(|e| {
            adjacency_at(g, &e.node, dk)
                .into_iter()
                .flat_map(Adjacency::edges)
                .filter(|back| back.node == *from)
                .map(move |back| (e.edge.clone(), back.edge.clone()))
                .collect::<Vec<_>>()
        })
        .collect()
}
