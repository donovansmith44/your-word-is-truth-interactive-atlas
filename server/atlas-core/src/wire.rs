use serde::{Deserialize, Serialize};

use crate::time::TimeRange;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<TimeRange>,
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    pub sref: Option<String>,
    pub places: Vec<ScenePlace>,
    pub arrows: Vec<SceneArrow>,
    pub narratives: Vec<SceneNarrative>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenePlace {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub brightness: u8,
    pub events: Vec<SceneEvent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneEvent {
    pub id: String,
    pub label: String,
    pub when: TimeRange,
    pub verse_groups: Vec<VerseGroup>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerseGroup {
    pub book: String,
    pub chapter: u16,
    pub verses: Vec<String>,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneArrow {
    pub narrative: String,
    pub color: String,
    pub from_place: String,
    pub to_place: String,
    pub from_event: String,
    pub to_event: String,
    pub order: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    fn populated_sref_serializes_as_ref_and_omits_window() {
        let scene = Scene {
            mode: "scripture".into(),
            window: None,
            sref: Some("GEN.1.1".into()),
            places: vec![],
            arrows: vec![],
            narratives: vec![],
        };
        let json = serde_json::to_string(&scene).unwrap();
        assert!(json.contains("\"ref\":\"GEN.1.1\""), "missing ref key: {json}");
        assert!(!json.contains("\"sref\""), "sref must never appear on the wire: {json}");
        assert!(!json.contains("\"window\""), "window must be omitted when None: {json}");

        let back: Scene = serde_json::from_str(&json).unwrap();
        assert_eq!(back, scene);
    }
}
