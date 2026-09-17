//! The storage PORT (design spec §9a): the store is a port, not a place.
//!
//! GraphQuery is THE shared contract (owner, 2026-08-22: "if graph and
//! graph snapshot share a contract why don't we write a shared
//! interface") — implemented by the concrete Graph itself (the canonical
//! instance, typed rather than asserted) and by every backend snapshot.
//! A snapshot is a versioned GraphQuery, nothing more. Conformance is
//! therefore a comparison of any implementation directly against the
//! Graph, and the same harness verifies backend-vs-backend migrations.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::explore::{EdgeEntry, EdgePage, EdgeQuery, EdgeSummary, Explorable, PositionRef};
use crate::graph::Graph;
use crate::id::{AnyNodeId, ContentAddressed, ContentHash, Pid, Position};
use crate::node::Node;

/// The version root: one stamp identifies one immutable compiled graph.
/// (Skeleton: hash of the node table's canonical bytes; production: the
/// Merkle root over every thing.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphVersion(pub ContentHash);

/// THE SHARED QUERY CONTRACT: what it means to answer graph questions.
/// The concrete Graph implements it (canonical instance); every backend
/// snapshot implements it; serving composes it and nothing else.
pub trait GraphQuery {
    /// Node lookup by erased id.
    fn node(&self, id: &AnyNodeId) -> Option<Node>;

    /// derive: the content-addressed store law — the canonical bytes of
    /// the thing a pid names; must hash back to the pid.
    fn derive(&self, pid: &Pid) -> Option<Vec<u8>>;

    /// Frontier counts for one position (kind → count; inhabited only).
    fn edge_summary(&self, p: &Position) -> EdgeSummary;

    /// One page of one edge kind at one position.
    fn edges(&self, p: &Position, q: &EdgeQuery) -> EdgePage;

    /// A window along one corpus's reading spine (unit ids, canonical
    /// order). Text composes via `node`.
    fn reading_window(&self, corpus: &'static str, start: usize, n: usize) -> Vec<AnyNodeId>;
}

/// The canonical instance: the Graph answers its own questions. Typed
/// conformance — "a snapshot is basically an instance of graph" is now
/// a supertrait fact, not an intuition.
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
}

/// A snapshot is a VERSIONED GraphQuery — the shared contract plus one
/// stamp. Immutable: publishing a new version never changes what an
/// open snapshot answers (law-tested).
pub trait GraphSnapshot: GraphQuery {
    fn version(&self) -> GraphVersion;
}

