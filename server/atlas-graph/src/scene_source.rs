//! OVERLAY-1 Task 4: `GraphSceneSource` -- the map scene's data, sourced
//! from the graph port, implementing `atlas_core::scene_source::SceneSource`
//! so `atlas_core::scene` composes against it without knowing the
//! difference.
//!
//! WHAT THIS STILL DOES, PLAINLY: it MATERIALISES events, places and
//! narratives ONCE, at construction, exactly the way
//! `legacy::atlas_data_overlay` does -- `event_from_node`/`place_from_node`/
//! `narrative_from_node` over `gs.event_ids`/`gs.place_ids`/
//! `gs.narrative_ids` (reused, never forked) -- and then owns the resulting
//! `Vec<Event>`/`Vec<Place>`/`Vec<Narrative>` in memory. This is not a
//! lazy source and does not pretend to be one.
//!
//! WHAT IT DOES NOT DO, which is the whole point of building it:
//!   * NO KJV copy. OVERLAY-1 Task 2 retired `verses_from_graph`'s own
//!     ~31,102-verse spine walk; this source never touches verse TEXT at
//!     all (`scene.rs` never read `d.verses` -- see the OVERLAY-1 survey
//!     §3). Verse text is read on demand by `GraphService::verse_text_of`.
//!   * NO `AtlasData` index web. `AtlasData::finish()` derives
//!     `verse_to_events`, `verse_to_places`, `verse_heading`,
//!     `heading_anchor_collisions`, the catechism trio and the timeline
//!     order -- none of which `scene.rs` reads. This source derives ONLY
//!     the four things the composer actually needs: `event_index`,
//!     `place_index`, `event_counts_by_place` and
//!     `event_bearing_place_ids`. (Task 5 re-sources the verse-web readers
//!     -- `handlers::chapter`, verse popovers, reader headings -- directly
//!     from the port; they are deliberately not this object's business.)
//!   * NO dependency on `AtlasData`'s graph-derived fields. The only thing
//!     it takes from `AtlasData` is the two genuinely sidecar-backed,
//!     curated-JSON maps `scene.rs` reads -- `place_history` and
//!     `place_name_aliases` -- copied at construction. Those are curated
//!     files, legitimately not graph nodes (survey §3, "NOT available").
//!
//! AND WHY IT IS SHAPED THIS WAY: it is an explicit, single-purpose object
//! the DB-4 batch replaces with SQL WITHOUT touching `scene.rs` again --
//! `events_in_window` (a linear filter over the 1,711 materialised events
//! today) becomes an `event_date` range seek, and `total_events_for` /
//! `event_bearing_place_ids` (a whole-collection counting pass today)
//! become a `GROUP BY` over `located_at`. The trait seam Task 3 cut is
//! what makes that a swap of this file rather than a rewrite of the
//! composer.
//!
//! THE ORDER LAW (why this file is so particular about sequence): the 25
//! scene responses `atlas-server/tests/scene_byte_identity.rs` pins were
//! captured against the OVERLAY path, and `AtlasData::finish()`'s
//! `events.sort_by_key(|e| e.when.from_year)` (`data.rs:1533`) is a STABLE
//! sort with NO secondary key -- so for the great majority of the 1,711
//! real events, which tie on `from_year`, the PRE-SORT insertion order
//! decides the served order. Task 1 measured 1,364 of 1,711 positions
//! differing between the compile-from-sources order and the overlay order
//! and ruled the OVERLAY order authoritative. This file therefore
//! reproduces `finish()`'s sequence exactly and in the same order:
//! `merge::apply_place_merges` (`data.rs:1521`), then
//! `event_merge::apply_event_merges` (`data.rs:1531`), then the stable
//! `sort_by_key` (`data.rs:1533`), then the derived indexes
//! (`data.rs:1535-1547`) and the event-bearing aggregate
//! (`data.rs:1592-1599`). `server/atlas-graph/tests/
//! scene_source_equivalence_real_data.rs` is the proof, byte for byte, on
//! all 25 pinned inputs.
//!
//! Per-event `places` vector order (`places[0]` is the true anchor,
//! load-bearing for `scene::build_arrows`'s arrow endpoints) comes
//! straight from the `located-at` edge order the port returns -- see
//! `legacy::event_from_node`'s own ANCHOR-PLACE ORDER LAW doc comment. It
//! is never re-sorted here.

