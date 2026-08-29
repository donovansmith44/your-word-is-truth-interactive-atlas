//! `bibex find <term>` -- case-insensitive substring match on the label of
//! every node this crate's `GraphService` already enumerates by kind
//! (Place/Event/Narrative/Era/Polity, the same `..._ids` companion fields
//! `atlas_server::handlers::{places,eras,polities,narratives}` etc. read
//! for their own listing endpoints). See CONTRACT.md's own "bibex find"
//! section for the disclosed scope limit (no Person/CatechismItem/
//! CommentaryItem/Translation/TextUnit search -- `GraphService` carries no
//! companion enumeration for those kinds, and building one for this
//! command alone would be new, un-server-shared enumeration logic).

use atlas_graph::GraphService;
use atlas_graph_types::id::AnyNodeId;
use atlas_server::graph_wire::describe_node;

use crate::error::CliError;

pub fn run(graph: &GraphService, term: &str) -> Result<String, CliError> {
    let snap = graph.snapshot();
    let needle = term.to_lowercase();

    let kinds: [(&str, &[AnyNodeId]); 5] =
        [("Place", &graph.place_ids), ("Event", &graph.event_ids), ("Narrative", &graph.narrative_ids), ("Era", &graph.era_ids), ("Polity", &graph.polity_ids)];

    let mut hits: Vec<(String, String, String)> = Vec::new(); // (kind, id, label)
    for (kind_name, ids) in kinds {
        for id in ids {
            let (label, _) = describe_node(id, &snap);
            if label.to_lowercase().contains(&needle) {
                hits.push((kind_name.to_string(), id.raw.clone(), label));
            }
        }
    }
    hits.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    if hits.is_empty() {
        return Err(CliError::empty_result(
            format!("no matches for '{term}'"),
            "searched Place/Event/Narrative/Era/Polity labels (Person/CatechismItem/CommentaryItem/Translation/TextUnit are not searched -- see CONTRACT.md)",
            "try a shorter or different substring",
        ));
    }

    let mut out = String::new();
    for (kind_name, id, label) in hits {
        out.push_str(&format!("{:<10} {:<28} {}\n", kind_name, id, label));
    }
    Ok(out)
}
