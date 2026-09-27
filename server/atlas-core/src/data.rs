//! The compiled-file schema the ETL writes and the server reads.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::time::{TimeRange, Year};

/// One book of the canon: its code, its name, and how many verses each of its
/// chapters holds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CanonBook {
    /// The three-letter code, such as `GEN`.
    pub code: String,
    /// The book's full name.
    pub name: String,
    /// The verse count of each chapter, in order.
    pub chapters: Vec<u16>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Canon {
    pub books: Vec<CanonBook>,
}

/// `verse_links` are attached by geocoding, not by event participation, and are what
/// light a place for a scripture reference.
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
    pub partners: Vec<String>,
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

/// `places[0]` is the anchor used for arrow endpoints; every listed place lights up.
/// `verses` is a container's verse set, never written onto a verse: the empty set is
/// lawful, and two containers covering one verse is expected, not an error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub label: String,
    pub when: TimeRange,
    pub places: Vec<String>,
    pub verses: Vec<String>,
    /// `"event"` or `"general"`. A general container has no date and no place: it carries
    /// the undated sentinel and an empty `places`, both supplied by the ETL and never
    /// curator-typed, so "do not fabricate a date" is structural.
    #[serde(default = "default_event_kind")]
    pub kind: String,
    /// Empty means one IMPLICIT witness, synthesized from `verses` grouped by book -- never
    /// zero witnesses, and never a reason to withhold this container's heading.
    #[serde(default)]
    pub witnesses: Vec<EventWitness>,
    /// Which section of Robertson's Harmony of the Gospels grounds this title, date and
    /// grouping. `None` where none was consulted, rather than an invented section number.
    #[serde(default)]
    pub robertson_section: Option<String>,
    /// The Acts sibling of `robertson_section`, kept a separate field so an Acts section
    /// can never be mistaken for a Robertson-verified one.
    #[serde(default)]
    pub acts_section: Option<String>,
    /// The same provenance field for a book outside the Gospels and Acts: this atlas's own
    /// sectioning.
    #[serde(default)]
    pub atlas_section: Option<String>,
    /// The same provenance field where the title IS literal KJV text -- a psalm's own
    /// superscription, quoted verbatim -- never this atlas's phrasing.
    #[serde(default)]
    pub kjv_superscription: Option<String>,
    /// Names only sources actually consulted for this container's date and grouping.
    /// `None` is honest, not a gap.
    #[serde(default)]
    pub ref_note: Option<String>,
    /// The sub-year tiebreak: the year model is year-granular, so two events days apart in
    /// one year need this to order them, and a narrative's legs must be non-decreasing by it
    /// within a year. Defaults to 0, which is never read for an event alone in its year.
    #[serde(default)]
    pub order_key: i32,
}

fn default_event_kind() -> String {
    "event".to_string()
}

/// Hand-written because `TimeRange` has no `Default` -- it validates through a fallible
/// constructor. The placeholder `when` is never read: every user states its own.
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

/// `book` is carried rather than derived from the verse ids, so a witness whose translation
/// entry is empty still says which book it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventWitness {
    pub book: String,
    /// Lowercase translation code -> flat, individually canonical verse ids.
    pub translations: HashMap<String, Vec<String>>,
    /// This account's own citation note, not the container's note about its date and
    /// grouping as a whole.
    #[serde(default)]
    pub ref_note: Option<String>,
    /// `None` where this account shares its container's section rather than repeating it.
    #[serde(default)]
    pub robertson_section: Option<String>,
}

/// A journey or storyline through the atlas: an ordered chain of events the map
/// draws as a run of arrows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Narrative {
    pub id: String,
    pub name: String,
    /// The colour its arrows are drawn in.
    pub color: String,
    /// The ids of its events, in order, never running backwards in time.
    pub legs: Vec<String>,
}

/// A named stretch of this atlas's timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Era {
    pub id: String,
    pub name: String,
    /// The first year of the era, negative for BC.
    #[schema(value_type = i32)]
    pub from_year: Year,
    /// The last year of the era.
    #[schema(value_type = i32)]
    pub to_year: Year,
}

