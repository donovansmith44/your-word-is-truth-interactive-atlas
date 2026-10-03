//! One node per Polity, carrying EVERY era of its lifetime as payload rather than one node per
//! (polity, era): a polity's id and `color_key` are constant across its eras, so splitting them
//! would either duplicate that identity or need a relation to tie the pieces back together.

use atlas_core::time::TimeRange;
use atlas_graph_types::id::{AnyNodeId, NodeKind, PolityId};
use atlas_graph_types::node::{Node, NodePayload, PolityEraPayload};
use atlas_graph_types::store::GraphQuery;

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
    atlas_graph_types::node::PolityDeltaPayload { event: d.event.clone(), verses: d.verses.iter().map(ToString::to_string).collect(), ref_note: d.ref_note.clone() }
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
    // A polity's display label is its MOST RECENT era's name, the map's own "current name wins"
    // convention; the bare id covers the empty-eras case the ETL already rules out.
    let label = p.eras.last().map(|e| e.name.clone()).unwrap_or_else(|| p.id.clone());
    Node {
        id: PolityId::new(p.id.clone()).erase(),
        payload: NodePayload::Polity { label, color_key: p.color_key, eras },
        provenance: "curated-polities".to_string(),
    }
}

/// One era of one polity whose years overlap a window: what a map draws a border for.
#[derive(Debug, Clone, PartialEq)]
pub struct Reign {
    pub polity: AnyNodeId,
    pub color_key: u8,
    pub era: PolityEraPayload,
}

/// Every reign overlapping `window`, by polity and then oldest era first, so a border change
/// paints older beneath newer.
pub fn reigns_in(q: &impl GraphQuery, window: &TimeRange) -> Vec<Reign> {
    let mut out: Vec<Reign> = Vec::new();
    for id in crate::service::ids_of_kind(q, NodeKind::Polity) {
        let Some(node) = q.node(&id) else { continue };
        let NodePayload::Polity { color_key, eras, .. } = node.payload else { continue };
        for era in eras {
            if window.intersects(&TimeRange { from_year: era.from_year, to_year: era.to_year }) {
                out.push(Reign { polity: id.clone(), color_key, era });
            }
        }
    }
    out.sort_by(|a, b| (&a.polity.raw, a.era.from_year).cmp(&(&b.polity.raw, b.era.from_year)));
    out
}

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
    fn the_reigns_in_a_window_are_every_overlapping_era_by_polity_then_oldest_first() {
        // Arrange
        let atlas = atlas_with_polity(Polity {
            id: "egypt".into(),
            color_key: 3,
            eras: vec![
                PolityEra { name: "Ptolemaic Egypt".into(), from: -332, to: -30, ref_note: String::new(), rings: vec![], transition: None, fall: None },
                PolityEra { name: "Egypt".into(), from: -3100, to: -332, ref_note: String::new(), rings: vec![], transition: None, fall: None },
                PolityEra { name: "Roman Egypt".into(), from: -30, to: 395, ref_note: String::new(), rings: vec![], transition: None, fall: None },
            ],
        });
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        normalize(&mut ctx);

        // Act
        let reigns = reigns_in(&ctx.graph, &TimeRange::new(-400, -300).unwrap());

        // Assert
        let reign = |name: &str, from_year: i32, to_year: i32| Reign {
            polity: polity_node_id("egypt"),
            color_key: 3,
            era: PolityEraPayload { name: name.into(), from_year, to_year, rings: vec![], ref_note: String::new(), transition: None, fall: None },
        };
        assert_eq!(reigns, vec![reign("Egypt", -3100, -332), reign("Ptolemaic Egypt", -332, -30)]);
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
