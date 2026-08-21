use serde::{Deserialize, Serialize};

use crate::time::{TimeRange, Year};

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
    /// Batch H (existence gating, deferred from E2): the curated
    /// established/destroyed bounds this place's name resolves against
    /// (`crate::history::resolve_existence`) -- plain years, never the full
    /// established/destroyed `PlaceDateClaim` (verses/note; that richer
    /// shape is `/api/place/{id}`'s own `HistoryOut`, not a per-marker wire
    /// cost). Both omitted (not `null` -- see `skip_serializing_if`) when
    /// this place has no curated history at all, or a curated history with
    /// neither claim -- the client's own gate never fires in that case
    /// (`existence_gates_label`'s own "always labels" reading). The map
    /// renders every place regardless of these bounds (a lit place is only
    /// ever here because a real event justifies it THIS window -- see
    /// `scene::lit_places`); it's the client's own label -- never the
    /// marker/dot -- that these bounds gate, and only in time mode (there is
    /// no window to test outside-ness against in scripture mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existence_from: Option<Year>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existence_to: Option<Year>,
    /// Batch HOTFIX-2 (same-place dedupe): ids of every OTHER compiled place
    /// record merged into this one at load time (`crate::merge::MERGE_PAIRS`)
    /// -- e.g. `["hazor_545"]` on the place carrying id `hazor-1`. Wire
    /// traceability only (no client behavior reads this today): the brief's
    /// own "note the absorbed record's id in the wire for traceability" --
    /// so a future investigation into "why does hazor-1 carry events from
    /// two different upstream lineages" doesn't have to go spelunking
    /// through source data by hand. Empty (and omitted -- `skip_serializing_if`,
    /// matching `existence_from`/`_to`'s own lean-wire convention) for the
    /// overwhelming majority of places, which were never part of a merge.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub merged_ids: Vec<String>,
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
    /// Batch H (existence gating): same fields, same rule, as
    /// `ScenePlace::existence_from`/`existence_to` -- see that doc comment.
    /// A quiet place is exactly where this matters most in practice (the
    /// "ever-present graph" shows a dot for every event-bearing place
    /// regardless of window, so a long-destroyed place's dot can otherwise
    /// sit on a plate captioned with a name it never bore at that time).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existence_from: Option<Year>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existence_to: Option<Year>,
    /// Batch HOTFIX-2: same field, same rule, as `ScenePlace::merged_ids`
    /// above -- a quiet place is exactly where this can matter too (the
    /// survivor of a merge is quiet in any window neither absorbed record's
    /// own events intersect).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub merged_ids: Vec<String>,
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
