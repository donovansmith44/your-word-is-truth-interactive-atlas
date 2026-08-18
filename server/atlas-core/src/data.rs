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

    /// Reads the eight compiled JSON files ETL writes under `dir` (exact
    /// filenames mirror `atlas-etl/src/main.rs`'s `write_json` calls:
    /// `canon.json`, `places.json`, `events.json`, `narratives.json`,
    /// `eras.json`, `books-meta.json`, `verses-kjv.json`, `cross-refs.json`)
    /// and assembles an `AtlasData`. Does NOT call `.finish()` — the derived
    /// indexes are `#[serde(skip)]` and come back empty from a fresh
    /// deserialize (see the struct doc comment); callers (the server's
    /// `main.rs`) must call `.finish()` themselves.
    pub fn load(dir: &std::path::Path) -> Result<Self, crate::CoreError> {
        let canon: Canon = read_json(dir, "canon.json")?;
        let places: Vec<Place> = read_json(dir, "places.json")?;
        let events: Vec<Event> = read_json(dir, "events.json")?;
        let narratives: Vec<Narrative> = read_json(dir, "narratives.json")?;
        let eras: Vec<Era> = read_json(dir, "eras.json")?;
        let books_meta: Vec<BookMeta> = read_json(dir, "books-meta.json")?;
        let verses: HashMap<String, String> = read_json(dir, "verses-kjv.json")?;
        let cross_refs: HashMap<String, Vec<CrossRef>> = read_json(dir, "cross-refs.json")?;
        Ok(Self::new(canon, places, events, narratives, eras, books_meta, verses, cross_refs))
    }
}

fn read_json<T: serde::de::DeserializeOwned>(dir: &std::path::Path, file: &str) -> Result<T, crate::CoreError> {
    let path = dir.join(file);
    let text = std::fs::read_to_string(&path)
        .map_err(|source| crate::CoreError::Io { path: path.display().to_string(), source })?;
    serde_json::from_str(&text).map_err(|source| crate::CoreError::Json { path: path.display().to_string(), source })
}

/// A small, hand-built demo world. Originally
/// `atlas_core::scene::tests::fixture()` (Task 3, `#[cfg(test)]`-only);
/// promoted here as a normal `pub` function — not `#[cfg(test)]` — because
/// `atlas-server`'s integration tests live in a different crate and need it
/// at ordinary (non-test) compile time. `#[doc(hidden)]` keeps it out of
/// generated docs since it is not part of the intended public API surface.
///
/// The `places`/`events`/`narratives` below are byte-for-byte the Task 3
/// fixture: `scene.rs`'s own tests assert specific ids/links from this exact
/// shape (`jericho`, `gilgal`, `hebron`+`GEN.13.18`, `e1..e5`, `conquest`,
/// `patriarchs-demo`) and must keep passing unmodified. `canon`/`eras`/
/// `books_meta`/`verses`/`cross_refs` were empty placeholders in the Task 3
/// version (no scene test reads them); this promotion fills them with a
/// small consistent demo dataset for atlas-server's endpoint tests, whose
/// verses/cross-refs deliberately reuse ids already present above (e.g.
/// `GEN.13.18`, `JOS.6.20`) so the two halves of the fixture cross-check.
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
        },
        Event {
            id: "e2".into(),
            label: "Jericho besieged".into(),
            when: TimeRange::new(-1406, -1406).unwrap(),
            places: vec!["jericho".into()],
            verses: vec!["JOS.6.1".into(), "JOS.6.2".into()],
        },
        Event {
            id: "e3".into(),
            label: "Jericho falls".into(),
            when: TimeRange::new(-1405, -1405).unwrap(),
            places: vec!["jericho".into()],
            verses: vec!["JOS.6.20".into(), "JOS.6.21".into(), "JOS.6.24".into()],
        },
        Event {
            id: "e4".into(),
            label: "Ai defeated".into(),
            when: TimeRange::new(-1405, -1405).unwrap(),
            places: vec!["ai".into()],
            verses: vec!["JOS.8.1".into(), "JOS.8.28".into()],
        },
        Event {
            id: "e5".into(),
            label: "Sarah buried at Machpelah".into(),
            when: TimeRange::new(-2000, -2000).unwrap(),
            places: vec!["hebron".into()],
            verses: vec!["GEN.23.1".into(), "GEN.23.19".into()],
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
            CanonBook { code: "GEN".into(), name: "Genesis".into(), chapters: vec![31] },
            CanonBook { code: "JOS".into(), name: "Joshua".into(), chapters: vec![3] },
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
            CrossRef { target: "JOS.6.20-21".into(), votes: 9 }, // same-chapter span target
            CrossRef { target: "JOS.1.3".into(), votes: 5 },     // plain single-verse target
            CrossRef { target: "GEN.13.18".into(), votes: 2 },   // single-verse target, different book
        ],
    );

    AtlasData::new(canon, places, events, narratives, eras, books_meta, verses, cross_refs).finish()
}
