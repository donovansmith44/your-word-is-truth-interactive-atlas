//! One `Map` per era: the world at the era's window, drawn. What a map shows is exactly what the
//! scene composer answers for that window, read through the same port a request reads, so the map
//! and the scene can never disagree; the maps follow one another in era order.

use anyhow::{Context, Result};

use atlas_core::scene::compose_time_scene;
use atlas_core::time::TimeRange;
use atlas_core::wire::Scene;
use atlas_graph_types::edge::{MapSuccession, Shown};
use atlas_graph_types::id::{AnyNodeId, MapId, PlaceId};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};

use crate::era_adapter::{chronological, PROVENANCE};
use crate::event_world::{event_node_id, narrative_node_id};
use crate::pipeline::BuildCtx;
use crate::polity_adapter::reigns_in;
use crate::scene_source::GraphSceneSource;
use crate::service::narrative_legs_of;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapAdapterStats {
    pub maps: usize,
    pub shown: usize,
    pub steps: usize,
}

pub fn map_node_id(era_id: &str) -> MapId {
    MapId::new(format!("era-{era_id}"))
}

/// Runs once the indexes are built: the scene is read through the port exactly as a request reads
/// it, which needs the resolved dates and the `located-at` index already in place.
pub fn derive(ctx: &mut BuildCtx) -> Result<MapAdapterStats> {
    let mut stats = MapAdapterStats::default();
    let source = GraphSceneSource::over(&ctx.graph, &ctx.chrono, &narrative_legs_of(&ctx.graph), ctx.atlas);
    let mut eras = ctx.eras.to_vec();
    chronological(&mut eras);
    let mut maps: Vec<MapId> = Vec::new();
    for era in &eras {
        let window = TimeRange::new(era.from_year, era.to_year).with_context(|| format!("era {} has an ill-formed window", era.id))?;
        let map = map_node_id(&era.id);
        ctx.graph.nodes.insert(
            map.erase(),
            Node {
                id: map.erase(),
                payload: NodePayload::Map { label: era.name.clone(), from_year: era.from_year, to_year: era.to_year },
                provenance: ProvenanceId::from(PROVENANCE),
            },
        );
        let polities: Vec<AnyNodeId> = reigns_in(&ctx.graph, &window).into_iter().map(|reign| reign.polity).collect();
        for node in shown_in(&compose_time_scene(&source, window), polities) {
            ctx.graph.shown.push(Shown { map: map.clone(), node, provenance: ProvenanceId::from(PROVENANCE) });
            stats.shown += 1;
        }
        maps.push(map);
        stats.maps += 1;
    }
    for step in maps.windows(2) {
        ctx.graph.map_succession.push(MapSuccession { prior: step[0].clone(), next: step[1].clone(), provenance: ProvenanceId::from(PROVENANCE) });
        stats.steps += 1;
    }
    Ok(stats)
}

/// Every drawable thing in the scene, once each, in id order: the lit and the quiet places, the
/// events at the lit places, the narratives in the legend, and the polities reigning in the window.
fn shown_in(scene: &Scene, polities: Vec<AnyNodeId>) -> Vec<AnyNodeId> {
    let places = scene
        .places
        .iter()
        .map(|p| p.id.as_str())
        .chain(scene.quiet_places.iter().map(|p| p.id.as_str()))
        .map(|id| PlaceId::new(id).erase());
    let events = scene.places.iter().flat_map(|p| p.events.iter().map(|e| event_node_id(&e.id)));
    let narratives = scene.narratives.iter().map(|n| narrative_node_id(&n.id));
    let mut all: Vec<AnyNodeId> = places.chain(events).chain(narratives).chain(polities).collect();
    all.sort();
    all.dedup();
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, Era, Polity, PolityEra};
    use atlas_graph_types::id::{EventId, NarrativeId, PolityId};
    use atlas_graph_types::store::GraphQuery;
    use std::collections::HashMap;

    const NO_XREFS: &str = "From Verse\tTo Verse\tVotes\t#comment\n";

    fn conquest_atlas_under_egypt() -> AtlasData {
        let mut atlas = atlas_core::data::demo_fixture();
        atlas.polities = vec![Polity {
            id: "egypt".into(),
            color_key: 3,
            eras: vec![PolityEra { name: "Egypt".into(), from: -2100, to: -1200, ref_note: "fixture".into(), rings: vec![], transition: None, fall: None }],
        }];
        atlas.finish()
    }

    fn shown(map: &str, node: AnyNodeId) -> Shown {
        Shown { map: map_node_id(map), node, provenance: PROVENANCE.into() }
    }

    #[test]
    fn each_eras_map_shows_the_scene_at_its_window_and_the_maps_follow_one_another_oldest_first() {
        // Arrange
        let atlas = conquest_atlas_under_egypt();
        let eras = vec![
            Era { id: "conquest".into(), name: "Conquest".into(), from_year: -1406, to_year: -1051 },
            Era { id: "patriarchs".into(), name: "Patriarchs".into(), from_year: -2100, to_year: -1877 },
        ];
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::with_eras(&canon, &verses, None, NO_XREFS, &atlas, &eras);
        let before_map: Vec<Box<dyn crate::pipeline::Pass>> = crate::pipeline::pipeline().into_iter().take_while(|p| p.name() != "map").collect();
        crate::pipeline::run_pipeline(&mut ctx, &before_map).unwrap();

        // Act
        let stats = derive(&mut ctx).unwrap();

        // Assert
        assert_eq!(stats, MapAdapterStats { maps: 2, shown: 16, steps: 1 });
        let place = |id: &str| PlaceId::new(id).erase();
        let event = |id: &str| EventId::new(id).erase();
        assert_eq!(
            ctx.graph.shown,
            vec![
                shown("patriarchs", event("e5")),
                shown("patriarchs", place("ai")),
                shown("patriarchs", place("gilgal")),
                shown("patriarchs", place("hebron")),
                shown("patriarchs", place("jericho")),
                shown("patriarchs", PolityId::new("egypt").erase()),
                shown("conquest", event("e1")),
                shown("conquest", event("e2")),
                shown("conquest", event("e3")),
                shown("conquest", event("e4")),
                shown("conquest", NarrativeId::new("conquest").erase()),
                shown("conquest", place("ai")),
                shown("conquest", place("gilgal")),
                shown("conquest", place("hebron")),
                shown("conquest", place("jericho")),
                shown("conquest", PolityId::new("egypt").erase()),
            ]
        );
        assert_eq!(
            ctx.graph.map_succession,
            vec![MapSuccession { prior: map_node_id("patriarchs"), next: map_node_id("conquest"), provenance: PROVENANCE.into() }]
        );
        let patriarchs = ctx.graph.node(&map_node_id("patriarchs").erase()).unwrap();
        assert_eq!(
            format!("{:?}", patriarchs.payload),
            format!("{:?}", NodePayload::Map { label: "Patriarchs".into(), from_year: -2100, to_year: -1877 })
        );
        assert_eq!(patriarchs.provenance, PROVENANCE);
    }
}
