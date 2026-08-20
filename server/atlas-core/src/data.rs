//! The atlas data model: the compiled-file schema that ETL writes and the
//! server reads. Every record type derives `Serialize + Deserialize`.

use std::collections::{HashMap, HashSet};

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

/// A curated landmark label (river/sea/mountain/region) rendered always-on,
/// non-interactive, on the World map — the "actual good and informative
/// map" landmark-labels feature. `kind` is one of `"water"`, `"mountain"`,
/// `"region"` (atlas-etl's `validate::run_landmarks` enforces the enum;
/// this struct itself accepts any string so a hand-authored curated file
/// with a typo fails ETL validation with a clear message rather than
/// silently refusing to deserialize with serde's own less-helpful error).
///
/// Batch C2 (design-direction.md's Atlas plate detail SECOND ADDENDUM,
/// "populated far field... broad standing regions... sized by geographic
/// extent"): `size` is an optional hint, one of `"sm"`, `"md"`, `"lg"`
/// (also enum-checked by `validate::run_landmarks`, same friendlier-error
/// reason as `kind`), `None` for every landmark curated before this batch.
/// Two effects, both in map.js: (1) font-size, mirroring BorderLayer's own
/// polity-label `data-size` sm/md/lg tiers; (2) tier-0 visibility —
/// `applyLabelTier` treats ANY landmark carrying an explicit size hint as a
/// deliberately-curated far-field context label, visible even at the FAR
/// (zoomed-out) tier regardless of its `kind`'s own default tier rule,
/// which otherwise only exempts `kind == "water"`. A landmark with no
/// `size` at all keeps the exact PRE-Batch-C2 kind-based tier behavior —
/// this field only ever ADDS visibility, never removes any.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Landmark {
    pub name: String,
    pub kind: String,
    pub lat: f64,
    pub lon: f64,
    #[serde(default)]
    pub size: Option<String>,
}

/// Batch E (time-accurate places): one curated period name for a place,
/// e.g. "Luz" for `bethel-1` before Jacob's naming. `when` is the window
/// this name applies for; `verses` are canonical refs SUPPORTING the name
/// claim (etl-validated to exist in the compiled KJV text, same trust class
/// `Event.verses`/`Place.verse_links` already have).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceNameEntry {
    pub name: String,
    pub when: TimeRange,
    pub verses: Vec<String>,
}

/// One curated hover blurb for a place, active over `when`. `breadth` is
/// `"era"` (a period-specific blurb) or `"broad"` (a whole-sweep summary
/// shown when the selected window spans more than one of this place's own
/// `"era"` ranges) -- validated against that two-value enum by
/// `atlas-etl::validate` (same "plain String, checked at ETL time" pattern
/// [`Landmark::kind`] already uses, for the same friendlier-error reason).
/// No `verses` field -- blurb text is curator-written prose (ours/CC0), not
/// a claim keyed to a specific verse the way names/established/destroyed are.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceBlurbEntry {
    pub text: String,
    pub when: TimeRange,
    pub breadth: String,
}

/// A biblically-supported established/destroyed date claim. `when` is
/// either a single year (`from_year == to_year`, the curated TOML's `year =
/// ...` shorthand) or a genuine range (`from = ...` / `to = ...`) --
/// collapsed to one `TimeRange` here since the client's own `YearText.
/// FormatRange` already renders an equal-endpoints range as a single year,
/// so no separate "was this a year or a range" flag is needed on the wire.
/// `note` is a short qualifier (e.g. `"traditional"`) the client renders as
/// a leading "c." on the date when present.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceDateClaim {
    pub when: TimeRange,
    pub verses: Vec<String>,
    pub note: Option<String>,
}

/// One place's whole curated history record (`data/curated/place-history.toml`,
/// compiled to `place-history.json`). `id` matches a real compiled `Place.id`
/// (etl-validated -- an unknown id hard-fails, same trust class as
/// `Event.places`). Not every place has one of these; `AtlasData::place_history`
/// only holds entries for the ~15-25 places this batch actually curated.
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

