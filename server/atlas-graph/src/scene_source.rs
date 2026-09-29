//! `GraphSceneSource`: the map scene's data, materialised once from the graph port. It carries no
//! verse text and none of `AtlasData`'s derived index web, only the derivations its readers ask for,
//! and its construction sequence must stay `AtlasData::finish()`'s, step for step.

use std::collections::{BTreeMap, HashMap, HashSet};

use atlas_core::data::{Event, Narrative, Place, PlaceHistory, PlaceNameAlias};
use atlas_core::refs::{ScriptureRef, VerseId};
use atlas_core::scene_source::SceneSource;
use atlas_core::time::TimeRange;
use atlas_graph_types::store::GraphQuery;

use crate::event_world::ChronologyDerivation;
use crate::service::GraphService;

/// Deliberately neither `Clone` -- exactly one such copy exists per process -- nor `Debug`, since a
/// derived `Debug` over these collections is megabytes of output that would be reached by accident.
pub struct GraphSceneSource {
    /// Post-merge, post-sort: the exact order `AtlasData::finish()` produces, which is the order the
    /// pinned scene responses were captured against.
    events: Vec<Event>,
    /// Post-place-merge -- absorbed ids removed, merged ids folded onto their survivor -- in node id
    /// order otherwise.
    places: Vec<Place>,
    /// Post-event-merge: a leg naming an absorbed event is repointed at its survivor.
    narratives: Vec<Narrative>,
    event_index: HashMap<String, usize>,
    place_index: HashMap<String, usize>,
    /// Every event's EVERY place counted once per event, not just the anchor: the all-time count a
    /// quiet place carries.
    event_counts_by_place: HashMap<String, u32>,
    /// Literally the key set of `event_counts_by_place`.
    event_bearing_place_ids: HashSet<String>,
    /// verse dot-ref -> every event id touching that verse: each event's OWN `verses` unioned with
    /// every witness's default-translation verses, deduped per event so a verse in both yields the id
    /// once. Built by iterating `events` in order, so a verse's list is in post-sort event order.
    verse_to_events: HashMap<String, Vec<String>>,
    /// verse dot-ref -> every place id whose curated `verse_links` names it, built by iterating
    /// `places` in order, so a verse's list is in `places`-vec order.
    verse_to_places: HashMap<String, Vec<String>>,
    /// Sidecar-only: curated JSON, not a graph node, copied at construction.
    place_history: HashMap<String, PlaceHistory>,
    place_name_aliases: HashMap<String, Vec<PlaceNameAlias>>,
}

impl GraphSceneSource {
    /// The source a finished service serves from.
    pub fn build(gs: &GraphService, sidecars: &atlas_core::data::AtlasData) -> Self {
        Self::over(&gs.snapshot(), &gs.chronology.chrono, &gs.narrative_legs, sidecars)
    }

