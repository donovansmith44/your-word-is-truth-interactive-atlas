//! The store is a port, not a place: the concrete graph implements the same query contract
//! every backend snapshot does, so admitting a backend is a comparison of its answers against
//! the canonical instance.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::adjacency::{Cursor, EdgeEntry, EdgeEntryWithNode, EdgePage, EdgePageWithNodes, EdgeQuery, EdgeSummary, Adjacent, NodePage, PositionRef};
use crate::graph::Graph;
use crate::id::{AnyNodeId, ContentAddressed, ContentHash, NodeKind, Pid, Position};
use crate::node::Node;

/// One stamp identifies one immutable compiled graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphVersion(pub ContentHash);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowRef {
    pub family: crate::canon::RowFamily,
    /// The row's ord within its own family.
    pub row_id: u64,
    pub provenance: crate::ingest::ProvenanceId,
}

/// What it means to answer graph questions. The concrete graph implements it, every backend
/// snapshot implements it, and serving composes it and nothing else.
pub trait GraphQuery {
    fn node(&self, id: &AnyNodeId) -> Option<Node>;

    /// The content-addressing law: the canonical bytes of the thing a pid names, which must
    /// hash back to that pid.
    fn derive(&self, pid: &Pid) -> Option<Vec<u8>>;

    /// Inhabited kinds only: a kind with no edges at this position is absent, not zero.
    fn edge_summary(&self, p: &Position) -> EdgeSummary;

    fn edges(&self, p: &Position, q: &EdgeQuery) -> EdgePage;

    /// Unit ids in canonical order; their text composes through `node`.
    fn reading_window(&self, corpus: &'static str, start: usize, n: usize) -> Vec<AnyNodeId>;

    // The methods below default to compositions of the five above wherever one exists. Three
    // are required instead, because enumeration, row identity and the spine index cannot be
    // derived from the five.

    /// In id byte order, paged.
    fn nodes_of_kind(&self, kind: NodeKind, cursor: Option<usize>, limit: usize) -> NodePage;

    /// Position `i` of the result answers `ids[i]`.
    fn nodes(&self, ids: &[AnyNodeId]) -> Vec<Option<Node>> {
        ids.iter().map(|i| self.node(i)).collect()
    }

    /// Each entry carries its target node, which is absent when the target is an edge.
    fn edges_with_nodes(&self, p: &Position, q: &EdgeQuery) -> EdgePageWithNodes {
        let page = self.edges(p, q);
        let ids: Vec<AnyNodeId> = page
            .entries
            .iter()
            .filter_map(|e| match &e.node {
                Position::Node(id) => Some(id.clone()),
                Position::Edge(_) => None,
            })
            .collect();
        let mut looked = self.nodes(&ids).into_iter();
        let entries = page
            .entries
            .into_iter()
            .map(|entry| {
                let node = match &entry.node {
                    Position::Node(_) => looked.next().flatten(),
                    Position::Edge(_) => None,
                };
                EdgeEntryWithNode { entry, node }
            })
            .collect();
        EdgePageWithNodes { kind: page.kind, entries, next: page.next }
    }

    /// `None` for a synthesised or unknown id. Where two rows mint one id, this is the first
    /// by `(family, ord)` and `rows_behind` lists them all.
    fn row_provenance(&self, e: &crate::edge::EdgeId) -> Option<RowRef>;

    /// One account attested by two sources is two rows behind one id, and both must stay
    /// reachable. The default answers with the single row named above, so a backend that keeps
    /// every row overrides this.
    fn rows_behind(&self, e: &crate::edge::EdgeId) -> Vec<RowRef> {
        self.row_provenance(e).into_iter().collect()
    }

    /// `None` off-spine, and for an unknown corpus.
    fn position_of(&self, corpus: &'static str, id: &AnyNodeId) -> Option<usize>;

    fn labels(&self, at: &[Position]) -> Vec<Option<String>>;

    fn references(&self, units: &[AnyNodeId]) -> Vec<Option<String>>;

    fn edge(&self, id: &crate::edge::EdgeId) -> Option<crate::edge::EdgeRecord>;
}

/// The canonical instance: the graph answers its own questions, so conformance is typed
/// rather than asserted.
impl GraphQuery for Graph {
    fn node(&self, id: &AnyNodeId) -> Option<Node> {
        self.nodes.get(id).cloned()
    }
    fn derive(&self, pid: &Pid) -> Option<Vec<u8>> {
        self.pid_index
            .get(pid)
            .and_then(|id| self.nodes.get(id))
            .map(|n| n.canonical_bytes())
    }
    fn edge_summary(&self, p: &Position) -> EdgeSummary {
        PositionRef(p.clone()).edge_summary(self)
    }
    fn edges(&self, p: &Position, q: &EdgeQuery) -> EdgePage {
        PositionRef(p.clone()).edges(self, q)
    }
    fn reading_window(&self, corpus: &'static str, start: usize, n: usize) -> Vec<AnyNodeId> {
        Graph::reading_window(self, corpus, start, n)
    }
    fn nodes_of_kind(&self, kind: NodeKind, cursor: Option<usize>, limit: usize) -> NodePage {
        let start = cursor.unwrap_or(0);
        // An id orders by `(kind, raw)`, so one kind is one contiguous range.
        let mut ids: Vec<AnyNodeId> = self
            .nodes
            .range(AnyNodeId { kind, raw: String::new() }..)
            .take_while(|(id, _)| id.kind == kind)
            .map(|(id, _)| id.clone())
            .skip(start)
            .take(limit.saturating_add(1))
            .collect();
        let more = ids.len() > limit;
        if more {
            ids.truncate(limit);
        }
        let next = if more { Some(start + ids.len()) } else { None };
        NodePage { ids, next }
    }
    fn row_provenance(&self, e: &crate::edge::EdgeId) -> Option<RowRef> {
        let r = self.edge_row(e)?;
        let provenance = self.row_provenance_of(r.family, r.row_ord as usize)?;
        Some(RowRef { family: r.family, row_id: u64::from(r.row_ord), provenance: provenance.to_string() })
    }
    fn rows_behind(&self, e: &crate::edge::EdgeId) -> Vec<RowRef> {
        self.rows_of_edge(e)
            .iter()
            .filter_map(|r| {
                let provenance = self.row_provenance_of(r.family, r.row_ord as usize)?;
                Some(RowRef { family: r.family, row_id: u64::from(r.row_ord), provenance: provenance.to_string() })
            })
            .collect()
    }
    fn position_of(&self, corpus: &'static str, id: &AnyNodeId) -> Option<usize> {
        self.spine_index.get(corpus).and_then(|m| m.get(id)).copied()
    }
    fn labels(&self, at: &[Position]) -> Vec<Option<String>> {
        at.iter().map(|p| self.labels.get(p).cloned()).collect()
    }
    fn references(&self, units: &[AnyNodeId]) -> Vec<Option<String>> {
        units.iter().map(|unit| self.references.get(unit).cloned()).collect()
    }
    fn edge(&self, id: &crate::edge::EdgeId) -> Option<crate::edge::EdgeRecord> {
        self.edges_by_id.get(id).cloned()
    }
}

/// The same contract plus one stamp, and immutable: publishing a new version never changes
/// what an already-open snapshot answers.
pub trait GraphSnapshot: GraphQuery {
    fn version(&self) -> GraphVersion;
}

pub trait GraphStore {
    type Snapshot: GraphSnapshot;
    fn current_version(&self) -> Option<GraphVersion>;
    fn open(&self, v: GraphVersion) -> Option<Self::Snapshot>;
    fn open_current(&self) -> Option<Self::Snapshot> {
        self.current_version().and_then(|v| self.open(v))
    }
}

/// The compiler publishes; serving never writes. Publishing is an atomic advance: a reader
/// sees the old version or the new one, never a mixture.
pub trait GraphPublisher {
    fn publish(&mut self, graph: Graph) -> Result<GraphVersion, crate::section_index::IndexError>;
}

/// A disclosed defect, not a hidden one: this hashes the NODE TABLE ONLY, so adding a row
/// with the nodes untouched leaves the root where it was and two different graphs share one
/// stamp. A law below asserts the defect; the other spelling of this function fixes it.
#[cfg(not(feature = "canon-ids"))]
fn version_of(g: &Graph) -> Result<GraphVersion, crate::section_index::IndexError> {
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
            crate::id::PositionKind::Version
        }
    }
    Ok(GraphVersion(V(g).pid().hash))
}

