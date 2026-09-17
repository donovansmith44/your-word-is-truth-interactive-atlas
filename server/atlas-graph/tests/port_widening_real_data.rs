//! DB-3 (spec §4, §6.2): every retired companion, proven equal to its port
//! composition over the COMMITTED graph before it was deleted. Each test's
//! ORACLE is the retired computation, moved here verbatim from
//! `service.rs` / `provenance.rs` at the commit that deleted it -- so the
//! equivalence is between the exact code that used to serve and the code
//! that serves now.
#![allow(clippy::type_complexity)]

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::OnceLock;

use atlas_graph::service::GraphService;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnyNodeId, NodeKind};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;

fn committed_graph() -> &'static Graph {
    static CACHED: OnceLock<Graph> = OnceLock::new();
    CACHED.get_or_init(|| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled/graph.bin");
        let dump = atlas_graph::artifact::read_file(&path).expect("data/compiled/graph.bin must exist");
        let (mut graph, ..) = atlas_graph::artifact::to_service_parts(dump).expect("to_service_parts");
        graph.build_indexes();
        atlas_graph::event_world::add_justified_by(&mut graph);
        graph
    })
}

fn service() -> &'static GraphService {
    static S: OnceLock<GraphService> = OnceLock::new();
    S.get_or_init(|| {
        GraphService::from_artifact(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled/graph.bin"))
            .expect("from_artifact")
    })
}

// ------------------------------------------------------ Task 4: the id lists

/// ORACLE: service.rs's five alphabetical lists, verbatim.
fn oracle_ids(g: &Graph, kind: NodeKind) -> Vec<AnyNodeId> {
    g.nodes.keys().filter(|id| id.kind == kind).cloned().collect()
}

/// ORACLE: service.rs's `era_ids`, verbatim (chronological by from_year, then raw).
fn oracle_era_ids(g: &Graph) -> Vec<AnyNodeId> {
    let mut era_nodes: Vec<(i32, AnyNodeId)> = g
        .nodes
        .iter()
        .filter_map(|(id, n)| match &n.payload {
            NodePayload::Era { from_year, .. } if id.kind == NodeKind::Era => Some((*from_year, id.clone())),
            _ => None,
        })
        .collect();
    era_nodes.sort_by_key(|(from_year, id)| (*from_year, id.raw.clone()));
    era_nodes.into_iter().map(|(_, id)| id).collect()
}

#[test]
fn the_five_alphabetical_id_lists_equal_nodes_of_kind() {
    let g = committed_graph();
    let s = service();
    for kind in [NodeKind::Polity, NodeKind::Narrative, NodeKind::Event, NodeKind::Place, NodeKind::Person] {
        let got = s.ids_of_kind(kind);
        assert_eq!(got, oracle_ids(g, kind), "{kind:?}");
        assert!(!got.is_empty(), "{kind:?} is inhabited in the shipped graph");
    }
    assert_eq!(s.ids_of_kind(NodeKind::LexiconEntry), Vec::<AnyNodeId>::new(), "uninhabited until LEX-1");
}

#[test]
fn the_eras_wire_order_is_reproduced_by_sorting_the_payloads() {
    let g = committed_graph();
    let s = service();
    let snap = s.snapshot();
    // The handler's own composition (handlers::eras): nodes_of_kind, then sort by (from_year, id).
    let mut got: Vec<(i32, AnyNodeId)> = s
        .ids_of_kind(NodeKind::Era)
        .into_iter()
        .filter_map(|id| match snap.node(&id)?.payload {
            NodePayload::Era { from_year, .. } => Some((from_year, id)),
            _ => None,
        })
        .collect();
    got.sort_by(|a, b| (a.0, &a.1.raw).cmp(&(b.0, &b.1.raw)));
    let got: Vec<AnyNodeId> = got.into_iter().map(|(_, id)| id).collect();
    assert!(!got.is_empty());
    assert_eq!(got, oracle_era_ids(g));
}

// keep the map alias in scope for the later tasks' oracles
#[allow(dead_code)]
type Positions = HashMap<AnyNodeId, usize>;
#[allow(dead_code)]
type ByKey = BTreeMap<String, Vec<String>>;
