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

// --------------------------------------- Task 5: positions, persons_by_verse

/// ORACLE: service.rs's bible_position / concord_position, verbatim.
fn oracle_positions(g: &Graph, corpus: &'static str) -> Positions {
    g.reading.get(corpus).map(|spine| spine.order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect()).unwrap_or_default()
}

/// ORACLE: service.rs's persons_by_verse, verbatim (Person mentions only, row order, keyed by dotted ref).
fn oracle_persons_by_verse(g: &Graph) -> HashMap<String, Vec<(String, String)>> {
    let mut out: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for row in &g.mentions {
        let atlas_graph_types::edge::MentionedEntity::Person(person_id) = &row.entity else { continue };
        let Some(key) = atlas_graph::legacy::locus_dot_ref(&row.locus) else { continue };
        let Some(label) = g.nodes.get(&person_id.erase()).and_then(|n| match &n.payload {
            NodePayload::Person { label, .. } => Some(label.clone()),
            _ => None,
        }) else {
            continue;
        };
        out.entry(key).or_default().push((person_id.0.clone(), label));
    }
    out
}

#[test]
fn every_spine_slot_is_answered_by_position_of_and_nothing_else_is() {
    let g = committed_graph();
    let s = service();
    let bible = oracle_positions(g, "bible");
    assert!(!bible.is_empty());
    for (id, want) in &bible {
        let (b, c, v) = atlas_graph::kjv_adapter::decode_text_unit(id).expect("bible spine id decodes");
        assert_eq!(s.position_of(b, c, v), Some(*want), "bible {id:?}");
    }
    let concord = oracle_positions(g, "concord");
    assert!(!concord.is_empty());
    for (id, want) in &concord {
        let (p, a, para) = atlas_graph::concord_adapter::decode_text_unit(id).expect("concord spine id decodes");
        assert_eq!(s.concord_position_of(p, a, para), Some(*want), "concord {id:?}");
    }
    assert_eq!(s.position_of(1, 1, 200), None, "GEN 1:200 is off the spine");
    assert_eq!(s.concord_position_of(99, 1, 1), None);
}

#[test]
fn persons_at_verse_equals_the_retired_persons_by_verse_over_every_verse() {
    let g = committed_graph();
    let s = service();
    let oracle = oracle_persons_by_verse(g);
    assert!(oracle.len() > 1000, "the shipped graph mentions persons in thousands of verses: {}", oracle.len());
    let mut inhabited = 0usize;
    for id in &g.reading["bible"].order {
        let (b, c, v) = atlas_graph::kjv_adapter::decode_text_unit(id).unwrap();
        let key = atlas_graph::kjv_adapter::dot_ref(b, c, v);
        let want = oracle.get(&key).cloned().unwrap_or_default();
        inhabited += usize::from(!want.is_empty());
        assert_eq!(s.persons_at_verse(b, c, v), want, "{key}");
    }
    assert_eq!(inhabited, oracle.len(), "every keyed verse is a spine verse");
}