/// Who wrote one book of the Bible, where, and when.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BookMeta {
    /// Not on the wire: a caller already knows which book it asked about.
    #[serde(skip_serializing)]
    pub book: String,
    /// The book's author, as this atlas records him.
    pub author: String,
    /// Where it was written, absent when that is not recorded.
    pub write_place: Option<String>,
    /// The earliest year it is dated to, negative for BC; absent when undated.
    pub write_from: Option<i32>,
    /// The latest year it is dated to; absent when undated.
    pub write_to: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossRef {
    pub target: String,
    pub votes: i32,
}

/// An always-on map label: a water, a mountain or a region, drawn at one point
/// and never interactive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct Landmark {
    pub name: String,
    /// `water`, `mountain` or `region`.
    pub kind: String,
    /// Latitude in degrees, north positive.
    pub lat: f64,
    /// Longitude in degrees, east positive.
    pub lon: f64,
    /// A size hint -- `sm`, `md` or `lg` -- for a label meant to stay readable when
    /// zoomed out; absent for most landmarks.
    #[serde(default)]
    pub size: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChronologyAnchor {
    /// Never renumbered or repositioned: other tables reference this row by it.
    pub id: String,
    pub label: String,
    pub year: Year,
    /// `None` where no single compiled event corresponds, or where binding one would
    /// misrepresent an already-disclosed adjacency. An honest gap, not a shortcut.
    #[serde(default)]
    pub event_id: Option<String>,
    /// Always carries an `event_id` when true: an era boundary needs a real timeline
    /// position to gate on.
    #[serde(default)]
    pub era_boundary: bool,
    pub source: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// The widest span a book's narrative NARRATES -- never the span it was written in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BookNarrationWindow {
    /// Canonical three-letter code, e.g. `"GEN"`.
    pub book: String,
    pub from_year: Year,
    pub to_year: Year,
    #[serde(default)]
    pub note: Option<String>,
}

/// `when` is the window this name applies over; `verses` are the refs supporting the claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceNameEntry {
    pub name: String,
    pub when: TimeRange,
    pub verses: Vec<String>,
}

/// `breadth` is `"era"` (period-specific) or `"broad"` (a whole-sweep summary, shown when
/// the window spans more than one of this place's era ranges). No verses: blurb text is
/// prose, not a claim keyed to a verse the way a name or a date is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceBlurbEntry {
    pub text: String,
    pub when: TimeRange,
    pub breadth: String,
}

/// A date this atlas claims for a place, with the verses it rests on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PlaceDateClaim {
    /// The year claimed, or the range; a single year is a span whose ends are equal.
    pub when: TimeRange,
    /// The verses supporting the claim.
    pub verses: Vec<String>,
    /// A qualifier such as `traditional`, shown as a leading "c." on the date;
    /// absent when the date needs none.
    pub note: Option<String>,
}

/// `id` matches a real compiled place id; most places have no record at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceHistory {
    pub id: String,
    #[serde(default)]
    pub names: Vec<PlaceNameEntry>,
    #[serde(default)]
    pub blurbs: Vec<PlaceBlurbEntry>,
    pub established: Option<PlaceDateClaim>,
    pub destroyed: Option<PlaceDateClaim>,
}

/// A flat, translation-keyed display name for a place whose default name is not the one
/// the text uses. Independent of any calendar window, and only ever the fallback when no
/// curated period name is active -- never a competitor to one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceNameAlias {
    pub id: String,
    pub translations: HashMap<String, String>,
    pub verses: Vec<String>,
}