#[cfg(feature = "canon-ids")]
fn version_of(g: &Graph) -> Result<GraphVersion, crate::section_index::IndexError> {
    crate::sections::version_root(g).map(GraphVersion)
}

#[derive(Clone)]
pub struct MemSnapshot {
    version: GraphVersion,
    graph: Arc<Graph>,
}

impl MemSnapshot {
    pub fn present(version: GraphVersion, graph: Arc<Graph>) -> MemSnapshot {
        MemSnapshot { version, graph }
    }
}

impl GraphQuery for MemSnapshot {
    fn node(&self, id: &AnyNodeId) -> Option<Node> {
        self.graph.node(id)
    }
    fn derive(&self, pid: &Pid) -> Option<Vec<u8>> {
        self.graph.derive(pid)
    }
    fn edge_summary(&self, p: &Position) -> EdgeSummary {
        self.graph.edge_summary(p)
    }
    fn edges(&self, p: &Position, q: &EdgeQuery) -> EdgePage {
        self.graph.edges(p, q)
    }
    fn reading_window(&self, corpus: &'static str, start: usize, n: usize) -> Vec<AnyNodeId> {
        self.graph.reading_window(corpus, start, n)
    }
    fn nodes_of_kind(&self, kind: NodeKind, cursor: Option<usize>, limit: usize) -> NodePage {
        self.graph.nodes_of_kind(kind, cursor, limit)
    }
    fn row_provenance(&self, e: &crate::edge::EdgeId) -> Option<RowRef> {
        self.graph.row_provenance(e)
    }
    fn rows_behind(&self, e: &crate::edge::EdgeId) -> Vec<RowRef> {
        self.graph.rows_behind(e)
    }
    fn position_of(&self, corpus: &'static str, id: &AnyNodeId) -> Option<usize> {
        self.graph.position_of(corpus, id)
    }
    fn labels(&self, at: &[Position]) -> Vec<Option<String>> {
        self.graph.labels(at)
    }
    fn references(&self, units: &[AnyNodeId]) -> Vec<Option<String>> {
        self.graph.references(units)
    }
    fn edge(&self, id: &crate::edge::EdgeId) -> Option<crate::edge::EdgeRecord> {
        self.graph.edge(id)
    }
}

impl GraphSnapshot for MemSnapshot {
    fn version(&self) -> GraphVersion {
        self.version
    }
}

/// Publishing swaps atomically: a reader holds its own snapshot, which never changes.
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
            .map(|g| MemSnapshot::present(v, Arc::clone(g)))
    }
}

impl GraphPublisher for MemStore {
    fn publish(&mut self, graph: Graph) -> Result<GraphVersion, crate::section_index::IndexError> {
        let v = version_of(&graph)?;
        self.versions.entry(v).or_insert_with(|| Arc::new(graph));
        self.current = Some(v);
        Ok(v)
    }
}

/// The node table PLUS every subject and object in the built indexes, so a sparse node table
/// cannot make a conformance check pass vacuously.
fn position_inventory(model: &Graph) -> BTreeSet<Position> {
    model.positions()
}

fn drain(q: &impl GraphQuery, p: &Position, kind: crate::edge::EdgeKind, limit: usize) -> Vec<EdgeEntry> {
    let mut cursor = Cursor::FIRST;
    let mut out = Vec::new();
    loop {
        let page = q.edges(p, &EdgeQuery { kind, cursor, limit });
        out.extend(page.entries);
        match page.next {
            Some(c) => cursor = c,
            None => break,
        }
    }
    out
}

