//! `Era` nodes for the map's era selector. A node's raw id is the curated era id itself.

use atlas_graph_types::id::EraId;
use atlas_graph_types::node::{Node, NodePayload};

use crate::pipeline::BuildCtx;

pub const PROVENANCE: &str = "curated-eras";

#[derive(Debug, Clone, Copy, Default)]
pub struct EraAdapterStats {
    pub eras: usize,
}

pub fn era_node_id(id: &str) -> atlas_graph_types::id::AnyNodeId {
    EraId::new(id.to_string()).erase()
}

fn era_node(e: &atlas_core::data::Era) -> Node {
    Node {
        id: EraId::new(e.id.clone()).erase(),
        payload: NodePayload::Era { label: e.name.clone(), from_year: e.from_year, to_year: e.to_year },
        provenance: PROVENANCE.to_string(),
    }
}

/// Oldest first, ties by id: the order the map's slider walks and the maps follow.
pub fn chronological(eras: &mut [atlas_core::data::Era]) {
    eras.sort_by(|a, b| (a.from_year, &a.id).cmp(&(b.from_year, &b.id)));
}

/// One node per curated era, and no relation rows: a time range is the whole of what an Era node
/// is for, and nothing else references it.
pub fn normalize(ctx: &mut BuildCtx) -> EraAdapterStats {
    let mut stats = EraAdapterStats::default();
    for e in ctx.eras {
        let node = era_node(e);
        ctx.graph.nodes.insert(node.id.clone(), node);
        stats.eras += 1;
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{Canon, Era};
    use atlas_graph_types::store::GraphQuery;
    use std::collections::HashMap;

    #[test]
    fn chronological_puts_the_oldest_era_first_and_breaks_a_tie_by_id() {
        // Arrange
        let era = |id: &str, from_year: i32| Era { id: id.into(), name: id.into(), from_year, to_year: from_year + 1 };
        let mut eras = vec![era("later", -1000), era("b-tied", -2000), era("a-tied", -2000)];
        // Act
        chronological(&mut eras);
        // Assert
        assert_eq!(eras.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), vec!["a-tied", "b-tied", "later"]);
    }

    #[test]
    fn one_node_per_curated_era_carrying_its_own_range() {
        let atlas = crate::event_world::empty_atlas();
        let eras = vec![
            Era { id: "patriarchs".into(), name: "Patriarchs".into(), from_year: -2166, to_year: -1877 },
            Era { id: "exodus".into(), name: "Exodus & Wilderness".into(), from_year: -1446, to_year: -1406 },
        ];
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::with_eras(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas, &eras);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.eras, 2);

        let node = ctx.graph.node(&era_node_id("patriarchs")).expect("patriarchs era node must exist");
        match node.payload {
            NodePayload::Era { label, from_year, to_year } => {
                assert_eq!(label, "Patriarchs");
                assert_eq!(from_year, -2166);
                assert_eq!(to_year, -1877);
            }
            other => panic!("expected Era payload, got {other:?}"),
        }
    }
}
