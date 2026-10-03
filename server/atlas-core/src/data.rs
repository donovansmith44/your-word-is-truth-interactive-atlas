
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::time::{TimeRange, Year};

pub use atlas_graph_types::id::{EventId, PersonId};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(description = "One book of the canon: its code, its name, and how many verses each of its chapters holds.")]
pub struct CanonBook {
    pub code: String,
    pub name: String,
    pub chapters: Vec<u16>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Canon {
    pub books: Vec<CanonBook>,
}

impl Canon {
    pub fn verses_in(&self, book: crate::refs::BookId, chapter: u16) -> Option<u16> {
        let chapter_index = usize::from(chapter).checked_sub(1)?;
        self.books.iter().find(|b| b.code == book.code())?.chapters.get(chapter_index).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub verse_links: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub id: String,
    pub name: String,
    pub gender: Option<String>,
    pub birth_year: Option<i32>,
    pub death_year: Option<i32>,
    pub also_called: Vec<String>,
    pub verse_links: Vec<String>,
    #[serde(default)]
    pub father: Vec<String>,
    #[serde(default)]
    pub mother: Vec<String>,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub spouses: Vec<String>,
    #[serde(default)]
    pub first_year: Option<i32>,
    #[serde(default)]
    pub last_year: Option<i32>,
    #[serde(default)]
    pub timeline: Vec<String>,
    #[serde(default)]
    pub eternal: bool,
    #[serde(default)]
    pub eternal_grounds: Vec<String>,
    pub dict_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EastonEntry {
    pub dict_lookup: String,
    pub dict_text: String,
    pub match_type: String,
    pub match_slugs: String,
    pub person_slug: Option<String>,
    pub place_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeopleGroup {
    pub id: String,
    pub label: String,
    pub verse_links: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeopleGroupSeed {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeopleGroupReclassify {
    pub person_slug: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParentageExclusion {
    pub parent: String,
    pub child: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrethrenSeed {
    pub a: String,
    pub b: String,
    pub justification: atlas_graph_types::edge::Justification,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParentageSeed {
    pub parent: String,
    pub child: String,
    pub parentage: atlas_graph_types::edge::Parentage,
    pub justification: atlas_graph_types::edge::Justification,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptureGroundSeed {
    pub from: String,
    #[serde(default)]
    pub to: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedAfterSeed {
    pub namesake_kind: String,
    pub namesake_id: String,
    pub eponym: String,
    #[serde(default)]
    pub text: Option<String>,
    pub grounds: Vec<ScriptureGroundSeed>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FulfillmentSeed {
    pub prophecy: ScriptureGroundSeed,
    pub fulfillment: ScriptureGroundSeed,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypologySeed {
    pub type_passage: ScriptureGroundSeed,
    pub antitype_passage: ScriptureGroundSeed,
    pub note: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventMentionSeed {
    pub event_id: String,
    pub verses: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventAnalogueSeed {
    pub a: String,
    pub b: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub label: String,
    pub when: TimeRange,
    pub places: Vec<String>,
    pub verses: Vec<String>,
    #[serde(default = "default_event_kind")]
    pub kind: EventKind,
    #[serde(default)]
    pub witnesses: Vec<EventWitness>,
    #[serde(default)]
    pub robertson_section: Option<String>,
    #[serde(default)]
    pub acts_section: Option<String>,
    #[serde(default)]
    pub atlas_section: Option<String>,
    #[serde(default)]
    pub kjv_superscription: Option<String>,
    #[serde(default)]
    pub ref_note: Option<String>,
    #[serde(default)]
    pub order_key: i32,
}

impl Event {
    pub fn date(&self) -> Option<TimeRange> {
        match self.kind {
            EventKind::Event => Some(self.when),
            EventKind::General => None,
        }
    }
}

atlas_graph_types::vocabulary! {
    EventKind {
        Event => "event",
        General => "general",
    }
}

fn default_event_kind() -> EventKind {
    EventKind::Event
}

impl Default for Event {
    fn default() -> Self {
        Event {
            id: String::new(),
            label: String::new(),
            when: TimeRange { from_year: 1, to_year: 1 },
            places: Vec::new(),
            verses: Vec::new(),
            kind: default_event_kind(),
            witnesses: Vec::new(),
            robertson_section: None,
            acts_section: None,
            atlas_section: None,
            kjv_superscription: None,
            ref_note: None,
            order_key: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventWitness {
    pub book: String,
    pub translations: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub ref_note: Option<String>,
    #[serde(default)]
    pub robertson_section: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(description = "A journey or storyline through the atlas: an ordered chain of events the map draws as a run of arrows.")]
pub struct Narrative {
    pub id: String,
    pub name: String,
    pub color: String,
    pub legs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era {
    pub id: String,
    pub name: String,
    pub from_year: Year,
    pub to_year: Year,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(description = "Who wrote one book of the Bible, where, and when.")]
pub struct BookMeta {
    #[serde(skip_serializing)]
    pub book: String,
    pub author: String,
    pub write_place: Option<String>,
    pub write_from: Option<i32>,
    pub write_to: Option<i32>,
}

impl BookMeta {
    pub fn written(&self) -> Result<Option<TimeRange>, crate::CoreError> {
        match (self.write_from, self.write_to) {
            (Some(from), Some(to)) => TimeRange::new(from, to).map(Some),
            (None, None) => Ok(None),
            _ => Err(crate::CoreError::OneEndedSpan),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BookAuthorship {
    pub book: String,
    pub author_ids: Vec<PersonId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossRef {
    pub target: String,
    pub votes: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(description = "An always-on map label: a water, a mountain or a region, drawn at one point and never interactive.")]
pub struct Landmark {
    pub name: String,
    pub kind: LandmarkKind,
    pub lat: f64,
    pub lon: f64,
    #[serde(default)]
    pub size: Option<LandmarkSize>,
}

atlas_graph_types::vocabulary! {
    LandmarkKind {
        Water => "water",
        Mountain => "mountain",
        Region => "region",
    }
}

atlas_graph_types::vocabulary! {
    LandmarkSize {
        Small => "sm",
        Medium => "md",
        Large => "lg",
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChronologyAnchor {
    pub id: String,
    pub label: String,
    pub year: Year,
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub era_boundary: bool,
    pub source: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BookNarrationWindow {
    pub book: String,
    pub from_year: Year,
    pub to_year: Year,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceNameEntry {
    pub name: String,
    pub when: TimeRange,
    pub verses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(description = "A date this atlas claims for a place, with the verses it rests on.")]
pub struct PlaceDateClaim {
    pub when: TimeRange,
    pub verses: Vec<String>,
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<EventId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceHistory {
    pub id: String,
    #[serde(default)]
    pub names: Vec<PlaceNameEntry>,
    pub established: Option<PlaceDateClaim>,
    pub destroyed: Option<PlaceDateClaim>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceNameAlias {
    pub id: String,
    pub translations: HashMap<String, String>,
    pub verses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolityEra {
    pub name: String,
    pub from: Year,
    pub to: Year,
    pub ref_note: String,
    pub rings: Vec<Vec<(f64, f64)>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<PolityDelta>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fall: Option<PolityDelta>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(description = "The event at one boundary of a polity's era: its rise, a change of its borders, or its fall.")]
pub struct PolityDelta {
    pub event: String,
    #[serde(default)]
    #[schema(required = true)]
    pub verses: Vec<crate::refs::VerseId>,
    pub ref_note: String,
    #[serde(skip_serializing)]
    pub for_era_from: Year,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandMaskRegion {
    pub name: String,
    pub ref_note: String,
    pub rings: Vec<Vec<(f64, f64)>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatechismItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default = "default_explanation_heading")]
    pub explanation_heading: String,
    pub explanation: String,
    #[serde(default)]
    pub where_written: Option<String>,
    #[serde(default)]
    pub verses: Vec<String>,
    #[serde(default)]
    pub ref_note: Option<String>,
    #[serde(default)]
    pub questions: Vec<CatechismQuestion>,
}

fn default_explanation_heading() -> String {
    "What does this mean?".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatechismQuestion {
    pub title: String,
    pub verses: Vec<String>,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatechismPart {
    pub id: String,
    pub title: String,
    pub items: Vec<CatechismItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polity {
    pub id: String,
    pub color_key: u8,
    pub eras: Vec<PolityEra>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AtlasData {
    pub canon: Canon,
    pub places: Vec<Place>,
    pub events: Vec<Event>,
    pub narratives: Vec<Narrative>,
    pub eras: Vec<Era>,
    pub books_meta: Vec<BookMeta>,
    pub verses: HashMap<String, String>,
    pub cross_refs: HashMap<String, Vec<CrossRef>>,

    #[serde(skip)]
    pub polities: Vec<Polity>,
    #[serde(skip)]
    pub landmarks: Vec<Landmark>,
    #[serde(skip)]
    pub place_history: HashMap<String, PlaceHistory>,

    #[serde(skip)]
    pub place_name_aliases: HashMap<String, Vec<PlaceNameAlias>>,

    #[serde(skip)]
    pub land_mask: Vec<Vec<(f64, f64)>>,

    #[serde(skip)]
    pub catechism: Vec<CatechismPart>,

    #[serde(skip)]
    pub chronology_anchors: Vec<ChronologyAnchor>,
    #[serde(skip)]
    pub book_narration_windows: Vec<BookNarrationWindow>,

    #[serde(skip)]
    pub people: Vec<Person>,

    #[serde(skip)]
    pub book_authorship: Vec<BookAuthorship>,

    #[serde(skip)]
    pub easton: Vec<EastonEntry>,

    #[serde(skip)]
    pub people_groups: Vec<PeopleGroup>,
    #[serde(skip)]
    pub people_group_seeds: Vec<PeopleGroupSeed>,
    #[serde(skip)]
    pub people_group_reclassify: Vec<PeopleGroupReclassify>,
    #[serde(skip)]
    pub named_after_seeds: Vec<NamedAfterSeed>,
    #[serde(skip)]
    pub fulfillment_seeds: Vec<FulfillmentSeed>,
    #[serde(skip)]
    pub typology_seeds: Vec<TypologySeed>,
    #[serde(skip)]
    pub parentage_seeds: Vec<ParentageSeed>,
    #[serde(skip)]
    pub parentage_exclusions: Vec<ParentageExclusion>,
    #[serde(skip)]
    pub brethren_seeds: Vec<BrethrenSeed>,
    #[serde(skip)]
    pub event_mentions: Vec<EventMentionSeed>,
    #[serde(skip)]
    pub event_analogues: Vec<EventAnalogueSeed>,
    #[serde(skip)]
    pub provenance_titles: crate::sources::ProvenanceTitles,

    #[serde(skip)]
    place_index: HashMap<String, usize>,
    #[serde(skip)]
    event_index: HashMap<String, usize>,
    #[serde(skip)]
    event_bearing_place_ids: HashSet<String>,
    #[serde(skip)]
    event_counts_by_place: HashMap<String, u32>,

    #[serde(skip)]
    verse_to_catechism: HashMap<String, Vec<(String, Option<String>)>>,
    #[serde(skip)]
    catechism_item_names: HashMap<String, String>,
    #[serde(skip)]
    catechism_item_index: HashMap<String, (usize, usize)>,

    #[serde(skip)]
    verse_heading: HashMap<String, HeadingEntry>,

    #[serde(skip)]
    heading_anchor_collisions: Vec<(String, String, String)>,

    #[serde(skip)]
    timeline_order: Vec<String>,
    #[serde(skip)]
    timeline_index: HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadingEntry {
    pub event_id: String,
    pub title: String,
    pub kind: EventKind,
}

fn heading_anchors_for(e: &Event) -> Vec<String> {
    if !e.witnesses.is_empty() {
        return e
            .witnesses
            .iter()
            .filter_map(|w| w.translations.get(crate::translation::DEFAULT_TRANSLATION))
            .filter_map(|verses| {
                verses
                    .iter()
                    .filter_map(|v| crate::refs::VerseId::parse_canonical(v).ok().map(|vid| ((vid.book.0, vid.chapter, vid.verse), v)))
                    .min_by_key(|(key, _)| *key)
                    .map(|(_, v)| v.clone())
            })
            .collect();
    }

    let mut seen_books: Vec<String> = Vec::new();
    let mut best: std::collections::HashMap<String, (u16, u16, String)> = std::collections::HashMap::new();
    for v in &e.verses {
        let Ok(vid) = crate::refs::VerseId::parse_canonical(v) else { continue };
        let book = vid.book.code().to_string();
        match best.get(&book) {
            None => {
                seen_books.push(book.clone());
                best.insert(book, (vid.chapter, vid.verse, v.clone()));
            }
            Some((c, ve, _)) if (vid.chapter, vid.verse) < (*c, *ve) => {
                best.insert(book, (vid.chapter, vid.verse, v.clone()));
            }
            Some(_) => {}
        }
    }
    seen_books.into_iter().filter_map(|b| best.remove(&b).map(|(_, _, v)| v)).collect()
}

fn heading_precedence(e: &Event) -> (u8, u8, std::cmp::Reverse<i32>, std::cmp::Reverse<i32>) {
    let layer: u8 = if !e.witnesses.is_empty()
        || e.robertson_section.is_some()
        || e.acts_section.is_some()
        || e.atlas_section.is_some()
        || e.kjv_superscription.is_some()
    {
        1
    } else {
        0
    };
    let kind: u8 = if e.kind == EventKind::Event { 1 } else { 0 };
    (layer, kind, std::cmp::Reverse(e.when.from_year), std::cmp::Reverse(e.order_key))
}

impl AtlasData {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        canon: Canon,
        places: Vec<Place>,
        events: Vec<Event>,
        narratives: Vec<Narrative>,
        eras: Vec<Era>,
        books_meta: Vec<BookMeta>,
        verses: HashMap<String, String>,
        cross_refs: HashMap<String, Vec<CrossRef>>,
    ) -> Self {
        Self {
            canon,
            places,
            events,
            narratives,
            eras,
            books_meta,
            verses,
            cross_refs,
            ..Default::default()
        }
    }

    pub fn finish(mut self) -> Self {
        // Both merges run before any derived index below is built, so every index comes
        // out already reflecting the merged graph, with no fixup pass.
        crate::merge::apply_place_merges(&mut self.places, &mut self.events);

        crate::event_merge::apply_event_merges(&mut self.events, &mut self.narratives);

        self.events.sort_by_key(|e| e.when.from_year);

        self.place_index = self
            .places
            .iter()
            .enumerate()
            .map(|(i, p)| (p.id.clone(), i))
            .collect();
        self.event_index = self
            .events
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id.clone(), i))
            .collect();

        let mut event_counts_by_place: HashMap<String, u32> = HashMap::new();
        for e in &self.events {
            for pid in &e.places {
                *event_counts_by_place.entry(pid.clone()).or_insert(0) += 1;
            }
        }
        self.event_bearing_place_ids = event_counts_by_place.keys().cloned().collect();
        self.event_counts_by_place = event_counts_by_place;

        let mut verse_to_catechism: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
        let mut catechism_item_names: HashMap<String, String> = HashMap::new();
        let mut catechism_item_index: HashMap<String, (usize, usize)> = HashMap::new();
        for (pi, part) in self.catechism.iter().enumerate() {
            for (ii, item) in part.items.iter().enumerate() {
                catechism_item_names.insert(item.id.clone(), item.name.clone());
                catechism_item_index.insert(item.id.clone(), (pi, ii));
                for v in &item.verses {
                    verse_to_catechism.entry(v.clone()).or_default().push((item.id.clone(), None));
                }
                for q in &item.questions {
                    for v in &q.verses {
                        verse_to_catechism.entry(v.clone()).or_default().push((item.id.clone(), Some(q.title.clone())));
                    }
                }
            }
        }
        self.verse_to_catechism = verse_to_catechism;
        self.catechism_item_names = catechism_item_names;
        self.catechism_item_index = catechism_item_index;

        // Two containers covering one verse is expected; the READER is what must be
        // decisive, so exactly one title heads a verse, chosen by `heading_precedence` and
        // never by iteration order. The non-chosen container stays reachable elsewhere.
        let narrative_leg_ids: HashSet<&str> =
            self.narratives.iter().flat_map(|n| n.legs.iter().map(|s| s.as_str())).collect();
        let mut verse_heading: HashMap<String, HeadingEntry> = HashMap::new();
        let mut verse_heading_precedence: HashMap<String, (u8, u8, std::cmp::Reverse<i32>, std::cmp::Reverse<i32>)> = HashMap::new();
        let mut real_anchor_owner: HashMap<String, String> = HashMap::new();
        let mut heading_anchor_collisions: Vec<(String, String, String)> = Vec::new();
        for e in &self.events {
            let heading_worthy = narrative_leg_ids.contains(e.id.as_str())
                || !e.witnesses.is_empty()
                || e.robertson_section.is_some()
                || e.acts_section.is_some()
                || e.atlas_section.is_some()
                || e.kjv_superscription.is_some();
            if !heading_worthy {
                continue;
            }
            let precedence = heading_precedence(e);
            let is_real_container = precedence.0 == 1;
            for anchor in heading_anchors_for(e) {
                if is_real_container {
                    match real_anchor_owner.get(&anchor) {
                        Some(owner) if owner != &e.id => {
                            heading_anchor_collisions.push((anchor.clone(), owner.clone(), e.id.clone()));
                        }
                        Some(_) => {}
                        None => {
                            real_anchor_owner.insert(anchor.clone(), e.id.clone());
                        }
                    }
                }
                let should_replace = match verse_heading_precedence.get(&anchor) {
                    None => true,
                    Some(&incumbent) => precedence > incumbent,
                };
                if should_replace {
                    verse_heading_precedence.insert(anchor.clone(), precedence);
                    verse_heading.insert(anchor, HeadingEntry { event_id: e.id.clone(), title: e.label.clone(), kind: e.kind });
                }
            }
        }
        self.verse_heading = verse_heading;
        self.heading_anchor_collisions = heading_anchor_collisions;

        let mut timeline_order: Vec<String> =
            self.events.iter().filter(|e| e.kind == EventKind::Event).map(|e| e.id.clone()).collect();
        timeline_order.sort_by_key(|id| {
            let e = self.event_by_id(id).expect("id just collected from self.events");
            (e.when.from_year, e.order_key)
        });
        self.timeline_index = timeline_order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect();
        self.timeline_order = timeline_order;

        self
    }

    pub fn event_by_id(&self, id: &str) -> Option<&Event> {
        self.event_index.get(id).map(|&i| &self.events[i])
    }

    pub fn place_by_id(&self, id: &str) -> Option<&Place> {
        self.place_index.get(id).map(|&i| &self.places[i])
    }

    pub fn place_history_for(&self, id: &str) -> Option<&PlaceHistory> {
        self.place_history.get(id)
    }

    pub fn place_name_alias_for(&self, id: &str) -> Option<&PlaceNameAlias> {
        self.place_name_aliases.get(id).and_then(|v| v.first())
    }

    pub fn place_name_aliases_for(&self, id: &str) -> &[PlaceNameAlias] {
        self.place_name_aliases.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn event_bearing_place_ids(&self) -> &HashSet<String> {
        &self.event_bearing_place_ids
    }

    pub fn total_events_for(&self, id: &str) -> u32 {
        self.event_counts_by_place.get(id).copied().unwrap_or(0)
    }

    pub fn heading_anchor_collisions(&self) -> &[(String, String, String)] {
        &self.heading_anchor_collisions
    }

    pub fn heading_for_verse(&self, verse: &str) -> Option<&HeadingEntry> {
        self.verse_heading.get(verse)
    }

    pub fn timeline_position(&self, id: &str) -> Option<usize> {
        self.timeline_index.get(id).copied()
    }

    pub fn timeline_event_at(&self, index: usize) -> Option<&Event> {
        self.timeline_order.get(index).and_then(|id| self.event_by_id(id))
    }

    pub fn catechism_items_for_span(&self, span: &crate::refs::ScriptureRef) -> Vec<crate::catechism::CatechismRef> {
        crate::catechism::items_for_span(span, &self.verse_to_catechism, &self.catechism_item_names)
    }

    pub fn catechism_item_by_id(&self, id: &str) -> Option<(&CatechismPart, &CatechismItem)> {
        let &(pi, ii) = self.catechism_item_index.get(id)?;
        Some((&self.catechism[pi], &self.catechism[pi].items[ii]))
    }
}

impl crate::scene_source::SceneSource for AtlasData {
    fn events_in_window(&self, w: &TimeRange) -> Vec<&Event> {
        self.events.iter().filter(|e| e.when.intersects(w)).collect()
    }

    fn events_matching_ref(&self, r: &crate::refs::ScriptureRef) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| {
                e.verses
                    .iter()
                    .any(|v| crate::scene::ref_contains(r, &crate::refs::VerseId::parse_canonical(v).expect("etl-validated verse id")))
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
        AtlasData::event_by_id(self, id)
    }

    fn place_by_id(&self, id: &str) -> Option<&Place> {
        AtlasData::place_by_id(self, id)
    }

    fn place_history_for(&self, id: &str) -> Option<&PlaceHistory> {
        AtlasData::place_history_for(self, id)
    }

    fn place_name_alias_for(&self, id: &str) -> Option<&PlaceNameAlias> {
        AtlasData::place_name_alias_for(self, id)
    }

    fn event_bearing_place_ids(&self) -> &HashSet<String> {
        AtlasData::event_bearing_place_ids(self)
    }

    fn total_events_for(&self, id: &str) -> u32 {
        AtlasData::total_events_for(self, id)
    }

    fn place_node(&self, id: &str) -> crate::wire::NodeRef {
        let label = AtlasData::place_by_id(self, id).map(|place| place.name.clone()).unwrap_or_default();
        crate::wire::NodeRef { id: crate::identity::NodeId::of_place(&atlas_graph_types::id::PlaceId::new(id)), kind: atlas_graph_types::id::NodeKind::Place, label }
    }
}

pub(crate) fn year_index(y: Year) -> i64 {
    if y > 0 {
        (y - 1) as i64
    } else {
        y as i64
    }
}

#[doc(hidden)]
pub fn demo_fixture() -> AtlasData {
    let places = vec![
        Place { id: "gilgal".into(), name: "Gilgal".into(), lat: 31.9000, lon: 35.4500, verse_links: vec![] },
        Place { id: "jericho".into(), name: "Jericho".into(), lat: 31.8703, lon: 35.4436, verse_links: vec![] },
        Place { id: "ai".into(), name: "Ai".into(), lat: 31.9339, lon: 35.2856, verse_links: vec![] },
        Place {
            id: "hebron".into(),
            name: "Hebron".into(),
            lat: 31.5326,
            lon: 35.0998,
            verse_links: vec!["GEN.13.18".into()],
        },
    ];
    let events = vec![
        Event {
            id: "e1".into(),
            label: "Camp at Gilgal".into(),
            when: TimeRange::new(-1406, -1406).unwrap(),
            places: vec!["gilgal".into()],
            verses: vec!["JOS.4.19".into(), "JOS.4.20".into()],
            ..Default::default()
        },
        Event {
            id: "e2".into(),
            label: "Jericho besieged".into(),
            when: TimeRange::new(-1406, -1406).unwrap(),
            places: vec!["jericho".into()],
            verses: vec!["JOS.6.1".into(), "JOS.6.2".into()],
            ..Default::default()
        },
        Event {
            id: "e3".into(),
            label: "Jericho falls".into(),
            when: TimeRange::new(-1405, -1405).unwrap(),
            places: vec!["jericho".into()],
            verses: vec!["JOS.6.20".into(), "JOS.6.21".into(), "JOS.6.24".into()],
            ..Default::default()
        },
        Event {
            id: "e4".into(),
            label: "Ai defeated".into(),
            when: TimeRange::new(-1405, -1405).unwrap(),
            places: vec!["ai".into()],
            verses: vec!["JOS.8.1".into(), "JOS.8.28".into()],
            ..Default::default()
        },
        Event {
            id: "e5".into(),
            label: "Sarah buried at Machpelah".into(),
            when: TimeRange::new(-2000, -2000).unwrap(),
            places: vec!["hebron".into()],
            verses: vec!["GEN.23.1".into(), "GEN.23.19".into()],
            ..Default::default()
        },
    ];
    let narratives = vec![
        Narrative {
            id: "conquest".into(),
            name: "The Conquest".into(),
            color: "#7C3AED".into(),
            legs: vec!["e1".into(), "e2".into(), "e3".into(), "e4".into()],
        },
        Narrative {
            id: "patriarchs-demo".into(),
            name: "Patriarchs (demo)".into(),
            color: "#D97706".into(),
            legs: vec!["e2".into()],
        },
    ];

    let canon = Canon {
        books: vec![
            CanonBook { code: "GEN".into(), name: "Genesis".into(), chapters: vec![31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 19] },
            CanonBook { code: "JOS".into(), name: "Joshua".into(), chapters: vec![3, 0, 0, 20, 0, 24, 0, 28] },
        ],
    };
    let eras = vec![
        Era { id: "patriarchs".into(), name: "Patriarchs".into(), from_year: -2166, to_year: -1877 },
        Era { id: "conquest-judges".into(), name: "Conquest & Judges".into(), from_year: -1406, to_year: -1051 },
    ];
    let books_meta = vec![
        BookMeta {
            book: "GEN".into(),
            author: "Moses".into(),
            write_place: None,
            write_from: Some(-1445),
            write_to: Some(-1405),
        },
        BookMeta {
            book: "JOS".into(),
            author: "Joshua".into(),
            write_place: Some("gilgal".into()),
            write_from: Some(-1400),
            write_to: Some(-1370),
        },
    ];

    let mut verses = HashMap::new();
    verses.insert("JOS.1.1".into(), "Now after the death of Moses it came to pass, that the LORD spake unto Joshua.".to_string());
    verses.insert("JOS.1.2".into(), "Moses my servant is dead; now therefore arise, go over this Jordan.".to_string());
    verses.insert("JOS.1.3".into(), "Every place that the sole of your foot shall tread upon, that have I given unto you.".to_string());
    verses.insert("JOS.4.19".into(), "And the people came up out of Jordan, and encamped in Gilgal.".to_string());
    verses.insert("JOS.4.20".into(), "And those twelve stones did Joshua pitch in Gilgal.".to_string());
    verses.insert("JOS.6.1".into(), "Now Jericho was straitly shut up because of the children of Israel.".to_string());
    verses.insert("JOS.6.2".into(), "And the LORD said unto Joshua, See, I have given into thine hand Jericho.".to_string());
    verses.insert(
        "JOS.6.20".into(),
        "So the people shouted, and the wall fell down flat, and they took the city.".to_string(),
    );
    verses.insert("JOS.6.21".into(), "And they utterly destroyed all that was in the city.".to_string());
    verses.insert("JOS.6.24".into(), "And they burnt the city with fire, and all that was therein.".to_string());
    verses.insert("JOS.8.1".into(), "And the LORD said unto Joshua, Fear not, neither be thou dismayed.".to_string());
    verses.insert("JOS.8.28".into(), "And Joshua burnt Ai, and made it an heap for ever.".to_string());
    verses.insert(
        "GEN.13.18".into(),
        "Then Abram removed his tent, and came and dwelt in the plain of Mamre, which is in Hebron.".to_string(),
    );
    verses.insert("GEN.23.1".into(), "And Sarah was an hundred and seven and twenty years old.".to_string());
    verses.insert(
        "GEN.23.19".into(),
        "And after this, Abraham buried Sarah his wife in the cave of the field of Machpelah.".to_string(),
    );

    let mut cross_refs = HashMap::new();
    cross_refs.insert(
        "JOS.6.20".into(),
        vec![
            CrossRef { target: "JOS.6.20-21".into(), votes: 9 },
            CrossRef { target: "JOS.1.3".into(), votes: 5 },
            CrossRef { target: "GEN.13.18".into(), votes: 2 },
        ],
    );
    cross_refs.insert(
        "JOS.6.21".into(),
        vec![
            CrossRef { target: "JOS.1.3".into(), votes: 4 },
            CrossRef { target: "JOS.6.20".into(), votes: 3 },
        ],
    );

    let mut data = AtlasData::new(canon, places, events, narratives, eras, books_meta, verses, cross_refs).finish();
    data.place_history.insert(
        "hebron".into(),
        PlaceHistory {
            id: "hebron".into(),
            names: vec![
                PlaceNameEntry { name: "Kirjath-arba".into(), when: TimeRange::new(-4004, -2001).unwrap(), verses: vec!["GEN.23.2".into()] },
            ],
            established: Some(PlaceDateClaim {
                when: TimeRange::new(-2000, -2000).unwrap(),
                verses: vec!["GEN.23.19".into()],
                note: Some("traditional".into()),
                event: None,
            }),
            destroyed: None,
        },
    );

    data.catechism = vec![CatechismPart {
        id: "demo-part".into(),
        title: "Demo Part".into(),
        items: vec![CatechismItem {
            id: "demo-item-1".into(),
            name: "Demo Catechism Item".into(),
            text: None,
            explanation_heading: "What does this mean?".into(),
            explanation: "Demo item explanation.".into(),
            where_written: Some("Demo where-written text.".into()),
            verses: vec!["JOS.6.20".into()],
            ref_note: None,
            questions: vec![CatechismQuestion {
                title: "Demo Question".into(),
                verses: vec!["JOS.6.21".into()],
                source: "brain-fuel/catechism".into(),
            }],
        }],
    }];
    data.finish()
}

#[cfg(test)]
mod heading_collision_tests {
    use super::*;
    use std::collections::HashMap;

    fn bare_leg() -> Event {
        Event {
            id: "bare_leg".into(),
            label: "A bare narrative-leg-only event".into(),
            when: crate::time::TimeRange::new(33, 33).unwrap(),
            verses: vec!["JHN.12.1".into()],
            ..Default::default()
        }
    }

    fn rich_leg() -> Event {
        Event {
            id: "rich_leg".into(),
            label: "A richer, witness-bearing event".into(),
            when: crate::time::TimeRange::new(33, 33).unwrap(),
            verses: vec!["JHN.12.1".into()],
            witnesses: vec![EventWitness {
                book: "JHN".into(),
                translations: HashMap::from([("kjv".to_string(), vec!["JHN.12.1".to_string(), "JHN.12.2".to_string()])]),
                ref_note: None,
                robertson_section: Some("Robertson (1922) fixture section".into()),
            }],
            ..Default::default()
        }
    }

    fn narrative_for(leg_id: &str) -> Narrative {
        Narrative { id: "narr".into(), name: "N".into(), color: "#000".into(), legs: vec![leg_id.to_string()] }
    }

    const SECTIONED_VERSE: &str = "ACT.2.1";

    fn sectioned_only(id: &str, mark: impl FnOnce(&mut Event)) -> Event {
        let mut e = Event {
            id: id.into(),
            label: format!("An event whose only claim to a heading is its {id}"),
            when: crate::time::TimeRange::new(33, 33).unwrap(),
            verses: vec![SECTIONED_VERSE.into()],
            ..Default::default()
        };
        mark(&mut e);
        e
    }

    fn heading_of(event: Event) -> Option<HeadingEntry> {
        AtlasData::new(Canon { books: vec![] }, vec![], vec![event], vec![], vec![], vec![], HashMap::new(), HashMap::new())
            .finish()
            .heading_for_verse(SECTIONED_VERSE)
            .cloned()
    }

    #[test]
    fn an_acts_section_alone_makes_an_event_head_its_verse() {
        // Arrange
        let event = sectioned_only("acts_section", |e| e.acts_section = Some("Acts fixture section".into()));

        // Act
        let heading = heading_of(event);

        // Assert
        assert_eq!(
            heading,
            Some(HeadingEntry {
                event_id: "acts_section".into(),
                title: "An event whose only claim to a heading is its acts_section".into(),
                kind: EventKind::Event,
            })
        );
    }

    #[test]
    fn an_atlas_section_alone_makes_an_event_head_its_verse() {
        // Arrange
        let event = sectioned_only("atlas_section", |e| e.atlas_section = Some("Atlas fixture section".into()));

        // Act
        let heading = heading_of(event);

        // Assert
        assert_eq!(
            heading,
            Some(HeadingEntry {
                event_id: "atlas_section".into(),
                title: "An event whose only claim to a heading is its atlas_section".into(),
                kind: EventKind::Event,
            })
        );
    }

    #[test]
    fn a_kjv_superscription_alone_makes_an_event_head_its_verse() {
        // Arrange
        let event = sectioned_only("kjv_superscription", |e| e.kjv_superscription = Some("A Psalm of David".into()));

        // Act
        let heading = heading_of(event);

        // Assert
        assert_eq!(
            heading,
            Some(HeadingEntry {
                event_id: "kjv_superscription".into(),
                title: "An event whose only claim to a heading is its kjv_superscription".into(),
                kind: EventKind::Event,
            })
        );
    }

    #[test]
    fn two_label_only_containers_sharing_a_verse_are_no_anchor_collision() {
        // Arrange
        let mut second_bare = bare_leg();
        second_bare.id = "other_bare_leg".into();
        let narratives = vec![Narrative {
            id: "narr".into(),
            name: "N".into(),
            color: "#000".into(),
            legs: vec!["bare_leg".into(), "other_bare_leg".into()],
        }];

        // Act
        let data = AtlasData::new(Canon { books: vec![] }, vec![], vec![bare_leg(), second_bare], narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();

        // Assert
        assert_eq!(data.heading_anchor_collisions().to_vec(), Vec::<(String, String, String)>::new());
    }

    #[test]
    fn two_curated_containers_sharing_a_verse_are_an_anchor_collision() {
        // Arrange
        let mut second_rich = rich_leg();
        second_rich.id = "other_rich_leg".into();
        let narratives = vec![Narrative {
            id: "narr".into(),
            name: "N".into(),
            color: "#000".into(),
            legs: vec!["rich_leg".into(), "other_rich_leg".into()],
        }];

        // Act
        let data = AtlasData::new(Canon { books: vec![] }, vec![], vec![rich_leg(), second_rich], narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();

        // Assert
        assert_eq!(
            data.heading_anchor_collisions().to_vec(),
            vec![("JHN.12.1".to_string(), "rich_leg".to_string(), "other_rich_leg".to_string())]
        );
    }

    #[test]
    fn heading_collision_prefers_the_richer_event_when_bare_is_first_in_order() {
        let events = vec![bare_leg(), rich_leg()];
        let narratives = vec![narrative_for("bare_leg")];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "rich_leg", "the richer (witness-bearing) event must win the collision, not whichever happened to sort first");
    }

    #[test]
    fn heading_collision_prefers_the_richer_event_regardless_of_vec_order() {
        let events = vec![rich_leg(), bare_leg()];
        let narratives = vec![narrative_for("bare_leg")];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "rich_leg");
    }

    #[test]
    fn heading_collision_shadowed_event_stays_reachable_via_event_membership() {
        let events = vec![bare_leg(), rich_leg()];
        let narratives = vec![narrative_for("bare_leg")];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let mut ids: Vec<String> = data
            .events
            .iter()
            .filter(|e| {
                e.verses.iter().any(|v| v == "JHN.12.1")
                    || e.witnesses.iter().any(|w| w.translations.values().flatten().any(|v| v == "JHN.12.1"))
            })
            .map(|e| e.id.clone())
            .collect();
        ids.sort();
        assert_eq!(ids, vec!["bare_leg".to_string(), "rich_leg".to_string()], "BOTH events must stay listed in the verse's own EVENT membership, win or lose the heading");
    }

    #[test]
    fn heading_collision_first_wins_when_richness_is_equal() {
        let mut other_bare = bare_leg();
        other_bare.id = "other_bare_leg".into();
        other_bare.label = "Another bare event, same verse".into();
        let events = vec![bare_leg(), other_bare];
        let narratives = vec![Narrative { id: "narr".into(), name: "N".into(), color: "#000".into(), legs: vec!["bare_leg".into(), "other_bare_leg".into()] }];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "bare_leg", "equal richness must keep first-wins (stable sort order), unchanged from before this fix");
    }

    #[test]
    fn heading_collision_tier3_earlier_chronology_wins_between_two_real_containers() {
        let mut earlier = rich_leg();
        earlier.id = "earlier_rich".into();
        earlier.when = crate::time::TimeRange::new(30, 30).unwrap();
        let mut later = rich_leg();
        later.id = "later_rich".into();
        later.when = crate::time::TimeRange::new(33, 33).unwrap();
        let events = vec![later, earlier];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "earlier_rich", "between two real containers of equal layer/kind, the chronologically earlier one must win");
    }

    #[test]
    fn heading_anchor_canonically_first_not_curated_import_order_no_witnesses() {
        let mut e = bare_leg();
        e.id = "theo_32_mirror".into();
        e.verses = vec!["GEN.6.7".into(), "GEN.6.1".into(), "GEN.6.2".into(), "GEN.6.3".into(), "GEN.6.4".into(), "GEN.6.5".into(), "GEN.6.6".into()];
        let narratives = vec![narrative_for("theo_32_mirror")];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], vec![e], narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("GEN.6.1").expect("GEN.6.1 -- the canonically first covered verse -- must anchor the heading, not GEN.6.7 (curated/import order)");
        assert_eq!(heading.event_id, "theo_32_mirror");
        assert!(data.heading_for_verse("GEN.6.7").is_none(), "the pre-fix anchor, GEN.6.7, must no longer claim the heading");
    }

    #[test]
    fn heading_anchor_canonically_first_not_curated_import_order_with_witnesses() {
        let mut e = rich_leg();
        e.id = "witness_import_order_mirror".into();
        e.verses = vec![];
        e.witnesses = vec![EventWitness {
            book: "GEN".into(),
            translations: HashMap::from([("kjv".to_string(), vec!["GEN.6.7".to_string(), "GEN.6.1".to_string(), "GEN.6.2".to_string()])]),
            ref_note: None,
            robertson_section: Some("fixture".into()),
        }];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], vec![e], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("GEN.6.1").expect("GEN.6.1 must anchor the heading via the witness branch too");
        assert_eq!(heading.event_id, "witness_import_order_mirror");
    }

    #[test]
    fn heading_collision_tier1_event_kind_beats_general_kind() {
        let mut event_kind = bare_leg();
        event_kind.id = "as_event".into();
        event_kind.kind = EventKind::Event;
        let mut general_kind = bare_leg();
        general_kind.id = "as_general".into();
        general_kind.kind = EventKind::General;
        let events = vec![general_kind, event_kind];
        let narratives = vec![Narrative { id: "narr".into(), name: "N".into(), color: "#000".into(), legs: vec!["as_event".into(), "as_general".into()] }];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "as_event", "kind=\"event\" must beat kind=\"general\" between two same-layer containers");
    }

    #[test]
    fn heading_collision_acts_section_counts_as_a_real_layer1_container() {
        let mut with_acts_section = bare_leg();
        with_acts_section.id = "as_acts".into();
        with_acts_section.acts_section = Some("Acts pericope (this project's own sectioning)".into());
        let mut plain_bare = bare_leg();
        plain_bare.id = "as_bare".into();
        let events = vec![plain_bare, with_acts_section];
        let narratives = vec![Narrative { id: "narr".into(), name: "N".into(), color: "#000".into(), legs: vec!["as_bare".into(), "as_acts".into()] }];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "as_acts", "acts_section alone must make a container real (layer 1), same as robertson_section");
    }

    #[test]
    fn heading_collision_atlas_section_counts_as_a_real_layer1_container() {
        let mut with_atlas_section = bare_leg();
        with_atlas_section.id = "as_atlas".into();
        with_atlas_section.atlas_section = Some("Atlas pericope (this project's own sectioning): GEN.1.1-2.3".into());
        let mut plain_bare = bare_leg();
        plain_bare.id = "as_bare2".into();
        let events = vec![plain_bare, with_atlas_section];
        let narratives = vec![Narrative { id: "narr".into(), name: "N".into(), color: "#000".into(), legs: vec!["as_bare2".into(), "as_atlas".into()] }];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "as_atlas", "atlas_section alone must make a container real (layer 1), same as robertson_section/acts_section");
    }

    #[test]
    fn heading_collision_kjv_superscription_counts_as_a_real_layer1_container() {
        let mut with_kjv_superscription = bare_leg();
        with_kjv_superscription.id = "as_kjv".into();
        with_kjv_superscription.kjv_superscription =
            Some("A Psalm of David, when he fled from Absalom his son (PSA.3.1).".into());
        let mut plain_bare = bare_leg();
        plain_bare.id = "as_bare3".into();
        let events = vec![plain_bare, with_kjv_superscription];
        let narratives = vec![Narrative { id: "narr".into(), name: "N".into(), color: "#000".into(), legs: vec!["as_bare3".into(), "as_kjv".into()] }];
        let data = AtlasData::new(Canon { books: vec![] }, vec![], events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let heading = data.heading_for_verse("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
        assert_eq!(heading.event_id, "as_kjv", "kjv_superscription alone must make a container real (layer 1), same as robertson_section/acts_section/atlas_section");
    }
}

#[cfg(test)]
mod catechism_tests {
    use super::*;
    use crate::refs::ScriptureRef;

    #[test]
    fn catechism_items_for_span_resolves_a_cited_verse() {
        let data = demo_fixture();
        let span = ScriptureRef::Verse(crate::refs::VerseId {
            book: crate::canon::resolve_alias("JOS").unwrap(),
            chapter: 6,
            verse: 20,
        });
        let out = data.catechism_items_for_span(&span);
        assert_eq!(
            out,
            vec![crate::catechism::CatechismRef { id: "demo-item-1".into(), name: "Demo Catechism Item".into(), question: None }]
        );
    }

    #[test]
    fn catechism_items_for_span_resolves_a_question_level_citation() {
        let data = demo_fixture();
        let span = ScriptureRef::Verse(crate::refs::VerseId {
            book: crate::canon::resolve_alias("JOS").unwrap(),
            chapter: 6,
            verse: 21,
        });
        let out = data.catechism_items_for_span(&span);
        assert_eq!(
            out,
            vec![crate::catechism::CatechismRef {
                id: "demo-item-1".into(),
                name: "Demo Catechism Item".into(),
                question: Some("Demo Question".into()),
            }]
        );
    }

    #[test]
    fn catechism_items_for_span_is_empty_for_an_uncited_verse() {
        let data = demo_fixture();
        let span = ScriptureRef::Verse(crate::refs::VerseId {
            book: crate::canon::resolve_alias("JOS").unwrap(),
            chapter: 1,
            verse: 1,
        });
        assert!(data.catechism_items_for_span(&span).is_empty());
    }

    #[test]
    fn catechism_item_by_id_resolves_the_owning_part_and_item() {
        let data = demo_fixture();
        let (part, item) = data.catechism_item_by_id("demo-item-1").expect("demo-item-1 must resolve");
        assert_eq!(part.id, "demo-part");
        assert_eq!(part.title, "Demo Part");
        assert_eq!(item.name, "Demo Catechism Item");
        assert_eq!(item.text, None);
        assert_eq!(item.verses, vec!["JOS.6.20".to_string()]);
    }

    #[test]
    fn catechism_item_by_id_returns_none_for_an_unknown_id() {
        let data = demo_fixture();
        assert!(data.catechism_item_by_id("no-such-item").is_none());
    }
}

#[cfg(test)]
mod year_index_tests {
    use super::*;

    #[test]
    fn year_index_is_contiguous_across_the_bc_ad_seam() {
        assert_eq!(year_index(-1), -1);
        assert_eq!(year_index(1), 0);
        assert_eq!(year_index(-2), -2);
        assert_eq!(year_index(2), 1);
        assert_eq!(year_index(1) - year_index(-1), 1);
    }
}

#[cfg(test)]
mod event_date_tests {
    use super::*;

    fn container(kind: EventKind, when: TimeRange) -> Event {
        Event {
            id: "sermon".into(),
            label: "The Sermon on the Mount".into(),
            when,
            places: vec![],
            verses: vec![],
            kind,
            witnesses: vec![],
            robertson_section: None,
            acts_section: None,
            atlas_section: None,
            kjv_superscription: None,
            ref_note: None,
            order_key: 0,
        }
    }

    #[test]
    fn an_events_date_is_the_span_it_was_placed_at() {
        // Arrange
        let placed = TimeRange::new(31, 31).unwrap();
        let event = container(EventKind::Event, placed);
        // Act
        let date = event.date();
        // Assert
        assert_eq!(date, Some(placed));
    }

    #[test]
    fn a_titled_passage_has_no_date_although_it_carries_the_undated_span() {
        // Arrange
        let passage = container(EventKind::General, TimeRange::undated());
        // Act
        let date = passage.date();
        // Assert
        assert_eq!(date, None);
    }
}

#[cfg(test)]
mod book_writing_tests {
    use super::*;
    use crate::CoreError;

    fn book(write_from: Option<i32>, write_to: Option<i32>) -> BookMeta {
        BookMeta { book: "NEH".into(), author: "Nehemiah".into(), write_place: Some("jerusalem".into()), write_from, write_to }
    }

    #[test]
    fn a_book_dated_at_both_ends_was_written_across_that_span() {
        // Arrange
        let nehemiah = book(Some(-430), Some(-400));
        // Act
        let written = nehemiah.written();
        // Assert
        assert_eq!(written, Ok(Some(TimeRange::new(-430, -400).unwrap())));
    }

    #[test]
    fn a_book_dated_at_neither_end_has_no_writing_date() {
        // Arrange
        let undated = book(None, None);
        // Act
        let written = undated.written();
        // Assert
        assert_eq!(written, Ok(None));
    }

    #[test]
    fn a_book_dated_at_one_end_only_is_refused_rather_than_given_a_bound_it_does_not_record() {
        // Arrange
        let one_ended = [book(Some(-430), None), book(None, Some(-400))];
        // Act
        let written = one_ended.map(|b| b.written());
        // Assert
        assert_eq!(written, [Err(CoreError::OneEndedSpan), Err(CoreError::OneEndedSpan)]);
    }

    #[test]
    fn a_book_dated_to_end_before_it_begins_is_refused() {
        // Arrange
        let inverted = book(Some(-400), Some(-430));
        // Act
        let written = inverted.written();
        // Assert
        assert_eq!(written, Err(CoreError::InvertedRange));
    }
}