/// Read-side port: versioned, snapshot-consistent access.
pub trait GraphStore {
    type Snapshot: GraphSnapshot;
    fn current_version(&self) -> Option<GraphVersion>;
    fn open(&self, v: GraphVersion) -> Option<Self::Snapshot>;
    fn open_current(&self) -> Option<Self::Snapshot> {
        self.current_version().and_then(|v| self.open(v))
    }
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
/// (Production: Merkle root. The LAW — same content ⇒ same version — is
/// what the tests pin.)
///
/// KNOWN DEFECT, disclosed rather than hidden (spec §3.1 defect 1): this
/// hashes the NODE TABLE ONLY. Add a row — a `located_at`, a `mentions` —
/// with the nodes untouched and the version root does not move, so two
/// materially different graphs share one stamp. `version_root_is_blind_to_rows`
/// below asserts that defect so it cannot be forgotten; the `canon-ids`
/// version underneath FIXES it.
#[cfg(not(feature = "canon-ids"))]
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

/// ON: the root is SHA-256-128 over the LOGICAL DUMP — every node, every
/// row of every family, every reading spine, which is what closes spec
/// §3.1 defect 1.
///
/// What the stamp deliberately EXCLUDES: `indexes`, `symmetric_indexes`
/// and `pid_index`. Those are DERIVED by `build_indexes` from the rows
/// already in the dump — hashing them would stamp the same information
/// twice and make the root depend on the index builder's internals rather
/// than on the graph's content. The dump is the AUTHORED state; the
/// derived state is a function of it.
#[cfg(feature = "canon-ids")]
fn version_of(g: &Graph) -> GraphVersion {
    GraphVersion(ContentHash(crate::sha256::sha256_prefixed_128(
        crate::canon::DOMAIN_PREFIX,
        &logical_dump(g),
    )))
}

/// The graph as one canonical byte stream — the thing the version root is
/// a digest of.
///
/// `pub` deliberately: DB-2b's artifact writer must hash the IDENTICAL
/// stream, section by section, by filtering this same walk down to the
/// families and nodes a section carries. Two independent implementations
/// of "the bytes of a graph" is precisely the bug that would make an
/// artifact's stamp disagree with the server's.
///
/// The format, exactly:
///
/// ```text
/// node\t<canonical JSON of the node>\n          (each node, `nodes` order)
/// <family>\t<canonical JSON of the row>\n       (RowFamily::ALL order, rows in table order)
/// spine\t<corpus>\t<id,id,id>\n                 (each corpus, `reading` order)
/// ```
///
/// The family NAME is the line's own first field, so the row bytes are
/// the row's plain `Canon::encode` — the family is said once, not twice.
/// Node and spine ids use `canon::ids::any_node_id_str`, the crate's one
/// canonical string spelling for an id. `DOMAIN_PREFIX` is NOT part of
/// this stream: it is hashed in front of it by `version_of`, so the dump
/// stays readable as itself.
#[cfg(feature = "canon-ids")]
pub fn logical_dump(g: &Graph) -> Vec<u8> {
    use crate::canon::ids::any_node_id_str;
    use crate::canon::{Canon, RowFamily};

    let mut out: Vec<u8> = Vec::new();

    let mut line = |tag: &str, bytes: &[u8]| {
        out.extend_from_slice(tag.as_bytes());
        out.push(b'\t');
        out.extend_from_slice(bytes);
        out.push(b'\n');
    };

    for node in g.nodes.values() {
        line("node", &node.encode());
    }

    // One arm per family, in `RowFamily::ALL` order. The exhaustive match
    // is the point: a new family cannot be added to the enum without the
    // compiler demanding its place in the version root.
    for family in RowFamily::ALL {
        macro_rules! rows {
            ($field:ident) => {
                for row in &g.$field {
                    line(family.name(), &row.encode());
                }
            };
        }
        match family {
            RowFamily::ContainsBible => rows!(contains_bible),
            RowFamily::ContainsConcord => rows!(contains_concord),
            RowFamily::Attests => rows!(attests),
            RowFamily::Succession => rows!(succession),
            RowFamily::CanonSuccession => rows!(canon_succession),
            RowFamily::DatedBy => rows!(dated_by),
            RowFamily::LocatedAt => rows!(located_at),
            RowFamily::Fulfills => rows!(fulfills),
            RowFamily::Typology => rows!(typology),
            RowFamily::NamedAfter => rows!(named_after),
            RowFamily::Catechism => rows!(catechism),
            RowFamily::CommentsOn => rows!(comments_on),
            RowFamily::SpokenBy => rows!(spoken_by),
            RowFamily::SpokenAt => rows!(spoken_at),
            RowFamily::Mentions => rows!(mentions),
            RowFamily::CrossRefs => rows!(cross_refs),
            RowFamily::Quotes => rows!(quotes),
            RowFamily::Confesses => rows!(confesses),
            RowFamily::CorrespondsBible => rows!(corresponds_bible),
            RowFamily::TemporalAdjacency => rows!(temporal_adjacency),
            RowFamily::Analogue => rows!(analogue),
        }
    }

    for (corpus, spine) in &g.reading {
        let ids: Vec<String> = spine.order.iter().map(any_node_id_str).collect();
        out.extend_from_slice(b"spine\t");
        out.extend_from_slice(corpus.as_bytes());
        out.push(b'\t');
        out.extend_from_slice(ids.join(",").as_bytes());
        out.push(b'\n');
    }

    out
}

/// A version-stamped handle to a Graph — the canonical presentation.
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

/// Pure delegation to the Graph's own GraphQuery — conformance by
/// construction, visible in the code.
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
}

