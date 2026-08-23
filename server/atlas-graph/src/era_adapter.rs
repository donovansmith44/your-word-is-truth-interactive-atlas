//! Batch M-C, controller decision 2: the era adapter -- `Era` nodes for
//! the map's own era selector/dropdown (design doc §3: "Era / Polity --
//! time-range and border-bearing nodes for the map"). Source: `ctx.eras`
//! (pre-parsed `data/curated/eras.toml` rows, via the SAME
//! `atlas_etl::curated::parse_eras` the pre-M-C `eras.json` compilation
//! path used -- see `pipeline::BuildCtx::eras`'s own doc comment for why
//! this is NOT read off `AtlasData`, unlike this batch's other adapters).
//!
//! Node id: the curated era id itself (`atlas_core::data::Era.id`, e.g.
//! `"patriarchs"`) -- stable, unique, human-legible, same "no numeric
//! re-encoding needed" reasoning M-B's own Event/Narrative/Anchor/Place id
//! grammar already established.

use atlas_graph_types::id::EraId;
use atlas_graph_types::node::{Node, NodePayload};

use crate::pipeline::BuildCtx;

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
        provenance: "curated-eras".to_string(),
    }
}

/// Pipeline-facing NORMALIZE entry point (`pipeline::NormalizePass`): one
/// node per curated era. No relation rows -- a time range is the whole of
/// what an Era node is for; nothing else references it (design's own
/// edge-kind table names no era-targeting relation beyond `DatedBy`'s
/// `EraOnly` placement form, which this batch's real data never exercises
/// -- see `event_world::choose_placement`'s own doc comment).
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
