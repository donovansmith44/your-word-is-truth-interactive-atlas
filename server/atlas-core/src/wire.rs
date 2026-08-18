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
