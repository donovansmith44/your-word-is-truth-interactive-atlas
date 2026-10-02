use crate::data::AtlasData;
use crate::scene_source::SceneSource;
use crate::wire::VerseGroup;

#[derive(Debug, Clone, PartialEq, serde::Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "An event next to the one asked about: enough to show it, and the id to travel to it with.")]
pub struct NarrativeAdjacentEvent {
    pub id: String,
    pub label: String,
    pub places: Vec<String>,
    pub verse_groups: Vec<VerseGroup>,
}

pub fn adjacent_event(src: &dyn SceneSource, event_id: &str) -> Option<NarrativeAdjacentEvent> {
    let e = src.event_by_id(event_id)?;
    let se = crate::scene::to_scene_event(e);
    Some(NarrativeAdjacentEvent { id: se.id, label: se.label, places: e.places.clone(), verse_groups: se.verse_groups })
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "Where an event sits in the atlas's whole chronology, whatever narratives it belongs to.")]
pub struct TimelinePosition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior: Option<NarrativeAdjacentEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub following: Option<NarrativeAdjacentEvent>,
}

pub fn global_timeline_position(d: &AtlasData, event_id: &str) -> Option<TimelinePosition> {
    let idx = d.timeline_position(event_id)?;
    let prior = idx.checked_sub(1).and_then(|i| d.timeline_event_at(i)).and_then(|e| adjacent_event(d as &dyn SceneSource, &e.id));
    let following = d.timeline_event_at(idx + 1).and_then(|e| adjacent_event(d as &dyn SceneSource, &e.id));
    Some(TimelinePosition { prior, following })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_timeline_mid_event_has_both_neighbors() {
        let d = crate::data::demo_fixture();
        let pos = global_timeline_position(&d, "e2").expect("e2 is a dated event");
        assert_eq!(pos.prior.as_ref().unwrap().id, "e1");
        assert_eq!(pos.following.as_ref().unwrap().id, "e3");
    }

    #[test]
    fn global_timeline_true_first_of_the_whole_atlas_has_no_prior() {
        let d = crate::data::demo_fixture();
        let pos = global_timeline_position(&d, "e5").expect("e5 is a dated event");
        assert!(pos.prior.is_none(), "e5 is the fixture's true first dated event -- no prior, not a disabled stub");
        assert_eq!(pos.following.as_ref().unwrap().id, "e1", "chronologically next regardless of narrative membership");
    }

    #[test]
    fn global_timeline_true_last_of_the_whole_atlas_has_no_following() {
        let d = crate::data::demo_fixture();
        let pos = global_timeline_position(&d, "e4").expect("e4 is a dated event");
        assert_eq!(pos.prior.as_ref().unwrap().id, "e3");
        assert!(pos.following.is_none(), "e4 is the fixture's true last dated event -- no following, not a disabled stub");
    }

    #[test]
    fn global_timeline_position_is_independent_of_narrative_membership() {
        let d = crate::data::demo_fixture();
        let via_e1 = global_timeline_position(&d, "e2").unwrap();
        assert_eq!(via_e1.prior.as_ref().unwrap().id, "e1");
        assert!(global_timeline_position(&d, "e5").is_some());
    }

    #[test]
    fn global_timeline_unknown_id_returns_none() {
        let d = crate::data::demo_fixture();
        assert!(global_timeline_position(&d, "no-such-event").is_none());
    }

    #[test]
    fn global_timeline_general_kind_event_returns_none() {
        use crate::data::{Canon, Event, Place};
        use std::collections::HashMap;

        let places = vec![Place { id: "p1".into(), name: "P1".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let events = vec![
            Event {
                id: "dated-1".into(),
                label: "A dated event".into(),
                when: crate::time::TimeRange::new(1, 1).unwrap(),
                places: vec!["p1".into()],
                verses: vec![],
                kind: crate::data::EventKind::Event,
                ..Default::default()
            },
            Event {
                id: "general-1".into(),
                label: "A general-kind container".into(),
                when: crate::time::TimeRange::undated(),
                places: vec![],
                verses: vec![],
                kind: crate::data::EventKind::General,
                ..Default::default()
            },
        ];
        let d = AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();

        assert!(global_timeline_position(&d, "general-1").is_none(), "general-kind: no timeline position at all, not a stub");
        assert!(global_timeline_position(&d, "dated-1").is_some());
    }

    #[test]
    fn global_timeline_same_date_run_resolves_by_stable_original_order() {
        use crate::data::{Canon, Event, Place};
        use std::collections::HashMap;

        let places = vec![Place { id: "p1".into(), name: "P1".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let ev = |id: &str| Event {
            id: id.into(),
            label: id.into(),
            when: crate::time::TimeRange::new(-4004, -4004).unwrap(),
            places: vec!["p1".into()],
            verses: vec![],
            ..Default::default()
        };
        let events = vec![ev("alpha"), ev("beta"), ev("gamma")];
        let d = AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();

        let alpha = global_timeline_position(&d, "alpha").unwrap();
        assert!(alpha.prior.is_none());
        assert_eq!(alpha.following.as_ref().unwrap().id, "beta");
        let beta = global_timeline_position(&d, "beta").unwrap();
        assert_eq!(beta.prior.as_ref().unwrap().id, "alpha");
        assert_eq!(beta.following.as_ref().unwrap().id, "gamma");
        let gamma = global_timeline_position(&d, "gamma").unwrap();
        assert_eq!(gamma.prior.as_ref().unwrap().id, "beta");
        assert!(gamma.following.is_none());
    }

    #[test]
    fn adjacent_event_verse_groups_equal_the_map_arrows_own_scene_event() {
        let d = crate::data::demo_fixture();

        let e3_via_narrative = &adjacent_event(&d as &dyn SceneSource, "e3").expect("e3 is a real event").verse_groups;

        let scene = crate::scene::compose_time_scene(&d, crate::time::TimeRange::new(-1405, -1405).unwrap());
        let jericho = scene.places.iter().find(|p| p.id == "jericho").expect("jericho is lit at -1405 (e3)");
        let e3_via_scene = &jericho.events.iter().find(|e| e.id == "e3").expect("e3 present in the -1405 scene").verse_groups;

        assert_eq!(e3_via_narrative, e3_via_scene, "the popover's PRIOR/FOLLOWING verse groups must equal the map arrow endpoint's own -- one graph, seen twice");
    }

}
