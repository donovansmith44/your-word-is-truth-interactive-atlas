//! `bibex kinds` -- the edge-kind vocabulary, read straight off the relation manifest that
//! `--kind` itself resolves against, so this listing cannot drift from what is accepted.

use atlas_graph_types::edge::{RelationId, SymRelationId};

/// `token` is the exact, copy-pasteable `--kind` value; `relation` is the manifest's own
/// declared name for it, never hand-authored prose that could drift from the manifest.
pub struct KindRow {
    pub token: String,
    pub relation: String,
    pub direction: &'static str,
}

/// In manifest declaration order, deliberately not alphabetized: alphabetizing would
/// separate a relation's forward and inverse rows, the one grouping this listing exists for.
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
    use atlas_graph_types::edge::EdgeKind;

    #[test]
    fn every_row_token_round_trips_through_from_label() {
        for row in rows() {
            assert!(EdgeKind::from_label(&row.token).is_some(), "'{}' (from {} {}) must be a real --kind token", row.token, row.relation, row.direction);
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