impl GraphSnapshot for MemSnapshot {
    fn version(&self) -> GraphVersion {
        self.version
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
            .map(|g| MemSnapshot::present(v, Arc::clone(g)))
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

// ---------------------------------------------------------------------
// The conformance law, executable.
// ---------------------------------------------------------------------

/// Every position the model knows: the node table PLUS every subject and
/// object in the built indexes — so a sparse node table cannot produce a
/// vacuously-passing check (the lying-snapshot test demonstrates this
/// deliberately).
fn position_inventory(model: &Graph) -> BTreeSet<Position> {
    let mut out: BTreeSet<Position> = model
        .nodes
        .keys()
        .map(|id| Position::Node(id.clone()))
        .collect();
    for ix in model.indexes.values() {
        out.extend(ix.fwd.keys().cloned());
        out.extend(ix.inv.keys().cloned());
    }
    // Survey gotcha 1: `symmetric_indexes` is the SAME kind of built,
    // never-authored adjacency `indexes` is (`BiIndex::build_symmetric`
    // instead of `build`) -- a row whose only lowering is symmetric (e.g.
    // an `Analogue` pair with no directed row alongside it) has both its
    // ends live ONLY here, so omitting this loop left them outside "every
    // position the model knows" (position_inventory_covers_symmetric_
    // indexes pins the gap this closes).
    for ix in model.symmetric_indexes.values() {
        out.extend(ix.fwd.keys().cloned());
        out.extend(ix.inv.keys().cloned());
    }
    out
}

fn drain(q: &impl GraphQuery, p: &Position, kind: crate::edge::EdgeKind, limit: usize) -> Vec<EdgeEntry> {
    let mut cursor = None;
    let mut out = Vec::new();
    loop {
        let page = q.edges(p, &EdgeQuery { kind, cursor, limit });
        out.extend(page.entries);
        match page.next {
            Some(c) => cursor = Some(c),
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

/// THE CONFORMANCE LAW: any GraphQuery implementation claiming to
/// present `model` must answer every question identically to the Graph
/// itself (which implements the same interface — no canonical-clone
/// dance). Works for backend-vs-model AND, via two calls, for
/// backend-vs-backend migration verification. Panics at the first
/// divergence, precisely named. This is the port admission requirement:
/// MemStore passes by construction; the serialized backend (M-C) and
/// any future database must pass the same call to exist.
pub fn assert_answers_match(candidate: &impl GraphQuery, model: &Graph) {
    for p in position_inventory(model) {
        if let Position::Node(id) = &p {
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

        let sa = candidate.edge_summary(&p);
        let sb = model.edge_summary(&p);
        assert_eq!(sa, sb, "conformance: edge_summary({:?}) diverges", p);

        for (kind, count) in sb {
            for limit in [1usize, count.max(1)] {
                assert_eq!(
                    drain(candidate, &p, kind, limit),
                    drain(model, &p, kind, limit),
                    "conformance: edges({:?}, {:?}, limit {}) diverges",
                    p,
                    kind,
                    limit
                );
            }
        }
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

        let v1 = store.publish(graph_with(&[("bible/1.1.1", "In the beginning")]));
        let snap1 = store.open(v1).unwrap();

        let v2 = store.publish(graph_with(&[
            ("bible/1.1.1", "In the beginning"),
            ("bible/1.1.2", "And the earth"),
        ]));
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

    /// True in BOTH states: `hex` is the one wire spelling and `from_hex`
    /// its strict inverse. Width is asserted separately, per state.
    #[test]
    fn content_hash_hex_and_from_hex_are_inverse() {
        let h = version_of(&graph_with(&[("bible/1.1.1", "In the beginning")])).0;
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
        let h = version_of(&graph_with(&[("bible/1.1.1", "In the beginning")])).0;
        assert_eq!(h.hex().len(), 16);
    }

    #[cfg(feature = "canon-ids")]
    #[test]
    fn hex_is_thirty_two_chars_while_the_hash_is_a_hundred_and_twenty_eight_bits() {
        let h = version_of(&graph_with(&[("bible/1.1.1", "In the beginning")])).0;
        assert_eq!(h.hex().len(), 32);
    }

    /// The content-addressing law, stated over the SHA-256 ids: identity
    /// follows content and nothing else. Two nodes built independently
    /// from equal parts share a pid; one byte of payload text apart, they
    /// do not.
    #[cfg(feature = "canon-ids")]
    #[test]
    fn equal_nodes_have_equal_pids_and_one_payload_byte_moves_them() {
        let a = unit("bible/1.1.1", "In the beginning");
        let b = unit("bible/1.1.1", "In the beginning");
        assert_eq!(a.pid(), b.pid(), "equal content, equal pid");

        let c = unit("bible/1.1.1", "In the beginninq"); // one byte apart
        assert_ne!(a.pid(), c.pid(), "one byte of payload is one different id");
        assert_eq!(a.pid().kind, c.pid().kind, "only the hash moved, not the kind");
    }

    /// Spec §3.1 defect 1, ON: the root covers ROWS. Adding a
    /// `succession` and a `located_at` with the node table untouched must
    /// move the version.
    /// The ONE setup both sides of spec §3.1 defect 1 argue over: two
    /// graphs with the SAME node table, one of which also carries a
    /// `succession` and a `located_at` row. Shared so the two cfg'd tests
    /// below differ in exactly the thing they disagree about — the
    /// assertion — and cannot drift into comparing different graphs.
    /// The node-table equality is asserted HERE, once, because it is the
    /// premise of both, not a claim either test is making.
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
        assert_ne!(version_of(&base), version_of(&rowed), "a row changes the root");
    }

    /// Spec §3.1 defect 1, OFF: the SAME comparison, asserted the other
    /// way. This is not a passing test celebrating correct behaviour --
    /// it PINS a known defect so it cannot be quietly inherited: today's
    /// root hashes the node table alone, so a graph that gained two edge
    /// rows still stamps as the graph that did not.
    #[cfg(not(feature = "canon-ids"))]
    #[test]
    fn version_root_is_blind_to_rows_the_documented_defect() {
        let (base, rowed) = base_and_rowed();
        assert_eq!(
            version_of(&base),
            version_of(&rowed),
            "spec §3.1 defect 1: the skeleton root is blind to rows -- \
             `canon-ids` fixes this, and the ON sibling of this test asserts the fix"
        );
    }

    /// The dump is the version root's preimage, so its SHAPE is a law:
    /// node lines first, then family lines tagged with the family's own
    /// table name, then one spine line per corpus.
    #[cfg(feature = "canon-ids")]
    #[test]
    fn logical_dump_carries_nodes_then_rows_then_spines() {
        let g = with_edges(graph_with(&[("bible/1.1.1", "a"), ("bible/1.1.2", "b")]));
        let dump = String::from_utf8(logical_dump(&g)).expect("the dump is UTF-8");
        let lines: Vec<&str> = dump.lines().collect();

        let tags: Vec<&str> = lines.iter().map(|l| l.split('\t').next().unwrap()).collect();
        assert_eq!(&tags[..2], &["node", "node"], "nodes lead, in `nodes` order");
        assert_eq!(
            &tags[2..],
            &["succession", "located_at", "spine"],
            "then families in RowFamily::ALL order, then the spines"
        );

        let spine = lines.last().unwrap();
        assert_eq!(
            *spine,
            "spine\tbible\tTextUnit:bible/1.1.1,TextUnit:bible/1.1.2",
            "the spine line is corpus + comma-joined canonical ids"
        );
        // The node line carries the node's own canonical JSON, not a
        // debug print -- decodable, which is the whole point.
        assert!(lines[0].starts_with("node\t{\"id\":\"TextUnit:bible/1.1.1\","), "{}", lines[0]);
    }

    #[test]
    fn conformance_snapshot_matches_the_graph_itself() {
        let g = with_edges(graph_with(&[("bible/1.1.1", "a"), ("bible/1.1.2", "b")]));
        let mut store = MemStore::default();
        let v = store.publish(with_edges(graph_with(&[
            ("bible/1.1.1", "a"),
            ("bible/1.1.2", "b"),
        ])));
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
                page.entries.pop(); // the lie: drop one connection
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
        }

        // Deliberately SPARSE node table: e1/jordan exist only in edge
        // rows — the index-derived position inventory must still catch
        // the lie (this is the harness-robustness law).
        let g = with_edges(graph_with(&[("bible/1.1.1", "a")]));
        let mut store = MemStore::default();
        let v = store.publish(with_edges(graph_with(&[("bible/1.1.1", "a")])));
        let liar = Lying(store.open(v).unwrap());
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_answers_match(&liar, &g)
        }));
        assert!(caught.is_err(), "the harness must catch a dropped edge");
    }

    /// Survey gotcha 1: `position_inventory` walked `indexes` but never
    /// `symmetric_indexes`, so a graph whose ONLY row is symmetric (no
    /// directed row at all) reports an inventory that omits both of that
    /// row's own ends -- a real blind spot in the port admission
    /// harness (`assert_answers_match` above quantifies "every position
    /// the model knows" over exactly this set). Analogue is the row: two
    /// Event ids, no directed row alongside them, no node table entries
    /// either (the same sparse-table shape `conformance_harness_catches_
    /// a_lying_snapshot` above already relies on).
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