/// Batch B2 ("borders v2, the cartographer's edition" -- supersedes the
/// snapshot-year GeoJSON model Batch B/C2 shipped, `AtlasData::borders` +
/// `nearest_border_year` below, both now deleted). One hand-authored
/// timerange era of ONE polity: `name` is time-accurate (may differ across a
/// single polity's own eras -- "Egypt" then later "Ptolemaic Egypt" is the
/// SAME `Polity::id`, so it keeps one color across the rename, see
/// `Polity::color_key`'s own comment), `from`/`to` are the (validated
/// non-overlapping-within-one-polity, see `atlas_etl::validate::run_polities`)
/// signed-year window this era's shape is drawn for, `ref_note` names only
/// the public-domain/general-knowledge source actually consulted while
/// drawing THIS era's rings (citation-integrity rule -- never a source that
/// wasn't opened), and `rings` is one or more CLOSED (first point repeats as
/// the last) simple polygon rings of `(lat, lon)` pairs -- deliberately
/// `[lat, lon]`, NOT GeoJSON's own `[lon, lat]` convention, matching how
/// every other coordinate pair in this app's curated data reads (`Place.lat`/
/// `Place.lon`, `Landmark.lat`/`Landmark.lon`) and how Leaflet's own
/// `L.latLng`/polygon APIs take points -- one less silent-transpose bug for a
/// curator hand-typing coordinates. Field names are `from`/`to` (not
/// `from_year`/`to_year`, unlike `Era`) and stay flat (no nested `TimeRange`)
/// on purpose: the batch brief's own TOML example and wire contract both
/// name them `from`/`to` verbatim, and keeping the curated TOML, the compiled
/// JSON, and the wire response byte-identical in shape means zero rename
/// mapping anywhere in the pipeline -- "easily reversible/modifiable"
/// (the user's own words) starts with the data staying exactly what it looks
/// like on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolityEra {
    pub name: String,
    pub from: Year,
    pub to: Year,
    pub ref_note: String,
    pub rings: Vec<Vec<(f64, f64)>>,
}

