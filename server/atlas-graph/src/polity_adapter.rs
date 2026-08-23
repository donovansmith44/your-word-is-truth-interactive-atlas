//! Batch M-C, controller decision 2: the polity adapter -- "polities
//! (border data as node payloads -- the map consumes payloads, not new
//! relation kinds)". Source: `ctx.atlas.polities` (`AtlasData.polities`,
//! itself loaded from `polities.json` -- kept standing this batch, per
//! the deletion inventory: `/api/polities` migrates to a graph VIEW, but
//! the underlying compiled file/field stays the adapter's own source,
//! same status as `event_world`'s own `atlas.events`/`.narratives`/
//! `.places`/`.chronology_anchors`).
//!
//! One node per Polity, carrying EVERY era of its own lifetime as payload
//! (`NodePayload::Polity::eras: Vec<PolityEraPayload>`) -- not one node
//! per (polity, era) pair. A polity's `id`/`color_key` are constant across
//! its own eras (one hue for its whole lifetime, `Polity::color_key`'s own
//! doc comment); modeling each era as a SEPARATE node would either
//! duplicate that identity across nodes or invent a new relation to tie
//! them back together -- exactly what "border data as node payloads, not
//! new relation kinds" rules out.

use atlas_graph_types::id::PolityId;
use atlas_graph_types::node::{Node, NodePayload, PolityEraPayload};

use crate::pipeline::BuildCtx;

#[derive(Debug, Clone, Copy, Default)]
pub struct PolityAdapterStats {
    pub polities: usize,
    pub eras: usize,
}

pub fn polity_node_id(id: &str) -> atlas_graph_types::id::AnyNodeId {
    PolityId::new(id.to_string()).erase()
}

fn delta_payload(d: &atlas_core::data::PolityDelta) -> atlas_graph_types::node::PolityDeltaPayload {
    atlas_graph_types::node::PolityDeltaPayload { event: d.event.clone(), verses: d.verses.clone(), ref_note: d.ref_note.clone() }
}

fn polity_node(p: &atlas_core::data::Polity) -> Node {
    let eras: Vec<PolityEraPayload> = p
        .eras
        .iter()
        .map(|era| PolityEraPayload {
            name: era.name.clone(),
            from_year: era.from,
            to_year: era.to,
            rings: era.rings.clone(),
            ref_note: era.ref_note.clone(),
            transition: era.transition.as_ref().map(delta_payload),
            fall: era.fall.as_ref().map(delta_payload),
        })
        .collect();
    // A polity's own display label is its MOST RECENT era's name (mirrors
    // the map's own "current name wins" convention -- e.g. "Ptolemaic
    // Egypt" outranks "Egypt" once it's the latest curated era); falls
    // back to the bare id for the structurally-impossible empty-eras case
    // (every real curated polity file has >= 1 era; validated ETL-side).
    let label = p.eras.last().map(|e| e.name.clone()).unwrap_or_else(|| p.id.clone());
    Node {
        id: PolityId::new(p.id.clone()).erase(),
        payload: NodePayload::Polity { label, color_key: p.color_key, eras },
        provenance: "curated-polities".to_string(),
    }
}

/// Pipeline-facing NORMALIZE entry point (`pipeline::NormalizePass`).
pub fn normalize(ctx: &mut BuildCtx) -> PolityAdapterStats {
    let mut stats = PolityAdapterStats::default();
    for p in &ctx.atlas.polities {
        stats.eras += p.eras.len();
        let node = polity_node(p);
        ctx.graph.nodes.insert(node.id.clone(), node);
        stats.polities += 1;
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, Polity, PolityDelta, PolityEra};
    use atlas_graph_types::store::GraphQuery;
    use std::collections::HashMap;

    fn atlas_with_polity(p: Polity) -> AtlasData {
        let mut d = AtlasData::new(Canon { books: vec![] }, vec![], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        d.polities = vec![p];
        d
    }

    #[test]
    fn polity_node_carries_every_era_with_border_and_delta_payload() {
        let polity = Polity {
            id: "egypt".into(),
            color_key: 3,
            eras: vec![
                PolityEra {
                    name: "Egypt".into(),
                    from: -3100,
                    to: -332,
                    ref_note: "1911 Britannica".into(),
                    rings: vec![vec![(30.0, 31.0), (31.0, 31.0), (31.0, 32.0), (30.0, 31.0)]],
                    transition: None,
                    fall: Some(PolityDelta {
                        event: "Alexander conquers Egypt".into(),
                        verses: vec![],
                        ref_note: "tradition only".into(),
                        for_era_from: -3100,
                    }),
                },
                PolityEra {
                    name: "Ptolemaic Egypt".into(),
                    from: -332,
                    to: -30,
                    ref_note: "1911 Britannica".into(),
                    rings: vec![vec![(30.0, 31.0), (31.0, 31.0), (31.0, 32.0), (30.0, 31.0)]],
                    transition: Some(PolityDelta {
                        event: "Ptolemy I founds the dynasty".into(),
                        verses: vec![],
                        ref_note: "tradition only".into(),
                        for_era_from: -332,
                    }),
                    fall: None,
                },
            ],
        };
        let atlas = atlas_with_polity(polity);
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.polities, 1);
        assert_eq!(stats.eras, 2);

        let node = ctx.graph.node(&polity_node_id("egypt")).expect("polity node must exist");
        match node.payload {
            NodePayload::Polity { label, color_key, eras } => {
                assert_eq!(label, "Ptolemaic Egypt", "label follows the most recent era");
                assert_eq!(color_key, 3);
                assert_eq!(eras.len(), 2);
                let fall = eras[0].fall.as_ref().expect("egypt's own first era carries a fall delta");
                assert_eq!(fall.event, "Alexander conquers Egypt");
                assert_eq!(fall.ref_note, "tradition only");
                let transition = eras[1].transition.as_ref().expect("egypt's own second era carries a transition delta");
                assert_eq!(transition.event, "Ptolemy I founds the dynasty");
                assert!(eras[1].transition.is_some() && eras[1].fall.is_none());
            }
            other => panic!("expected Polity payload, got {other:?}"),
        }
    }
}