fn node_eq(a: &Option<Node>, b: &Option<Node>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => {
            x.id == y.id
                && x.provenance == y.provenance
                && format!("{:?}", x.payload) == format!("{:?}", y.payload)
        }
        _ => false,
    }
}

fn check_position_answers<Q: GraphQuery>(candidate: &Q, model: &Graph, p: &Position) {
    if let Position::Node(id) = p {
        let a = candidate.node(id);
        let b = model.node(id);
        assert!(node_eq(&a, &b), "conformance: node({:?}) diverges", id);
        if let Some(n) = &b {
            let pid = n.pid();
            assert_eq!(
                candidate.derive(&pid),
                model.derive(&pid),
                "conformance: derive({:?}) diverges",
                pid
            );
        }
    }

    assert_eq!(candidate.labels(std::slice::from_ref(p)), model.labels(std::slice::from_ref(p)), "conformance: labels({:?}) diverges", p);

    let sa = candidate.edge_summary(p);
    let sb = model.edge_summary(p);
    assert_eq!(sa, sb, "conformance: edge_summary({:?}) diverges", p);

    for (kind, count) in sb {
        for limit in [1usize, count.max(1)] {
            assert_eq!(
                drain(candidate, p, kind, limit),
                drain(model, p, kind, limit),
                "conformance: edges({:?}, {:?}, limit {}) diverges",
                p,
                kind,
                limit
            );
        }
    }
}

fn check_position_rows<Q: GraphQuery>(candidate: &Q, model: &Graph, p: &Position) {
    for (kind, _) in model.edge_summary(p) {
        let q = EdgeQuery { kind, cursor: Cursor::FIRST, limit: 1 };
        let a = candidate.edges_with_nodes(p, &q);
        let b = model.edges_with_nodes(p, &q);
        assert_eq!((a.kind, a.next, a.entries.len()), (b.kind, b.next, b.entries.len()), "conformance: edges_with_nodes({p:?}, {kind:?}) page shape diverges");
        for (x, y) in a.entries.iter().zip(&b.entries) {
            assert_eq!(x.entry, y.entry, "conformance: edges_with_nodes entry diverges at {p:?}");
            assert!(node_eq(&x.node, &y.node), "conformance: edges_with_nodes node diverges at {p:?}");
            assert_eq!(
                candidate.row_provenance(&x.entry.edge),
                model.row_provenance(&y.entry.edge),
                "conformance: row_provenance({:?}) diverges",
                x.entry.edge
            );
            assert_eq!(
                candidate.rows_behind(&x.entry.edge),
                model.rows_behind(&y.entry.edge),
                "conformance: rows_behind({:?}) diverges",
                x.entry.edge
            );
        }
    }
    if let Position::Node(id) = p {
        for corpus in model.reading.keys() {
            assert_eq!(candidate.position_of(corpus, id), model.position_of(corpus, id), "conformance: position_of({corpus}, {id:?}) diverges");
        }
    }
}

/// Every check is a pure read of both sides, so the work partitions with no coordination.
/// Positions are STRIPED rather than sliced: per-position cost varies by orders of magnitude,
/// and striping averages that out without a work-stealing queue.
fn sweep_positions<Q: GraphQuery + Sync>(
    candidate: &Q,
    model: &Graph,
    inventory: &[Position],
    check: fn(&Q, &Graph, &Position),
) {
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(inventory.len().max(1));
    if workers <= 1 {
        for p in inventory {
            check(candidate, model, p);
        }
        return;
    }
    std::thread::scope(|scope| {
        for worker in 0..workers {
            scope.spawn(move || {
                for p in inventory.iter().skip(worker).step_by(workers) {
                    check(candidate, model, p);
                }
            });
        }
    });
}

/// Any implementation claiming to present `model` must answer every question identically to
/// the graph itself; it panics at a divergence, naming it. `Sync` is required because the
/// sweeps run across cores: a candidate that cannot be shared is not a backend to serve from.
pub fn assert_answers_match<Q: GraphQuery + Sync>(candidate: &Q, model: &Graph) {
    let inventory: Vec<Position> = position_inventory(model).into_iter().collect();

    sweep_positions(candidate, model, &inventory, check_position_answers::<Q>);

    for kind in NodeKind::ALL {
        let mut cursor = None;
        let mut got: Vec<AnyNodeId> = Vec::new();
        loop {
            let page = candidate.nodes_of_kind(kind, cursor, 97);
            got.extend(page.ids);
            match page.next {
                Some(c) => cursor = Some(c),
                None => break,
            }
        }
        let want = model.nodes_of_kind(kind, None, usize::MAX).ids;
        assert_eq!(got, want, "conformance: nodes_of_kind({kind:?}) diverges");
    }
    let all_ids: Vec<AnyNodeId> = model.nodes.keys().cloned().collect();
    for chunk in all_ids.chunks(1000) {
        let a = candidate.nodes(chunk);
        let b = model.nodes(chunk);
        assert_eq!(a.len(), b.len(), "conformance: nodes() length diverges");
        for (x, y) in a.iter().zip(&b) {
            assert!(node_eq(x, y), "conformance: nodes() diverges");
        }
        assert_eq!(candidate.references(chunk), model.references(chunk), "conformance: references() diverges");
    }
    sweep_positions(candidate, model, &inventory, check_position_rows::<Q>);

    for id in model.edges_by_id.keys() {
        let at = Position::Edge(id.clone());
        assert_eq!(candidate.edge(id), model.edge(id), "conformance: edge({id:?}) diverges");
        assert_eq!(candidate.labels(std::slice::from_ref(&at)), model.labels(std::slice::from_ref(&at)), "conformance: labels({at:?}) diverges");
    }

    for (corpus, spine) in &model.reading {
        let len = spine.order.len();
        for (start, n) in [(0usize, len), (0, 1.min(len)), (1.min(len), len.saturating_sub(1))] {
            assert_eq!(
                candidate.reading_window(corpus, start, n),
                model.reading_window(corpus, start, n),
                "conformance: reading_window({}, {}, {}) diverges",
                corpus,
                start,
                n
            );
        }
    }
}