use std::collections::{HashMap, HashSet};

use atlas_core::data::{Event, Narrative, Place, PlaceHistory, PlaceNameAlias};
use atlas_core::refs::{ScriptureRef, VerseId};
use atlas_core::scene_source::SceneSource;
use atlas_core::time::TimeRange;

use crate::service::GraphService;

/// The scene's data, materialised once from the graph port. See the module
/// doc comment for what it deliberately does and does not carry.
#[derive(Debug, Clone)]
pub struct GraphSceneSource {
    /// Post-merge, post-sort -- the exact order `AtlasData::finish()`
    /// produces on the overlay path, which is the order the 25 pinned
    /// scene responses were captured against.
    events: Vec<Event>,
    /// Post-`apply_place_merges` (absorbed ids removed, merged ids folded
    /// onto their survivor), in `gs.place_ids` order otherwise.
    places: Vec<Place>,
    /// Post-`apply_event_merges` (a leg naming an absorbed event is
    /// repointed at its survivor).
    narratives: Vec<Narrative>,
    /// `data.rs:1541` -- id -> index into `events`, for `event_by_id`.
    event_index: HashMap<String, usize>,
    /// `data.rs:1535` -- id -> index into `places`, for `place_by_id`.
    place_index: HashMap<String, usize>,
    /// `data.rs:1592-1597` -- every event's EVERY place counted once per
    /// event (not just the anchor); the all-time count
    /// `scene::quiet_places` carries as `total_events`.
    event_counts_by_place: HashMap<String, u32>,
    /// `data.rs:1598` -- literally the key set of `event_counts_by_place`.
    event_bearing_place_ids: HashSet<String>,
    /// Sidecar-only (curated JSON, not a graph node): copied from
    /// `AtlasData` at construction.
    place_history: HashMap<String, PlaceHistory>,
    /// Sidecar-only, same as `place_history`.
    place_name_aliases: HashMap<String, Vec<PlaceNameAlias>>,
}

impl GraphSceneSource {
    /// Builds the whole source in one pass over the graph.
    ///
    /// `sidecars` is an `AtlasData` whose CURATED-JSON maps are read --
    /// `place_history` and `place_name_aliases` only, both of which
    /// `AtlasData::load` fills from `data/compiled/place-history.json` /
    /// `place-names-kjv.json` and which `finish()` never touches. It is
    /// therefore safe (and intended) to pass a bare, un-`finish()`ed
    /// `AtlasData::load(...)` here: none of its graph-derived fields
    /// (`events`/`places`/`narratives`) is read.
    ///
    /// The sequence below is `AtlasData::finish()`'s, step for step, and
    /// must stay that way -- see the module doc comment's ORDER LAW.
    pub fn build(gs: &GraphService, sidecars: &atlas_core::data::AtlasData) -> Self {
        let snap = gs.snapshot();

        // Step 0: materialise, in the SAME id order `legacy::
        // atlas_data_overlay` walks (`gs.event_ids`/`place_ids`/
        // `narrative_ids`), through the SAME reused per-node builders.
        // This is the pre-sort insertion order the stable sort below
        // preserves for every `from_year` tie, so it is load-bearing.
        let mut events: Vec<Event> = gs.event_ids.iter().filter_map(|id| crate::legacy::event_from_node(id, &snap, &gs.chronology.chrono)).collect();
        let mut places: Vec<Place> = gs.place_ids.iter().filter_map(|id| crate::legacy::place_from_node(id, &snap)).collect();
        let empty_legs: Vec<String> = Vec::new();
        let mut narratives: Vec<Narrative> = gs
            .narrative_ids
            .iter()
            .filter_map(|id| crate::legacy::narrative_from_node(id, &snap, gs.narrative_legs.get(&id.raw).unwrap_or(&empty_legs)))
            .collect();

        // Step 1 (`data.rs:1521`): the curated same-place dedupe, FIRST --
        // it rewrites `events[*].places` as well as removing absorbed
        // places, so every derivation below must come after it.
        atlas_core::merge::apply_place_merges(&mut places, &mut events);

        // Step 2 (`data.rs:1531`): the curated duplicate-event
        // rectification, immediately after the place merge and before the
        // sort -- it REMOVES events, so doing it after the sort would
        // leave a different surviving order.
        atlas_core::event_merge::apply_event_merges(&mut events, &mut narratives);

        // Step 3 (`data.rs:1533`): the stable sort, single key, no
        // secondary -- reproduced verbatim, ties and all.
        events.sort_by_key(|e| e.when.from_year);

        // Step 4 (`data.rs:1535-1547`): the two id -> index maps the
        // composer's `event_by_id`/`place_by_id` reads use.
        let place_index: HashMap<String, usize> = places.iter().enumerate().map(|(i, p)| (p.id.clone(), i)).collect();
        let event_index: HashMap<String, usize> = events.iter().enumerate().map(|(i, e)| (e.id.clone(), i)).collect();

        // Step 5 (`data.rs:1592-1599`): one pass over every event's EVERY
        // place -- all of `e.places`, not only the anchor `places[0]` --
        // building the all-time count and, from its key set, the
        // event-bearing id set. Merged/absorbed ids never appear here:
        // `apply_place_merges` (step 1) already rewrote every event's
        // `places` entry onto its survivor id, so counting after it is
        // what makes the survivor carry the union of both records' events.
        let mut event_counts_by_place: HashMap<String, u32> = HashMap::new();
        for e in &events {
            for pid in &e.places {
                *event_counts_by_place.entry(pid.clone()).or_insert(0) += 1;
            }
        }
        let event_bearing_place_ids: HashSet<String> = event_counts_by_place.keys().cloned().collect();

        Self {
            events,
            places,
            narratives,
            event_index,
            place_index,
            event_counts_by_place,
            event_bearing_place_ids,
            place_history: sidecars.place_history.clone(),
            place_name_aliases: sidecars.place_name_aliases.clone(),
        }
    }
}

