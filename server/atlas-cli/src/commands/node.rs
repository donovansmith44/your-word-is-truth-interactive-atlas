//! `bibex node <id>` -- one node's card and its edge summary.

use atlas_graph::GraphService;
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;
use atlas_contract::graph_wire::{decode_node_id, describe_node};

use crate::error::CliError;

fn bad_ref_err(id_raw: &str) -> CliError {
    CliError::bad_ref(
        format!("'{id_raw}' is not a valid node id"),
        "expected KIND:raw (e.g. text-unit:GEN.1.1, Event:ab_ur, Place:jericho)",
        "run 'bibex find <term>' to locate an id, or 'bibex tutorial' for worked examples",
    )
}

fn not_found_err(id_raw: &str) -> CliError {
    CliError::not_found(
        format!("no node named '{id_raw}'"),
        "the id parsed fine but this graph has no node with that raw id",
        "try 'bibex find <term>' to locate the id you meant",
    )
}

struct ResolvedCard {
    id_raw: String,
    kind: String,
    label: String,
    provenance: String,
    /// Each kind's label IS the exact `--kind` token that resolves back to it, so a summary
    /// row can be pasted straight into `bibex edges`.
    edge_summary: Vec<(String, usize)>,
}

fn resolve(graph: &GraphService, id_raw: &str) -> Result<ResolvedCard, CliError> {
    let node_id = decode_node_id(id_raw).ok_or_else(|| bad_ref_err(id_raw))?;

    let snap = graph.snapshot();
    let node = snap.node(&node_id).ok_or_else(|| not_found_err(id_raw))?;

    let label = describe_node(&node_id, &snap);
    let summary = snap.edge_summary(&Position::Node(node_id.clone()));

    Ok(ResolvedCard {
        id_raw: id_raw.to_string(),
        kind: node_id.kind.name().to_string(),
        label,
        provenance: node.provenance.clone(),
        edge_summary: summary.into_iter().map(|(kind, count)| (kind.label().to_string(), count)).collect(),
    })
}

pub fn run(graph: &GraphService, id_raw: &str) -> Result<String, CliError> {
    let card = resolve(graph, id_raw)?;

    let mut out = String::new();
    out.push_str(&format!("id:         {}\n", card.id_raw));
    out.push_str(&format!("kind:       {}\n", card.kind));
    out.push_str(&format!("label:      {}\n", card.label));
    out.push_str(&format!("provenance: {}\n", card.provenance));
    out.push_str("edges:\n");
    if card.edge_summary.is_empty() {
        out.push_str("  (no edges)\n");
    } else {
        for (kind, count) in &card.edge_summary {
            out.push_str(&format!("  {kind:<16} {count}\n"));
        }
    }
    Ok(out)
}

pub fn run_json(graph: &GraphService, id_raw: &str) -> Result<serde_json::Value, CliError> {
    let card = resolve(graph, id_raw)?;
    let edge_summary: Vec<_> = card.edge_summary.iter().map(|(kind, count)| serde_json::json!({"kind": kind, "count": count})).collect();
    Ok(serde_json::json!({
        "id": card.id_raw,
        "kind": card.kind,
        "label": card.label,
        "provenance": card.provenance,
        "edge_summary": edge_summary,
    }))
}
