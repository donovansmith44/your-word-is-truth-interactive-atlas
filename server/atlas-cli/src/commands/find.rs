//! `bibex find <term>` -- case-insensitive substring match on the label of
//! every node this crate can enumerate WITHOUT new parallel query logic:
//! `GraphService`'s own `..._ids` companion fields (Place/Event/Narrative/
//! Era/Polity, the same fields `atlas_server::handlers::{places,eras,
//! polities,narratives}` etc. read for their own listing endpoints; Person,
//! BIBEX-1 addendum ticket 2, the SAME companion shape, see `GraphService::
//! person_ids`'s own doc comment) plus `AtlasData.catechism` (already
//! loaded by `load::load` off `catechism.json` -- no new plumbing at all,
//! CatechismItem's id/name sit right there unused until this batch). See
//! CONTRACT.md's own "bibex find" section for the disclosed, still-excluded
//! kinds and why (PeopleGroup/CommentaryItem/Translation/TextUnit).

use atlas_core::data::AtlasData;
use atlas_graph::GraphService;
use atlas_graph_types::id::{AnyNodeId, NodeKind};
use atlas_server::graph_wire::{describe_node, encode_node_id};

use crate::error::CliError;

struct Hit {
    kind: &'static str,
    id: String,
    label: String,
}

/// BIBEX-1 addendum (ticket 2, ruling 2, "FIND COVERS EVERYTHING NAMED"):
/// the widened kind list, shared by the no-argument `bad_usage` scope
/// message, the zero-match `empty_result` scope message, and this
/// function's own search loop -- ONE list, never three that could drift.
pub(crate) const SEARCHED_KINDS: &str = "Place/Event/Narrative/Era/Polity/Person/CatechismItem";
pub(crate) const EXCLUDED_KINDS: &str =
    "PeopleGroup/CommentaryItem/Translation/TextUnit are not searched -- PeopleGroup has no `bibex node`-resolvable id yet (graph_wire::decode_node_id carries no PeopleGroup arm, pending the U5 rebinding), CommentaryItem has no id/label enumeration surface at its 50k+ scale, Translation has none either (a fixed 6-row set), and TextUnit is covered directly by 'bibex verse'/'bibex chapter' instead -- see CONTRACT.md";

fn hits(graph: &GraphService, data: &AtlasData, term: &str) -> Vec<Hit> {
    let snap = graph.snapshot();
    let needle = term.to_lowercase();

    let mut out: Vec<Hit> = Vec::new();

    let kinds: [(&'static str, &[AnyNodeId]); 6] = [
        ("Place", &graph.place_ids),
        ("Event", &graph.event_ids),
        ("Narrative", &graph.narrative_ids),
        ("Era", &graph.era_ids),
        ("Polity", &graph.polity_ids),
        // BIBEX-1 addendum (ticket 2): the owner's own "PERSONS above all"
        // -- `GraphService::person_ids`, the identical companion-
        // enumeration shape as the five kinds above.
        ("Person", &graph.person_ids),
    ];
    for (kind_name, ids) in kinds {
        for id in ids {
            let (label, _) = describe_node(id, &snap);
            if label.to_lowercase().contains(&needle) {
                out.push(Hit { kind: kind_name, id: encode_node_id(id), label });
            }
        }
    }

    // BIBEX-1 addendum (ticket 2): CatechismItem, off `AtlasData.catechism`
    // -- already loaded by `load::load` (compiled `catechism.json`), no new
    // enumeration surface needed at all.
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

/// BIBEX-1 (--json mode): an array of `{kind, id, label}` objects (`kind`
/// is necessary here, unlike the addendum's own bare `{id, label}` shape
/// elsewhere -- `find`'s whole point is a search spanning MULTIPLE node
/// kinds in one flat list, so the kind that disambiguates each row travels
/// with it, exactly as it already does in the plain-mode `kind id label`
/// column). Zero matches is still the `empty_result` taxonomy class on
/// stderr, same as plain mode (CONTRACT.md: "errors stay fail-loud" applies
/// under --json too) -- never a silently empty JSON array standing in for
/// a real miss.
pub fn run_json(graph: &GraphService, data: &AtlasData, term: &str) -> Result<serde_json::Value, CliError> {
    let hits = hits(graph, data, term);

    if hits.is_empty() {
        return Err(CliError::empty_result(format!("no matches for '{term}'"), format!("searched {SEARCHED_KINDS} labels ({EXCLUDED_KINDS})"), "try a shorter or different substring"));
    }

    Ok(serde_json::Value::Array(hits.iter().map(|h| serde_json::json!({"kind": h.kind, "id": h.id, "label": h.label})).collect()))
}
