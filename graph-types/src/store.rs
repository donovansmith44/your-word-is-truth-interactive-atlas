//! The storage PORT (design spec §9a): the store is a port, not a place.
//! Serving binds ONLY to these traits; the compiled artifact is defined
//! logically (content-addressed things + indexes + a version root),
//! never as a file format. Laws certify any backend through this port.
//! Two implementations keep it honest: the in-memory reference below
//! (M-A's), the serialized artifact (M-C's) — a database, if ever
//! wanted, is implementation #3 into a worn socket.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::explore::{EdgePage, EdgeQuery, EdgeSummary, Explorable, PositionRef};
use crate::graph::Graph;
use crate::id::{AnyNodeId, ContentAddressed, ContentHash, Pid, Position};
use crate::node::Node;

/// The version root: one stamp identifies one immutable compiled graph.
/// (Skeleton: hash of the node table's canonical bytes; production: the
/// Merkle root over every thing.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphVersion(pub ContentHash);

/// Read-side port: versioned, snapshot-consistent access. A snapshot is
/// IMMUTABLE — publishing a new version never changes what an open
/// snapshot answers (the law tests pin this).
pub trait GraphStore {
    type Snapshot: GraphSnapshot;
    fn current_version(&self) -> Option<GraphVersion>;
    fn open(&self, v: GraphVersion) -> Option<Self::Snapshot>;
    /// Convenience: open the current version, if any.
    fn open_current(&self) -> Option<Self::Snapshot> {
        self.current_version().and_then(|v| self.open(v))
    }
}

/// One immutable version — the query surface, and nothing else. This IS
/// the contract serving depends on; everything above it (endpoints,
/// windows, popovers) composes these calls.
pub trait GraphSnapshot {
    fn version(&self) -> GraphVersion;

    /// Node lookup by erased id.
    fn node(&self, id: &AnyNodeId) -> Option<Node>;

    /// derive: the content-addressed store law — the canonical bytes of
    /// the thing a pid names. Self-verifying: implementations must
    /// return bytes that hash back to the pid (the law test pins it).
    fn derive(&self, pid: &Pid) -> Option<Vec<u8>>;

    /// Frontier counts for one position (kind → count; inhabited only).
    fn edge_summary(&self, p: &Position) -> EdgeSummary;

    /// One page of one edge kind at one position.
    fn edges(&self, p: &Position, q: &EdgeQuery) -> EdgePage;

    /// A window along one corpus's reading spine (unit ids, in canonical
    /// order). Text composes via `node` (renderings live on the node).
    fn reading_window(&self, corpus: &'static str, start: usize, n: usize) -> Vec<AnyNodeId>;
}

/// Write-side port: the COMPILER publishes; serving never writes.
/// Publishing is an atomic advance — readers see the old version or the
/// new one, never a mixture.
pub trait GraphPublisher {
    fn publish(&mut self, graph: Graph) -> GraphVersion;
}

// ---------------------------------------------------------------------
// Reference implementation: the in-memory store (implementation #1).
// ---------------------------------------------------------------------

/// Skeleton version derivation: hash the node ids + payload debug forms.
/// (Production: Merkle root. The LAW — same content ⇒ same version;
/// different content ⇒ different version — is what the tests pin.)
fn version_of(g: &Graph) -> GraphVersion {
    struct V<'a>(&'a Graph);
    impl<'a> ContentAddressed for V<'a> {
        fn canonical_bytes(&self) -> Vec<u8> {
            let mut s = String::new();
            for (id, node) in &self.0.nodes {
                s.push_str(&format!("{:?}|{:?}\n", id, node.payload));
            }
            s.into_bytes()
        }
        fn position_kind(&self) -> crate::id::PositionKind {
            crate::id::PositionKind::Exploration // stand-in kind for the root stamp
        }
    }
    GraphVersion(V(g).pid().hash)
}

#[derive(Clone)]
pub struct MemSnapshot {
    version: GraphVersion,
    graph: Arc<Graph>,
}

impl GraphSnapshot for MemSnapshot {
    fn version(&self) -> GraphVersion {
        self.version
    }
    fn node(&self, id: &AnyNodeId) -> Option<Node> {
        self.graph.nodes.get(id).cloned()
    }
    fn derive(&self, pid: &Pid) -> Option<Vec<u8>> {
        self.graph
            .nodes
            .values()
            .find(|n| &n.pid() == pid)
            .map(|n| n.canonical_bytes())
    }
    fn edge_summary(&self, p: &Position) -> EdgeSummary {
        PositionRef(p.clone()).edge_summary(&self.graph)
    }
    fn edges(&self, p: &Position, q: &EdgeQuery) -> EdgePage {
        PositionRef(p.clone()).edges(&self.graph, q)
    }
    fn reading_window(&self, corpus: &'static str, start: usize, n: usize) -> Vec<AnyNodeId> {
        self.graph.reading_window(corpus, start, n)
    }
}

