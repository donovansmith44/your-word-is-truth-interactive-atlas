//! `bibex node <id>` -- card + edge summary. Mirrors
//! `atlas_server::graph_handlers::node_card` exactly (same id grammar,
//! same `GraphQuery` calls), minus the HTTP wire wrapping -- see
//! CONTRACT.md's own "bibex node" section.

use atlas_graph::GraphService;
use atlas_graph_types::id::Position;
use atlas_graph_types::store::GraphQuery;
use atlas_server::graph_wire::{decode_node_id, describe_node};

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

/// Resolved shape shared by `run` (plain) and `run_json` -- ONE resolution,
/// TWO renderings, the same discipline `edges.rs`'s own `resolve` uses
/// (never risk the two output modes drifting on what counts as a valid
/// id/what the edge-summary rows are).
struct ResolvedCard {
    id_raw: String,
    kind: String,
    label: String,
    provenance: String,
    /// BIBEX-1 addendum (ticket 2, ruling 3, "must show each kind's exact
    /// --kind TOKEN"): `kind.label()` (`EdgeKind::label()`) IS already the
    /// exact, copy-pasteable `--kind` value `graph_wire::parse_edge_kind`
    /// accepts back (its own total inverse, `graph_wire.rs`'s own doc
    /// comment) -- this was already true before this addendum; declared
    /// explicitly here and in CONTRACT.md, and proven by
    /// `kinds::tests::every_row_token_round_trips_through_parse_edge_kind`
    /// plus this crate's own `node_edge_summary_kind_token_works_directly_
    /// in_edges` integration test (the real "see it -> use it" loop).
    edge_summary: Vec<(String, usize)>,
}

fn resolve(graph: &GraphService, id_raw: &str) -> Result<ResolvedCard, CliError> {
    let node_id = decode_node_id(id_raw).ok_or_else(|| bad_ref_err(id_raw))?;

    let snap = graph.snapshot();
    let node = snap.node(&node_id).ok_or_else(|| not_found_err(id_raw))?;

    let (label, _kind) = describe_node(&node_id, &snap);
    let summary = snap.edge_summary(&Position::Node(node_id.clone()));

    Ok(ResolvedCard {
        id_raw: id_raw.to_string(),
        kind: format!("{:?}", node_id.kind),
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

/// BIBEX-1 (--json mode): `{id, kind, label, provenance, edge_summary:
/// [{kind, count}]}` -- field names reused verbatim from
/// `atlas_server::graph_handlers::NodeCardOut`/`EdgeSummaryEntryOut` (the
/// SAME wire shape `/api/node/{id}` already serves, minus `version`/
/// `description`: this crate never computes either -- CONTRACT.md's own
/// "--json mode" section has the full field table).
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
