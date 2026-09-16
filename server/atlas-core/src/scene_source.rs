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
//!
//! Ten methods total (not eleven -- an earlier commit message/report for
//! this task miscounted, see task-3-report.md's Correction section). The
//! survey's own "eleven distinct `d.*` reads" tally counted `d.events` twice
//! (once per predicate: `when.intersects` for time mode, a verse-ref match
//! for scripture mode) among nine other named reads; this trait consolidates
//! that pair into two methods, `events_in_window`/`events_matching_ref`,
//! rather than reproducing the eleven-item tally as eleven methods.

use std::collections::HashSet;

use crate::data::{Event, Narrative, Place, PlaceHistory, PlaceNameAlias};
use crate::refs::ScriptureRef;
use crate::time::TimeRange;

/// The exact reads `atlas_core::scene` makes against its data -- nothing
/// else. See each method's doc comment for the `scene.rs` call site(s) it
/// backs.
pub trait SceneSource {
    /// Events whose resolved `when` intersects the window (time mode,
    /// `compose_time_scene`). Borrowed: `AtlasData` filters its own `events`
    /// vec in place; a future owning implementation (Task 4) can return the
    /// same shape by borrowing from its own materialised `Vec<Event>`.
    fn events_in_window(&self, w: &TimeRange) -> Vec<&Event>;

    /// Events whose own `verses` contain a verse inside `r` (scripture mode,
    /// `compose_scripture_scene`).
    fn events_matching_ref(&self, r: &ScriptureRef) -> Vec<&Event>;

    /// Every place (scripture mode's second pass, `compose_scripture_scene`).
    fn places(&self) -> &[Place];

    /// Every narrative (`build_arrows`/`legend`).
    fn narratives(&self) -> &[Narrative];

    /// Look up one event by id (narrative leg resolution, `build_arrows`/
    /// `legend`).
    fn event_by_id(&self, id: &str) -> Option<&Event>;

    /// Look up one place by id (`lit_places`/`quiet_places`).
    fn place_by_id(&self, id: &str) -> Option<&Place>;

    /// This place's curated history record, if any
    /// (`compose_scripture_scene`/`lit_places`/`quiet_places`).
    fn place_history_for(&self, id: &str) -> Option<&PlaceHistory>;

    /// This place's primary curated KJV display-name alias, if any
    /// (`compose_scripture_scene`/`lit_places`/`quiet_places`).
    fn place_name_alias_for(&self, id: &str) -> Option<&PlaceNameAlias>;

    /// Ids of every event-bearing place, all-time (quiet places,
    /// `quiet_places`). `&HashSet<String>` -- the real type
    /// `AtlasData::event_bearing_place_ids` already returns; `scene.rs` only
    /// ever iterates and filters it, so a set is exactly as usable as a
    /// slice here and this avoids materialising a second collection.
    fn event_bearing_place_ids(&self) -> &HashSet<String>;

    /// All-time event count for a place (`quiet_places`'s `total_events`
    /// field).
    fn total_events_for(&self, id: &str) -> u32;
}
