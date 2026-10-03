
use atlas_core::data::AtlasData;
use atlas_contract::provenance::titled;
use atlas_contract::wire::Provenance;
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

struct ResolvedNode {
    id_raw: String,
    kind: String,
    label: String,
    provenance: Provenance,
    /// Each kind's label IS the exact `--kind` token that resolves back to it, so a summary
    /// row can be pasted straight into `bibex edges`.
    edge_summary: Vec<(String, usize)>,
}

fn resolve(data: &AtlasData, graph: &GraphService, id_raw: &str) -> Result<ResolvedNode, CliError> {
    let node_id = decode_node_id(id_raw).ok_or_else(|| bad_ref_err(id_raw))?;

    let snap = graph.snapshot();
    let node = snap.node(&node_id).ok_or_else(|| not_found_err(id_raw))?;

    let label = describe_node(&node_id, &snap).map_err(CliError::unlabelled)?;
    let summary = snap.edge_summary(&Position::Node(node_id.clone()));

    Ok(ResolvedNode {
        id_raw: id_raw.to_string(),
        kind: node_id.kind.name().to_string(),
        label,
        provenance: titled(&node.provenance, data).map_err(|refused| {
            CliError::integrity_failed(refused.message, "the compiled artifact holds a provenance it compiled no title for", "recompile the artifact with atlas-graph-compile")
        })?,
        edge_summary: summary.into_iter().map(|(kind, count)| (kind.label().to_string(), count)).collect(),
    })
}

pub fn run(data: &AtlasData, graph: &GraphService, id_raw: &str) -> Result<String, CliError> {
    let record = resolve(data, graph, id_raw)?;

    let mut out = String::new();
    out.push_str(&format!("id:         {}\n", record.id_raw));
    out.push_str(&format!("kind:       {}\n", record.kind));
    out.push_str(&format!("label:      {}\n", record.label));
    out.push_str(&format!("provenance: {}\n", record.provenance.title));
    out.push_str("edges:\n");
    if record.edge_summary.is_empty() {
        out.push_str("  (no edges)\n");
    } else {
        for (kind, count) in &record.edge_summary {
            out.push_str(&format!("  {kind:<16} {count}\n"));
        }
    }
    Ok(out)
}

pub fn run_json(data: &AtlasData, graph: &GraphService, id_raw: &str) -> Result<serde_json::Value, CliError> {
    let record = resolve(data, graph, id_raw)?;
    let edge_summary: Vec<_> = record.edge_summary.iter().map(|(kind, count)| serde_json::json!({"kind": kind, "count": count})).collect();
    Ok(serde_json::json!({
        "id": record.id_raw,
        "kind": record.kind,
        "label": record.label,
        "provenance": record.provenance,
        "edge_summary": edge_summary,
    }))
}