#[cfg(test)]
mod laws {
    use super::*;
    use crate::edge::{Analogue, Justification, LocatedAt, Succession};
    use crate::id::{EventId, NodeKind, PlaceId};
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

    fn with_edges(mut g: Graph) -> Graph {
        g.succession.push(
            Succession::new(
                crate::id::NarrativeId::new("n"),
                vec![EventId::new("e1"), EventId::new("e2")],
                "p".into(),
                Justification::default(),
            )
            .unwrap(),
        );
        g.located_at.push(LocatedAt {
            event: EventId::new("e1"),
            place: PlaceId::new("jordan"),
            provenance: "p".into(),
            justification: Justification::default(),
        });
        g.build_indexes();
        g
    }

    #[test]
    fn publish_is_atomic_advance_and_snapshots_are_immutable() {
        let mut store = MemStore::default();
        assert!(store.current_version().is_none());

        let v1 = store.publish(graph_with(&[("bible/1.1.1", "In the beginning")])).unwrap();
        let snap1 = store.open(v1).unwrap();

        let v2 = store.publish(graph_with(&[
            ("bible/1.1.1", "In the beginning"),
            ("bible/1.1.2", "And the earth"),
        ])).unwrap();
        assert_ne!(v1, v2, "different content, different version");
        assert_eq!(store.current_version(), Some(v2));

        assert_eq!(snap1.reading_window("bible", 0, 10).len(), 1);
        let snap2 = store.open_current().unwrap();
        assert_eq!(snap2.reading_window("bible", 0, 10).len(), 2);
        assert!(store.open(v1).is_some());
    }

    #[test]
    fn same_content_same_version() {
        let mut store = MemStore::default();
        let a = store.publish(graph_with(&[("bible/1.1.1", "In the beginning")])).unwrap();
        let b = store.publish(graph_with(&[("bible/1.1.1", "In the beginning")])).unwrap();
        assert_eq!(a, b, "content addressing dedups versions");
    }

