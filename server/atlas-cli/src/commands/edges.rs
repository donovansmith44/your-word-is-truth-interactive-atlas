//! `atlas edges <id> [--kind K] [--limit N] [--cursor C]` -- one frontier
//! page, the exact `(Position, EdgeQuery)` shape
//! `atlas_server::graph_handlers::node_edges` serves. See CONTRACT.md's
//! own "atlas edges" section.

use atlas_graph::GraphService;
use atlas_graph_types::explore::EdgeQuery;
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;
use atlas_server::graph_wire::{decode_node_id, describe_position, parse_edge_kind};

use crate::error::CliError;

const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 200;

pub struct EdgesArgs<'a> {
    pub id_raw: &'a str,
    pub kind_raw: Option<&'a str>,
    pub limit: Option<usize>,
    pub cursor: Option<usize>,
}

pub fn run(graph: &GraphService, args: EdgesArgs) -> Result<String, CliError> {
    let node_id = decode_node_id(args.id_raw).ok_or_else(|| {
        CliError::bad_ref(
            format!("'{}' is not a valid node id", args.id_raw),
            "expected KIND:raw (e.g. text-unit:GEN.1.1, Event:ab_ur, Place:jericho)",
            "run 'atlas find <term>' to locate an id, or 'atlas node <id>' to see what kind it is",
        )
    })?;

    let snap = graph.snapshot();
    if snap.node(&node_id).is_none() {
        return Err(CliError::not_found(
            format!("no node named '{}'", args.id_raw),
            "the id parsed fine but this graph has no node with that raw id",
            "try 'atlas find <term>' to locate the id you meant",
        ));
    }

    let kind_raw = args.kind_raw.ok_or_else(|| {
        CliError::bad_usage(
            "--kind is required for 'atlas edges'",
            "a node can carry several distinct edge kinds; there is no honest default one to pick",
            "run 'atlas node <id>' first to see which kinds are inhabited for this id, then pass --kind <one of them>",
        )
    })?;
    let kind = parse_edge_kind(kind_raw).ok_or_else(|| {
        CliError::bad_ref(
            format!("'{kind_raw}' is not a known edge kind"),
            "edge kinds are the labels graph-types' own relation manifest defines (e.g. cites, cited-by, attests, mentions)",
            "run 'atlas node <id>' to see which kinds this id actually carries",
        )
    })?;

    let limit = args.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let page = snap.edges(&Position::Node(node_id), &EdgeQuery { kind, cursor: args.cursor, limit });

    if page.entries.is_empty() {
        return Err(CliError::empty_result(
            format!("no '{kind_raw}' edges at '{}'", args.id_raw),
            "the id and kind both parsed fine, but this node has zero edges of that kind at this position",
            "run 'atlas node <id>' to see which kinds actually have entries here",
        ));
    }

    let mut out = String::new();
    for entry in &page.entries {
        let (id, kind_str, label) = describe_position(&entry.node, &snap);
        out.push_str(&format!("{:<24} {:<12} {:<28} {}\n", entry.edge.0, kind_str, id, label));
    }
    match page.next {
        Some(n) => out.push_str(&format!("more: continue with --cursor {n}\n")),
        None => out.push_str("(end of list)\n"),
    }
    Ok(out)
}
