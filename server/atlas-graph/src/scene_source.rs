//! OVERLAY-1 Task 4: `GraphSceneSource` -- the map scene's data, sourced
//! from the graph port, implementing `atlas_core::scene_source::SceneSource`
//! so `atlas_core::scene` composes against it without knowing the
//! difference.
//!
//! WHAT THIS STILL DOES, PLAINLY: it MATERIALISES events, places and
//! narratives ONCE, at construction, exactly the way the boot-time overlay
//! OVERLAY-1 Task 5 deleted (`legacy::atlas_data_overlay`) did --
//! `event_from_node`/`place_from_node`/
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
//!   * NO `AtlasData` index web beyond what a live reader actually asks
//!     for. `AtlasData::finish()` derives `verse_heading`,
//!     `heading_anchor_collisions`, the catechism trio and the timeline
//!     order. The composer (`scene.rs`) reads NONE of them, so the
//!     `SceneSource` impl below derives only the four things it needs:
//!     `event_index`, `place_index`, `event_counts_by_place` and
//!     `event_bearing_place_ids`. `verse_heading`/
//!     `heading_anchor_collisions` are not here at all --
//!     `GraphService::heading_index` (`heading.rs`) already serves the
//!     reader, and `atlas_etl::validate` is what reads the collision list,
//!     at compile time -- and the catechism trio is genuinely
//!     sidecar-derived and stays on `AtlasData`.
//!
//!     The two verse-web indexes `finish()` USED to derive,
//!     `verse_to_events` and `verse_to_places`, were deleted from
//!     `AtlasData` by OVERLAY-1 Task 5 and rebuilt HERE, because their only
//!     readers are runtime ones: `bibex verse`'s EVENTS/PASSAGES sections
//!     and, for places, `handlers::chapter`'s place-mention half plus
//!     `bibex verse`'s PLACES section. Both are INHERENT methods, not
//!     `SceneSource` methods -- the trait stays at the ten reads the
//!     composer makes, so whatever DB-4 decides about the trait's own
//!     borrowed-vs-owned returns (see this module's closing section) is
//!     independent of these two INHERENT methods. They are built from the
//!     materialised
//!     collections in exactly the passes `finish()` used, so their
//!     per-verse ORDER is identical (see each field's own doc comment),
//!     and the regression pins that used to live beside those `AtlasData`
//!     fields moved into this file's own test module with them.
//!   * NO dependency on `AtlasData`'s graph-derived fields. The only thing
//!     it takes from `AtlasData` is the two genuinely sidecar-backed,
//!     curated-JSON maps `scene.rs` reads -- `place_history` and
//!     `place_name_aliases` -- copied at construction. Those are curated
//!     files, legitimately not graph nodes (survey §3, "NOT available").
//!
//! AND WHY IT IS SHAPED THIS WAY: it is an explicit, single-purpose object
//! the DB-4 batch can replace with SQL -- `events_in_window` (a linear
//! filter over the 1,711 materialised events today) becomes an `event_date`
//! range seek, and `total_events_for` / `event_bearing_place_ids` (a
//! whole-collection counting pass today) become a `GROUP BY` over
//! `located_at`. But that swap is not free of `scene.rs`: the
//! `SceneSource` trait returns BORROWS (`Vec<&Event>`, `&[Place]`,
//! `&[Narrative]`, `&HashSet<String>`), so a SQL-backed implementation must
//! either (a) keep a materialised cache behind the trait the same shape as
//! this file's -- memory does not move, only the source of truth does -- or
//! (b) widen the trait to owned returns, which touches `scene.rs`'s own
//! `kept: Vec<&Event>` plumbing once. This module does not resolve that
//! fork; DB-2's plan is what decides it.
//!
//! THE ORDER LAW (why this file is so particular about sequence): the 25
//! scene responses `atlas-server/tests/scene_byte_identity.rs` pins were
//! captured against the OVERLAY path, and `AtlasData::finish()`'s
//! `events.sort_by_key(|e| e.when.from_year)` is a STABLE
//! sort with NO secondary key -- so for the great majority of the 1,711
//! real events, which tie on `from_year`, the PRE-SORT insertion order
//! decides the served order. Task 1 measured 1,364 of 1,711 positions
//! differing between the compile-from-sources order and the overlay order
//! and ruled the OVERLAY order authoritative. This file therefore
//! reproduces `AtlasData::finish()`'s sequence exactly and in the same
//! order: `merge::apply_place_merges`, then
//! `event_merge::apply_event_merges`, then the stable `sort_by_key`, then
//! the two id->index maps, then the event-bearing aggregate. Every citation
//! in this file names the FUNCTION (`AtlasData::finish`, in
//! `atlas-core/src/data.rs`) rather than a line number: Task 5's own
//! deletions inside that very function moved every line number Task 4 wrote
//! down, which is exactly the failure mode line citations have.
//!
//! `server/atlas-server/tests/scene_byte_identity.rs` is the standing
//! proof: its 25 pinned response hashes are composed from THIS object now,
//! and are byte-for-byte the values captured against the overlay path.
//! (Task 4's own transitional equivalence test, and Task 1's
//! `overlay_equivalence.rs`, were deleted together with the overlay in Task
//! 5 -- their reference side no longer exists.)
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
///
/// Deliberately NOT `Clone` (OVERLAY-1 Task 4 review): this owns 1,711
/// materialised events plus every place and narrative, and the entire point
/// of the batch is that exactly ONE such copy exists per process. Not
/// `Debug` either -- a derived `Debug` over those collections is megabytes
/// of output nobody wants and, being derived, would be reached by accident.
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
    /// `AtlasData::finish`'s own event index -- id -> index into `events`,
    /// for `event_by_id`.
    event_index: HashMap<String, usize>,
    /// `AtlasData::finish`'s own place index -- id -> index into `places`,
    /// for `place_by_id`.
    place_index: HashMap<String, usize>,
    /// `AtlasData::finish`'s own aggregate pass -- every event's EVERY
    /// place counted once per event (not just the anchor); the all-time
    /// count `scene::quiet_places` carries as `total_events`.
    event_counts_by_place: HashMap<String, u32>,
    /// Literally the key set of `event_counts_by_place`, exactly as
    /// `AtlasData::finish` derives it.
    event_bearing_place_ids: HashSet<String>,
    /// OVERLAY-1 Task 5: verse dot-ref -> every event id touching that
    /// verse, built in the SAME single pass `AtlasData::finish`'s own
    /// `verse_to_events` used -- each event's OWN `verses` UNIONED with
    /// every witness's default-translation verses, deduped per event so a
    /// verse present in both yields that id exactly once. Built by
    /// iterating `events` in order, so a verse's id list comes out in
    /// post-sort event order, identical to `finish()`'s. Read by `bibex
    /// verse` (EVENTS/PASSAGES); never by the composer.
    verse_to_events: HashMap<String, Vec<String>>,
    /// OVERLAY-1 Task 5: verse dot-ref -> every place id whose curated
    /// `verse_links` names it, built in the SAME single pass
    /// `AtlasData::finish`'s own `verse_to_places` used, iterating `places`
    /// in order -- so a verse's id list comes out in `places`-vec order,
    /// identical to `finish()`'s. Read by `handlers::chapter` and `bibex
    /// verse`; never by the composer.
    verse_to_places: HashMap<String, Vec<String>>,
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

        // Step 0: materialise, in the SAME id order the deleted overlay
        // walked (the retired `event_ids`/`place_ids`/`narrative_ids`
        // companions; DB-3: the port's `nodes_of_kind`, the identical
        // BTreeMap-by-id order), through the SAME reused per-node builders.
        // This is the pre-sort insertion order the stable sort below
        // preserves for every `from_year` tie, so it is load-bearing
        // (`scene_byte_identity.rs`'s 25 hashes are the proof).
        use atlas_graph_types::id::NodeKind;
        let mut events: Vec<Event> = gs.ids_of_kind(NodeKind::Event).iter().filter_map(|id| crate::legacy::event_from_node(id, &snap, &gs.chronology.chrono)).collect();
        let mut places: Vec<Place> = gs.ids_of_kind(NodeKind::Place).iter().filter_map(|id| crate::legacy::place_from_node(id, &snap)).collect();
        let empty_legs: Vec<String> = Vec::new();
        let mut narratives: Vec<Narrative> = gs
            .ids_of_kind(NodeKind::Narrative)
            .iter()
            .filter_map(|id| crate::legacy::narrative_from_node(id, &snap, gs.narrative_legs.get(&id.raw).unwrap_or(&empty_legs)))
            .collect();

        // Step 1 (`AtlasData::finish`): the curated same-place dedupe,
        // FIRST --
        // it rewrites `events[*].places` as well as removing absorbed
        // places, so every derivation below must come after it.
        atlas_core::merge::apply_place_merges(&mut places, &mut events);

        // Step 2 (`AtlasData::finish`): the curated duplicate-event
        // rectification, immediately after the place merge and before the
        // sort -- it REMOVES events, so doing it after the sort would
        // leave a different surviving order.
        atlas_core::event_merge::apply_event_merges(&mut events, &mut narratives);

        // Step 3 (`AtlasData::finish`): ORDER-1 (DB-4a) -- the total key
        // `(from_year, id)`. The same order as before by construction: the
        // input was id-ordered (`nodes_of_kind`) and the sort was stable, so
        // ties already fell to id; the key now says so.
        events.sort_by(|a, b| (a.when.from_year, &a.id).cmp(&(b.when.from_year, &b.id)));

        // Step 4 (`AtlasData::finish`): the two id -> index maps the
        // composer's `event_by_id`/`place_by_id` reads use.
        let place_index: HashMap<String, usize> = places.iter().enumerate().map(|(i, p)| (p.id.clone(), i)).collect();
        let event_index: HashMap<String, usize> = events.iter().enumerate().map(|(i, e)| (e.id.clone(), i)).collect();

        // Step 5 (`AtlasData::finish`): one pass over every event's EVERY
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

        // Step 6 (OVERLAY-1 Task 5, `AtlasData::finish`'s own two verse-web
        // passes): the only two index-web entries with a live reader
        // outside the composer, reproduced verbatim -- including the event
        // UNION rule (an event's own verses PLUS every witness's own
        // default-translation verses, deduped per event), which is a real
        // behavior `finish()` fixed once and which `bibex verse`'s output
        // depends on. Both iterate their collection in its final order, so
        // every per-verse id list comes out in the identical order the
        // deleted `AtlasData` indexes produced -- `bibex`'s pinned
        // transcripts are the check on that.
        let mut verse_to_events: HashMap<String, Vec<String>> = HashMap::new();
        for e in &events {
            let mut seen: HashSet<&str> = HashSet::new();
            for v in e
                .verses
                .iter()
                .chain(e.witnesses.iter().filter_map(|w| w.translations.get(atlas_core::translation::DEFAULT_TRANSLATION)).flatten())
            {
                if seen.insert(v.as_str()) {
                    verse_to_events.entry(v.clone()).or_default().push(e.id.clone());
                }
            }
        }
        let mut verse_to_places: HashMap<String, Vec<String>> = HashMap::new();
        for p in &places {
            for v in &p.verse_links {
                verse_to_places.entry(v.clone()).or_default().push(p.id.clone());
            }
        }

        Self {
            events,
            places,
            narratives,
            event_index,
            place_index,
            event_counts_by_place,
            event_bearing_place_ids,
            verse_to_events,
            verse_to_places,
            place_history: sidecars.place_history.clone(),
            place_name_aliases: sidecars.place_name_aliases.clone(),
        }
    }

    /// Every event id touching `verse`, in post-sort event order -- the
    /// exact list, and the exact order, the deleted
    /// `AtlasData::events_for_verse` returned; an empty slice when the
    /// verse is in no event, the same honest default. INHERENT, not a
    /// `SceneSource` method: the composer never asks this.
    pub fn events_for_verse(&self, verse: &str) -> &[String] {
        self.verse_to_events.get(verse).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Every place id whose curated `verse_links` names `verse`, in
    /// `places`-vec order -- the exact list, and the exact order, the
    /// deleted `AtlasData::places_for_verse` returned. INHERENT, not a
    /// `SceneSource` method, for the same reason as `events_for_verse`.
    pub fn places_for_verse(&self, verse: &str) -> &[String] {
        self.verse_to_places.get(verse).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// `place_by_id` without the `SceneSource` trait in scope -- the same
    /// lookup, for the handlers and CLI commands that hold this object for
    /// its verse-web indexes and need the `Place` record too.
    pub fn place(&self, id: &str) -> Option<&Place> {
        self.place_index.get(id).map(|&i| &self.places[i])
    }

    /// `event_by_id` without the trait in scope -- see `place`.
    pub fn event(&self, id: &str) -> Option<&Event> {
        self.event_index.get(id).map(|&i| &self.events[i])
    }

    /// The narrative list without the trait in scope -- see `place`.
    pub fn narrative_list(&self) -> &[Narrative] {
        &self.narratives
    }
}

/// Every method mirrors the exact `AtlasData` read `scene.rs` makes, per
/// `SceneSource`'s own per-method doc comments.
impl SceneSource for GraphSceneSource {
    /// A linear filter over the materialised events today -- DB-4 can
    /// replace this body with an `event_date` range seek (see this module's
    /// own doc comment for the borrowed-return fork that decision still
    /// has to resolve). Order is the `events` vec's own order, which is
    /// `finish()`'s post-sort order.
    fn events_in_window(&self, w: &TimeRange) -> Vec<&Event> {
        self.events.iter().filter(|e| e.when.intersects(w)).collect()
    }

    /// `atlas_core::scene::compose_scripture_scene`'s own predicate, run
    /// through the `pub` helper that is its true home,
    /// `atlas_core::scene::ref_contains`: an event matches when any of its
    /// OWN `verses` falls inside the ref -- never its witnesses'. That
    /// widening belongs to `events_for_verse` above, which the composer
    /// does not read; keeping the two apart is what lets a scripture scene
    /// and a verse popover legitimately differ.
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
    /// `AtlasData::place_name_alias_for`'s own rule.
    fn place_name_alias_for(&self, id: &str) -> Option<&PlaceNameAlias> {
        self.place_name_aliases.get(id).and_then(|v| v.first())
    }

    fn event_bearing_place_ids(&self) -> &HashSet<String> {
        &self.event_bearing_place_ids
    }

    /// 0 for a place with no events -- the same honest default
    /// `AtlasData::total_events_for` gives.
    fn total_events_for(&self, id: &str) -> u32 {
        self.event_counts_by_place.get(id).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, EventWitness};
    use std::collections::HashMap as Map;

    const KJV_FIXTURE: &str = r#"{
      "translation": "KJV",
      "books": [
        { "name": "Genesis", "chapters": [
          { "chapter": 1, "verses": [
            { "verse": 1, "text": "In the beginning God created the heaven and the earth." },
            { "verse": 2, "text": "And the earth was without form, and void." },
            { "verse": 3, "text": "And God said, Let there be light: and there was light." }
          ] },
          { "chapter": 2, "verses": [
            { "verse": 1, "text": "Thus the heavens and the earth were finished." }
          ] }
        ] }
      ]
    }"#;
    const NO_XREFS: &str = "From Verse\tTo Verse\tVotes\t#comment\n";

    /// One place with a curated `verse_link`, and one event whose top-level
    /// `verses` and whose WITNESS's verses overlap on exactly one verse --
    /// the three cases the two verse-web indexes have to get right.
    fn fixture_atlas() -> AtlasData {
        let places = vec![Place { id: "p1".into(), name: "P1".into(), lat: 1.0, lon: 2.0, verse_links: vec!["GEN.2.1".into()] }];
        let events = vec![Event {
            id: "e1".into(),
            label: "A container over GEN.1".into(),
            when: TimeRange::new(-4004, -4004).unwrap(),
            places: vec!["p1".into()],
            // GEN.1.2 is deliberately in BOTH halves -- the dedupe case.
            verses: vec!["GEN.1.1".into(), "GEN.1.2".into()],
            witnesses: vec![EventWitness {
                book: "GEN".into(),
                translations: Map::from([("kjv".to_string(), vec!["GEN.1.2".to_string(), "GEN.1.3".to_string()])]),
                ref_note: None,
                robertson_section: None,
            }],
            ..Default::default()
        }];
        AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], Map::new(), Map::new()).finish()
    }

    fn fixture_source() -> GraphSceneSource {
        let atlas = fixture_atlas();
        let gs = GraphService::from_sources(KJV_FIXTURE, NO_XREFS, &atlas).expect("the fixture graph must build");
        GraphSceneSource::build(&gs, &atlas)
    }

    /// OVERLAY-1 Task 5 re-homes Batch T requirement 3's own regression pin
    /// (a REAL, live-caught bug: the reverse index used to be built from
    /// `Event::verses` ALONE, so a verse cited ONLY by a witness -- e.g.
    /// MAT.26.6, Matthew's account of the Bethany anointing, when
    /// `pw_bethany`'s own top-level `verses` names John only -- resolved to
    /// NO event, even though the same verse correctly anchored that event's
    /// reader heading). It was pinned against `AtlasData::finish()`'s own
    /// `verse_to_events`; that index now lives here, so the pin does too.
    #[test]
    fn events_for_verse_resolves_a_witness_only_verse_not_in_the_top_level_verses_field() {
        let src = fixture_source();
        assert_eq!(src.events_for_verse("GEN.1.3"), &["e1".to_string()], "a verse cited only by a WITNESS must still resolve to its event");
    }

    #[test]
    fn events_for_verse_still_resolves_the_top_level_verses_field_too() {
        let src = fixture_source();
        assert_eq!(src.events_for_verse("GEN.1.1"), &["e1".to_string()]);
    }

    /// The union is DEDUPED per event: a verse present in both an event's
    /// own `verses` and one of its witnesses yields that event id once, not
    /// twice (`bibex verse` would otherwise print the passage twice).
    #[test]
    fn events_for_verse_never_double_lists_an_event_for_one_verse() {
        let src = fixture_source();
        assert_eq!(src.events_for_verse("GEN.1.2"), &["e1".to_string()], "GEN.1.2 is in BOTH the event's own verses and its witness's");
    }

    #[test]
    fn events_for_verse_is_empty_for_a_verse_in_no_event() {
        let src = fixture_source();
        assert!(src.events_for_verse("GEN.2.1").is_empty());
    }

    /// Batch R requirement 5's own pin, likewise re-homed: `places_for_verse`
    /// is the reverse of `Place::verse_links`, and it is what
    /// `handlers::chapter` populates each `VerseOut.places` from.
    #[test]
    fn places_for_verse_resolves_the_reverse_of_verse_links() {
        let src = fixture_source();
        assert_eq!(src.places_for_verse("GEN.2.1"), &["p1".to_string()]);
    }

    #[test]
    fn places_for_verse_is_empty_for_an_unlinked_verse() {
        let src = fixture_source();
        assert!(src.places_for_verse("GEN.1.1").is_empty());
    }
}
