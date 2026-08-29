//! `bibex edges <id> [--kind K] [--limit N] [--cursor C]` -- one frontier
//! page, the exact `(Position, EdgeQuery)` shape
//! `atlas_server::graph_handlers::node_edges` serves. See CONTRACT.md's
//! own "bibex edges" section.

use atlas_graph::GraphService;
use atlas_graph_types::explore::EdgeQuery;
use atlas_graph_types::id::{NodeKind, Position};
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

/// One resolved page row, ALREADY described (id/kind/label) -- shared by
/// `run` (plain) and `run_json`, the same "one resolution, two renderings"
/// discipline `node.rs`'s own `ResolvedCard`/`resolve` uses.
struct ResolvedEntry {
    edge: String,
    id: String,
    kind: String,
    label: String,
}

struct ResolvedPage {
    /// The canonical `--kind` token this page answered for
    /// (`EdgeKind::label()`, not necessarily byte-identical to whatever
    /// case/spelling the caller typed -- though in practice they're the
    /// same string, since `parse_edge_kind` only accepts exact labels).
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
    let kind = parse_edge_kind(kind_raw).ok_or_else(|| {
        CliError::bad_ref(
            format!("'{kind_raw}' is not a known edge kind"),
            "edge kinds are the labels graph-types' own relation manifest defines (e.g. cites, cited-by, attests, mentions)",
            "run 'bibex node <id>' to see which kinds this id actually carries",
        )
    })?;

    let limit = args.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let page = snap.edges(&Position::Node(node_id), &EdgeQuery { kind, cursor: args.cursor, limit });

    // FIX ROUND 1 (review S-3/Q-2): the SAME PeopleGroup filter
    // `atlas_server::graph_handlers::node_edges` applies (PG-1a, "the
    // U5-rebinding seam") -- `graph_wire::decode_node_id` carries no
    // "PeopleGroup" arm, so a PeopleGroup-kind neighbor id handed back
    // here could never be resolved by `bibex node <id>`/`bibex edges <id>`
    // afterward: a real, reachable dead end this command would otherwise
    // create and never disclose. Filtered HERE, mirroring the server's own
    // serving-boundary projection exactly (the underlying graph query
    // above is unfiltered, same as the server's) -- one revert away once
    // U5 lands, same as the server's own comment says.
    let entries: Vec<ResolvedEntry> = page
        .entries
        .iter()
        .filter(|e| !matches!(&e.node, Position::Node(id) if id.kind == NodeKind::PeopleGroup))
        .map(|e| {
            let (id, kind_str, label) = describe_position(&e.node, &snap);
            ResolvedEntry { edge: e.edge.0.clone(), id, kind: kind_str, label }
        })
        .collect();

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
        out.push_str(&format!("{:<24} {:<12} {:<28} {}\n", entry.edge, entry.kind, entry.id, entry.label));
    }
    match page.next {
        Some(n) => out.push_str(&format!("more: continue with --cursor {n}\n")),
        None => out.push_str("(end of list)\n"),
    }
    Ok(out)
}

/// BIBEX-1 (--json mode): `{kind, entries: [{edge, node: {id, kind,
/// label}}], next}` -- field names reused verbatim from
/// `atlas_server::graph_handlers::EdgePageOut`/`EdgeEntryOut`/`NodeRefOut`
/// (the SAME wire shape `/api/node/{id}/edges` already serves, minus
/// `version`). CONTRACT.md's own "--json mode" section has the full field
/// table.
pub fn run_json(graph: &GraphService, args: EdgesArgs) -> Result<serde_json::Value, CliError> {
    let page = resolve(graph, &args)?;
    let entries: Vec<_> = page.entries.iter().map(|e| serde_json::json!({"edge": e.edge, "node": {"id": e.id, "kind": e.kind, "label": e.label}})).collect();
    Ok(serde_json::json!({"kind": page.kind_label, "entries": entries, "next": page.next}))
}