/// Every method mirrors the exact `AtlasData` read `scene.rs` makes, per
/// `SceneSource`'s own per-method doc comments.
impl SceneSource for GraphSceneSource {
    /// A linear filter over the materialised events today -- DB-4 replaces
    /// this body with an `event_date` range seek, without `scene.rs`
    /// changing. Order is the `events` vec's own order, which is
    /// `finish()`'s post-sort order.
    fn events_in_window(&self, w: &TimeRange) -> Vec<&Event> {
        self.events.iter().filter(|e| e.when.intersects(w)).collect()
    }

    /// `scene.rs:31-35`'s own predicate: an event matches when any of its
    /// OWN `verses` (never its witnesses' -- that widening belongs to
    /// `finish()`'s `verse_to_events`, which the composer does not read)
    /// falls inside the ref.
    fn events_matching_ref(&self, r: &ScriptureRef) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| {
                e.verses
                    .iter()
                    .any(|v| atlas_core::scene::ref_contains(r, &VerseId::parse_canonical(v).expect("etl-validated verse id")))
            })
            .collect()
    }

    fn places(&self) -> &[Place] {
        &self.places
    }

    fn narratives(&self) -> &[Narrative] {
        &self.narratives
    }

    fn event_by_id(&self, id: &str) -> Option<&Event> {
        self.event_index.get(id).map(|&i| &self.events[i])
    }

    fn place_by_id(&self, id: &str) -> Option<&Place> {
        self.place_index.get(id).map(|&i| &self.places[i])
    }

    fn place_history_for(&self, id: &str) -> Option<&PlaceHistory> {
        self.place_history.get(id)
    }

    /// "Primary" = the FIRST curated alias, in curated-file order --
    /// `AtlasData::place_name_alias_for`'s own rule (`data.rs:1820`).
    fn place_name_alias_for(&self, id: &str) -> Option<&PlaceNameAlias> {
        self.place_name_aliases.get(id).and_then(|v| v.first())
    }

    fn event_bearing_place_ids(&self) -> &HashSet<String> {
        &self.event_bearing_place_ids
    }

    /// 0 for a place with no events -- the same honest default
    /// `AtlasData::total_events_for` gives (`data.rs:1848`).
    fn total_events_for(&self, id: &str) -> u32 {
        self.event_counts_by_place.get(id).copied().unwrap_or(0)
    }
}
