//! `bibex edges <id> [--kind K] [--limit N] [--cursor C]` -- one page of a node's edges.

use atlas_graph::GraphService;
use atlas_graph_types::edge::EdgeKind;
use atlas_graph_types::adjacency::EdgeQuery;
use atlas_graph_types::id::{NodeKind, Position};
use atlas_graph_types::store::GraphQuery;
use atlas_contract::graph_wire::{decode_node_id, describe_position, edge_ref, labelled_positions};
use atlas_contract::wire::{EdgeRef, PositionRef};

use crate::error::CliError;

const DEFAULT_LIMIT: usize = 20;
const AN_EDGE: &str = "Edge";
const MAX_LIMIT: usize = 200;

pub struct EdgesArgs<'a> {
    pub id_raw: &'a str,
    pub kind_raw: Option<&'a str>,
    pub limit: Option<usize>,
    pub cursor: Option<usize>,
}

struct ResolvedEntry {
    edge: EdgeRef,
    neighbour: PositionRef,
}

impl ResolvedEntry {
    fn line(&self) -> String {
        let (id, kind, label) = match &self.neighbour {
            PositionRef::Node { node } => (node.id.as_str(), node.kind.name(), node.label.as_str()),
            PositionRef::Edge { edge } => (edge.id.as_str(), AN_EDGE, edge.id.as_str()),
        };
        format!("{:<24} {:<12} {:<28} {}\n", self.edge.id, kind, id, label)
    }
}

struct ResolvedPage {
    /// The canonical token for the kind this page answered, not necessarily the spelling
    /// the caller typed.
    kind_label: String,
    entries: Vec<ResolvedEntry>,
    next: Option<usize>,
}

fn resolve(graph: &GraphService, args: &EdgesArgs) -> Result<ResolvedPage, CliError> {
    let node_id = decode_node_id(args.id_raw).ok_or_else(|| {
        CliError::bad_ref(
            format!("'{}' is not a valid node id", args.id_raw),
            "expected KIND:raw (e.g. text-unit:GEN.1.1, Event:ab_ur, Place:jericho)",
            "run 'bibex find <term>' to locate an id, or 'bibex node <id>' to see what kind it is",
        )
    })?;

    let snap = graph.snapshot();
    if snap.node(&node_id).is_none() {
        return Err(CliError::not_found(
            format!("no node named '{}'", args.id_raw),
            "the id parsed fine but this graph has no node with that raw id",
            "try 'bibex find <term>' to locate the id you meant",
        ));
    }

    let kind_raw = args.kind_raw.ok_or_else(|| {
        CliError::bad_usage(
            "--kind is required for 'bibex edges'",
            "a node can carry several distinct edge kinds; there is no honest default one to pick",
            "run 'bibex node <id>' first to see which kinds are inhabited for this id, then pass --kind <one of them>",
        )
    })?;
    let kind = EdgeKind::from_label(kind_raw).ok_or_else(|| {
        CliError::bad_ref(
            format!("'{kind_raw}' is not a known edge kind"),
            "edge kinds are the labels graph-types' own relation manifest defines (e.g. cites, cited-by, attests, mentions)",
            "run 'bibex node <id>' to see which kinds this id actually carries",
        )
    })?;

    let limit = args.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let page = snap.edges(&Position::Node(node_id), &EdgeQuery { kind, cursor: args.cursor, limit });

    // A PeopleGroup neighbour is filtered out because its id cannot be decoded back by
    // `node`/`edges`: handing one out would be a dead end this command never discloses.
    let entries: Vec<ResolvedEntry> = page
        .entries
        .iter()
        .filter(|e| !matches!(&e.node, Position::Node(id) if id.kind == NodeKind::PeopleGroup))
        .map(|e| {
            let neighbour = describe_position(&e.node, &snap)?;
            let label = labelled_positions(&[Position::Edge(e.edge.clone())], &snap)?.remove(0);
            Ok(ResolvedEntry { edge: edge_ref(&e.edge, label)?, neighbour })
        })
        .collect::<Result<_, atlas_contract::error::ApiError>>()
        .map_err(CliError::unlabelled)?;

    if entries.is_empty() {
        return Err(CliError::empty_result(
            format!("no '{kind_raw}' edges at '{}'", args.id_raw),
            "the id and kind both parsed fine, but this node has zero edges of that kind at this position",
            "run 'bibex node <id>' to see which kinds actually have entries here",
        ));
    }

    Ok(ResolvedPage { kind_label: kind.label().to_string(), entries, next: page.next })
}

pub fn run(graph: &GraphService, args: EdgesArgs) -> Result<String, CliError> {
    let page = resolve(graph, &args)?;

    let mut out = String::new();
    for entry in &page.entries {
        out.push_str(&entry.line());
    }
    match page.next {
        Some(n) => out.push_str(&format!("more: continue with --cursor {n}\n")),
        None => out.push_str("(end of list)\n"),
    }
    Ok(out)
}

pub fn run_json(graph: &GraphService, args: EdgesArgs) -> Result<serde_json::Value, CliError> {
    let page = resolve(graph, &args)?;
    let entries: Vec<_> = page.entries.iter().map(|e| serde_json::json!({"edge": e.edge, "neighbour": e.neighbour})).collect();
    Ok(serde_json::json!({"kind": page.kind_label, "entries": entries, "next": page.next}))
}