/// `from`/`to` are validated non-overlapping within one polity. `rings` are one or more
/// CLOSED simple polygons of `(lat, lon)` pairs -- deliberately not GeoJSON's `[lon, lat]`,
/// matching every other coordinate pair here and the map library's own point order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolityEra {
    pub name: String,
    pub from: Year,
    pub to: Year,
    pub ref_note: String,
    pub rings: Vec<Vec<(f64, f64)>>,
    /// The delta at this era's start -- for a polity's first era, its rise. `None` is a
    /// deliberate "uneventful boundary", not a gap to paper over.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<PolityDelta>,
    /// Only ever curated on a polity's final era. Absence where this atlas's span simply
    /// outlives the polity is not an authoring gap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fall: Option<PolityDelta>,
}

/// The event at one boundary of a polity's era: its rise, a change of its
/// borders, or its fall.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PolityDelta {
    /// A short description of what happened.
    pub event: String,
    /// The verses grounding it. May be empty for an event history records but no
    /// single verse pinpoints; never a fabricated reference.
    #[serde(default)]
    #[schema(required = true)]
    pub verses: Vec<String>,
    /// The sources actually consulted for this event.
    pub ref_note: String,
    /// A curator-authored echo of the hosting era's `from`, cross-checked by the
    /// ETL: TOML attaches a nested table to the most recently opened `[[era]]`, not
    /// to whichever era the surrounding prose describes.
    #[serde(skip_serializing)]
    pub for_era_from: Year,
}

/// Geometry only, used solely to clip polity washes so they never spill into open sea;
/// never drawn as a layer of its own. `rings` follow `PolityEra::rings`' convention.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LandMaskRegion {
    pub name: String,
    pub ref_note: String,
    pub rings: Vec<Vec<(f64, f64)>>,
}

/// `text` is `None` for an item that poses its own bespoke question with no separate
/// prompt to quote first. `explanation_heading` is verbatim, defaulting to the common
/// phrase so the curated file spells out only the items that really differ.
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
    /// Question-level citations, kept separate from the item-level `verses` above: two
    /// granularities of citation, not a second copy of one.
    #[serde(default)]
    pub questions: Vec<CatechismQuestion>,
}

fn default_explanation_heading() -> String {
    "What does this mean?".to_string()
}

/// `verses` is a flat list of individually canonical refs, never a range string.
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

/// `color_key` is derived from `id` alone, never an era name, so a polity keeps one tint
/// across a rename; it is assigned in one pass over the whole roster so the values cannot
/// collide, and stored rather than recomputed per request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polity {
    pub id: String,
    pub color_key: u8,
    pub eras: Vec<PolityEra>,
}

/// The derived indexes are `#[serde(skip)]`, so `finish()` must be called again after
/// deserializing. `places`/`events`/`narratives`/`verses` are compile-time inputs only:
/// they are empty on every serving path, which composes from the graph instead.
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

    /// In curated-file order; the first row for an id is the one single-name callers show.
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
    pub event_mentions: Vec<EventMentionSeed>,
    #[serde(skip)]
    pub event_analogues: Vec<EventAnalogueSeed>,

    #[serde(skip)]
    place_index: HashMap<String, usize>,
    #[serde(skip)]
    event_index: HashMap<String, usize>,
    /// Every place touched by at least one event, in any window.
    #[serde(skip)]
    event_bearing_place_ids: HashSet<String>,
    /// All-time, in any window -- not the count for a scene's own window.
    #[serde(skip)]
    event_counts_by_place: HashMap<String, u32>,

    /// The title is `None` for a citation embedded in the item itself and `Some` for one
    /// carried by a question.
    #[serde(skip)]
    verse_to_catechism: HashMap<String, Vec<(String, Option<String>)>>,
    #[serde(skip)]
    catechism_item_names: HashMap<String, String>,
    #[serde(skip)]
    catechism_item_index: HashMap<String, (usize, usize)>,

    /// One entry per WITNESS, so a multi-witness container heads each book it appears in at
    /// that account's own first verse -- never just one book.
    #[serde(skip)]
    verse_heading: HashMap<String, HeadingEntry>,

    /// Two independently curated containers should never claim one anchor verse: curated
    /// sectioning partitions, it does not overlap. Separate from the display resolution,
    /// which legitimately picks a winner; the ETL fails loud on every entry here.
    #[serde(skip)]
    heading_anchor_collisions: Vec<(String, String, String)>,

    /// A STABLE sort by `(from_year, order_key)`, so a tie keeps the original compiled
    /// order. Undated containers are excluded -- there is no date to sort them by. Built
    /// after the identity merges, so one occurrence never appears twice on two scales.
    #[serde(skip)]
    timeline_order: Vec<String>,
    #[serde(skip)]
    timeline_index: HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadingEntry {
    pub event_id: String,
    pub title: String,
    /// Carried so a reader can tell whether this heading leads to a dated, traversable
    /// container or an undated one, without a second fetch to find out.
    pub kind: String,
}

