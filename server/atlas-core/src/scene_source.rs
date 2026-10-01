//! Exactly the reads the scene composer makes against its data, and nothing more, so a
//! second data source can stand in for `AtlasData`.

use std::collections::HashSet;

use crate::data::{Event, Narrative, Place, PlaceHistory, PlaceNameAlias};
use crate::refs::ScriptureRef;
use crate::time::TimeRange;

pub trait SceneSource {
    fn events_in_window(&self, w: &TimeRange) -> Vec<&Event>;

    fn events_matching_ref(&self, r: &ScriptureRef) -> Vec<&Event>;

    fn places(&self) -> &[Place];

    fn narratives(&self) -> &[Narrative];

    fn event_by_id(&self, id: &str) -> Option<&Event>;

    fn place_by_id(&self, id: &str) -> Option<&Place>;

    fn place_history_for(&self, id: &str) -> Option<&PlaceHistory>;

    fn place_name_alias_for(&self, id: &str) -> Option<&PlaceNameAlias>;

    fn event_bearing_place_ids(&self) -> &HashSet<String>;

    fn total_events_for(&self, id: &str) -> u32;

    fn place_node(&self, id: &str) -> crate::wire::NodeRef;
}
