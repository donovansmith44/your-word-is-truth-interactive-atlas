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
    /// Batch E2 (the ever-present graph, user direction 2026-08-19: "all of
    /// the cities in our graph are available in any timerange rather than
    /// just loading those which are biblically active at the time"): every
    /// event-bearing place NOT in `places` for this window -- period
    /// relevance is now expressed by GLOW (whether a place is in `places`),
    /// not existence. QUIET-1: for every time-mode window, `places` union
    /// `quiet_places` equals the FULL event-bearing place set (fixed
    /// cardinality, derived from the data -- see
    /// `AtlasData::event_bearing_place_ids`), and the two are disjoint.
    /// ALWAYS an array (never omitted) on both modes -- but always EMPTY for
    /// scripture-mode scenes, which never gain quiet places at all (chosen
    /// over an `Option`/omitted-key representation for wire regularity with
    /// `places` itself, which is likewise never optional; a client reading
    /// `quiet_places` needs no null-check either way).
    pub quiet_places: Vec<QuietPlace>,
    pub arrows: Vec<SceneArrow>,
    pub narratives: Vec<SceneNarrative>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenePlace {
    pub id: String,
    pub name: String,
    /// Batch E: the period-true name resolved for this scene's own window
    /// (`crate::history::resolve_display_name`) -- `name`'s original,
    /// always-present value unchanged; this is the ADDITIONAL field the
    /// client renders on map labels / hover card title / arrow-tip text.
    /// Equal to `name` whenever this place has no curated history, or none
    /// of its curated ranges intersects the window.
    pub display_name: String,
    pub lat: f64,
    pub lon: f64,
    pub brightness: u8,
    pub events: Vec<SceneEvent>,
}

/// Batch E2: one "quiet" place on a time-mode scene -- an event-bearing
/// place that isn't lit (in `Scene::places`) for this particular window.
/// Deliberately LEAN -- no `events`/`verse_groups` at all, unlike
/// `ScenePlace`: a quiet place has none active this window BY DEFINITION,
/// so carrying an always-empty list would only bloat the payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuietPlace {
    pub id: String,
    /// Same window-resolution rules `ScenePlace::display_name` uses
    /// (`crate::history::resolve_display_name`) -- QUIET-1 requires this to
    /// never contradict what the lit side would show for the SAME window
    /// (e.g. a place that crosses from quiet to lit as the window nudges by
    /// a year must not also silently change its displayed name).
    pub display_name: String,
    pub lat: f64,
    pub lon: f64,
    /// This place's ALL-TIME event count (every event touching it, in ANY
    /// window) -- NOT scoped to the current scene's window, since a quiet
    /// place has zero events IN this window by definition; a window-scoped
    /// count would always read zero and tell the client nothing.
    pub total_events: u32,
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
