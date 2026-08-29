//! `bibex kinds` -- BIBEX-1 addendum (ticket 2, EDGE-KIND DISCOVERABILITY,
//! owner order mid-batch 2026-08-29: "add `bibex kinds`... so the
//! vocabulary itself is discoverable from nothing"). Lists the full
//! edge-kind vocabulary `bibex edges --kind`/`bibex node <id>`'s own edge
//! summary rows accept, straight off graph-types' own `relations!` manifest
//! (`RelationId::ALL`/`SymRelationId::ALL`) -- the SAME total enumeration
//! `graph_wire::parse_edge_kind` itself scans (`graph_wire.rs`'s own doc
//! comment: "a new relation never needs a second hand-written table here"),
//! so this listing can never drift out of sync with what a real `--kind`
//! value is actually accepted. See CONTRACT.md's own "bibex kinds"
//! section.

use atlas_graph_types::edge::{RelationId, SymRelationId};

/// One row of the vocabulary: `token` is the exact, copy-pasteable
/// `--kind` value; `relation` is the manifest's OWN Rust identifier for
/// this relation (`RelationId`/`SymRelationId`'s own `{:?}` name, e.g.
/// "Cites", "Attests") -- the addendum's own "one-line descriptions from
/// the relations! manifest names" wording: the description IS the
/// manifest's own declared name, never new, hand-authored prose that
/// could drift from what the manifest actually says.
pub struct KindRow {
    pub token: String,
    pub relation: String,
    pub direction: &'static str,
}

/// Every row `bibex edges --kind`/`bibex node`'s edge-summary tokens can
/// ever be, in manifest declaration order (`RelationId::ALL` order, each
/// directed relation's forward row then its inverse row, then every
/// symmetric relation) -- a stable, reproducible order, not alphabetized
/// (alphabetizing would separate a relation's own forward/inverse pair,
/// the one grouping this listing exists to make legible).
pub fn rows() -> Vec<KindRow> {
    let mut out = Vec::new();
    for r in RelationId::ALL {
        out.push(KindRow { token: r.forward_label().to_string(), relation: format!("{r:?}"), direction: "forward" });
        out.push(KindRow { token: r.inverse_label().to_string(), relation: format!("{r:?}"), direction: "inverse" });
    }
    for s in SymRelationId::ALL {
        out.push(KindRow { token: s.label().to_string(), relation: format!("{s:?}"), direction: "symmetric" });
    }
    out
}

pub fn run() -> String {
    let mut out = String::new();
    out.push_str("--kind TOKEN         RELATION           DIRECTION\n");
    for row in rows() {
        out.push_str(&format!("{:<20} {:<18} {}\n", row.token, row.relation, row.direction));
    }
    out
}

pub fn run_json() -> serde_json::Value {
    let rows: Vec<serde_json::Value> = rows().into_iter().map(|r| serde_json::json!({"token": r.token, "relation": r.relation, "direction": r.direction})).collect();
    serde_json::Value::Array(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_server::graph_wire::parse_edge_kind;

    #[test]
    fn every_row_token_round_trips_through_parse_edge_kind() {
        for row in rows() {
            assert!(parse_edge_kind(&row.token).is_some(), "'{}' (from {} {}) must be a real --kind token", row.token, row.relation, row.direction);
        }
    }

    #[test]
    fn rows_are_nonempty_and_cover_both_directed_and_symmetric() {
        let rows = rows();
        assert!(rows.iter().any(|r| r.direction == "forward"));
        assert!(rows.iter().any(|r| r.direction == "inverse"));
        assert!(rows.iter().any(|r| r.direction == "symmetric"));
    }
}