/// Each anchor is the CANONICALLY FIRST covered verse, not whichever the data happened to
/// list first. An unparseable verse is skipped rather than panicking: this runs before
/// validation, whose job is to report a bad ref as an aggregated, curator-facing error.
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

/// Compared lexicographically: a strictly greater tuple wins a collision, an equal one
/// keeps first-wins. Tiers are curated container over label-only, dated over undated, then
/// earlier chronology -- via `Reverse`, because `i32::MAX - year` overflows on a BC year.
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
    let kind: u8 = if e.kind == "event" { 1 } else { 0 };
    (layer, kind, std::cmp::Reverse(e.when.from_year), std::cmp::Reverse(e.order_key))
}

impl AtlasData {
    /// Leaves the derived indexes empty; call `finish()` to populate them.
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

    /// Idempotent: safe to call more than once -- the ETL calls it before writing and the
    /// server again after reading.
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
                    verse_heading.insert(anchor, HeadingEntry { event_id: e.id.clone(), title: e.label.clone(), kind: e.kind.clone() });
                }
            }
        }
        self.verse_heading = verse_heading;
        self.heading_anchor_collisions = heading_anchor_collisions;

        let mut timeline_order: Vec<String> =
            self.events.iter().filter(|e| e.kind == "event").map(|e| e.id.clone()).collect();
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

    /// The first-authored alias for this id. See `place_name_aliases_for` for the full list.
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

    /// A verse is an anchor exactly when it is the first verse of a heading-worthy
    /// container's account: a narrative leg, or one carrying curated witnesses or a
    /// provenance section. A multi-witness container anchors one heading per account.
    pub fn heading_for_verse(&self, verse: &str) -> Option<&HeadingEntry> {
        self.verse_heading.get(verse)
    }

    /// `None` for an undated or unknown id, by absence from the index rather than a branch.
    pub fn timeline_position(&self, id: &str) -> Option<usize> {
        self.timeline_index.get(id).copied()
    }

    pub fn timeline_event_at(&self, index: usize) -> Option<&Event> {
        self.timeline_order.get(index).and_then(|id| self.event_by_id(id))
    }

    /// First-seen order, no duplicates.
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
}

/// Removes the zero-year gap: `..., -2, -1, 1, 2, ...` becomes `..., -2, -1, 0, 1, ...`, so
/// midpoint arithmetic across the BC/AD boundary is off by nothing.
pub(crate) fn year_index(y: Year) -> i64 {
    if y > 0 {
        (y - 1) as i64
    } else {
        y as i64
    }
}

/// A hand-built demo world, `pub` rather than `#[cfg(test)]` because another crate's
/// integration tests need it at ordinary compile time. Tests assert specific ids and links
/// from this exact shape, so changing it changes them.
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
            blurbs: vec![PlaceBlurbEntry {
                text: "Abraham buried Sarah in the cave of Machpelah here.".into(),
                when: TimeRange::new(-2166, -1877).unwrap(),
                breadth: "era".into(),
            }],
            established: Some(PlaceDateClaim {
                when: TimeRange::new(-2000, -2000).unwrap(),
                verses: vec!["GEN.23.19".into()],
                note: Some("traditional".into()),
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
        event_kind.kind = "event".into();
        let mut general_kind = bare_leg();
        general_kind.id = "as_general".into();
        general_kind.kind = "general".into();
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
