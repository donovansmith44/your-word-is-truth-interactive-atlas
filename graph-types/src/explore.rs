//! Exploring means yielding frontiers and nothing else; `Holdings` is the act of doing it,
//! with set semantics -- a position is arrived at once.

use std::collections::{BTreeMap, BTreeSet};

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

/// Yielding frontiers, nothing else: payload and card live on the data side, deliberately apart.
pub trait Explorable {
    fn edge_summary(&self, g: &Graph) -> EdgeSummary;
    fn edges(&self, g: &Graph, q: &EdgeQuery) -> EdgePage;
}

/// The generic position handle every surface consumes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PositionRef(pub Position);

fn raw_neighbors(g: &Graph, p: &Position, kind: EdgeKind) -> Vec<EdgeEntry> {
    match kind {
        EdgeKind::Directed(rel, dir) => {
            let ix = match g.indexes.get(&rel) {
                Some(ix) => ix,
                None => return Vec::new(),
            };
            let map = match dir {
                Direction::Forward => &ix.fwd,
                Direction::Inverse => &ix.inv,
            };
            map.get(p)
                .map(|v| {
                    v.iter()
                        .map(|(eid, o, m)| EdgeEntry {
                            edge: eid.clone(),
                            node: o.clone(),
                            meta: m.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default()
        }
        // Both ends of a symmetric relation are interchangeable, so both populate the SAME
        // forward map at build time and querying from either end reads that one map.
        EdgeKind::Symmetric(rel) => {
            let ix = match g.symmetric_indexes.get(&rel) {
                Some(ix) => ix,
                None => return Vec::new(),
            };
            ix.fwd
                .get(p)
                .map(|v| {
                    v.iter()
                        .map(|(eid, o, m)| EdgeEntry {
                            edge: eid.clone(),
                            node: o.clone(),
                            meta: m.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default()
        }
    }
}

impl Explorable for PositionRef {
    fn edge_summary(&self, g: &Graph) -> EdgeSummary {
        let mut out = EdgeSummary::new();
        for rel in crate::edge::RelationId::ALL {
            for dir in [Direction::Forward, Direction::Inverse] {
                let k = EdgeKind::Directed(*rel, dir);
                let n = raw_neighbors(g, &self.0, k).len();
                if n > 0 {
                    out.insert(k, n);
                }
            }
        }
        // Symmetric kinds must appear here too: a summary that omits real connections would
        // hide them from a frontier that renders a section only when its count is positive.
        for rel in crate::edge::SymRelationId::ALL {
            let k = EdgeKind::Symmetric(*rel);
            let n = raw_neighbors(g, &self.0, k).len();
            if n > 0 {
                out.insert(k, n);
            }
        }
        out
    }

    fn edges(&self, g: &Graph, q: &EdgeQuery) -> EdgePage {
        let all = raw_neighbors(g, &self.0, q.kind);
        let start = q.cursor.unwrap_or(0);
        let entries: Vec<_> = all.iter().skip(start).take(q.limit).cloned().collect();
        let next = if start + entries.len() < all.len() {
            Some(start + entries.len())
        } else {
            None
        };
        EdgePage { kind: q.kind, entries, next }
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

    /// The union over held positions of the TOTAL frontier of that kind -- every page.
    pub fn step(&self, g: &Graph, k: EdgeKind) -> Holdings {
        self.bind(|p| {
            Holdings(
                raw_neighbors(g, p, k)
                    .into_iter()
                    .map(|e| e.node)
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
    let fwd = raw_neighbors(g, from, kind);
    let dk = dual(kind);
    fwd.into_iter()
        .flat_map(|e| {
            raw_neighbors(g, &e.node, dk)
                .into_iter()
                .filter(|back| back.node == *from)
                .map(move |back| (e.edge.clone(), back.edge))
                .collect::<Vec<_>>()
        })
        .collect()
}