/// One hand-authored polity, `data/curated/polities/{id}.toml` (one file per
/// polity, "easily reversible/modifiable... I'm expecting you to get this
/// wrong" per the user's own direction -- a wrong shape is ONE file, ONE
/// `[[era]]` table, plain coordinate lists, nothing derived). `color_key` is
/// computed ONCE at ETL compile time -- based on `id` alone, never the era
/// `name`, so a polity keeps its plate tint across a rename, e.g. Egypt
/// staying the same hue through "Ptolemaic Egypt"; this REFINES Batch C2's
/// own name-hash, which kept re-deriving the tint from whatever single name
/// a snapshot-year feature happened to carry that year -- an index into
/// map.js's own `POLITY_TINTS` palette (16 tints as of fix round 1, M1; 8
/// through Batch C2/B2). Fix round 1 (M1): assignment is no longer a plain
/// per-id hash computed by `curated::parse_polity` in isolation (that
/// collided too often -- 11 of the original batch's 14 polities shared a
/// hue) -- `curated::parse_polity` now leaves this field provisional (`0`),
/// and `atlas_etl::main::process_polities` overwrites every polity's real,
/// collision-free value in one pass, via `atlas_etl::polities::
/// assign_color_keys`, once the full sorted roster is available (see that
/// function's own doc comment for the linear-probing algorithm). Stored
/// here either way (not recomputed per-request); `/api/polities` just
/// copies it onto every emitted era row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Polity {
    pub id: String,
    pub color_key: u8,
    pub eras: Vec<PolityEra>,
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

    /// Batch B2: hand-authored per-polity timerange borders (supersedes the
    /// snapshot-year GeoJSON model -- see `Polity`'s own doc comment).
    /// `#[serde(skip)]` because -- like `landmarks`/`place_history` below --
    /// this isn't populated via the struct-level derive: `AtlasData::load`
    /// fills it in from its own `polities.json`, a plain `Vec<Polity>` on
    /// disk (one entry per curated `data/curated/polities/{id}.toml` file).
    /// `demo_fixture()` leaves this empty (no server test needs real polity
    /// geometry); real data always has the full curated roster once
    /// atlas-etl has run.
    #[serde(skip)]
    pub polities: Vec<Polity>,
    /// Curated landmark labels (rivers/seas/mountains/regions). Same
    /// `#[serde(skip)]`-plus-bespoke-`load()` treatment as `borders` above,
    /// for the same reason (loaded from its own `landmarks.json`, not part
    /// of the original eight-file/eight-field pattern). `demo_fixture()`
    /// leaves this empty too.
    #[serde(skip)]
    pub landmarks: Vec<Landmark>,
    /// Batch E: curated place histories (period names, blurbs, established/
    /// destroyed dates), keyed by place id. Same `#[serde(skip)]`-plus-
    /// bespoke-`load()` treatment as `borders`/`landmarks` above -- loaded
    /// from its own `place-history.json` (a plain `Vec<PlaceHistory>` on
    /// disk, matching every sibling compiled file's array shape), indexed
    /// into a map here since every lookup site wants it by place id.
    /// `demo_fixture()` leaves this empty too (no scene test depends on it).
    #[serde(skip)]
    pub place_history: HashMap<String, PlaceHistory>,

    /// Derived: place id -> index into `places`. Built by `finish()`.
    #[serde(skip)]
    place_index: HashMap<String, usize>,
    /// Derived: event id -> index into `events`. Built by `finish()`.
    #[serde(skip)]
    event_index: HashMap<String, usize>,
    /// Derived: canonical verse id -> event ids that reference it. Built by `finish()`.
    #[serde(skip)]
    verse_to_events: HashMap<String, Vec<String>>,
    /// Batch E2 (the ever-present graph): "cities in our graph" per the
    /// user's own direction quoted in batch-e2-brief.md -- ids of every
    /// place touched by >=1 event, ANY window (206 in the real compiled
    /// data, but NEVER hardcoded -- always derived fresh from `events` by
    /// `finish()`, so a future data change is picked up automatically).
    /// `scene::quiet_places` subtracts a window's own lit set from this to
    /// get that window's quiet places.
    #[serde(skip)]
    event_bearing_place_ids: HashSet<String>,
    /// Batch E2: a place's ALL-TIME event count (every event touching it,
    /// in ANY window, not just the scene's own) -- the source of
    /// `QuietPlace::total_events`. Built alongside `event_bearing_place_ids`
    /// by `finish()` in the same single pass over `events`.
    #[serde(skip)]
    event_counts_by_place: HashMap<String, u32>,
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

        // Batch E2: one pass over every event's `places` builds BOTH the
        // event-bearing id set and each one's all-time event count -- "cities
        // in our graph" is derived here, never hardcoded (see the two
        // fields' own doc comments).
        let mut event_counts_by_place: HashMap<String, u32> = HashMap::new();
        for e in &self.events {
            for pid in &e.places {
                *event_counts_by_place.entry(pid.clone()).or_insert(0) += 1;
            }
        }
        self.event_bearing_place_ids = event_counts_by_place.keys().cloned().collect();
        self.event_counts_by_place = event_counts_by_place;

        self
    }

    pub fn event_by_id(&self, id: &str) -> Option<&Event> {
        self.event_index.get(id).map(|&i| &self.events[i])
    }

    pub fn place_by_id(&self, id: &str) -> Option<&Place> {
        self.place_index.get(id).map(|&i| &self.places[i])
    }

    /// Batch E: this place's curated history record, if any (most places
    /// have none -- `place_history` only covers the curated ~15-25).
    pub fn place_history_for(&self, id: &str) -> Option<&PlaceHistory> {
        self.place_history.get(id)
    }

    /// Batch E2: ids of every event-bearing place ("cities in our graph",
    /// user direction 2026-08-19 quoted in batch-e2-brief.md) -- the fixed-
    /// cardinality set QUIET-1 checks `places` union `quiet_places` against
    /// for every time-mode window. Derived from `events` by `finish()`.
    pub fn event_bearing_place_ids(&self) -> &HashSet<String> {
        &self.event_bearing_place_ids
    }

    /// Batch E2: a place's ALL-TIME event count (every event touching it, in
    /// ANY window) -- `QuietPlace::total_events`'s source. 0 for a place
    /// with no events at all; such a place is never actually looked up by
    /// `scene::quiet_places` (it only iterates `event_bearing_place_ids`),
    /// but 0 is a safe, honest default rather than panicking if ever called
    /// directly.
    pub fn total_events_for(&self, id: &str) -> u32 {
        self.event_counts_by_place.get(id).copied().unwrap_or(0)
    }

    /// Event ids whose `verses` include the given canonical verse id.
    pub fn events_for_verse(&self, verse: &str) -> &[String] {
        self.verse_to_events.get(verse).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Reads the compiled JSON files ETL writes under `dir` (exact
    /// filenames mirror `atlas-etl/src/main.rs`'s `write_json` calls:
    /// `canon.json`, `places.json`, `events.json`, `narratives.json`,
    /// `eras.json`, `books-meta.json`, `verses-kjv.json`, `cross-refs.json`,
    /// plus `polities.json`, plus `landmarks.json`, plus `place-history.json`)
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

        let polities: Vec<Polity> = read_json(dir, "polities.json")?;
        let landmarks: Vec<Landmark> = read_json(dir, "landmarks.json")?;
        let place_history_list: Vec<PlaceHistory> = read_json(dir, "place-history.json")?;
        let place_history: HashMap<String, PlaceHistory> =
            place_history_list.into_iter().map(|h| (h.id.clone(), h)).collect();

        let mut data = Self::new(canon, places, events, narratives, eras, books_meta, verses, cross_refs);
        data.polities = polities;
        data.place_history = place_history;
        data.landmarks = landmarks;
        Ok(data)
    }
}

