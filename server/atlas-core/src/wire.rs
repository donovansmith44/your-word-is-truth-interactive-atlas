use serde::{Deserialize, Serialize};

use crate::identity::NodeId;
use crate::label::{TimeRange, Year};
use crate::refs::{ScriptureRef, VerseId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "The map at one moment of enquiry: which places are lit, which are only present, and which arrows run between them.")]
pub struct Scene {
    pub mode: SceneMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<TimeRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<ScriptureRef>,
    pub places: Vec<ScenePlace>,
    pub quiet_places: Vec<QuietPlace>,
    pub arrows: Vec<SceneArrow>,
    pub narratives: Vec<SceneNarrative>,
}

atlas_graph_types::vocabulary! {
    SceneMode {
        Time => "time",
        Scripture => "scripture",
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A reference to a node: enough to show it, and the id to fetch it with.")]
pub struct NodeRef {
    pub id: NodeId,
    pub kind: atlas_graph_types::id::NodeKind,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One lit place on the map, with the events that light it.")]
pub struct ScenePlace {
    pub id: String,
    pub node: NodeRef,
    pub name: String,
    pub display_name: String,
    pub lat: f64,
    pub lon: f64,
    #[schema(maximum = 255)]
    pub brightness: u8,
    pub events: Vec<SceneEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existence_from: Option<Year>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existence_to: Option<Year>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub merged_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A place this atlas knows an event at, drawn on the map but not lit for the span asked about.")]
pub struct QuietPlace {
    pub id: String,
    pub node: NodeRef,
    pub display_name: String,
    pub lat: f64,
    pub lon: f64,
    pub total_events: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existence_from: Option<Year>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existence_to: Option<Year>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub merged_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One event at a place: when it happened, and where it is narrated.")]
pub struct SceneEvent {
    pub id: String,
    pub label: String,
    pub when: TimeRange,
    pub verse_groups: Vec<VerseGroup>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A run of verses from one chapter of one book.")]
pub struct VerseGroup {
    pub book: String,
    pub chapter: u16,
    pub verses: Vec<VerseId>,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One leg of a narrative drawn on the map: a movement from one place to the next.")]
pub struct SceneArrow {
    pub narrative: String,
    pub color: String,
    pub from_place: String,
    pub to_place: String,
    pub from_event: String,
    pub to_event: String,
    pub order: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One narrative with at least one leg in view.")]
pub struct SceneNarrative {
    pub id: String,
    pub name: String,
    pub color: String,
    pub legs_in_scene: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scene_mode_round_trips_through_the_two_modes_a_scene_can_be_composed_in() {
        // Arrange
        let every_variant = SceneMode::ALL;
        // Act
        let json = serde_json::to_string(&every_variant).unwrap();
        let back: Vec<SceneMode> = serde_json::from_str(&json).unwrap();
        // Assert
        assert_eq!(json, r#"["time","scripture"]"#);
        assert_eq!(back, every_variant.to_vec());
    }

    #[test]
    fn a_populated_reference_serializes_as_ref_and_omits_window() {
        let scene = Scene {
            mode: SceneMode::Scripture,
            window: None,
            r#ref: Some(ScriptureRef::parse("GEN.1.1").unwrap()),
            places: vec![],
            quiet_places: vec![],
            arrows: vec![],
            narratives: vec![],
        };
        let json = serde_json::to_string(&scene).unwrap();
        assert!(json.contains("\"ref\":\"GEN.1.1\""), "missing ref key: {json}");
        assert!(!json.contains("\"r#ref\""), "the field's own spelling must never appear on the wire: {json}");
        assert!(!json.contains("\"window\""), "window must be omitted when None: {json}");
        assert!(json.contains("\"quiet_places\":[]"), "quiet_places must be present (empty, not omitted): {json}");

        let back: Scene = serde_json::from_str(&json).unwrap();
        assert_eq!(back, scene);
    }
}
