//! Exploration: the capability (Explorable — frontiers, nothing else)
//! and the act (Holdings — the powerset monad's carrier). The monad
//! laws run as tests; step/page agreement is the trait↔monad bridge.

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

/// Per-entry relation metadata (the types artifact's §10 open question,
/// answered by demonstrated need — M-B's EventWorld workaround existed
/// because entries could not carry this): a succession entry names its
/// narrative; a citation entry carries its votes rank. The SAME meta is
/// visible from both directions of a row (one row, two projections), so
/// the bijection witness extends to meta. Extending this enum is a
/// deliberate act reviewed like any relation change.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeMeta {
    None,
    Narrative(crate::id::NarrativeId),  // follows-in/precedes-in: which chain
    Votes(u32),                         // cites/cited-by: ranking
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

/// What EXPLORATION means — yielding frontiers, nothing else. Card and
/// payload live on NodeData/the view side (deliberate split).
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
        // Symmetric relations: skeleton serves none yet; the shape is
        // the same (one row table, ends interchangeable).
        EdgeKind::Symmetric(_) => Vec::new(),
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

/// The exploration state: the powerset monad's carrier. Set semantics —
/// arrive at a position once; edges never dedup (they live in pages,
/// each with its EdgeId).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Holdings(pub BTreeSet<Position>);

impl Holdings {
    /// return: hold one thing, frontier not yet consulted.
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

    /// bind at one generating arrow: union over held positions of the
    /// TOTAL frontier of kind k (all pages — the totaled trait calls).
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

    /// Kleisli chain.
    pub fn steps(&self, g: &Graph, path: &[EdgeKind]) -> Holdings {
        path.iter().fold(self.clone(), |h, k| h.step(g, *k))
    }
}

/// The bijection witness, queryable: traversing forward then asking the
/// target for the inverse entry finds the same EdgeId.
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