/// The in-memory store: versions held live; publish swaps atomically
/// (readers hold Arc snapshots; an open snapshot never changes).
#[derive(Default)]
pub struct MemStore {
    versions: BTreeMap<GraphVersion, Arc<Graph>>,
    current: Option<GraphVersion>,
}

impl GraphStore for MemStore {
    type Snapshot = MemSnapshot;
    fn current_version(&self) -> Option<GraphVersion> {
        self.current
    }
    fn open(&self, v: GraphVersion) -> Option<MemSnapshot> {
        self.versions
            .get(&v)
            .map(|g| MemSnapshot { version: v, graph: Arc::clone(g) })
    }
}

impl GraphPublisher for MemStore {
    fn publish(&mut self, graph: Graph) -> GraphVersion {
        let v = version_of(&graph);
        self.versions.entry(v).or_insert_with(|| Arc::new(graph));
        self.current = Some(v);
        v
    }
}

#[cfg(test)]
mod laws {
    use super::*;
    use crate::id::NodeKind;
    use crate::ingest::ProvenanceId;
    use crate::node::NodePayload;

    fn unit(raw: &str, text: &str) -> Node {
        let mut renderings = crate::text::LayerMap::new();
        renderings.insert(crate::text::TranslationId("kjv".into()), text.into());
        Node {
            id: AnyNodeId { kind: NodeKind::TextUnit, raw: raw.into() },
            payload: NodePayload::TextUnit { corpus: "bible", renderings },
            provenance: ProvenanceId::from("kjv-source"),
        }
    }

    fn graph_with(texts: &[(&str, &str)]) -> Graph {
        let mut g = Graph::default();
        let mut order = Vec::new();
        for (raw, text) in texts {
            let n = unit(raw, text);
            order.push(n.id.clone());
            g.nodes.insert(n.id.clone(), n);
        }
        g.reading.insert("bible", crate::graph::ReadingSpine { order });
        g.build_indexes();
        g
    }

    #[test]
    fn publish_is_atomic_advance_and_snapshots_are_immutable() {
        let mut store = MemStore::default();
        assert!(store.current_version().is_none());

        let v1 = store.publish(graph_with(&[("bible/1.1.1", "In the beginning")]));
        let snap1 = store.open(v1).unwrap();

        let v2 = store.publish(graph_with(&[
            ("bible/1.1.1", "In the beginning"),
            ("bible/1.1.2", "And the earth"),
        ]));
        assert_ne!(v1, v2, "different content, different version");
        assert_eq!(store.current_version(), Some(v2));

        // The v1 snapshot still answers as v1: immutable reads.
        assert_eq!(snap1.reading_window("bible", 0, 10).len(), 1);
        let snap2 = store.open_current().unwrap();
        assert_eq!(snap2.reading_window("bible", 0, 10).len(), 2);
        // And v1 remains openable after the advance.
        assert!(store.open(v1).is_some());
    }

    #[test]
    fn same_content_same_version() {
        let mut store = MemStore::default();
        let a = store.publish(graph_with(&[("bible/1.1.1", "In the beginning")]));
        let b = store.publish(graph_with(&[("bible/1.1.1", "In the beginning")]));
        assert_eq!(a, b, "content addressing dedups versions");
    }

    #[test]
    fn derive_round_trip_self_verifies() {
        let mut store = MemStore::default();
        let v = store.publish(graph_with(&[("bible/1.1.1", "In the beginning")]));
        let snap = store.open(v).unwrap();
        let n = snap
            .node(&AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() })
            .unwrap();
        let bytes = snap.derive(&n.pid()).expect("derivable from its pid");
        assert_eq!(bytes, n.canonical_bytes(), "derive returns the canonical form");
    }

    #[test]
    fn window_partitions_concatenate_identically_through_the_port() {
        let mut store = MemStore::default();
        let v = store.publish(graph_with(&[
            ("bible/1.1.1", "a"),
            ("bible/1.1.2", "b"),
            ("bible/1.1.3", "c"),
            ("bible/1.1.4", "d"),
            ("bible/1.1.5", "e"),
        ]));
        let snap = store.open(v).unwrap();
        let whole = snap.reading_window("bible", 0, 5);
        for split in 1..5 {
            let mut parts = snap.reading_window("bible", 0, split);
            parts.extend(snap.reading_window("bible", split, 5 - split));
            assert_eq!(parts, whole, "windows are honest partitions");
        }
    }
}