    #[test]
    fn derive_round_trip_self_verifies() {
        let mut store = MemStore::default();
        let v = store.publish(graph_with(&[("bible/1.1.1", "In the beginning")])).unwrap();
        let snap = store.open(v).unwrap();
        let n = snap
            .node(&AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() })
            .unwrap();
        let bytes = snap.derive(&n.pid()).expect("derivable from its pid");
        assert_eq!(bytes, n.canonical_bytes(), "derive returns the canonical form");
    }

    #[test]
    fn content_hash_hex_and_from_hex_are_inverse() {
        let h = version_of(&graph_with(&[("bible/1.1.1", "In the beginning")])).unwrap().0;
        let s = h.hex();
        assert!(
            s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "lowercase hex only, no `0x`, no padding ambiguity: {s:?}"
        );
        assert_eq!(ContentHash::from_hex(&s), Some(h), "from_hex(hex(h)) == h");
        assert_eq!(ContentHash::from_hex(&"A".repeat(s.len())), None, "uppercase is refused");
        assert_eq!(ContentHash::from_hex(&s[1..]), None, "a short string is refused");
        assert_eq!(ContentHash::from_hex(&format!("{s}0")), None, "a long string is refused");
    }

    #[cfg(not(feature = "canon-ids"))]
    #[test]
    fn hex_is_sixteen_chars_while_the_hash_is_sixty_four_bits() {
        let h = version_of(&graph_with(&[("bible/1.1.1", "In the beginning")])).unwrap().0;
        assert_eq!(h.hex().len(), 16);
    }

    #[cfg(feature = "canon-ids")]
    #[test]
    fn hex_is_thirty_two_chars_while_the_hash_is_a_hundred_and_twenty_eight_bits() {
        let h = version_of(&graph_with(&[("bible/1.1.1", "In the beginning")])).unwrap().0;
        assert_eq!(h.hex().len(), 32);
    }

    #[cfg(feature = "canon-ids")]
    #[test]
    fn equal_nodes_have_equal_pids_and_one_payload_byte_moves_them() {
        let a = unit("bible/1.1.1", "In the beginning");
        let b = unit("bible/1.1.1", "In the beginning");
        assert_eq!(a.pid(), b.pid(), "equal content, equal pid");

        let c = unit("bible/1.1.1", "In the beginninq");
        assert_ne!(a.pid(), c.pid(), "one byte of payload is one different id");
        assert_eq!(a.pid().kind, c.pid().kind, "only the hash moved, not the kind");
    }

    fn base_and_rowed() -> (Graph, Graph) {
        let texts = [("bible/1.1.1", "a"), ("bible/1.1.2", "b")];
        let base = graph_with(&texts);
        let rowed = with_edges(graph_with(&texts));
        assert_eq!(
            base.nodes.keys().collect::<Vec<_>>(),
            rowed.nodes.keys().collect::<Vec<_>>(),
            "premise: the two graphs differ in ROWS only"
        );
        (base, rowed)
    }

    #[cfg(feature = "canon-ids")]
    #[test]
    fn version_root_covers_rows_not_only_nodes() {
        let (base, rowed) = base_and_rowed();
        assert_ne!(version_of(&base).unwrap(), version_of(&rowed).unwrap(), "a row changes the root");
    }

    #[cfg(not(feature = "canon-ids"))]
    #[test]
    fn version_root_is_blind_to_rows_the_documented_defect() {
        let (base, rowed) = base_and_rowed();
        assert_eq!(
            version_of(&base).unwrap(),
            version_of(&rowed).unwrap(),
            "spec §3.1 defect 1: the skeleton root is blind to rows -- \
             `canon-ids` fixes this, and the ON sibling of this test asserts the fix"
        );
    }

    #[cfg(feature = "canon-ids")]
    #[test]
    fn edge_ids_hash_canonical_edge_bytes_not_debug_text() {
        use crate::edge::{entry_id, RelationId};
        let s = Position::Node(EventId::new("e1").erase());
        let o = Position::Node(PlaceId::new("jordan").erase());
        let id = entry_id(RelationId::LocatedAt, &s, &o);
        let expected = crate::sha256::sha256_prefixed_128(crate::canon::DOMAIN_PREFIX, &crate::canon::ids::edge_canonical_bytes("LocatedAt", &s, &o));
        let mut hex = String::new();
        for b in expected {
            hex.push_str(&format!("{b:02x}"));
        }
        assert_eq!(id.0, format!("LocatedAt:{hex}"));
        assert_eq!(id.0.len(), "LocatedAt:".len() + 32);
    }

    const PARALLEL_ID_WITHOUT_CANON_IDS: &str = "Parallel:5d66728a994d8f39";
    const PARALLEL_ID_WITH_CANON_IDS: &str = "Parallel:6e8a07c6798657c8680765038904c648";

    #[test]
    fn a_symmetric_edge_id_is_taken_over_the_lower_end_first_whichever_end_a_caller_names() {
        // Arrange
        let event = Position::Node(EventId::new("e1").erase());
        let place = Position::Node(PlaceId::new("jordan").erase());
        let pinned = if cfg!(feature = "canon-ids") { PARALLEL_ID_WITH_CANON_IDS } else { PARALLEL_ID_WITHOUT_CANON_IDS };
        // Act
        let ids = (
            crate::edge::entry_id_symmetric(crate::edge::SymRelationId::Parallel, &event, &place),
            crate::edge::entry_id_symmetric(crate::edge::SymRelationId::Parallel, &place, &event),
        );
        // Assert
        assert_eq!(ids, (crate::edge::EdgeId(pinned.into()), crate::edge::EdgeId(pinned.into())));
    }

    #[test]
    fn conformance_snapshot_matches_the_graph_itself() {
        let g = with_edges(graph_with(&[("bible/1.1.1", "a"), ("bible/1.1.2", "b")]));
        let mut store = MemStore::default();
        let v = store.publish(with_edges(graph_with(&[
            ("bible/1.1.1", "a"),
            ("bible/1.1.2", "b"),
        ]))).unwrap();
        let snap = store.open(v).unwrap();
        assert_answers_match(&snap, &g);
    }

    #[test]
    fn conformance_harness_catches_a_lying_snapshot() {
        struct Lying(MemSnapshot);
        impl GraphQuery for Lying {
            fn node(&self, id: &AnyNodeId) -> Option<Node> {
                self.0.node(id)
            }
            fn derive(&self, pid: &Pid) -> Option<Vec<u8>> {
                self.0.derive(pid)
            }
            fn edge_summary(&self, p: &Position) -> EdgeSummary {
                self.0.edge_summary(p)
            }
            fn edges(&self, p: &Position, q: &EdgeQuery) -> EdgePage {
                let mut page = self.0.edges(p, q);
                page.entries.pop();
                page
            }
            fn reading_window(
                &self,
                corpus: &'static str,
                start: usize,
                n: usize,
            ) -> Vec<AnyNodeId> {
                self.0.reading_window(corpus, start, n)
            }
            fn nodes_of_kind(&self, k: NodeKind, c: Option<usize>, l: usize) -> NodePage {
                self.0.nodes_of_kind(k, c, l)
            }
            fn row_provenance(&self, e: &crate::edge::EdgeId) -> Option<RowRef> {
                self.0.row_provenance(e)
            }
            fn position_of(&self, c: &'static str, id: &AnyNodeId) -> Option<usize> {
                self.0.position_of(c, id)
            }
            fn labels(&self, at: &[Position]) -> Vec<Option<String>> {
                self.0.labels(at)
            }
            fn references(&self, units: &[AnyNodeId]) -> Vec<Option<String>> {
                self.0.references(units)
            }
            fn edge(&self, id: &crate::edge::EdgeId) -> Option<crate::edge::EdgeRecord> {
                self.0.edge(id)
            }
        }

        let g = with_edges(graph_with(&[("bible/1.1.1", "a")]));
        let mut store = MemStore::default();
        let v = store.publish(with_edges(graph_with(&[("bible/1.1.1", "a")]))).unwrap();
        let liar = Lying(store.open(v).unwrap());
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_answers_match(&liar, &g)
        }));
        assert!(caught.is_err(), "the harness must catch a dropped edge");
    }

    #[test]
    fn position_inventory_covers_symmetric_indexes() {
        let mut g = Graph::default();
        g.analogue.push(Analogue {
            a: EventId::new("e1"),
            b: EventId::new("e2"),
            provenance: "p".into(),
        });
        g.build_indexes();

        let inventory = position_inventory(&g);
        assert!(
            inventory.contains(&Position::Node(AnyNodeId {
                kind: NodeKind::Event,
                raw: "e1".into()
            })),
            "a symmetric-only graph's first end must be a known position"
        );
        assert!(
            inventory.contains(&Position::Node(AnyNodeId {
                kind: NodeKind::Event,
                raw: "e2".into()
            })),
            "a symmetric-only graph's second end must be a known position"
        );
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
        ])).unwrap();
        let snap = store.open(v).unwrap();
        let whole = snap.reading_window("bible", 0, 5);
        for split in 1..5 {
            let mut parts = snap.reading_window("bible", 0, split);
            parts.extend(snap.reading_window("bible", split, 5 - split));
            assert_eq!(parts, whole, "windows are honest partitions");
        }
    }

    #[test]
    fn nodes_of_kind_pages_in_id_order_with_edge_page_semantics() {
        let g = with_edges(graph_with(&[("bible/1.1.2", "b"), ("bible/1.1.1", "a"), ("bible/1.1.3", "c")]));
        let all = g.nodes_of_kind(NodeKind::TextUnit, None, 10);
        let raws: Vec<&str> = all.ids.iter().map(|i| i.raw.as_str()).collect();
        assert_eq!(raws, ["bible/1.1.1", "bible/1.1.2", "bible/1.1.3"], "byte order of raw within the kind");
        assert_eq!(all.next, None);
        let first = g.nodes_of_kind(NodeKind::TextUnit, None, 2);
        assert_eq!((first.ids.len(), first.next), (2, Some(2)));
        let rest = g.nodes_of_kind(NodeKind::TextUnit, Some(2), 2);
        assert_eq!((rest.ids.len(), rest.next), (1, None));
        assert_eq!(g.nodes_of_kind(NodeKind::TextUnit, Some(9), 2), NodePage { ids: vec![], next: None });
        assert_eq!(g.nodes_of_kind(NodeKind::Place, None, 5), NodePage { ids: vec![], next: None });
        assert_eq!(g.nodes_of_kind(NodeKind::Polity, None, 5), NodePage { ids: vec![], next: None });
        assert_eq!(g.nodes_of_kind(NodeKind::TextUnit, Some(1), 0).next, Some(1));
        assert_eq!(g.nodes_of_kind(NodeKind::TextUnit, Some(3), 0).next, None);
    }

    #[test]
    fn nodes_answers_positionally_and_edges_with_nodes_carries_the_targets() {
        let g = with_edges(graph_with(&[("bible/1.1.1", "a")]));
        let ids = vec![
            AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() },
            AnyNodeId { kind: NodeKind::TextUnit, raw: "nope".into() },
        ];
        let got = g.nodes(&ids);
        assert!(got[0].is_some() && got[1].is_none());
        let e1 = Position::Node(EventId::new("e1").erase());
        let kind = crate::edge::EdgeKind::Directed(crate::edge::RelationId::LocatedAt, crate::edge::Direction::Forward);
        let page = g.edges_with_nodes(&e1, &EdgeQuery { kind, cursor: Cursor::FIRST, limit: 10 });
        assert_eq!(page.entries.len(), 1);
        assert!(page.entries[0].node.is_none());
        assert_eq!(page.entries[0].entry, g.edges(&e1, &EdgeQuery { kind, cursor: Cursor::FIRST, limit: 10 }).entries[0]);
        let v = Position::Node(AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() });
        let g2 = {
            let mut g2 = graph_with(&[("bible/1.1.1", "a")]);
            g2.attests.push(crate::edge::Attests {
                event: EventId::new("e1"),
                attestation: crate::text::LocusRange::new(
                    crate::text::Locus::whole(crate::text::VerseRef { book: 1, chapter: 1, verse: 1 }),
                    crate::text::Locus::whole(crate::text::VerseRef { book: 1, chapter: 1, verse: 1 }),
                )
                .unwrap(),
                provenance: "p".into(),
                justification: Justification::default(),
            });
            g2.build_indexes();
            g2
        };
        let kind = crate::edge::EdgeKind::Directed(crate::edge::RelationId::Attests, crate::edge::Direction::Forward);
        let page = g2.edges_with_nodes(&e1, &EdgeQuery { kind, cursor: Cursor::FIRST, limit: 10 });
        assert_eq!(page.entries[0].entry.node, v);
        assert_eq!(page.entries[0].node.as_ref().map(|n| n.id.clone()), Some(AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() }));
    }

    #[test]
    fn row_provenance_names_the_row_and_position_of_names_the_spine_slot() {
        let g = with_edges(graph_with(&[("bible/1.1.1", "a"), ("bible/1.1.2", "b")]));
        let e1 = Position::Node(EventId::new("e1").erase());
        let kind = crate::edge::EdgeKind::Directed(crate::edge::RelationId::LocatedAt, crate::edge::Direction::Forward);
        let entry = g.edges(&e1, &EdgeQuery { kind, cursor: Cursor::FIRST, limit: 1 }).entries[0].clone();
        let r = g.row_provenance(&entry.edge).expect("a located_at row produced this edge");
        assert_eq!((r.family, r.row_id, r.provenance.as_str()), (crate::canon::RowFamily::LocatedAt, 0, "p"));
        assert_eq!(g.row_provenance(&crate::edge::EdgeId("LocatedAt:0000000000000000".into())), None);
        assert_eq!(g.row_provenance(&crate::edge::EdgeId("garbage".into())), None);
        let id = AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.2".into() };
        assert_eq!(g.position_of("bible", &id), Some(1));
        assert_eq!(g.position_of("concord", &id), None);
        assert_eq!(g.position_of("bible", &EventId::new("e1").erase()), None);
        let mut store = MemStore::default();
        let v = store.publish(with_edges(graph_with(&[("bible/1.1.1", "a"), ("bible/1.1.2", "b")]))).unwrap();
        let snap = store.open(v).unwrap();
        assert_eq!(snap.position_of("bible", &id), Some(1));
        assert_eq!(snap.row_provenance(&entry.edge).map(|r| r.family), Some(crate::canon::RowFamily::LocatedAt));
        assert_eq!(snap.nodes_of_kind(NodeKind::TextUnit, None, 9).ids.len(), 2);
    }

    fn located_at(place: &str, provenance: &str) -> LocatedAt {
        LocatedAt { event: EventId::new("e1"), place: PlaceId::new(place), provenance: provenance.into(), justification: Justification::default() }
    }

    #[test]
    fn a_adjacency_lists_an_edge_once_however_many_rows_mint_it() {
        // Arrange
        let mut g = graph_with(&[("bible/1.1.1", "a")]);
        g.located_at.push(located_at("jordan", "event-witnesses"));
        g.located_at.push(located_at("jordan", "attestation-corrections"));
        g.build_indexes();
        let e1 = Position::Node(EventId::new("e1").erase());
        let jordan = Position::Node(PlaceId::new("jordan").erase());
        let kind = crate::edge::EdgeKind::Directed(crate::edge::RelationId::LocatedAt, crate::edge::Direction::Forward);
        let edge = crate::edge::entry_id(crate::edge::RelationId::LocatedAt, &e1, &jordan);

        // Act
        let page = g.edges(&e1, &EdgeQuery { kind, cursor: Cursor::FIRST, limit: 10 });
        let summary = g.edge_summary(&e1);
        let rows: Vec<(u64, String)> = g.rows_behind(&edge).into_iter().map(|r| (r.row_id, r.provenance)).collect();
        let first = g.row_provenance(&edge).map(|r| r.row_id);

        // Assert
        assert_eq!(
            (page, summary, rows, first),
            (
                EdgePage { kind, entries: vec![EdgeEntry { edge, node: jordan, meta: crate::adjacency::EdgeMeta::None }], previous: None, next: None },
                [(kind, 1)].into_iter().collect(),
                vec![(0, "event-witnesses".to_string()), (1, "attestation-corrections".to_string())],
                Some(0),
            )
        );
    }

    #[test]
    fn a_page_walks_edges_and_its_cursor_is_the_first_row_of_the_edge_it_continues_from() {
        // Arrange
        let mut g = graph_with(&[("bible/1.1.1", "a")]);
        for (place, provenance) in [("jordan", "a"), ("jordan", "b"), ("bethel", "a"), ("hebron", "a"), ("hebron", "b")] {
            g.located_at.push(located_at(place, provenance));
        }
        g.build_indexes();
        let e1 = Position::Node(EventId::new("e1").erase());
        let kind = crate::edge::EdgeKind::Directed(crate::edge::RelationId::LocatedAt, crate::edge::Direction::Forward);
        let place_of = |page: &EdgePage| page.entries.iter().map(|e| e.node.clone()).collect::<Vec<_>>();
        let place = |name: &str| Position::Node(PlaceId::new(name).erase());

        // Act
        let walked: Vec<(Vec<Position>, Option<Cursor>)> = [(Cursor::FIRST, 1), (Cursor(2), 1), (Cursor(3), 1), (Cursor::FIRST, 2), (Cursor(3), 5)]
            .into_iter()
            .map(|(cursor, limit)| {
                let page = g.edges(&e1, &EdgeQuery { kind, cursor, limit });
                (place_of(&page), page.next)
            })
            .collect();

        // Assert
        assert_eq!(
            walked,
            vec![
                (vec![place("jordan")], Some(Cursor(2))),
                (vec![place("bethel")], Some(Cursor(3))),
                (vec![place("hebron")], None),
                (vec![place("jordan"), place("bethel")], Some(Cursor(3))),
                (vec![place("hebron")], None),
            ]
        );
    }

    #[test]
    fn the_harness_catches_a_snapshot_that_lies_about_the_new_methods() {
        struct LiesAboutRows(MemSnapshot);
        impl GraphQuery for LiesAboutRows {
            fn node(&self, id: &AnyNodeId) -> Option<Node> {
                self.0.node(id)
            }
            fn derive(&self, pid: &Pid) -> Option<Vec<u8>> {
                self.0.derive(pid)
            }
            fn edge_summary(&self, p: &Position) -> EdgeSummary {
                self.0.edge_summary(p)
            }
            fn edges(&self, p: &Position, q: &EdgeQuery) -> EdgePage {
                self.0.edges(p, q)
            }
            fn reading_window(&self, c: &'static str, s: usize, n: usize) -> Vec<AnyNodeId> {
                self.0.reading_window(c, s, n)
            }
            fn nodes_of_kind(&self, k: NodeKind, c: Option<usize>, l: usize) -> NodePage {
                self.0.nodes_of_kind(k, c, l)
            }
            fn row_provenance(&self, e: &crate::edge::EdgeId) -> Option<RowRef> {
                self.0.row_provenance(e).map(|mut r| {
                    r.provenance.push('!');
                    r
                })
            }
            fn position_of(&self, c: &'static str, id: &AnyNodeId) -> Option<usize> {
                self.0.position_of(c, id)
            }
            fn labels(&self, at: &[Position]) -> Vec<Option<String>> {
                self.0.labels(at)
            }
            fn references(&self, units: &[AnyNodeId]) -> Vec<Option<String>> {
                self.0.references(units)
            }
            fn edge(&self, id: &crate::edge::EdgeId) -> Option<crate::edge::EdgeRecord> {
                self.0.edge(id)
            }
        }
        let g = with_edges(graph_with(&[("bible/1.1.1", "a"), ("bible/1.1.2", "b")]));
        let mut store = MemStore::default();
        let v = store.publish(with_edges(graph_with(&[("bible/1.1.1", "a"), ("bible/1.1.2", "b")]))).unwrap();
        let snap = store.open(v).unwrap();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| assert_answers_match(&LiesAboutRows(snap), &g)));
        assert!(caught.is_err(), "a provenance lie must fail conformance");
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Lie {
        Relabels,
        MovesAnEnd,
        Rereferences,
    }

    struct Altered {
        honest: MemSnapshot,
        lie: Option<Lie>,
    }

    impl GraphQuery for Altered {
        fn node(&self, id: &AnyNodeId) -> Option<Node> {
            self.honest.node(id)
        }
        fn derive(&self, pid: &Pid) -> Option<Vec<u8>> {
            self.honest.derive(pid)
        }
        fn edge_summary(&self, p: &Position) -> EdgeSummary {
            self.honest.edge_summary(p)
        }
        fn edges(&self, p: &Position, q: &EdgeQuery) -> EdgePage {
            self.honest.edges(p, q)
        }
        fn reading_window(&self, c: &'static str, s: usize, n: usize) -> Vec<AnyNodeId> {
            self.honest.reading_window(c, s, n)
        }
        fn nodes_of_kind(&self, k: NodeKind, c: Option<usize>, l: usize) -> NodePage {
            self.honest.nodes_of_kind(k, c, l)
        }
        fn row_provenance(&self, e: &crate::edge::EdgeId) -> Option<RowRef> {
            self.honest.row_provenance(e)
        }
        fn position_of(&self, c: &'static str, id: &AnyNodeId) -> Option<usize> {
            self.honest.position_of(c, id)
        }
        fn labels(&self, at: &[Position]) -> Vec<Option<String>> {
            self.honest.labels(at).into_iter().map(|label| label.map(|l| if self.lie == Some(Lie::Relabels) { format!("{l}!") } else { l })).collect()
        }
        fn references(&self, units: &[AnyNodeId]) -> Vec<Option<String>> {
            self.honest.references(units).into_iter().map(|reference| reference.map(|r| if self.lie == Some(Lie::Rereferences) { format!("{r}!") } else { r })).collect()
        }
        fn edge(&self, id: &crate::edge::EdgeId) -> Option<crate::edge::EdgeRecord> {
            self.honest.edge(id).map(|mut record| {
                if self.lie == Some(Lie::MovesAnEnd) {
                    record.object = record.subject.clone();
                }
                record
            })
        }
    }

    const GENESIS_1_1_RAW: &str = "bible/1.1.1";
    const GENESIS_1_1_REFERENCE: &str = "GEN.1.1";

    fn model() -> Graph {
        referenced(labelled(with_edges(graph_with(&[(GENESIS_1_1_RAW, "a")]))))
    }

    fn referenced(mut g: Graph) -> Graph {
        g.references = [(AnyNodeId { kind: NodeKind::TextUnit, raw: GENESIS_1_1_RAW.into() }, GENESIS_1_1_REFERENCE.to_string())].into_iter().collect();
        g
    }

    fn labelled(mut g: Graph) -> Graph {
        g.edges_by_id = g.edge_records();
        g.labels = g.positions().into_iter().map(|p| { let shown = format!("{p:?}"); (p, shown) }).collect();
        g
    }

    fn admitted(candidate: &Altered, model: &Graph) -> bool {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| assert_answers_match(candidate, model))).is_ok()
    }

    fn altered(lie: Option<Lie>) -> Altered {
        let mut store = MemStore::default();
        let v = store.publish(model()).unwrap();
        Altered { honest: store.open(v).unwrap(), lie }
    }

    #[test]
    fn every_held_position_answers_one_compiled_label() {
        // Arrange
        let model = model();
        let verse = Position::Node(AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() });
        let unheld = Position::Node(AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/9.9.9".into() });

        // Act
        let read = model.labels(&[verse.clone(), unheld]);
        let verdicts = (admitted(&altered(None), &model), admitted(&altered(Some(Lie::Relabels)), &model));

        // Assert
        assert_eq!((read, verdicts), (vec![Some(format!("{verse:?}")), None], (true, false)));
    }

    #[test]
    fn every_text_unit_answers_one_compiled_reference_and_no_other_node_answers_one() {
        // Arrange
        let model = model();
        let verse = AnyNodeId { kind: NodeKind::TextUnit, raw: GENESIS_1_1_RAW.into() };
        let unheld = AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/9.9.9".into() };
        let event = EventId::new("e1").erase();

        // Act
        let read = model.references(&[verse, unheld, event]);
        let verdicts = (admitted(&altered(None), &model), admitted(&altered(Some(Lie::Rereferences)), &model));

        // Assert
        assert_eq!((read, verdicts), (vec![Some(GENESIS_1_1_REFERENCE.to_string()), None, None], (true, false)));
    }

    #[test]
    fn every_edge_is_read_by_its_id_with_its_ends_and_its_meta() {
        // Arrange
        let model = model();
        let e1 = Position::Node(EventId::new("e1").erase());
        let e2 = Position::Node(EventId::new("e2").erase());
        let jordan = Position::Node(PlaceId::new("jordan").erase());
        let located = crate::edge::entry_id(crate::edge::RelationId::LocatedAt, &e1, &jordan);
        let followed = crate::edge::entry_id(crate::edge::RelationId::Succession, &e1, &e2);

        // Act
        let read = (model.edge(&located), model.edge(&followed), model.edge(&crate::edge::EdgeId("LocatedAt:00".into())));
        let verdicts = (admitted(&altered(None), &model), admitted(&altered(Some(Lie::MovesAnEnd)), &model));

        // Assert
        assert_eq!(
            (read, verdicts),
            (
                (
                    Some(crate::edge::EdgeRecord {
                        id: located,
                        kind: crate::edge::EdgeKind::Directed(crate::edge::RelationId::LocatedAt, crate::edge::Direction::Forward),
                        subject: e1.clone(),
                        object: jordan,
                        meta: crate::adjacency::EdgeMeta::None,
                    }),
                    Some(crate::edge::EdgeRecord {
                        id: followed,
                        kind: crate::edge::EdgeKind::Directed(crate::edge::RelationId::Succession, crate::edge::Direction::Forward),
                        subject: e1,
                        object: e2,
                        meta: crate::adjacency::EdgeMeta::Narrative(crate::id::NarrativeId::new("n")),
                    }),
                    None,
                ),
                (true, false),
            )
        );
    }

    #[test]
    fn a_symmetric_edge_is_recorded_once_from_its_lesser_end() {
        // Arrange
        let mut g = Graph::default();
        g.analogue.push(Analogue { a: EventId::new("e2"), b: EventId::new("e1"), provenance: "p".into() });
        g.build_indexes();
        let (lesser, greater) = (Position::Node(EventId::new("e1").erase()), Position::Node(EventId::new("e2").erase()));
        let id = crate::edge::entry_id_symmetric(crate::edge::SymRelationId::Analogue, &lesser, &greater);

        // Act
        let records = g.edge_records();

        // Assert
        assert_eq!(
            records.into_iter().collect::<Vec<_>>(),
            vec![(
                id.clone(),
                crate::edge::EdgeRecord { id, kind: crate::edge::EdgeKind::Symmetric(crate::edge::SymRelationId::Analogue), subject: lesser, object: greater, meta: crate::adjacency::EdgeMeta::None }
            )]
        );
    }
}
