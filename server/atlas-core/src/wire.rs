use serde::{Deserialize, Serialize};

use crate::time::{TimeRange, Year};

/// The map at one moment of enquiry: which places are lit, which are only
/// present, and which arrows run between them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub mode: SceneMode,
    /// The span of years asked about; absent when a passage was asked about instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<TimeRange>,
    /// The passage asked about; absent when a span of years was asked about instead.
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub sref: Option<String>,
    /// The places something in view happens at.
    pub places: Vec<ScenePlace>,
    /// Every other place this atlas knows an event at: present on the map but not
    /// lit. Always sent, and always empty for a passage.
    pub quiet_places: Vec<QuietPlace>,
    /// The movements narratives draw between the places in view.
    pub arrows: Vec<SceneArrow>,
    /// The narratives with at least one leg in view.
    pub narratives: Vec<SceneNarrative>,
}

crate::vocabulary! {
    /// Which question a scene answers: what was happening in a span of years,
    /// or where a passage happens.
    SceneMode {
        Time => "time",
        Scripture => "scripture",
    }
}

/// One lit place on the map, with the events that light it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenePlace {
    pub id: String,
    /// The place's default name.
    pub name: String,
    /// The name to show: the one this place bore in the span asked about, or its
    /// default name when no period name applies.
    pub display_name: String,
    /// Latitude in degrees, north positive.
    pub lat: f64,
    /// Longitude in degrees, east positive.
    pub lon: f64,
    /// How brightly to draw this place, from how much of what is in view happens
    /// here.
    pub brightness: u8,
    pub events: Vec<SceneEvent>,
    /// The year this place was founded, where that is recorded; absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<i32>)]
    pub existence_from: Option<Year>,
    /// The year it ceased to exist, where that is recorded; absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<i32>)]
    pub existence_to: Option<Year>,
    /// The ids of other records for this same place, folded into this one. Omitted
    /// when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub merged_ids: Vec<String>,
}

/// A place this atlas knows an event at, drawn on the map but not lit for the
/// span asked about.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct QuietPlace {
    pub id: String,
    /// The name to show, resolved for the same span the lit places use.
    pub display_name: String,
    /// Latitude in degrees, north positive.
    pub lat: f64,
    /// Longitude in degrees, east positive.
    pub lon: f64,
    /// How many events touch this place in any year. A count for the span asked
    /// about would always be zero, which is what makes the place quiet.
    pub total_events: u32,
    /// The year this place was founded, where that is recorded; absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<i32>)]
    pub existence_from: Option<Year>,
    /// The year it ceased to exist, where that is recorded; absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<i32>)]
    pub existence_to: Option<Year>,
    /// The ids of other records for this same place, folded into this one. Omitted
    /// when there are none.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub merged_ids: Vec<String>,
}

/// One event at a place: when it happened, and where it is narrated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneEvent {
    pub id: String,
    pub label: String,
    /// The years the event spans.
    pub when: TimeRange,
    /// The passages narrating it, grouped by book and chapter.
    pub verse_groups: Vec<VerseGroup>,
}

/// A run of verses from one chapter of one book.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct VerseGroup {
    /// The book's three-letter code, such as `GEN`.
    pub book: String,
    pub chapter: u16,
    /// The verse numbers and ranges covered, such as `3` or `3-5`.
    pub verses: Vec<String>,
    /// How many verses the group covers in all.
    pub count: u32,
}

/// One leg of a narrative drawn on the map: a movement from one place to the
/// next.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneArrow {
    /// The id of the narrative this leg belongs to.
    pub narrative: String,
    /// The colour that narrative is drawn in.
    pub color: String,
    /// The id of the place the movement starts from.
    pub from_place: String,
    /// The id of the place it arrives at.
    pub to_place: String,
    /// The id of the event at the starting place.
    pub from_event: String,
    /// The id of the event at the arriving place.
    pub to_event: String,
    /// This leg's position along the narrative, counting from its first.
    pub order: u32,
}

/// One narrative with at least one leg in view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SceneNarrative {
    pub id: String,
    /// The narrative's name.
    pub name: String,
    /// The colour its arrows are drawn in.
    pub color: String,
    /// How many of its legs are in view.
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
    fn populated_sref_serializes_as_ref_and_omits_window() {
        let scene = Scene {
            mode: SceneMode::Scripture,
            window: None,
            sref: Some("GEN.1.1".into()),
            places: vec![],
            quiet_places: vec![],
            arrows: vec![],
            narratives: vec![],
        };
        let json = serde_json::to_string(&scene).unwrap();
        assert!(json.contains("\"ref\":\"GEN.1.1\""), "missing ref key: {json}");
        assert!(!json.contains("\"sref\""), "sref must never appear on the wire: {json}");
        assert!(!json.contains("\"window\""), "window must be omitted when None: {json}");
        // Batch E2: `quiet_places` stays a present, empty array here -- NEVER
        // an omitted key -- even for a scripture-mode scene (this fixture's
        // own mode), per the doc comment's "always an array" wire choice.
        assert!(json.contains("\"quiet_places\":[]"), "quiet_places must be present (empty, not omitted): {json}");

        let back: Scene = serde_json::from_str(&json).unwrap();
        assert_eq!(back, scene);
    }
}
