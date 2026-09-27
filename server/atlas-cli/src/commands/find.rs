//! `bibex find <term>` -- case-insensitive substring match on the label of every node kind
//! this crate can enumerate without new query logic.

use atlas_core::data::AtlasData;
use atlas_graph::GraphService;
use atlas_graph_types::id::{AnyNodeId, NodeKind};
use atlas_contract::graph_wire::{describe_node, encode_node_id};

use crate::error::CliError;

struct Hit {
    kind: &'static str,
    id: String,
    label: String,
}

/// One list, shared by the usage message, the empty-result message and the search loop, so
/// the three cannot drift apart.
pub(crate) const SEARCHED_KINDS: &str = "Place/Event/Narrative/Era/Polity/Person/CatechismItem";
pub(crate) const EXCLUDED_KINDS: &str =
    "PeopleGroup/CommentaryItem/Translation/TextUnit are not searched -- PeopleGroup has no `bibex node`-resolvable id yet (graph_wire::decode_node_id carries no PeopleGroup arm, pending the U5 rebinding), CommentaryItem has no id/label enumeration surface at its 50k+ scale, Translation has none either (a fixed 6-row set), and TextUnit is covered directly by 'bibex verse'/'bibex chapter' instead -- see CONTRACT.md";

fn hits(graph: &GraphService, data: &AtlasData, term: &str) -> Vec<Hit> {
    let snap = graph.snapshot();
    let needle = term.to_lowercase();

    let mut out: Vec<Hit> = Vec::new();

    let kinds: [(&'static str, NodeKind); 6] = [
        ("Place", NodeKind::Place),
        ("Event", NodeKind::Event),
        ("Narrative", NodeKind::Narrative),
        ("Era", NodeKind::Era),
        ("Polity", NodeKind::Polity),
        ("Person", NodeKind::Person),
    ];
    for (kind_name, kind) in kinds {
        for id in &graph.ids_of_kind(kind) {
            let label = describe_node(id, &snap);
            if label.to_lowercase().contains(&needle) {
                out.push(Hit { kind: kind_name, id: encode_node_id(id), label });
            }
        }
    }

    for part in &data.catechism {
        for item in &part.items {
            if item.name.to_lowercase().contains(&needle) {
                let id = AnyNodeId { kind: NodeKind::CatechismItem, raw: item.id.clone() };
                out.push(Hit { kind: "CatechismItem", id: encode_node_id(&id), label: item.name.clone() });
            }
        }
    }

    out.sort_by(|a, b| (a.kind, &a.id).cmp(&(b.kind, &b.id)));
    out
}

pub fn run(graph: &GraphService, data: &AtlasData, term: &str) -> Result<String, CliError> {
    let hits = hits(graph, data, term);

    if hits.is_empty() {
        return Err(CliError::empty_result(format!("no matches for '{term}'"), format!("searched {SEARCHED_KINDS} labels ({EXCLUDED_KINDS})"), "try a shorter or different substring"));
    }

    let mut out = String::new();
    for hit in &hits {
        out.push_str(&format!("{:<14} {:<28} {}\n", hit.kind, hit.id, hit.label));
    }
    Ok(out)
}

/// `kind` travels with each row because this search spans several node kinds in one flat
/// list. Zero matches is still the `empty_result` class, never a silently empty array.
pub fn run_json(graph: &GraphService, data: &AtlasData, term: &str) -> Result<serde_json::Value, CliError> {
    let hits = hits(graph, data, term);

    if hits.is_empty() {
        return Err(CliError::empty_result(format!("no matches for '{term}'"), format!("searched {SEARCHED_KINDS} labels ({EXCLUDED_KINDS})"), "try a shorter or different substring"));
    }

    Ok(serde_json::Value::Array(hits.iter().map(|h| serde_json::json!({"kind": h.kind, "id": h.id, "label": h.label})).collect()))
}