    /// Over any port handle: a finished service's snapshot, or the compiler's in-progress graph once
    /// its indexes are built, so a map composed at compile time reads exactly what a request reads.
    /// Only `sidecars`' curated-JSON maps are read, so a bare, un-`finish()`ed `AtlasData` is safe and
    /// intended here: none of its graph-derived fields is touched. The sequence below is
    /// `AtlasData::finish()`'s, step for step, and must stay that way.
    pub fn over(
        q: &impl GraphQuery,
        chrono: &ChronologyDerivation,
        narrative_legs: &BTreeMap<String, Vec<String>>,
        sidecars: &atlas_core::data::AtlasData,
    ) -> Self {
        // Materialised in node id order, which is the pre-sort insertion order the stable sort below
        // preserves for every `from_year` tie, so it is load-bearing.
        use atlas_graph_types::id::NodeKind;
        use crate::service::ids_of_kind;
        let mut events: Vec<Event> = ids_of_kind(q, NodeKind::Event).iter().filter_map(|id| crate::legacy::event_from_node(id, q, chrono)).collect();
        let mut places: Vec<Place> = ids_of_kind(q, NodeKind::Place).iter().filter_map(|id| crate::legacy::place_from_node(id, q)).collect();
        let empty_legs: Vec<String> = Vec::new();
        let mut narratives: Vec<Narrative> = ids_of_kind(q, NodeKind::Narrative)
            .iter()
            .filter_map(|id| crate::legacy::narrative_from_node(id, q, narrative_legs.get(&id.raw).unwrap_or(&empty_legs)))
            .collect();

        // The curated same-place dedupe comes FIRST: it rewrites `events[*].places` as well as
        // removing absorbed places, so every derivation below must follow it.
        atlas_core::merge::apply_place_merges(&mut places, &mut events);

        // The curated duplicate-event rectification comes before the sort: it REMOVES events, so doing
        // it after would leave a different surviving order.
        atlas_core::event_merge::apply_event_merges(&mut events, &mut narratives);

        // A total key, `(from_year, id)`. The order is unchanged by construction: the input was id
        // ordered and the sort was stable, so ties already fell to id -- the key now says so.
        events.sort_by(|a, b| (a.when.from_year, &a.id).cmp(&(b.when.from_year, &b.id)));

        let place_index: HashMap<String, usize> = places.iter().enumerate().map(|(i, p)| (p.id.clone(), i)).collect();
        let event_index: HashMap<String, usize> = events.iter().enumerate().map(|(i, e)| (e.id.clone(), i)).collect();

        // Every place of every event, not only the anchor. Counting after the place merge is what
        // makes a survivor carry the union of both records' events.
        let mut event_counts_by_place: HashMap<String, u32> = HashMap::new();
        for e in &events {
            for pid in &e.places {
                *event_counts_by_place.entry(pid.clone()).or_insert(0) += 1;
            }
        }
        let event_bearing_place_ids: HashSet<String> = event_counts_by_place.keys().cloned().collect();

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

    /// Every event id touching `verse`, in post-sort event order; an empty slice when the verse is in
    /// no event. Inherent rather than a `SceneSource` method: the composer never asks this.
    pub fn events_for_verse(&self, verse: &str) -> &[String] {
        self.verse_to_events.get(verse).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Every place id whose curated `verse_links` names `verse`, in `places`-vec order. Inherent, for
    /// the same reason as `events_for_verse`.
    pub fn places_for_verse(&self, verse: &str) -> &[String] {
        self.verse_to_places.get(verse).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// `place_by_id` without the `SceneSource` trait in scope, for the callers that hold this object
    /// for its verse-web indexes and need the `Place` record too.
    pub fn place(&self, id: &str) -> Option<&Place> {
        self.place_index.get(id).map(|&i| &self.places[i])
    }

    pub fn event(&self, id: &str) -> Option<&Event> {
        self.event_index.get(id).map(|&i| &self.events[i])
    }

    pub fn narrative_list(&self) -> &[Narrative] {
        &self.narratives
    }
}

/// Every method mirrors the exact `AtlasData` read the composer makes.
impl SceneSource for GraphSceneSource {
    /// Order is the `events` vec's own order, which is the post-sort order.
    fn events_in_window(&self, w: &TimeRange) -> Vec<&Event> {
        self.events.iter().filter(|e| e.when.intersects(w)).collect()
    }

    /// An event matches when any of its OWN `verses` falls inside the ref, never its witnesses'. That
    /// widening belongs to `events_for_verse`, which the composer does not read: keeping the two apart
    /// is what lets a scripture scene and a verse popover legitimately differ.
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

    fn place_name_alias_for(&self, id: &str) -> Option<&PlaceNameAlias> {
        self.place_name_aliases.get(id).and_then(|v| v.first())
    }

    fn event_bearing_place_ids(&self) -> &HashSet<String> {
        &self.event_bearing_place_ids
    }

    /// 0 for a place with no events, the honest default.
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

    fn fixture_atlas() -> AtlasData {
        let places = vec![Place { id: "p1".into(), name: "P1".into(), lat: 1.0, lon: 2.0, verse_links: vec!["GEN.2.1".into()] }];
        let events = vec![Event {
            id: "e1".into(),
            label: "A container over GEN.1".into(),
            when: TimeRange::new(-4004, -4004).unwrap(),
            places: vec!["p1".into()],
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
