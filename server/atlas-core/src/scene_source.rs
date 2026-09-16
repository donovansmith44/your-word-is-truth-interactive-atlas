//! The seam between `atlas_core::scene` (the map's scene composer) and its
//! data. `SceneSource` is defined by exactly what `scene.rs` reads -- nothing
//! else -- so a second implementation (the graph-backed source, Task 4) can
//! stand in for `AtlasData` without `scene.rs` knowing the difference.
//!
//! Two implementations: `AtlasData` (this task, delegates to its existing
//! methods/fields) and the graph-backed source (Task 4, owns its materialised
//! `Vec<Event>`/`Vec<Place>`/`Vec<Narrative>`).
//!
//! `crate::merge::absorbed_ids_for` (a const table) and
//! `crate::history::resolve_display_name`/`resolve_existence` (free
//! functions taking already-fetched `Option<&PlaceHistory>`/
//! `Option<&PlaceNameAlias>`) are NOT part of this trait -- `scene.rs` calls
//! them directly, unchanged.

use std::collections::HashSet;

use crate::data::{Event, Narrative, Place, PlaceHistory, PlaceNameAlias};
use crate::refs::ScriptureRef;
use crate::time::TimeRange;

/// The exact reads `atlas_core::scene` makes against its data -- nothing
/// else. See each method's doc comment for the `scene.rs` call site(s) it
/// backs.
pub trait SceneSource {
    /// Events whose resolved `when` intersects the window (time mode,
    /// `scene.rs:17`). Borrowed: `AtlasData` filters its own `events` vec in
    /// place; a future owning implementation (Task 4) can return the same
    /// shape by borrowing from its own materialised `Vec<Event>`.
    fn events_in_window(&self, w: &TimeRange) -> Vec<&Event>;

    /// Events whose own `verses` contain a verse inside `r` (scripture mode,
    /// `scene.rs:31-35`).
    fn events_matching_ref(&self, r: &ScriptureRef) -> Vec<&Event>;

    /// Every place (scripture mode's second pass, `scene.rs:44`).
    fn places(&self) -> &[Place];

    /// Every narrative (`build_arrows`/`legend`, `scene.rs:117,378`).
    fn narratives(&self) -> &[Narrative];

    /// Look up one event by id (narrative leg resolution, `scene.rs:121,386`).
    fn event_by_id(&self, id: &str) -> Option<&Event>;

    /// Look up one place by id (`scene.rs:168,218`).
    fn place_by_id(&self, id: &str) -> Option<&Place>;

    /// This place's curated history record, if any (`scene.rs:67,169,219`).
    fn place_history_for(&self, id: &str) -> Option<&PlaceHistory>;

    /// This place's primary curated KJV display-name alias, if any
    /// (`scene.rs:71,173,222`).
    fn place_name_alias_for(&self, id: &str) -> Option<&PlaceNameAlias>;

    /// Ids of every event-bearing place, all-time (quiet places,
    /// `scene.rs:214`). `&HashSet<String>` -- the real type
    /// `AtlasData::event_bearing_place_ids` already returns; `scene.rs` only
    /// ever iterates and filters it, so a set is exactly as usable as a
    /// slice here and this avoids materialising a second collection.
    fn event_bearing_place_ids(&self) -> &HashSet<String>;

    /// All-time event count for a place (`quiet_places.total_events`,
    /// `scene.rs:225`).
    fn total_events_for(&self, id: &str) -> u32;
}