/// Maps a signed calendar year (never zero) onto a contiguous integer line
/// with the zero-year gap removed: `..., -2, -1, 1, 2, ...` becomes
/// `..., -2, -1, 0, 1, ...` (AD years shift down by one; BC years are
/// unchanged, since they're already contiguous below zero). Used by
/// `nearest_border_year`'s midpoint math above and, via `pub(crate)`, by
/// `crate::history`'s own zero-aware window-midpoint math (same
/// straddle-BC/AD problem, same fix -- one source of truth for it).
pub(crate) fn year_index(y: Year) -> i64 {
    if y > 0 {
        (y - 1) as i64
    } else {
        y as i64
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
    // Batch G1 (GET /api/xrefs/{sref}): JOS.6.21's own list, added so
    // atlas-server's own endpoint test can exercise the REAL span-
    // aggregation handler end to end against the span JOS.6.20-21, not just
    // atlas_core::xrefs's pure unit/property tests. Deliberately overlaps
    // JOS.6.20's own JOS.1.3 target (5 + 4 = 9, proving votes are SUMMED
    // across member verses, not just taken from whichever verse is seen
    // first) and cites JOS.6.20 itself (a self-target once the span is
    // JOS.6.20-21, proving the drop rule) -- see
    // verse_chapter_place_and_404's own JOS.6.20 assertions above, which
    // this entry (a different map key) leaves completely untouched.
    cross_refs.insert(
        "JOS.6.21".into(),
        vec![
            CrossRef { target: "JOS.1.3".into(), votes: 4 },
            CrossRef { target: "JOS.6.20".into(), votes: 3 },
        ],
    );

    let mut data = AtlasData::new(canon, places, events, narratives, eras, books_meta, verses, cross_refs).finish();
    // Batch E: one small curated history record (hebron -- already present
    // above with a real event, e5, and its own GEN.23.* verses) so
    // atlas-server's own integration tests can exercise the REAL
    // `/api/place/{id}?from=&to=` resolution pipeline end to end, not just
    // atlas_core::history's pure functions in isolation. Values are
    // plausible, not independently curated the way data/curated/
    // place-history.toml's real content is -- this fixture exists only to
    // give the endpoint something to resolve.
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
    data
}

#[cfg(test)]
mod year_index_tests {
    use super::*;

    // Batch B2: `nearest_border_year` (the snapshot-year model it served)
    // and its own dedicated test module are gone -- see `Polity`'s doc
    // comment. `year_index` itself survives (still used by
    // `crate::history`'s zero-aware window-midpoint math), so its own unit
    // coverage survives with it, just no longer nested under a
    // border-flavored module name.
    #[test]
    fn year_index_is_contiguous_across_the_bc_ad_seam() {
        assert_eq!(year_index(-1), -1);
        assert_eq!(year_index(1), 0);
        assert_eq!(year_index(-2), -2);
        assert_eq!(year_index(2), 1);
        // -1 and 1 (adjacent calendar years, matching time::next_year) must
        // be exactly 1 apart in index space.
        assert_eq!(year_index(1) - year_index(-1), 1);
    }
}
