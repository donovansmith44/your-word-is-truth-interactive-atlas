//! `bibex node <id>` -- card + edge summary. Mirrors
//! `atlas_server::graph_handlers::node_card` exactly (same id grammar,
//! same `GraphQuery` calls), minus the HTTP wire wrapping -- see
//! CONTRACT.md's own "bibex node" section.

use atlas_graph::GraphService;
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;
use atlas_server::graph_wire::{decode_node_id, describe_node};

use crate::error::CliError;

pub fn run(graph: &GraphService, id_raw: &str) -> Result<String, CliError> {
    let node_id = decode_node_id(id_raw).ok_or_else(|| {
        CliError::bad_ref(
            format!("'{id_raw}' is not a valid node id"),
            "expected KIND:raw (e.g. text-unit:GEN.1.1, Event:ab_ur, Place:jericho)",
            "run 'bibex find <term>' to locate an id, or 'bibex tutorial' for worked examples",
        )
    })?;

    let snap = graph.snapshot();
    let node = snap.node(&node_id).ok_or_else(|| {
        CliError::not_found(
            format!("no node named '{id_raw}'"),
            "the id parsed fine but this graph has no node with that raw id",
            "try 'bibex find <term>' to locate the id you meant",
        )
    })?;

    let (label, _kind) = describe_node(&node_id, &snap);
    let summary = snap.edge_summary(&Position::Node(node_id.clone()));

    let mut out = String::new();
    out.push_str(&format!("id:         {}\n", id_raw));
    out.push_str(&format!("kind:       {:?}\n", node_id.kind));
    out.push_str(&format!("label:      {label}\n"));
    out.push_str(&format!("provenance: {}\n", node.provenance));
    out.push_str("edges:\n");
    if summary.is_empty() {
        out.push_str("  (no edges)\n");
    } else {
        for (kind, count) in &summary {
            out.push_str(&format!("  {:<16} {}\n", kind.label(), count));
        }
    }
    Ok(out)
}
