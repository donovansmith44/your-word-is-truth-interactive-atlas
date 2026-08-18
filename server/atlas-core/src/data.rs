//! The atlas data model: the compiled-file schema that ETL writes and the
//! server reads. Every record type derives `Serialize + Deserialize`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::time::{TimeRange, Year};

/// Verse counts per chapter for one book.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CanonBook {
    pub code: String,
    pub name: String,
    pub chapters: Vec<u16>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Canon {
    pub books: Vec<CanonBook>,
}

/// A geocoded place. `verse_links` are canonical verse ids attached by
/// geocoding (not by event participation) and drive scripture-mode lighting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub verse_links: Vec<String>,
}

/// A datable happening. `places[0]` is the anchor place used for arrow
/// endpoints; `places` may list more than one place (e.g. a campaign
/// touching several locations), all of which light up in time mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub label: String,
    pub when: TimeRange,
    pub places: Vec<String>,
    pub verses: Vec<String>,
}

/// An ordered chain of event ids (`legs`) that `scene::build_arrows` turns
/// into consecutive arrows. ETL validates legs are non-decreasing by
/// `when.from_year`; scene composition trusts that invariant rather than
/// re-deriving it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BookMeta {
    pub book: String,
    pub author: String,
    pub write_place: Option<String>,
    pub write_from: Option<i32>,
    pub write_to: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossRef {
    pub target: String,
    pub votes: i32,
}

/// The whole compiled atlas: ETL builds one of these and calls `.finish()`
/// before writing it to disk; the server deserializes the file and calls
/// `.finish()` again to rebuild the derived indexes (they are `#[serde(skip)]`
/// and therefore empty immediately after deserialization).
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

    /// Derived: place id -> index into `places`. Built by `finish()`.
    #[serde(skip)]
    place_index: HashMap<String, usize>,
    /// Derived: event id -> index into `events`. Built by `finish()`.
    #[serde(skip)]
    event_index: HashMap<String, usize>,
    /// Derived: canonical verse id -> event ids that reference it. Built by `finish()`.
    #[serde(skip)]
    verse_to_events: HashMap<String, Vec<String>>,
}

impl AtlasData {
    /// Builds an `AtlasData` from its eight schema fields, leaving the derived
    /// indexes empty (call `.finish()` to populate them). This is the
    /// ergonomic, one-expression way to construct one from outside this
    /// module: the index fields are private, so a plain struct literal
    /// naming them (including via `..Default::default()`, which still
    /// requires visibility of every field it fills in) cannot be written
    /// from another module. It is not the *only* way — `AtlasData::default()`
    /// followed by per-field assignment on the public schema fields also
    /// works, since those fields are all `pub`, just less conveniently.
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

    /// Sorts `events` chronologically by `when.from_year` and (re)builds the
    /// derived lookup indexes. Idempotent — safe to call more than once
    /// (e.g. once in ETL before writing, once in the server after reading).
    pub fn finish(mut self) -> Self {
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

        let mut verse_to_events: HashMap<String, Vec<String>> = HashMap::new();
        for e in &self.events {
            for v in &e.verses {
                verse_to_events.entry(v.clone()).or_default().push(e.id.clone());
            }
        }
        self.verse_to_events = verse_to_events;

        self
    }

    pub fn event_by_id(&self, id: &str) -> Option<&Event> {
        self.event_index.get(id).map(|&i| &self.events[i])
    }

    pub fn place_by_id(&self, id: &str) -> Option<&Place> {
        self.place_index.get(id).map(|&i| &self.places[i])
    }

    /// Event ids whose `verses` include the given canonical verse id.
    pub fn events_for_verse(&self, verse: &str) -> &[String] {
        self.verse_to_events.get(verse).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
