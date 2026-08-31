use std::collections::{HashMap, HashSet};

use atlas_core::data::{AtlasData, BookMeta, BookNarrationWindow, Canon, ChronologyAnchor, CrossRef, Era, Event, EventWitness, LandMaskRegion, Narrative, Place, PlaceBlurbEntry, PlaceDateClaim, PlaceHistory, PlaceNameAlias, PlaceNameEntry, Polity, PolityDelta, PolityEra};
use atlas_core::event_merge::{EventDistinct, EventMerge};
use atlas_core::merge::PlaceMerge;
use atlas_core::time::TimeRange;

// ---------------------------------------------------------------------
// kjv.rs
// ---------------------------------------------------------------------

#[test]
fn kjv_parses_and_keys_canonically() {
    let (canon, verses) = atlas_etl::kjv::parse(include_str!("fixtures/kjv-sample.json")).unwrap();
    assert_eq!(canon.books[0].code, "GEN");
    assert!(verses.contains_key("GEN.1.1"));
}

#[test]
fn kjv_normalizes_roman_numeral_and_of_john_names() {
    let (canon, verses) = atlas_etl::kjv::parse(include_str!("fixtures/kjv-sample.json")).unwrap();
    let codes: Vec<&str> = canon.books.iter().map(|b| b.code.as_str()).collect();
    assert!(codes.contains(&"1SA"), "expected 1SA (from 'I Samuel') in {codes:?}");
    assert!(codes.contains(&"REV"), "expected REV (from 'Revelation of John') in {codes:?}");
    assert!(verses.contains_key("1SA.1.1"));
    assert!(verses.contains_key("REV.1.1"));
    assert_eq!(verses["GEN.1.1"], "In the beginning God created the heaven and the earth.");
}

#[test]
fn kjv_unresolved_book_name_hard_errors() {
    let err = atlas_etl::kjv::parse(include_str!("fixtures/kjv-bad-book.json")).unwrap_err();
    assert!(err.to_string().contains("Not A Real Book"), "{err}");
}

// ---------------------------------------------------------------------
// geo.rs
// ---------------------------------------------------------------------

#[test]
fn geo_slug_collisions_get_suffixes() {
    let places = atlas_etl::geo::parse(include_str!("fixtures/geo-ancient-sample.jsonl")).unwrap();
    let antiochs: Vec<&str> = places.iter().filter(|p| p.name == "Antioch").map(|p| p.id.as_str()).collect();
    assert_eq!(antiochs, vec!["antioch", "antioch-2"]);
}

#[test]
fn geo_filters_non_kjv_verse_links_and_skips_unresolvable_places() {
    let places = atlas_etl::geo::parse(include_str!("fixtures/geo-ancient-sample.jsonl")).unwrap();
    // "Ghostplace" has no identifications at all -> no coordinate -> dropped.
    assert_eq!(places.len(), 4, "{places:#?}");
    assert!(places.iter().all(|p| p.name != "Ghostplace"));

    // "Nowhereton"'s only verse link is not tagged "kjv" in translations -> filtered out.
    let nowhereton = places.iter().find(|p| p.name == "Nowhereton").unwrap();
    assert!(nowhereton.verse_links.is_empty(), "{:?}", nowhereton.verse_links);

    // lon,lat are reversed in the raw "lonlat" string; Place stores real lat/lon.
    let abana = places.iter().find(|p| p.name == "Abana").unwrap();
    assert_eq!(abana.verse_links, vec!["2KI.5.12".to_string()]);
    assert_eq!(abana.lat, 33.513542);
    assert_eq!(abana.lon, 36.305000);
}

// ---------------------------------------------------------------------
// theographic.rs
// ---------------------------------------------------------------------

fn theo_place_slug_by_name() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("antioch".to_string(), "antioch".to_string());
    m
}

#[test]
fn theographic_undated_events_dropped_not_fatal() {
    let (events, _new_places, stats) = atlas_etl::theographic::parse_events(
        include_str!("fixtures/theographic-places-sample.json"),
        include_str!("fixtures/theographic-verses-sample.json"),
        include_str!("fixtures/theographic-events-sample.json"),
        &theo_place_slug_by_name(),
    )
    .unwrap();
    assert_eq!(stats.undated, 1);
    assert_eq!(stats.total, 4);
    assert_eq!(events.len(), 3);
}

#[test]
fn theographic_dates_parsed_and_places_joined_or_created() {
    let (events, new_places, stats) = atlas_etl::theographic::parse_events(
        include_str!("fixtures/theographic-places-sample.json"),
        include_str!("fixtures/theographic-verses-sample.json"),
        include_str!("fixtures/theographic-events-sample.json"),
        &theo_place_slug_by_name(),
    )
    .unwrap();

    // "-4003" astronomical == 4004 BC == historical -4004 (matches the README's
    // own worked example and the curated "primeval" era boundary).
    let creation = events.iter().find(|e| e.label.contains("Creation")).unwrap();
    assert_eq!(creation.when, TimeRange::new(-4004, -4004).unwrap());
    assert_eq!(creation.places, vec!["antioch".to_string()]); // joined by name to the geo place
    assert_eq!(creation.verses, vec!["ACT.13.1".to_string()]);

    // ISO-ish "0046-03-01" -> historical year 46 (positive, no BC shift); place name
    // "Nineveh" has no geo match, so a new Place is synthesized from lat/lon.
    let jonah = events.iter().find(|e| e.label.contains("Nineveh")).unwrap();
    assert_eq!(jonah.when, TimeRange::new(46, 46).unwrap());
    assert_eq!(jonah.verses, vec!["JON.1.2".to_string()]);
    assert_eq!(jonah.places.len(), 1);
    let new_place_id = &jonah.places[0];
    let np = new_places.iter().find(|p| &p.id == new_place_id).expect("synthesized place returned");
    assert_eq!(np.name, "Nineveh");
    assert_eq!(np.lat, 36.36);
    assert_eq!(np.lon, 43.15);
    assert!(np.verse_links.is_empty());

    // Dated event with no `locations` key at all -> kept, with empty places.
    let no_place_event = events.iter().find(|e| e.label.contains("no linked place")).unwrap();
    assert!(no_place_event.places.is_empty());

    assert_eq!(stats.no_place, 1);
    assert_eq!(stats.new_places, 1);
    assert_eq!(stats.dated, 3);
}

// ---------------------------------------------------------------------
// xrefs.rs
// ---------------------------------------------------------------------

#[test]
fn xrefs_sorted_desc_and_no_self() {
    let (map, _stats) = atlas_etl::xrefs::parse(include_str!("fixtures/xrefs-sample.txt")).unwrap();
    let gen11 = map.get("GEN.1.1").expect("Gen.1.1 should have cross-refs");
    let votes: Vec<i32> = gen11.iter().map(|c| c.votes).collect();
    assert_eq!(votes, vec![20, 7, 3], "{gen11:?}");
    assert!(gen11.iter().all(|c| c.target != "GEN.1.1"), "self-reference must be dropped: {gen11:?}");
    assert_eq!(gen11[0].target, "JOB.26.13");
    assert_eq!(gen11[1].target, "COL.1.16-17");
    assert_eq!(gen11[2].target, "PSA.124.8");
}

#[test]
fn xrefs_drops_unparseable_target_and_counts() {
    let (_map, stats) = atlas_etl::xrefs::parse(include_str!("fixtures/xrefs-sample.txt")).unwrap();
    assert_eq!(stats.dropped_unparseable, 1, "{stats:?}"); // "NotARealRef"
    assert_eq!(stats.dropped_self, 1, "{stats:?}"); // Gen.1.1 -> Gen.1.1
}

#[test]
fn xrefs_canonicalizes_same_chapter_and_cross_chapter_spans() {
    let (map, _stats) = atlas_etl::xrefs::parse(include_str!("fixtures/xrefs-sample.txt")).unwrap();
    let col = map.get("COL.1.16").expect("Col.1.16 row");
    assert_eq!(col[0].target, "COL.1.16-19"); // same chapter: shorthand

    let matt = map.get("MAT.5.3").expect("Matt.5.3 row");
    assert_eq!(matt[0].target, "MAT.5.3-MAT.6.2"); // cross-chapter: full repeat

    let rev = map.get("REV.22.21").expect("Rev.22.21 row");
    assert_eq!(rev[0].votes, -2); // negative votes kept
    assert_eq!(rev[0].target, "ROM.16.23");
}

#[test]
fn xrefs_filter_missing_first_verse_drops_and_counts() {
    let mut map: HashMap<String, Vec<CrossRef>> = HashMap::new();
    map.insert(
        "GEN.1.1".to_string(),
        vec![
            CrossRef { target: "PSA.124.8".to_string(), votes: 20 },
            CrossRef { target: "REV.22.21".to_string(), votes: 5 },
        ],
    );
    let mut verses: HashMap<String, String> = HashMap::new();
    verses.insert("PSA.124.8".to_string(), "text".to_string());
    // REV.22.21 is deliberately absent from the compiled KJV verses map.

    let (filtered, dropped) = atlas_etl::xrefs::filter_missing_first_verse(map, &verses);
    assert_eq!(dropped, 1);
    assert_eq!(filtered["GEN.1.1"].len(), 1);
    assert_eq!(filtered["GEN.1.1"][0].target, "PSA.124.8");
}

// ---------------------------------------------------------------------
// curated.rs
// ---------------------------------------------------------------------

#[test]
fn curated_parsers_handle_valid_toml_and_expand_verse_ranges() {
    let eras = atlas_etl::curated::parse_eras(include_str!("fixtures/eras-sample.toml")).unwrap();
    assert_eq!(eras.len(), 2);
    assert_eq!(eras[0].id, "primeval");
    assert_eq!(eras[0].from_year, -4004);

    let books = atlas_etl::curated::parse_books(include_str!("fixtures/books-sample.toml")).unwrap();
    assert_eq!(books.len(), 2);
    assert_eq!(books[0].book, "GEN");
    assert_eq!(books[0].write_place.as_deref(), Some("some-place"));
    assert_eq!(books[1].write_place, None);

    let narrative = atlas_etl::curated::parse_narrative(include_str!("fixtures/narrative-sample.toml")).unwrap();
    assert_eq!(narrative.id, "exodus");
    assert_eq!(narrative.legs, vec!["ex_1".to_string(), "ex_2".to_string()]);

    let events = atlas_etl::curated::parse_events_extra(include_str!("fixtures/events-extra-sample.toml")).unwrap();
    assert_eq!(events.len(), 2);
    let red_sea = events.iter().find(|e| e.id == "ex_2").unwrap();
    // "EXO.14.21-31" is a curator convenience range; ETL expands it to individual
    // canonical verse ids so validate::run's non-canon-verse-id check accepts it.
    assert_eq!(red_sea.verses.len(), 11, "{:?}", red_sea.verses);
    assert!(red_sea.verses.contains(&"EXO.14.21".to_string()));
    assert!(red_sea.verses.contains(&"EXO.14.31".to_string()));
}

#[test]
fn events_extra_zero_year_hard_errors() {
    let err = atlas_etl::curated::parse_events_extra(include_str!("fixtures/event-year-zero.toml")).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("zero"), "{err}");
}

// ---------------------------------------------------------------------
// Batch T2 (general-kind PASSAGEs -- requirement 2's own promotion rule):
// `kind = "general"` is now REAL curated data, not just a modeled-but-
// unused enum value. A general-kind event has no defensible date/place,
// so `from_year`/`to_year`/`places` become OPTIONAL in the curated TOML
// (never hand-typed by a curator for a general-kind row -- the parser
// itself supplies `TimeRange::undated()`, never a curator-authored
// number, so "do not fabricate a date" is enforced structurally, not by
// convention). `kind = "event"` (the default when the field is absent,
// unchanged back-compat) still requires both, exactly as before.
// ---------------------------------------------------------------------

#[test]
fn events_extra_general_kind_parses_without_places_or_dates() {
    let events = atlas_etl::curated::parse_events_extra(include_str!("fixtures/events-extra-general.toml")).unwrap();
    assert_eq!(events.len(), 1);
    let g = &events[0];
    assert_eq!(g.kind, "general");
    assert!(g.places.is_empty(), "{:?}", g.places);
    assert_eq!(g.when, TimeRange::undated(), "a general-kind event's `when` must be the undated sentinel, never a curator-typed number");
    assert_eq!(g.verses, vec!["MAT.27.1".to_string()]);
}

#[test]
fn events_extra_general_kind_with_from_year_hard_errors() {
    // Fabrication guard: a curator writing `kind = "general"` AND a
    // from_year/to_year together is almost certainly a mistake (either the
    // kind or the date is wrong) -- fail loud rather than silently pick one.
    let err = atlas_etl::curated::parse_events_extra(include_str!("fixtures/event-general-with-year.toml")).unwrap_err();
    let msg = err.to_string().to_lowercase();
    assert!(msg.contains("general"), "{err}");
    assert!(msg.contains("from_year") || msg.contains("date"), "{err}");
}

#[test]
fn events_extra_general_kind_with_places_hard_errors() {
    let err = atlas_etl::curated::parse_events_extra(include_str!("fixtures/event-general-with-places.toml")).unwrap_err();
    let msg = err.to_string().to_lowercase();
    assert!(msg.contains("general"), "{err}");
    assert!(msg.contains("place"), "{err}");
}

// ---------------------------------------------------------------------
// Batch T2 (Acts provenance -- owner's own ambiguity ruling: "acts
// sections get their own provenance key, NOT robertson_section"): a new,
// flat, event_id-keyed curated file (data/curated/acts-sections.toml),
// parsed the SAME way event-witnesses.toml already is -- see
// `curated::parse_event_witnesses`'s own doc comment for why this flat
// shape carries no nested-table mis-attachment risk. Merged onto the
// FULL combined event set (Theographic + events-extra.toml) by
// `main.rs`, the identical mechanism event-witnesses.toml already uses
// (so it can target a bare Theographic event directly, with no
// events-extra.toml duplication).
// ---------------------------------------------------------------------

#[test]
fn acts_sections_parses_a_flat_event_id_keyed_list() {
    let sections = atlas_etl::curated::parse_acts_sections(include_str!("fixtures/acts-sections-sample.toml")).unwrap();
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].0, "theo-999");
    assert!(sections[0].1.contains("Acts 1:4-5"));
    assert_eq!(sections[1].0, "theo-998");
    assert!(sections[1].1.contains("Acts 2:1-13"));
}

#[test]
fn events_extra_event_kind_missing_from_year_hard_errors() {
    // `kind` absent (defaults to "event", unchanged back-compat) still
    // requires from_year/to_year -- the new Option<i32> plumbing must not
    // silently relax the pre-existing event-kind requirement.
    let err = atlas_etl::curated::parse_events_extra(include_str!("fixtures/event-missing-year.toml")).unwrap_err();
    let msg = err.to_string().to_lowercase();
    assert!(msg.contains("from_year") || msg.contains("to_year") || msg.contains("date"), "{err}");
}

// ---------------------------------------------------------------------
// validate.rs
// ---------------------------------------------------------------------

fn empty_atlas() -> AtlasData {
    AtlasData::new(
        Canon { books: vec![] },
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
        HashMap::new(),
        HashMap::new(),
    )
    .finish()
}

fn full_eras() -> Vec<Era> {
    let rows: [(&str, &str, i32, i32); 10] = [
        ("primeval", "Primeval", -4004, -2167),
        ("patriarchs", "Patriarchs", -2166, -1877),
        ("egypt-exodus", "Egypt & Exodus", -1876, -1407),
        ("conquest-judges", "Conquest & Judges", -1406, -1051),
        ("united-kingdom", "United Kingdom", -1050, -932),
        ("divided-kingdom", "Divided Kingdom", -931, -587),
        ("exile", "Exile", -586, -539),
        ("return", "Return & Second Temple", -538, -6),
        ("gospels", "Gospels", -5, 29),
        ("early-church", "Early Church", 30, 100),
    ];
    rows.iter()
        .map(|(id, name, from_year, to_year)| Era { id: id.to_string(), name: name.to_string(), from_year: *from_year, to_year: *to_year })
        .collect()
}

#[test]
fn validate_dangling_narrative_leg_fails() {
    let narrative = atlas_etl::curated::parse_narrative(include_str!("fixtures/narrative-dangling-leg.toml")).unwrap();
    let mut data = empty_atlas();
    data.narratives = vec![narrative]; // legs=["ghost_event"], but data.events is empty
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().contains("dangling leg"), "{err}");
}

#[test]
fn validate_era_gap_fails() {
    let eras = atlas_etl::curated::parse_eras(include_str!("fixtures/eras-gap.toml")).unwrap();
    let mut data = empty_atlas();
    data.eras = eras;
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    let msg = err.to_string();
    assert!(msg.to_lowercase().contains("gap"), "{msg}");
    assert!(msg.to_lowercase().contains("zero"), "{msg}");
}

#[test]
fn validate_era_coverage_bounds_fail() {
    // Internally contiguous (no gap: next_year(-1) == 1) but doesn't span [-4004,100].
    let eras = vec![
        Era { id: "a".into(), name: "A".into(), from_year: -100, to_year: -1 },
        Era { id: "b".into(), name: "B".into(), from_year: 1, to_year: 50 },
    ];
    let mut data = empty_atlas();
    data.eras = eras;
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("coverage"), "{err}");
}

#[test]
fn validate_nonchronological_legs_fail() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![
        Event { id: "e1".into(), label: "E1".into(), when: TimeRange::new(-5, -5).unwrap(), places: vec!["p".into()], verses: vec![], ..Default::default() },
        Event { id: "e2".into(), label: "E2".into(), when: TimeRange::new(-9, -9).unwrap(), places: vec!["p".into()], verses: vec![], ..Default::default() },
    ];
    let narratives = vec![Narrative { id: "n".into(), name: "N".into(), color: "#fff".into(), legs: vec!["e1".into(), "e2".into()] }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.narratives = narratives;
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("chronolog"), "{err}");
}

#[test]
fn validate_duplicate_event_ids_fail() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![
        Event { id: "dup".into(), label: "A".into(), when: TimeRange::new(-5, -5).unwrap(), places: vec!["p".into()], verses: vec![], ..Default::default() },
        Event { id: "dup".into(), label: "B".into(), when: TimeRange::new(-3, -3).unwrap(), places: vec!["p".into()], verses: vec![], ..Default::default() },
    ];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("duplicate"), "{err}");
}

#[test]
fn validate_noncanon_verse_link_on_place_fails() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec!["NOT.A.VERSE".into()] }];
    let mut data = empty_atlas();
    data.places = places;
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("verse"), "{err}");
}

#[test]
fn validate_unknown_place_in_event_fails() {
    let events = atlas_etl::curated::parse_events_extra(include_str!("fixtures/event-bad-place.toml")).unwrap();
    let mut data = empty_atlas();
    data.events = events;
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("unknown place"), "{err}");
}

#[test]
fn validate_valid_data_passes() {
    let places = vec![
        Place { id: "gilgal".into(), name: "Gilgal".into(), lat: 31.9, lon: 35.45, verse_links: vec!["GEN.1.1".into()] },
        Place { id: "jericho".into(), name: "Jericho".into(), lat: 31.87, lon: 35.44, verse_links: vec![] },
    ];
    let events = vec![
        Event { id: "e1".into(), label: "E1".into(), when: TimeRange::new(-1406, -1406).unwrap(), places: vec!["gilgal".into()], verses: vec!["JOS.4.19".into()], ..Default::default() },
        Event { id: "e2".into(), label: "E2".into(), when: TimeRange::new(-1405, -1405).unwrap(), places: vec!["jericho".into()], verses: vec!["JOS.6.1".into()], ..Default::default() },
    ];
    let narratives = vec![Narrative { id: "n".into(), name: "N".into(), color: "#fff".into(), legs: vec!["e1".into(), "e2".into()] }];
    let books_meta = vec![BookMeta { book: "GEN".into(), author: "Moses".into(), write_place: None, write_from: None, write_to: None }];
    let mut verses = HashMap::new();
    verses.insert("GEN.1.1".to_string(), "In the beginning...".to_string());

    let data = AtlasData::new(Canon { books: vec![] }, places, events, narratives, full_eras(), books_meta, verses, HashMap::new()).finish();
    assert!(atlas_etl::validate::run(&data).is_ok(), "{:?}", atlas_etl::validate::run(&data).err());
}

// ---------------------------------------------------------------------
// Batch T2 fix-round-1 (review finding I-1): the exact invariant whose
// violation caused the real 72-event silent-heading-drop bug (fixed in
// data only by commit 9679583) -- `heading_anchors_for`
// (atlas-core/src/data.rs) uses ONLY `e.witnesses` once any witness row
// exists, never falling back to `e.verses`, so a top-level book with no
// matching witness row silently loses its own reader heading and
// PARALLEL ACCOUNTS entry. This reproduces the historical 72-row shape
// directly: an event witnessed only for MRK, but whose own top-level
// `verses` also touch MAT -- exactly the "primary citation in book X,
// explicit witness rows for OTHER books, no witness row for X itself"
// pattern every one of the 72 affected events shared.
// ---------------------------------------------------------------------

#[test]
fn validate_event_witness_missing_for_own_top_level_book_fails() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![Event {
        id: "e1".into(),
        label: "E1".into(),
        when: TimeRange::new(33, 33).unwrap(),
        places: vec!["p".into()],
        verses: vec!["MAT.27.1".into()], // top-level book MAT ...
        witnesses: vec![witness("MRK", &["MRK.15.1"])], // ... but only MRK has a witness row
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.verses = mat_verses();
    data.eras = full_eras();
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("no witness row covers"), "{msg}");
    assert!(msg.contains("'MAT'"), "{msg}");
    assert!(msg.contains("e1"), "{msg}");
}

#[test]
fn validate_event_witness_covers_own_top_level_book_passes() {
    // Same shape as the failing case above, EXCEPT MAT now also has its
    // own explicit witness row (the actual fix commit 9679583 applied) --
    // must pass.
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![Event {
        id: "e1".into(),
        label: "E1".into(),
        when: TimeRange::new(33, 33).unwrap(),
        places: vec!["p".into()],
        verses: vec!["MAT.27.1".into()],
        witnesses: vec![witness("MAT", &["MAT.27.1"]), witness("MRK", &["MRK.15.1"])],
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.verses = mat_verses();
    data.eras = full_eras();
    let data = data.finish();
    assert!(atlas_etl::validate::run(&data).is_ok(), "{:?}", atlas_etl::validate::run(&data).err());
}

#[test]
fn validate_event_with_no_witnesses_and_top_level_verses_only_passes() {
    // The common, pre-existing single-implicit-witness shape (no
    // `[[witness]]` rows at all) -- this check must never fire when
    // `e.witnesses` is empty; unchanged, still legal.
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![Event {
        id: "e1".into(),
        label: "E1".into(),
        when: TimeRange::new(33, 33).unwrap(),
        places: vec!["p".into()],
        verses: vec!["MAT.27.1".into()],
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.verses = mat_verses();
    data.eras = full_eras();
    let data = data.finish();
    assert!(atlas_etl::validate::run(&data).is_ok(), "{:?}", atlas_etl::validate::run(&data).err());
}

// ---------------------------------------------------------------------
// Batch T requirement 1: PASSAGE/EVENT data model validation --
// date-outside-span, kind enum, witness book/verse/overlap checks, and
// requirement 2's order_key chronological-leg tiebreak.
// ---------------------------------------------------------------------

fn mat_verses() -> HashMap<String, String> {
    // A handful of real-shaped (not necessarily literal) verse keys under
    // one book/chapter, enough for the "exists in the compiled KJV text"
    // half of every witness-verse check below.
    let mut verses = HashMap::new();
    for v in 1..=10 {
        verses.insert(format!("MAT.27.{v}"), format!("Verse {v} text."));
    }
    for v in 1..=10 {
        verses.insert(format!("MRK.15.{v}"), format!("Verse {v} text."));
    }
    verses
}

#[test]
fn validate_event_date_before_atlas_span_fails() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![Event {
        id: "e1".into(),
        label: "Too early".into(),
        when: TimeRange::new(-5000, -5000).unwrap(), // outside [-4004,100]
        places: vec!["p".into()],
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().contains("outside"), "{err}");
}

#[test]
fn validate_event_date_after_atlas_span_fails() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![Event {
        id: "e1".into(),
        label: "Too late".into(),
        when: TimeRange::new(200, 200).unwrap(), // outside [-4004,100]
        places: vec!["p".into()],
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().contains("outside"), "{err}");
}

#[test]
fn validate_event_invalid_kind_fails() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![Event {
        id: "e1".into(),
        label: "E1".into(),
        when: TimeRange::new(-5, -5).unwrap(),
        places: vec!["p".into()],
        kind: "bogus".into(),
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().contains("invalid kind"), "{err}");
}

fn witness(book: &str, verses: &[&str]) -> EventWitness {
    EventWitness {
        book: book.into(),
        translations: HashMap::from([("kjv".to_string(), verses.iter().map(|s| s.to_string()).collect())]),
        ref_note: Some("test fixture".into()),
        robertson_section: None,
    }
}

#[test]
fn validate_event_witness_unknown_book_code_fails() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![Event {
        id: "e1".into(),
        label: "E1".into(),
        when: TimeRange::new(33, 33).unwrap(),
        places: vec!["p".into()],
        witnesses: vec![witness("XYZ", &["MAT.27.1"])],
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.verses = mat_verses();
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().contains("not a real canonical book code"), "{err}");
}

#[test]
fn validate_event_witness_zero_verses_fails() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![Event {
        id: "e1".into(),
        label: "E1".into(),
        when: TimeRange::new(33, 33).unwrap(),
        places: vec!["p".into()],
        witnesses: vec![witness("MAT", &[])],
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.verses = mat_verses();
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().contains("zero verses"), "{err}");
}

#[test]
fn validate_event_witness_noncanonical_verse_fails() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![Event {
        id: "e1".into(),
        label: "E1".into(),
        when: TimeRange::new(33, 33).unwrap(),
        places: vec!["p".into()],
        witnesses: vec![witness("MAT", &["not-a-ref"])],
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.verses = mat_verses();
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().contains("not a canonical single-verse ref"), "{err}");
}

#[test]
fn validate_event_witness_verse_missing_from_compiled_kjv_text_fails() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![Event {
        id: "e1".into(),
        label: "E1".into(),
        when: TimeRange::new(33, 33).unwrap(),
        places: vec!["p".into()],
        witnesses: vec![witness("MAT", &["MAT.27.999"])], // parses canonically, but not in mat_verses()
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.verses = mat_verses();
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().contains("does not exist in the compiled KJV text"), "{err}");
}

#[test]
fn validate_event_witness_overlapping_ranges_in_the_same_book_fails() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![Event {
        id: "e1".into(),
        label: "E1".into(),
        when: TimeRange::new(33, 33).unwrap(),
        places: vec!["p".into()],
        witnesses: vec![witness("MAT", &["MAT.27.1", "MAT.27.2", "MAT.27.3"]), witness("MAT", &["MAT.27.3", "MAT.27.4"])],
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.verses = mat_verses();
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().contains("overlapping verse ranges"), "{err}");
}

#[test]
fn validate_event_witnesses_valid_multi_book_data_passes() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![Event {
        id: "e1".into(),
        label: "E1".into(),
        when: TimeRange::new(33, 33).unwrap(),
        places: vec!["p".into()],
        witnesses: vec![witness("MAT", &["MAT.27.1", "MAT.27.2"]), witness("MRK", &["MRK.15.1", "MRK.15.2"])],
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.verses = mat_verses();
    data.eras = full_eras();
    let data = data.finish();
    assert!(atlas_etl::validate::run(&data).is_ok(), "{:?}", atlas_etl::validate::run(&data).err());
}

// ---------------------------------------------------------------------
// Batch T2: within-layer anchor collisions (owner's own ruling: "Robertson
// sections within one Gospel should partition, not collide with each
// other -- a within-layer anchor collision is a curation error your
// validation must catch"). Distinct from `heading_precedence`'s own
// collision RESOLUTION (data.rs's `heading_collision_tests` module,
// tier-1/2/3) -- that mechanism decisively picks a winner for DISPLAY and
// is fine with a real-container-vs-narrative-leg-freebie collision (e.g.
// `pw_bethany`/`jm_bethany`, both real curated data). This check instead
// fails LOUD, at ETL time, whenever two DIFFERENT real (curated
// `witnesses` non-empty and/or `robertson_section` present) containers
// anchor the identical verse -- that specific shape should never happen
// for correctly-partitioned curated sections, so silently resolving it
// would hide a real curation mistake.
// ---------------------------------------------------------------------

#[test]
fn validate_two_real_containers_sharing_an_anchor_verse_fails() {
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![
        Event {
            id: "sect_a".into(),
            label: "Section A".into(),
            when: TimeRange::new(33, 33).unwrap(),
            places: vec!["p".into()],
            verses: vec!["MAT.27.1".into(), "MAT.27.2".into()],
            robertson_section: Some("Robertson (1922) §1".into()),
            ..Default::default()
        },
        Event {
            id: "sect_b".into(),
            label: "Section B (mis-drawn boundary)".into(),
            when: TimeRange::new(33, 33).unwrap(),
            places: vec!["p".into()],
            verses: vec!["MAT.27.1".into()], // same first verse as sect_a -- a partition error
            robertson_section: Some("Robertson (1922) §2".into()),
            ..Default::default()
        },
    ];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.verses = mat_verses();
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    let msg = err.to_string().to_lowercase();
    assert!(msg.contains("anchor"), "{err}");
    assert!(err.to_string().contains("sect_a") && err.to_string().contains("sect_b"), "{err}");
}

#[test]
fn validate_freebie_and_real_container_sharing_anchor_is_allowed() {
    // The EXISTING, legal shape (pw_bethany/jm_bethany, fix-round-1): a
    // bare narrative-leg-only event (no witnesses/robertson_section --
    // heading-worthy only via the freebie rule) sharing an anchor with a
    // REAL curated container is NOT a within-layer collision (only one
    // side is layer-1) -- must keep passing after this batch's new check.
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![
        Event {
            id: "bare".into(),
            label: "Bare freebie".into(),
            when: TimeRange::new(33, 33).unwrap(),
            places: vec!["p".into()],
            verses: vec!["MAT.27.1".into()],
            ..Default::default()
        },
        Event {
            id: "rich".into(),
            label: "Real curated container".into(),
            when: TimeRange::new(33, 33).unwrap(),
            places: vec!["p".into()],
            verses: vec!["MAT.27.1".into()],
            robertson_section: Some("Robertson (1922) §1".into()),
            ..Default::default()
        },
    ];
    let narratives = vec![Narrative { id: "n".into(), name: "N".into(), color: "#fff".into(), legs: vec!["bare".into()] }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.narratives = narratives;
    data.verses = mat_verses();
    data.eras = full_eras();
    let data = data.finish();
    assert!(atlas_etl::validate::run(&data).is_ok(), "{:?}", atlas_etl::validate::run(&data).err());
}

#[test]
fn validate_general_kind_event_with_undated_sentinel_passes() {
    let events = vec![Event {
        id: "gen1".into(),
        label: "A general passage".into(),
        when: TimeRange::undated(),
        places: vec![],
        verses: vec!["MAT.27.1".into()],
        kind: "general".into(),
        robertson_section: Some("Robertson (1922) §1".into()),
        ..Default::default()
    }];
    let mut data = empty_atlas();
    data.events = events;
    data.verses = mat_verses();
    data.eras = full_eras();
    let data = data.finish();
    assert!(atlas_etl::validate::run(&data).is_ok(), "{:?}", atlas_etl::validate::run(&data).err());
}

#[test]
fn validate_narrative_legs_order_key_tiebreak_fails_when_reversed() {
    // Two events sharing the SAME year -- year alone can't order them, so
    // the leg array's own order must match order_key. Here it doesn't:
    // legs = [e1 (order_key=20), e2 (order_key=10)] is a REVERSED tiebreak.
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![
        Event { id: "e1".into(), label: "E1".into(), when: TimeRange::new(33, 33).unwrap(), places: vec!["p".into()], order_key: 20, ..Default::default() },
        Event { id: "e2".into(), label: "E2".into(), when: TimeRange::new(33, 33).unwrap(), places: vec!["p".into()], order_key: 10, ..Default::default() },
    ];
    let narratives = vec![Narrative { id: "n".into(), name: "N".into(), color: "#fff".into(), legs: vec!["e1".into(), "e2".into()] }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.narratives = narratives;
    let data = data.finish();
    let err = atlas_etl::validate::run(&data).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("chronolog"), "{err}");
    assert!(err.to_string().contains("order_key"), "{err}");
}

#[test]
fn validate_narrative_legs_order_key_tiebreak_passes_when_ascending() {
    // Same same-year pair, correctly ordered by order_key this time.
    let places = vec![Place { id: "p".into(), name: "P".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
    let events = vec![
        Event { id: "e1".into(), label: "E1".into(), when: TimeRange::new(33, 33).unwrap(), places: vec!["p".into()], order_key: 10, ..Default::default() },
        Event { id: "e2".into(), label: "E2".into(), when: TimeRange::new(33, 33).unwrap(), places: vec!["p".into()], order_key: 20, ..Default::default() },
    ];
    let narratives = vec![Narrative { id: "n".into(), name: "N".into(), color: "#fff".into(), legs: vec!["e1".into(), "e2".into()] }];
    let mut data = empty_atlas();
    data.places = places;
    data.events = events;
    data.narratives = narratives;
    data.eras = full_eras();
    let data = data.finish();
    assert!(atlas_etl::validate::run(&data).is_ok(), "{:?}", atlas_etl::validate::run(&data).err());
}

// ---------------------------------------------------------------------
// landmarks (curated::parse_landmarks + validate::run_landmarks)
// ---------------------------------------------------------------------

#[test]
fn landmarks_valid_toml_parses_and_validates() {
    let landmarks = atlas_etl::curated::parse_landmarks(include_str!("fixtures/landmarks-sample.toml")).unwrap();
    assert_eq!(landmarks.len(), 3);
    assert!(atlas_etl::validate::run_landmarks(&landmarks, &atlas_etl::polities::BIBLICAL_WORLD_BBOX).is_ok());
}

#[test]
fn landmarks_bad_kind_fails_validation() {
    let landmarks = atlas_etl::curated::parse_landmarks(include_str!("fixtures/landmarks-bad-kind.toml")).unwrap();
    let err = atlas_etl::validate::run_landmarks(&landmarks, &atlas_etl::polities::BIBLICAL_WORLD_BBOX).unwrap_err();
    assert!(err.to_string().contains("invalid kind"), "{err}");
}

// Batch C2: the optional far-field `size` hint (sm/md/lg) is enum-checked
// the same way `kind` already is -- see Landmark::size's own doc comment.
#[test]
fn landmarks_bad_size_fails_validation() {
    let landmarks = atlas_etl::curated::parse_landmarks(include_str!("fixtures/landmarks-bad-size.toml")).unwrap();
    let err = atlas_etl::validate::run_landmarks(&landmarks, &atlas_etl::polities::BIBLICAL_WORLD_BBOX).unwrap_err();
    assert!(err.to_string().contains("invalid size"), "{err}");
}

#[test]
fn landmarks_out_of_bbox_fails_validation() {
    let landmarks = atlas_etl::curated::parse_landmarks(include_str!("fixtures/landmarks-out-of-bbox.toml")).unwrap();
    let err = atlas_etl::validate::run_landmarks(&landmarks, &atlas_etl::polities::BIBLICAL_WORLD_BBOX).unwrap_err();
    assert!(err.to_string().contains("outside the clip bbox"), "{err}");
}

// ---------------------------------------------------------------------
// polities (Batch B2: curated::parse_polity + validate::run_polities)
// ---------------------------------------------------------------------

fn square_ring(south: f64, west: f64, north: f64, east: f64) -> Vec<(f64, f64)> {
    vec![(south, west), (south, east), (north, east), (north, west), (south, west)]
}

fn test_bbox() -> atlas_etl::polities::Bbox {
    atlas_etl::polities::Bbox { south: 0.0, north: 50.0, west: 0.0, east: 50.0 }
}

// Batch M requirement 1: a tiny compiled-KJV-text stand-in covering exactly
// the verses `polities-sample.toml`'s own [era.transition]/[era.fall]
// blocks cite, so `polities_valid_toml_parses_and_validates` below exercises
// the REAL "does this verse exist" path (not just "does it parse
// canonically") the same way every other real caller does.
fn test_verses() -> HashMap<String, String> {
    HashMap::from([
        ("GEN.1.1".to_string(), "In the beginning God created the heaven and the earth.".to_string()),
        ("GEN.1.2".to_string(), "And the earth was without form, and void...".to_string()),
        ("GEN.1.3".to_string(), "And God said, Let there be light: and there was light.".to_string()),
    ])
}

fn one_era_polity(id: &str, name: &str, from: i32, to: i32, ring: Vec<(f64, f64)>) -> Polity {
    // Fix round 1 (M1): color_key is no longer a per-id hash reachable from
    // outside the crate (see `polities::assign_color_keys`'s own doc
    // comment) -- these fixtures only exercise `validate::run_polities`,
    // which never reads color_key at all, so a plain 0 (matching this
    // file's own other polity fixtures below, e.g. `polities_overlapping_
    // eras_fail_validation`) is exactly as good as a real hash here.
    Polity {
        id: id.into(),
        color_key: 0,
        eras: vec![PolityEra { name: name.into(), from, to, ref_note: "fixture".into(), rings: vec![ring], transition: None, fall: None }],
    }
}

#[test]
fn polities_valid_toml_parses_and_validates() {
    let polity = atlas_etl::curated::parse_polity(include_str!("fixtures/polities-sample.toml")).unwrap();
    assert!(atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).is_ok());
}

#[test]
fn polities_zero_year_fails_validation() {
    let polity = one_era_polity("z", "Z", 0, 100, square_ring(10.0, 10.0, 20.0, 20.0));
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("zero"), "{err}");
}

#[test]
fn polities_inverted_range_fails_validation() {
    let polity = one_era_polity("i", "I", 100, -100, square_ring(10.0, 10.0, 20.0, 20.0));
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("inverted"), "{err}");
}

#[test]
fn polities_year_outside_atlas_span_fails_validation() {
    let polity = one_era_polity("o", "O", -5000, -4500, square_ring(10.0, 10.0, 20.0, 20.0));
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().contains("outside [-4004,100]"), "{err}");
}

#[test]
fn polities_overlapping_eras_fail_validation() {
    let polity = Polity {
        id: "overlap".into(),
        color_key: 0,
        eras: vec![
            PolityEra { name: "A".into(), from: -1000, to: -500, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)], transition: None, fall: None },
            PolityEra { name: "B".into(), from: -600, to: -100, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)], transition: None, fall: None },
        ],
    };
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().contains("overlaps"), "{err}");
}

#[test]
fn polities_adjacent_non_overlapping_eras_pass_validation() {
    let polity = Polity {
        id: "adjacent".into(),
        color_key: 0,
        eras: vec![
            PolityEra { name: "A".into(), from: -1000, to: -500, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)], transition: None, fall: None },
            PolityEra { name: "B".into(), from: -499, to: -100, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)], transition: None, fall: None },
        ],
    };
    assert!(atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).is_ok());
}

#[test]
fn polities_unclosed_ring_fails_validation() {
    let mut ring = square_ring(10.0, 10.0, 20.0, 20.0);
    ring.pop(); // drop the closing repeat -- no longer a closed ring
    let polity = one_era_polity("u", "U", -1000, -500, ring);
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().contains("not a closed ring"), "{err}");
}

#[test]
fn polities_self_intersecting_ring_fails_validation() {
    // The same "bowtie" shape polities.rs's own ring_is_simple unit test
    // uses -- verifying the validator actually WIRES that checker in, not
    // just that the checker itself works in isolation.
    let bowtie = vec![(10.0, 10.0), (20.0, 20.0), (20.0, 10.0), (10.0, 20.0), (10.0, 10.0)];
    let polity = one_era_polity("b", "B", -1000, -500, bowtie);
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().contains("self-intersects"), "{err}");
}

#[test]
fn polities_out_of_bbox_point_fails_validation() {
    let polity = one_era_polity("far", "Far", -1000, -500, square_ring(60.0, 60.0, 70.0, 70.0)); // outside test_bbox()'s 0..50
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().contains("outside the clip bbox"), "{err}");
}

#[test]
fn polities_duplicate_id_fails_validation() {
    let a = one_era_polity("dup", "A", -1000, -500, square_ring(10.0, 10.0, 20.0, 20.0));
    let b = one_era_polity("dup", "B", -400, -100, square_ring(10.0, 10.0, 20.0, 20.0));
    let err = atlas_etl::validate::run_polities(&[a, b], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().contains("duplicate"), "{err}");
}

// --- Batch M requirement 1: [era.transition]/[era.fall] delta validation ---

fn one_era_polity_with_transition(id: &str, name: &str, from: i32, to: i32, transition: PolityDelta) -> Polity {
    let mut p = one_era_polity(id, name, from, to, square_ring(10.0, 10.0, 20.0, 20.0));
    p.eras[0].transition = Some(transition);
    p
}

#[test]
fn polities_transition_with_real_verses_passes_validation() {
    let polity = one_era_polity_with_transition(
        "t",
        "T",
        -1000,
        -500,
        PolityDelta { event: "T rises".into(), verses: vec!["GEN.1.1".into()], ref_note: "fixture".into(), for_era_from: -1000 },
    );
    assert!(atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).is_ok());
}

#[test]
fn polities_transition_empty_event_fails_validation() {
    let polity = one_era_polity_with_transition(
        "t",
        "T",
        -1000,
        -500,
        PolityDelta { event: "".into(), verses: vec![], ref_note: "fixture".into(), for_era_from: -1000 },
    );
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().contains("event is empty"), "{err}");
}

#[test]
fn polities_transition_empty_ref_note_fails_validation() {
    let polity = one_era_polity_with_transition(
        "t",
        "T",
        -1000,
        -500,
        PolityDelta { event: "T rises".into(), verses: vec![], ref_note: "".into(), for_era_from: -1000 },
    );
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().contains("ref_note is empty"), "{err}");
}

#[test]
fn polities_transition_verse_not_in_compiled_kjv_fails_validation() {
    let polity = one_era_polity_with_transition(
        "t",
        "T",
        -1000,
        -500,
        PolityDelta { event: "T rises".into(), verses: vec!["GEN.99.99".into()], ref_note: "fixture".into(), for_era_from: -1000 },
    );
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().contains("does not exist in the compiled KJV text"), "{err}");
}

#[test]
fn polities_transition_malformed_verse_fails_validation() {
    let polity = one_era_polity_with_transition(
        "t",
        "T",
        -1000,
        -500,
        PolityDelta { event: "T rises".into(), verses: vec!["not a ref".into()], ref_note: "fixture".into(), for_era_from: -1000 },
    );
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().contains("not a canonical single-verse ref"), "{err}");
}

#[test]
fn polities_fall_on_the_final_era_passes_validation() {
    let mut polity = Polity {
        id: "faller".into(),
        color_key: 0,
        eras: vec![
            PolityEra { name: "A".into(), from: -1000, to: -500, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)], transition: None, fall: None },
            PolityEra { name: "B".into(), from: -499, to: -100, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)], transition: None, fall: None },
        ],
    };
    polity.eras[1].fall = Some(PolityDelta { event: "B falls".into(), verses: vec![], ref_note: "fixture".into(), for_era_from: -499 });
    assert!(atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).is_ok());
}

#[test]
fn polities_fall_on_a_non_final_era_fails_validation() {
    let mut polity = Polity {
        id: "wrongfall".into(),
        color_key: 0,
        eras: vec![
            PolityEra { name: "A".into(), from: -1000, to: -500, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)], transition: None, fall: None },
            PolityEra { name: "B".into(), from: -499, to: -100, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)], transition: None, fall: None },
        ],
    };
    // "A" is NOT this polity's chronologically final era ("B" ends later) --
    // a [era.fall] there describes an end that isn't actually this polity's
    // own end, so it must fail loud rather than silently compile.
    // for_era_from correctly names A (-1000) here -- this fixture is
    // deliberately isolating the "wrong final era" check from the
    // "wrong for_era_from" check below, not conflating the two failure
    // reasons in one assertion.
    polity.eras[0].fall = Some(PolityDelta { event: "A falls (wrong)".into(), verses: vec![], ref_note: "fixture".into(), for_era_from: -1000 });
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().contains("is not this polity's chronologically final era"), "{err}");
}

// Fix round 1 (I1): the ACTUAL historical mis-attachment bug, reproduced as
// a fixture. TOML's array-of-tables rule attaches a nested
// `[era.transition]` table to whichever `[[era]]` was MOST RECENTLY OPENED
// -- a block textually authored to describe era B's own rise (placed
// before B's own `[[era]]` header, with a comment saying so) actually lands
// on era A's own struct field instead. `for_era_from` is the curator's own
// declared intent (B's own `from`); the era it's ACTUALLY attached to (A)
// has a different `from` -- this is precisely what shipped silently once
// (7 of 22 deltas, this batch's own self-review) with nothing structural
// catching it before this check existed. Confirmed red-then-green: this
// test fails to compile/panics with a missing-field error before
// `for_era_from` existed on `PolityDelta`, and fails validation (not a
// panic) once the field exists but before this check was added; green only
// once both the field and the check are in place together.
#[test]
fn polities_transition_for_era_from_mismatch_fails_validation() {
    let mut polity = Polity {
        id: "misattached".into(),
        color_key: 0,
        eras: vec![
            PolityEra { name: "A".into(), from: -1000, to: -500, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)], transition: None, fall: None },
            PolityEra { name: "B".into(), from: -499, to: -100, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)], transition: None, fall: None },
        ],
    };
    // Authored FOR era B's own rise (for_era_from = B's own from, -499) but
    // sitting on era A's own struct field -- exactly the shape a misplaced
    // [era.transition] block parses into.
    polity.eras[0].transition = Some(PolityDelta { event: "B rises (misattached)".into(), verses: vec![], ref_note: "fixture".into(), for_era_from: -499 });
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox(), &test_verses()).unwrap_err();
    assert!(err.to_string().contains("does not match this era's own from"), "{err}");
    assert!(err.to_string().contains("for_era_from=-499"), "{err}");
}

// ---------------------------------------------------------------------
// land-mask (Batch R requirement 1: curated::parse_land_mask + validate::run_land_mask)
// ---------------------------------------------------------------------

#[test]
fn land_mask_valid_toml_parses_and_validates() {
    let regions = atlas_etl::curated::parse_land_mask(include_str!("fixtures/land-mask-sample.toml")).unwrap();
    assert_eq!(regions.len(), 2);
    assert_eq!(regions[0].name, "Testland coast");
    assert_eq!(regions[0].rings[0][0], (10.0, 10.0), "rings are [lat, lon], first pair verbatim");
    assert!(atlas_etl::validate::run_land_mask(&regions, &test_bbox()).is_ok());
}

#[test]
fn land_mask_empty_regions_fail_validation() {
    let err = atlas_etl::validate::run_land_mask(&[], &test_bbox()).unwrap_err();
    assert!(err.to_string().contains("no regions"), "{err}");
}

#[test]
fn land_mask_unclosed_ring_fails_validation() {
    let mut ring = square_ring(10.0, 10.0, 20.0, 20.0);
    ring.pop();
    let region = LandMaskRegion { name: "u".into(), ref_note: "fixture".into(), rings: vec![ring] };
    let err = atlas_etl::validate::run_land_mask(&[region], &test_bbox()).unwrap_err();
    assert!(err.to_string().contains("not a closed ring"), "{err}");
}

#[test]
fn land_mask_self_intersecting_ring_fails_validation() {
    let bowtie = vec![(10.0, 10.0), (20.0, 20.0), (20.0, 10.0), (10.0, 20.0), (10.0, 10.0)];
    let region = LandMaskRegion { name: "b".into(), ref_note: "fixture".into(), rings: vec![bowtie] };
    let err = atlas_etl::validate::run_land_mask(&[region], &test_bbox()).unwrap_err();
    assert!(err.to_string().contains("self-intersects"), "{err}");
}

#[test]
fn land_mask_out_of_bbox_point_fails_validation() {
    let region = LandMaskRegion { name: "far".into(), ref_note: "fixture".into(), rings: vec![square_ring(60.0, 60.0, 70.0, 70.0)] }; // outside test_bbox()'s 0..50
    let err = atlas_etl::validate::run_land_mask(&[region], &test_bbox()).unwrap_err();
    assert!(err.to_string().contains("outside the clip bbox"), "{err}");
}

// ---------------------------------------------------------------------
// catechism (Batch F: curated.rs::parse_catechism + validate.rs::run_catechism)
// ---------------------------------------------------------------------

#[test]
fn catechism_valid_toml_parses_and_validates() {
    let parts = atlas_etl::curated::parse_catechism(include_str!("fixtures/catechism-sample.toml")).unwrap();
    assert_eq!(parts.len(), 2);
    let commandments = parts.iter().find(|p| p.id == "ten-commandments").unwrap();
    assert_eq!(commandments.items.len(), 2);
    let close = commandments.items.iter().find(|i| i.id == "commandments-close").unwrap();
    assert_eq!(close.verses, vec!["EXO.20.5".to_string(), "EXO.20.6".to_string()]);

    let baptism = parts.iter().find(|p| p.id == "baptism").unwrap();
    let b1 = &baptism.items[0];
    assert_eq!(b1.text, None);
    assert_eq!(b1.explanation_heading, "What is Baptism?");

    let mut verses = HashMap::new();
    verses.insert("EXO.20.5".to_string(), "text".to_string());
    verses.insert("EXO.20.6".to_string(), "text".to_string());
    verses.insert("MAT.28.19".to_string(), "text".to_string());
    assert!(atlas_etl::validate::run_catechism(&parts, &verses).is_ok(), "{:?}", atlas_etl::validate::run_catechism(&parts, &verses).err());
}

fn catechism_item(id: &str, name: &str, verses: &[&str]) -> atlas_core::data::CatechismItem {
    atlas_core::data::CatechismItem {
        id: id.into(),
        name: name.into(),
        text: None,
        explanation_heading: "What does this mean?".into(),
        explanation: "E".into(),
        where_written: None,
        verses: verses.iter().map(|v| v.to_string()).collect(),
        ref_note: None,
        questions: Vec::new(),
    }
}

/// PARTS-1: the two fields `CatechismPart` gained default the same way
/// `catechism.toml`'s own parts do -- curated, with no part-level
/// questions -- so a test that predates them reads unchanged.
fn catechism_part(id: &str, title: &str, items: Vec<atlas_core::data::CatechismItem>) -> atlas_core::data::CatechismPart {
    atlas_core::data::CatechismPart { id: id.into(), title: title.into(), items, questions: Vec::new(), curated: true }
}

fn catechism_question(title: &str, verses: &[&str], source: &str) -> atlas_core::data::CatechismQuestion {
    atlas_core::data::CatechismQuestion {
        title: title.into(),
        verses: verses.iter().map(|v| v.to_string()).collect(),
        source: source.into(),
    }
}

#[test]
fn catechism_duplicate_part_id_fails_validation() {
    let parts = vec![
        catechism_part("dup", "A", vec![catechism_item("a1", "A1", &[])]),
        catechism_part("dup", "B", vec![catechism_item("b1", "B1", &[])]),
    ];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("duplicate catechism part"), "{err}");
}

#[test]
fn catechism_duplicate_item_id_across_different_parts_fails_validation() {
    // Item ids are looked up GLOBALLY (AtlasData::catechism_item_by_id is
    // not scoped to a part), so a collision across two DIFFERENT parts must
    // be caught too, not just within one part.
    let parts = vec![
        catechism_part("p1", "P1", vec![catechism_item("shared", "A", &[])]),
        catechism_part("p2", "P2", vec![catechism_item("shared", "B", &[])]),
    ];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("duplicate catechism item"), "{err}");
}

#[test]
fn catechism_empty_part_fails_validation() {
    let parts = vec![catechism_part("empty", "Empty", vec![])];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("neither items nor part-level questions"), "{err}");
}

/// PARTS-1: the other half of the widened law -- a part carrying only
/// part-level questions is VALID. Daily Prayers and the Table of Duties
/// are real sections of the Small Catechism that Luther never divided
/// into numbered items, so requiring items would have made them
/// unrepresentable.
#[test]
fn catechism_part_with_questions_but_no_items_passes_validation() {
    let mut part = catechism_part("daily-prayers", "Daily Prayers", vec![]);
    part.curated = false;
    part.questions = vec![catechism_question("Morning Prayer", &["PSA.5.3"], "brain-fuel/catechism")];
    let verses = HashMap::from([("PSA.5.3".to_string(), "My voice shalt thou hear in the morning".to_string())]);
    atlas_etl::validate::run_catechism(&[part], &verses).expect("items are no longer the only way a part carries content");
}

#[test]
fn catechism_verse_missing_from_compiled_kjv_text_fails_validation() {
    let parts = vec![catechism_part("p", "P", vec![catechism_item("i1", "I1", &["GEN.99.99"])])];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("does not exist in the compiled KJV text"), "{err}");
}

#[test]
fn catechism_non_canonical_verse_fails_validation() {
    let parts = vec![catechism_part("p", "P", vec![catechism_item("i1", "I1", &["NOT.A.VERSE"])])];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("not a canonical single-verse ref"), "{err}");
}

// --- Batch F2: run_catechism's own question-level checks --------------

#[test]
fn catechism_question_with_verse_missing_from_compiled_text_fails_validation() {
    let mut item = catechism_item("i1", "I1", &[]);
    item.questions = vec![catechism_question("Q1", &["GEN.99.99"], "brain-fuel/catechism")];
    let parts = vec![catechism_part("p", "P", vec![item])];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("does not exist in the compiled KJV text"), "{err}");
    assert!(err.to_string().contains("Q1"), "{err}");
}

#[test]
fn catechism_question_with_non_canonical_verse_fails_validation() {
    let mut item = catechism_item("i1", "I1", &[]);
    item.questions = vec![catechism_question("Q1", &["NOT.A.VERSE"], "brain-fuel/catechism")];
    let parts = vec![catechism_part("p", "P", vec![item])];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("not a canonical single-verse ref"), "{err}");
}

#[test]
fn catechism_question_with_zero_verses_fails_validation() {
    let mut item = catechism_item("i1", "I1", &[]);
    item.questions = vec![catechism_question("Q1", &[], "brain-fuel/catechism")];
    let parts = vec![catechism_part("p", "P", vec![item])];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("has zero verses"), "{err}");
}

#[test]
fn catechism_valid_questions_pass_validation() {
    let mut verses = HashMap::new();
    verses.insert("MAT.28.19".to_string(), "text".to_string());
    let mut item = catechism_item("i1", "I1", &[]);
    item.questions = vec![catechism_question("Q1", &["MAT.28.19"], "brain-fuel/catechism")];
    let parts = vec![catechism_part("p", "P", vec![item])];
    assert!(atlas_etl::validate::run_catechism(&parts, &verses).is_ok());
}

// ---------------------------------------------------------------------
// osis.rs
// ---------------------------------------------------------------------

#[test]
fn osis_parse_verse_resolves_abbreviations() {
    let v = atlas_etl::osis::parse_verse("1Kgs.17.1").expect("should parse");
    assert_eq!(v.book.code(), "1KI");
    assert_eq!(v.chapter, 17);
    assert_eq!(v.verse, 1);
    assert_eq!(atlas_etl::osis::canonical(&v), "1KI.17.1");
    assert!(atlas_etl::osis::parse_verse("Zzz.1.1").is_none());
}

// ---------------------------------------------------------------------
// report.rs
// ---------------------------------------------------------------------

#[test]
fn report_contains_expected_sections() {
    let report = atlas_etl::report::Report {
        counts: atlas_etl::report::Counts {
            canon_books: 66,
            places: 10,
            events: 5,
            narratives: 2,
            eras: 10,
            books_meta: 66,
            verses: 100,
            cross_ref_sources: 3,
            people: 3067,
        },
        pct_events_dated: 80.0,
        pct_verses_geocoded: 12.5,
        narrative_leg_counts: vec![("exodus".to_string(), 10)],
        slug_collisions: vec!["antioch -> antioch, antioch-2".to_string()],
        warnings: vec!["books.toml: unknown write_place 'nope' on GEN, dropped".to_string()],
        xref_dropped_unparseable: 1,
        xref_dropped_self: 1,
        xref_dropped_missing_first_verse: 2,
        polities: vec![atlas_etl::report::PolityStats { id: "egypt".to_string(), eras: 4, points: 180 }],
        landmarks_count: 19,
        land_mask_regions: 6,
        land_mask_rings: 6,
        land_mask_points: 135,
        catechism_parts: 6,
        catechism_items: 33,
        catechism_noncurated_parts: 4,
        catechism_part_questions: 79,
        catechism_items_reachable: 30,
        catechism_distinct_verses: 210,
        catechism_per_part: vec![("The Ten Commandments".to_string(), 11, 11)],
    };
    let text = atlas_etl::report::write(&report);
    assert!(text.contains("66"), "{text}");
    assert!(text.contains("exodus"), "{text}");
    assert!(text.contains("antioch-2"), "{text}");
    assert!(text.contains("unknown write_place"), "{text}");
    assert!(text.contains("egypt"), "{text}"); // polity id
    assert!(text.contains("4 era(s)"), "{text}");
    assert!(text.contains("180 points"), "{text}");
    assert!(text.contains("19 curated landmarks"), "{text}");
    assert!(text.contains("6 region(s), 6 ring(s), 135 points"), "{text}");
    assert!(text.contains("6 chief part(s), 33 item(s) total"), "{text}");
    assert!(text.contains("30/33 items reachable"), "{text}");
    assert!(text.contains("210 distinct verse(s)"), "{text}");
    assert!(text.contains("The Ten Commandments: 11/11 reachable"), "{text}");
    assert!(text.contains("3067"), "{text}"); // Batch P: people count
}

// ---------------------------------------------------------------------
// place-history (Batch E: curated.rs::parse_place_history + validate.rs::run_place_history)
// ---------------------------------------------------------------------

#[test]
fn place_history_valid_toml_parses_names_blurbs_and_dates() {
    let history = atlas_etl::curated::parse_place_history(include_str!("fixtures/place-history-sample.toml")).unwrap();
    assert_eq!(history.len(), 2);

    let bethel = history.iter().find(|h| h.id == "bethel-1").unwrap();
    assert_eq!(bethel.names.len(), 2);
    assert_eq!(bethel.names[0].name, "Luz");
    assert_eq!(bethel.names[0].when, TimeRange::new(-4004, -2092).unwrap());
    assert_eq!(bethel.names[0].verses, vec!["GEN.28.19".to_string()]);
    assert_eq!(bethel.blurbs.len(), 1);
    assert_eq!(bethel.blurbs[0].breadth, "era");

    let jerusalem = history.iter().find(|h| h.id == "jerusalem").unwrap();
    let established = jerusalem.established.as_ref().unwrap();
    assert_eq!(established.when, TimeRange::new(-1003, -1003).unwrap()); // `year = -1003` shorthand
    assert_eq!(established.note.as_deref(), Some("traditional"));
    let destroyed = jerusalem.destroyed.as_ref().unwrap();
    assert_eq!(destroyed.when, TimeRange::new(-586, -586).unwrap()); // `from`/`to` shape, equal endpoints
    assert_eq!(destroyed.verses, vec!["2KI.25.9".to_string(), "2KI.25.10".to_string()]);
}

#[test]
fn place_history_zero_year_hard_errors() {
    let err = atlas_etl::curated::parse_place_history(include_str!("fixtures/place-history-year-zero.toml")).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("zero"), "{err}");
}

#[test]
fn place_history_ambiguous_established_date_hard_errors() {
    let err = atlas_etl::curated::parse_place_history(include_str!("fixtures/place-history-ambiguous-date.toml")).unwrap_err();
    assert!(err.to_string().contains("BOTH"), "{err}");
}

#[test]
fn place_history_missing_established_date_hard_errors() {
    let err = atlas_etl::curated::parse_place_history(include_str!("fixtures/place-history-missing-date.toml")).unwrap_err();
    assert!(err.to_string().contains("needs either"), "{err}");
}

fn name_entry(name: &str, from: i32, to: i32, verses: &[&str]) -> PlaceNameEntry {
    PlaceNameEntry { name: name.into(), when: TimeRange::new(from, to).unwrap(), verses: verses.iter().map(|v| v.to_string()).collect() }
}

fn blurb_entry(text: &str, from: i32, to: i32, breadth: &str) -> PlaceBlurbEntry {
    PlaceBlurbEntry { text: text.into(), when: TimeRange::new(from, to).unwrap(), breadth: breadth.into() }
}

fn some_verses() -> HashMap<String, String> {
    let mut v = HashMap::new();
    v.insert("GEN.28.19".to_string(), "And he called the name of that place Bethel...".to_string());
    v.insert("2SA.5.7".to_string(), "Nevertheless David took the strong hold of Zion...".to_string());
    // Batch GAZ-1-R1: the real KJV text (GEN.2.13), not a placeholder --
    // `run_place_names_kjv`'s own new verbatim-substring law needs a real
    // match for `place_names_kjv_valid_data_passes_validation` below.
    v.insert("GEN.2.13".to_string(), "And the name of the second river is Gihon: the same is it that compasseth the whole land of Ethiopia.".to_string());
    v
}

#[test]
fn place_history_unknown_place_id_fails_validation() {
    let history = vec![PlaceHistory { id: "not-a-real-place".into(), names: vec![], blurbs: vec![], established: None, destroyed: None }];
    let place_ids: HashSet<&str> = ["bethel-1", "jerusalem"].into_iter().collect();
    let err = atlas_etl::validate::run_place_history(&history, &place_ids, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("unknown place id"), "{err}");
}

#[test]
fn place_history_non_canonical_verse_fails_validation() {
    let history = vec![PlaceHistory {
        id: "bethel-1".into(),
        names: vec![name_entry("Luz", -4004, -2092, &["NOT.A.VERSE"])],
        blurbs: vec![],
        established: None,
        destroyed: None,
    }];
    let place_ids: HashSet<&str> = ["bethel-1"].into_iter().collect();
    let err = atlas_etl::validate::run_place_history(&history, &place_ids, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("not a canonical single-verse ref"), "{err}");
}

#[test]
fn place_history_verse_missing_from_compiled_kjv_text_fails_validation() {
    // GEN.99.99 parses fine structurally (a real book, positive chapter/verse)
    // but does not exist in the compiled KJV text -- Batch E's own
    // strengthened check (beyond plain `VerseId::parse_canonical`) catches it.
    let history = vec![PlaceHistory {
        id: "bethel-1".into(),
        names: vec![name_entry("Luz", -4004, -2092, &["GEN.99.99"])],
        blurbs: vec![],
        established: None,
        destroyed: None,
    }];
    let place_ids: HashSet<&str> = ["bethel-1"].into_iter().collect();
    let err = atlas_etl::validate::run_place_history(&history, &place_ids, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("does not exist in the compiled KJV text"), "{err}");
}

#[test]
fn place_history_year_outside_atlas_span_fails_validation() {
    let history = vec![PlaceHistory {
        id: "bethel-1".into(),
        names: vec![name_entry("Luz", -5000, -2092, &["GEN.28.19"])], // -5000 < -4004
        blurbs: vec![],
        established: None,
        destroyed: None,
    }];
    let place_ids: HashSet<&str> = ["bethel-1"].into_iter().collect();
    let err = atlas_etl::validate::run_place_history(&history, &place_ids, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("outside [-4004,100]"), "{err}");
}

#[test]
fn place_history_overlapping_name_ranges_fail_validation() {
    let history = vec![PlaceHistory {
        id: "bethel-1".into(),
        names: vec![
            name_entry("Luz", -4004, -1900, &["GEN.28.19"]),
            name_entry("Bethel", -2000, 100, &["GEN.28.19"]), // overlaps [-2000,-1900] with Luz above
        ],
        blurbs: vec![],
        established: None,
        destroyed: None,
    }];
    let place_ids: HashSet<&str> = ["bethel-1"].into_iter().collect();
    let err = atlas_etl::validate::run_place_history(&history, &place_ids, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("name ranges") && err.to_string().contains("overlap"), "{err}");
}

#[test]
fn place_history_overlapping_same_breadth_blurbs_fail_validation() {
    let history = vec![PlaceHistory {
        id: "jerusalem".into(),
        names: vec![],
        blurbs: vec![
            blurb_entry("first", -4004, -500, "era"),
            blurb_entry("second", -600, 100, "era"), // overlaps [-600,-500] with "first", same breadth
        ],
        established: None,
        destroyed: None,
    }];
    let place_ids: HashSet<&str> = ["jerusalem"].into_iter().collect();
    let err = atlas_etl::validate::run_place_history(&history, &place_ids, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("'era' blurb ranges overlap"), "{err}");
}

#[test]
fn place_history_blurb_overlap_across_breadths_is_allowed() {
    // A "broad" range is EXPECTED to overlap every "era" range it summarizes
    // -- only same-breadth overlaps are an error (batch-e-brief.md
    // Requirement 2: "blurb ranges may overlap across breadths but not
    // within one breadth").
    let history = vec![PlaceHistory {
        id: "jerusalem".into(),
        names: vec![],
        blurbs: vec![blurb_entry("era one", -4004, -587, "era"), blurb_entry("whole sweep", -4004, 100, "broad")],
        established: None,
        destroyed: None,
    }];
    let place_ids: HashSet<&str> = ["jerusalem"].into_iter().collect();
    assert!(atlas_etl::validate::run_place_history(&history, &place_ids, &some_verses()).is_ok());
}

#[test]
fn place_history_invalid_blurb_breadth_fails_validation() {
    let history = vec![PlaceHistory {
        id: "jerusalem".into(),
        names: vec![],
        blurbs: vec![blurb_entry("oops", -100, -50, "century")], // not "era" or "broad"
        established: None,
        destroyed: None,
    }];
    let place_ids: HashSet<&str> = ["jerusalem"].into_iter().collect();
    let err = atlas_etl::validate::run_place_history(&history, &place_ids, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("invalid breadth"), "{err}");
}

#[test]
fn place_history_duplicate_place_id_fails_validation() {
    let history = vec![
        PlaceHistory { id: "jerusalem".into(), names: vec![], blurbs: vec![], established: None, destroyed: None },
        PlaceHistory { id: "jerusalem".into(), names: vec![], blurbs: vec![], established: None, destroyed: None },
    ];
    let place_ids: HashSet<&str> = ["jerusalem"].into_iter().collect();
    let err = atlas_etl::validate::run_place_history(&history, &place_ids, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("duplicate"), "{err}");
}

#[test]
fn place_history_established_verse_outside_canon_fails_validation() {
    let history = vec![PlaceHistory {
        id: "jerusalem".into(),
        names: vec![],
        blurbs: vec![],
        established: Some(PlaceDateClaim {
            when: TimeRange::new(-1003, -1003).unwrap(),
            verses: vec!["NOT.A.VERSE".into()],
            note: None,
        }),
        destroyed: None,
    }];
    let place_ids: HashSet<&str> = ["jerusalem"].into_iter().collect();
    let err = atlas_etl::validate::run_place_history(&history, &place_ids, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("established") && err.to_string().contains("not a canonical"), "{err}");
}

#[test]
fn place_history_valid_data_passes_validation() {
    let history = vec![PlaceHistory {
        id: "bethel-1".into(),
        names: vec![name_entry("Luz", -4004, -2092, &["GEN.28.19"]), name_entry("Bethel", -2091, 100, &["GEN.28.19"])],
        blurbs: vec![blurb_entry("A patriarchal altar site.", -2091, -1877, "era")],
        established: None,
        destroyed: Some(PlaceDateClaim { when: TimeRange::new(-1003, -1003).unwrap(), verses: vec!["2SA.5.7".into()], note: Some("traditional".into()) }),
    }];
    let place_ids: HashSet<&str> = ["bethel-1"].into_iter().collect();
    let result = atlas_etl::validate::run_place_history(&history, &place_ids, &some_verses());
    assert!(result.is_ok(), "{:?}", result.err());
}

// ---------------------------------------------------------------------
// place-names-kjv (Batch E3: curated.rs::parse_place_names_kjv +
// validate.rs::run_place_names_kjv)
// ---------------------------------------------------------------------

fn cush_place() -> Place {
    Place { id: "cush-2".into(), name: "Cush 2".into(), lat: 32.54, lon: 44.42, verse_links: vec!["GEN.2.13".into()] }
}

fn alias_row(id: &str, kjv_name: &str, verses: &[&str]) -> PlaceNameAlias {
    PlaceNameAlias {
        id: id.into(),
        translations: HashMap::from([("kjv".to_string(), kjv_name.to_string())]),
        verses: verses.iter().map(|v| v.to_string()).collect(),
    }
}

#[test]
fn place_names_kjv_valid_toml_parses_and_wraps_translation() {
    let aliases = atlas_etl::curated::parse_place_names_kjv(include_str!("fixtures/place-names-kjv-sample.toml")).unwrap();
    assert_eq!(aliases.len(), 2);
    let cush = aliases.iter().find(|a| a.id == "cush-2").unwrap();
    assert_eq!(cush.translations.get("kjv").map(String::as_str), Some("Ethiopia"));
    assert_eq!(cush.verses, vec!["GEN.2.13".to_string()]);
}

#[test]
fn place_names_kjv_unknown_place_id_fails_validation() {
    let aliases = vec![alias_row("not-a-real-place", "Ethiopia", &["GEN.2.13"])];
    let places = vec![cush_place()];
    let err = atlas_etl::validate::run_place_names_kjv(&aliases, &places, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("unknown place id"), "{err}");
}

#[test]
fn place_names_kjv_duplicate_alias_id_fails_validation() {
    let aliases = vec![alias_row("cush-2", "Ethiopia", &["GEN.2.13"]), alias_row("cush-2", "Ethiopia", &["GEN.2.13"])];
    let places = vec![cush_place()];
    let err = atlas_etl::validate::run_place_names_kjv(&aliases, &places, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("duplicate"), "{err}");
}

#[test]
fn place_names_kjv_alias_equal_to_canonical_name_fails_validation() {
    // "Cush 2" strips (same ETL disambiguation-suffix rule resolve_display_name
    // itself applies) to "Cush" -- an alias of exactly "Cush" for this place
    // is pure noise, req 1's own named error case.
    let aliases = vec![alias_row("cush-2", "Cush", &["GEN.2.13"])];
    let places = vec![cush_place()];
    let err = atlas_etl::validate::run_place_names_kjv(&aliases, &places, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("equal to") || err.to_string().contains("noise"), "{err}");
}

#[test]
fn place_names_kjv_noncanonical_verse_fails_validation() {
    let aliases = vec![alias_row("cush-2", "Ethiopia", &["NOT.A.VERSE"])];
    let places = vec![cush_place()];
    let err = atlas_etl::validate::run_place_names_kjv(&aliases, &places, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("not a canonical single-verse ref"), "{err}");
}

#[test]
fn place_names_kjv_verse_missing_from_compiled_kjv_text_fails_validation() {
    let aliases = vec![alias_row("cush-2", "Ethiopia", &["GEN.99.99"])];
    let places = vec![cush_place()];
    let err = atlas_etl::validate::run_place_names_kjv(&aliases, &places, &some_verses()).unwrap_err();
    assert!(err.to_string().contains("does not exist in the compiled KJV text"), "{err}");
}

#[test]
fn place_names_kjv_valid_data_passes_validation() {
    // Batch GAZ-1-R1: GEN.2.13 (not GEN.28.19 -- an arbitrary "reuses
    // some_verses()'s own populated verse" pick that predates the
    // verbatim-substring law and never actually said "Ethiopia") --
    // GEN.2.13 is this row's own REAL citation in the real committed
    // data/curated/place-names-kjv.toml, and matches cush_place()'s own
    // verse_links besides.
    let aliases = vec![alias_row("cush-2", "Ethiopia", &["GEN.2.13"])];
    let places = vec![cush_place()];
    let result = atlas_etl::validate::run_place_names_kjv(&aliases, &places, &some_verses());
    assert!(result.is_ok(), "{:?}", result.err());
}

// ---------------------------------------------------------------------
// atlas_core::merge::MERGE_PAIRS + validate::run_place_merges
// (Batch HOTFIX-2 fix-round-1: review findings I-1 -- a bad table entry
// must fail the ETL build loudly, naming the entry, instead of the silent
// no-op `apply_place_merges` itself has to tolerate for idempotence -- and
// I-3 -- the distance re-check must run in every build profile, against
// REAL per-call coordinates, not a hand-copied snapshot baked into a test)
// ---------------------------------------------------------------------

fn merge_place(id: &str, lat: f64, lon: f64) -> Place {
    Place { id: id.into(), name: id.into(), lat, lon, verse_links: vec![] }
}

#[test]
fn run_place_merges_unknown_survivor_id_fails_naming_the_entry() {
    let pairs = [PlaceMerge { survivor: "not-a-real-place", absorbed: "hazor_545", reason: "test" }];
    let places = vec![merge_place("hazor_545", 33.0174, 35.5681)];
    let err = atlas_etl::validate::run_place_merges(&pairs, &places).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("not-a-real-place"), "{msg}");
    assert!(msg.contains("survivor id"), "{msg}");
    assert!(msg.contains("does not exist"), "{msg}");
}

#[test]
fn run_place_merges_unknown_absorbed_id_fails_naming_the_entry() {
    let pairs = [PlaceMerge { survivor: "hazor-1", absorbed: "also-not-real", reason: "test" }];
    let places = vec![merge_place("hazor-1", 33.0183, 35.5692)];
    let err = atlas_etl::validate::run_place_merges(&pairs, &places).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("also-not-real"), "{msg}");
    assert!(msg.contains("absorbed id"), "{msg}");
    assert!(msg.contains("does not exist"), "{msg}");
}

#[test]
fn run_place_merges_over_threshold_pair_fails_validation() {
    // ~2km apart (0.018 deg latitude) -- both ids present and real, so only
    // the DISTANCE check can catch this; closes I-3 (a real per-call
    // coordinate check, not a hand-copied snapshot).
    let pairs = [PlaceMerge { survivor: "a", absorbed: "b", reason: "test" }];
    let places = vec![merge_place("a", 32.735, 35.55555), merge_place("b", 32.753, 35.55555)];
    let err = atlas_etl::validate::run_place_merges(&pairs, &places).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("'a'") && msg.contains("'b'"), "{msg}");
    assert!(msg.to_lowercase().contains("apart"), "{msg}");
    assert!(msg.contains("threshold"), "{msg}");
}

#[test]
fn run_place_merges_valid_pairs_pass_validation() {
    // Today's real two curated pairs' own real coordinates (data/compiled/places.json).
    let pairs = [
        PlaceMerge { survivor: "hazor-1", absorbed: "hazor_545", reason: "test" },
        PlaceMerge { survivor: "kedesh-4", absorbed: "kedesh-naphtali", reason: "test" },
    ];
    let places = vec![
        merge_place("hazor-1", 33.018333, 35.569167),
        merge_place("hazor_545", 33.01746212803129, 35.56813718),
        merge_place("kedesh-4", 32.735, 35.55555),
        merge_place("kedesh-naphtali", 32.735, 35.55555),
    ];
    let result = atlas_etl::validate::run_place_merges(&pairs, &places);
    assert!(result.is_ok(), "{:?}", result.err());
}

// ---------------------------------------------------------------------
// atlas_core::event_merge::EVENT_MERGE_PAIRS/EVENT_DISTINCT_PAIRS +
// validate::run_event_merges (HOTFIX-4 fix round 1, review finding I-2:
// the validator itself had ZERO committed regression tests -- the
// red-then-green test the original batch report cited,
// `event_merge::red_then_green_baptism_pair_collapses_to_one_event_on_the_ad33_scale`,
// exercises `apply_event_merges` (the MERGE mechanism), not
// `run_event_merges` (this VALIDATOR) -- a genuinely different function.
// Mirrors `run_place_merges`'s own quartet immediately above, exactly:
// unknown survivor fails; unknown absorbed fails; an unlisted
// over-threshold near-duplicate pair fails loud naming both ids and the
// jaccard score; listed pairs (both list kinds) pass.
// ---------------------------------------------------------------------

fn freebie_event(id: &str, verses: &[&str]) -> Event {
    Event { id: id.into(), label: id.into(), when: TimeRange::new(1, 1).unwrap(), verses: verses.iter().map(|s| s.to_string()).collect(), ..Default::default() }
}

fn real_event(id: &str, verses: &[&str]) -> Event {
    Event {
        id: id.into(),
        label: id.into(),
        when: TimeRange::new(1, 1).unwrap(),
        verses: verses.iter().map(|s| s.to_string()).collect(),
        robertson_section: Some("test".into()),
        ..Default::default()
    }
}

#[test]
fn run_event_merges_unknown_survivor_id_fails_naming_the_entry() {
    let pairs = [EventMerge { survivor: "not-a-real-event", absorbed: "theo-999", reason: "test" }];
    let events = vec![freebie_event("theo-999", &["MAT.1.1"])];
    let err = atlas_etl::validate::run_event_merges(&pairs, &[], &events).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("not-a-real-event"), "{msg}");
    assert!(msg.contains("survivor id"), "{msg}");
    assert!(msg.contains("does not exist"), "{msg}");
}

#[test]
fn run_event_merges_unknown_absorbed_id_fails_naming_the_entry() {
    let pairs = [EventMerge { survivor: "real-1", absorbed: "also-not-real", reason: "test" }];
    let events = vec![real_event("real-1", &["MAT.1.1"])];
    let err = atlas_etl::validate::run_event_merges(&pairs, &[], &events).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("also-not-real"), "{msg}");
    assert!(msg.contains("absorbed id"), "{msg}");
    assert!(msg.contains("does not exist"), "{msg}");
}

#[test]
fn run_event_merges_over_threshold_unlisted_pair_fails_loud_naming_both_ids_and_jaccard() {
    // Identical verse sets (jaccard 1.000) -- both ids real/present in the
    // event set, so ONLY the unlisted-duplicate sweep can catch this
    // (proves the validator really does find a near-duplicate the curated
    // tables don't mention, the same live proof the review's own temporary
    // probe test made -- now a permanent, committed one).
    let events = vec![freebie_event("theo-1", &["MAT.1.1", "MAT.1.2"]), real_event("real-1", &["MAT.1.1", "MAT.1.2"])];
    let err = atlas_etl::validate::run_event_merges(&[], &[], &events).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("'theo-1'") && msg.contains("'real-1'"), "{msg}");
    assert!(msg.contains("jaccard"), "{msg}");
    assert!(msg.contains("1.000"), "{msg}");
}

#[test]
fn run_event_merges_listed_pairs_pass_validation() {
    // Both list KINDS suppress the sweep -- a merge pair (theo-1/real-1)
    // AND a distinct pair (theo-2/real-2), each otherwise an unlisted
    // jaccard-1.0 near-duplicate the test immediately above would flag.
    let merge_pairs = [EventMerge { survivor: "real-1", absorbed: "theo-1", reason: "test" }];
    let distinct_pairs = [EventDistinct { a: "theo-2", b: "real-2", reason: "test" }];
    let events = vec![
        freebie_event("theo-1", &["MAT.1.1", "MAT.1.2"]),
        real_event("real-1", &["MAT.1.1", "MAT.1.2"]),
        freebie_event("theo-2", &["MRK.1.1", "MRK.1.2"]),
        real_event("real-2", &["MRK.1.1", "MRK.1.2"]),
    ];
    let result = atlas_etl::validate::run_event_merges(&merge_pairs, &distinct_pairs, &events);
    assert!(result.is_ok(), "{:?}", result.err());
}

// ---------------------------------------------------------------------
// Batch W4 fix round 1 (batch-w4-review.md Critical-1's own SYSTEMIC
// GUARD): validate::run_cross_book_duplicates -- the second, orthogonal
// duplicate-identity sweep, mirroring the run_event_merges quartet above.
// Two of the original four (unknown-survivor-id / unknown-absorbed-id) are
// deliberately NOT mirrored here: this validator's own doc comment
// explains why -- dangling-id checking is run_event_merges's own job
// against the SAME merge_pairs/distinct_pairs/events, and re-checking here
// would only ever duplicate that function's errors verbatim.
// ---------------------------------------------------------------------

fn titled_dated_event(id: &str, label: &str, year: i32, place: &str, verses: &[&str]) -> Event {
    Event {
        id: id.into(),
        label: label.into(),
        when: TimeRange::new(year, year).unwrap(),
        places: vec![place.to_string()],
        verses: verses.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

#[test]
fn run_cross_book_duplicates_unlisted_pair_fails_loud_naming_both_ids_and_title_jaccard() {
    // Cross-book shape: identical labels, same year, common place, but
    // DISJOINT verse sets (different books) -- exactly what verse_jaccard
    // (run_event_merges's own sweep, tested above) is structurally blind
    // to, and exactly the shape batch-w4-review.md Critical-1 found live
    // (jer_the_fall_of_jerusalem_retold/exl_jerusalem,
    // jer_jeremiah_stays_with_gedaliah+jer_the_assassination_of_gedaliah/
    // exl_mizpah).
    let events = vec![
        titled_dated_event("a-1", "Gedaliah governs the remnant at Mizpah", -586, "mizpah", &["2KI.25.22"]),
        titled_dated_event("a-2", "Gedaliah governs the remnant at Mizpah", -586, "mizpah", &["JER.40.7"]),
    ];
    let err = atlas_etl::validate::run_cross_book_duplicates(&[], &[], &events).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("'a-1'") && msg.contains("'a-2'"), "{msg}");
    assert!(msg.contains("title jaccard"), "{msg}");
    assert!(msg.contains("1.000"), "{msg}");
}

#[test]
fn run_cross_book_duplicates_listed_pairs_pass_validation() {
    // Both list kinds suppress the sweep here too -- a merge pair (a-1/a-2)
    // AND a distinct pair (b-1/b-2), each otherwise an unlisted
    // title-jaccard-1.0 cross-book candidate the test immediately above
    // would flag.
    let merge_pairs = [EventMerge { survivor: "a-1", absorbed: "a-2", reason: "test" }];
    let distinct_pairs = [EventDistinct { a: "b-1", b: "b-2", reason: "test" }];
    let events = vec![
        titled_dated_event("a-1", "Gedaliah governs the remnant at Mizpah", -586, "mizpah", &["2KI.25.22"]),
        titled_dated_event("a-2", "Gedaliah governs the remnant at Mizpah", -586, "mizpah", &["JER.40.7"]),
        titled_dated_event("b-1", "Paul arrives at Rome", 60, "rome", &["ACT.28.14"]),
        titled_dated_event("b-2", "Paul arrives at Rome", 60, "rome", &["ACT.28.16"]),
    ];
    let result = atlas_etl::validate::run_cross_book_duplicates(&merge_pairs, &distinct_pairs, &events);
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn run_cross_book_duplicates_ignores_a_legitimate_low_title_similarity_neighbor() {
    // Same year, same place, but genuinely different titles (title jaccard
    // well under TITLE_JACCARD_THRESHOLD) -- the "legitimate
    // same-place-same-year neighbor" shape the threshold is tuned not to
    // flood on (see event_merge's own threshold-derivation doc comment;
    // the real curated instance of this shape, ret_susa/
    // neh_nehemiah_hears_report, is documented in EVENT_DISTINCT_PAIRS --
    // this is a synthetic, further-below-threshold pair so the regression
    // doesn't depend on today's real curated titles staying byte-identical).
    let events = vec![
        titled_dated_event("c-1", "Nehemiah hears of Jerusalem's ruin in Susa", -445, "susa", &["NEH.1.1"]),
        titled_dated_event("c-2", "The people gather at Mizpah to mourn", -445, "susa", &["NEH.1.2"]),
    ];
    let result = atlas_etl::validate::run_cross_book_duplicates(&[], &[], &events);
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn run_cross_book_duplicates_ignores_general_kind_events() {
    // Same title/year/place as the positive-detection test above, but both
    // rows are kind="general" -- out of scope for a TIMELINE-node duplicate
    // check by definition (no `when` genuinely comparable). Exercised
    // through the real validator entry point's own dated-event filter, not
    // just cross_book_duplicate_candidate's internal kind guard (belt and
    // suspenders, deliberately -- see run_cross_book_duplicates's own doc
    // comment).
    let mut a = titled_dated_event("d-1", "Gedaliah governs the remnant at Mizpah", -586, "mizpah", &["2KI.25.22"]);
    let mut b = titled_dated_event("d-2", "Gedaliah governs the remnant at Mizpah", -586, "mizpah", &["JER.40.7"]);
    a.kind = "general".into();
    b.kind = "general".into();
    let result = atlas_etl::validate::run_cross_book_duplicates(&[], &[], &[a, b]);
    assert!(result.is_ok(), "{:?}", result.err());
}

// ---------------------------------------------------------------------
// Batch CHRON-1 (THE CHRONOLOGY AUTHORITY LAW's own DIRECT enforcement):
// validate::run_no_two_opinions. Unlike run_event_merges/
// run_cross_book_duplicates above, this validator runs on the POST-merge
// event set and checks PLACEMENT AGREEMENT directly, not "is this pair
// listed" -- red-then-green: two synthetic same-verse events with
// DIFFERENT from_year, neither merged nor distinct-listed, must fail
// (proving the law's own direct enforcement actually catches a live
// violation); the real post-triage EVENT_DISTINCT_PAIRS corpus, and an
// agreeing-placement duplicate, must both pass.
// ---------------------------------------------------------------------

fn placed_event(id: &str, label: &str, from_year: i32, to_year: i32, order_key: i32, verses: &[&str]) -> Event {
    Event {
        id: id.into(),
        label: label.into(),
        when: TimeRange::new(from_year, to_year).unwrap(),
        order_key,
        verses: verses.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

#[test]
fn run_no_two_opinions_red_then_green_planted_violation_fails_loud() {
    // RED: two events, identical verse set (jaccard 1.000, well over the
    // 0.5 threshold), but DIFFERENT from_year -- exactly the leper-pair
    // shape (two ids, two dates, one episode) THE CHRONOLOGY AUTHORITY LAW
    // forbids, planted directly rather than merely asserted.
    let events = vec![
        placed_event("planted-a", "A leper healed", 30, 30, 450, &["MAT.8.2", "MAT.8.3", "MAT.8.4"]),
        placed_event("planted-b", "Healing the Leper", 31, 31, 0, &["MAT.8.2", "MAT.8.3", "MAT.8.4"]),
    ];
    let err = atlas_etl::validate::run_no_two_opinions(&[], &events).unwrap_err();
    let msg = err.to_string();
    // GREEN: the validator actually catches it, naming both ids and the score.
    assert!(msg.contains("'planted-a'") && msg.contains("'planted-b'"), "{msg}");
    assert!(msg.contains("jaccard"), "{msg}");
    assert!(msg.contains("1.000"), "{msg}");
    assert!(msg.contains("DISAGREE"), "{msg}");
}

#[test]
fn run_no_two_opinions_a_distinct_listed_pair_with_disagreeing_placements_passes() {
    // Same shape as the planted violation above, but the pair is
    // EXPLICITLY documented in EVENT_DISTINCT_PAIRS -- a genuinely distinct
    // pair is EXPECTED to keep two independent placements; that is what
    // "distinct" means, and the law's own direct enforcement must not flag it.
    let distinct_pairs = [EventDistinct { a: "planted-a", b: "planted-b", reason: "test: genuinely distinct" }];
    let events = vec![
        placed_event("planted-a", "A leper healed", 30, 30, 450, &["MAT.8.2", "MAT.8.3", "MAT.8.4"]),
        placed_event("planted-b", "Healing the Leper", 31, 31, 0, &["MAT.8.2", "MAT.8.3", "MAT.8.4"]),
    ];
    let result = atlas_etl::validate::run_no_two_opinions(&distinct_pairs, &events);
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn run_no_two_opinions_agreeing_placements_pass_even_though_unlisted() {
    // Heavy overlap, but the SAME placement on both sides -- no conflicting
    // opinion reaches a reader, so this passes without needing an
    // EVENT_DISTINCT_PAIRS entry at all (this validator's own job is
    // narrower than run_event_merges's own "every candidate must be
    // listed" sweep -- it only fails on an actual placement DISAGREEMENT).
    let events = vec![
        placed_event("agree-a", "A leper healed", 30, 30, 450, &["MAT.8.2", "MAT.8.3", "MAT.8.4"]),
        placed_event("agree-b", "Healing the Leper", 30, 30, 450, &["MAT.8.2", "MAT.8.3", "MAT.8.4"]),
    ];
    let result = atlas_etl::validate::run_no_two_opinions(&[], &events);
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn run_no_two_opinions_below_threshold_overlap_passes_regardless_of_placement() {
    // Low overlap (jaccard well under 0.5) with disagreeing placements --
    // not the same episode, so a placement disagreement is expected and
    // not a violation.
    let events = vec![
        placed_event("low-a", "Event A", 30, 30, 0, &["MAT.8.1", "MAT.8.2", "MAT.8.3", "MAT.8.4"]),
        placed_event("low-b", "Event B", 60, 60, 0, &["MAT.8.4", "MRK.1.1", "MRK.1.2", "MRK.1.3"]),
    ];
    let result = atlas_etl::validate::run_no_two_opinions(&[], &events);
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn run_no_two_opinions_ignores_general_kind_events() {
    // kind="general" events carry no comparable `when` by definition
    // (Batch T2's own undated policy) -- out of scope for this check, same
    // "kind == event" gate run_cross_book_duplicates already applies.
    let mut a = placed_event("gen-a", "A leper healed", 30, 30, 450, &["MAT.8.2", "MAT.8.3", "MAT.8.4"]);
    let mut b = placed_event("gen-b", "Healing the Leper", 31, 31, 0, &["MAT.8.2", "MAT.8.3", "MAT.8.4"]);
    a.kind = "general".into();
    b.kind = "general".into();
    let result = atlas_etl::validate::run_no_two_opinions(&[], &[a, b]);
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn run_no_two_opinions_exempts_every_real_distinct_pair() {
    // FIX ROUND 1 (review finding I-7): renamed from
    // `..._passes_on_the_real_post_triage_corpus` -- that name overclaimed
    // what this test actually proves. It builds SYNTHETIC events (fabricated
    // `GEN.{i}.1` verses) one pair per real EVENT_DISTINCT_PAIRS row, so it
    // proves the exemption LOOKUP covers every real entry, not that the
    // real compiled corpus passes this check end to end -- the genuine
    // corpus proof is `cargo run -p atlas-etl` (which calls
    // run_no_two_opinions against the real post-merge data and DID run,
    // clean, at this batch's own recompile step), not this unit test. Every
    // pair this module's own module doc documents as genuinely distinct
    // must be exempt from the direct, placement-based check (not just the
    // pairwise sweep's own "is it listed" bar) -- proven here via synthetic
    // events matching each real distinct pair's own id/placement shape
    // rather than the full compiled corpus, so this proves the FUNCTION's
    // own exemption lookup works against every real entry, not a stale
    // subset.
    use atlas_core::event_merge::EVENT_DISTINCT_PAIRS;
    let mut events = Vec::new();
    for (i, pair) in EVENT_DISTINCT_PAIRS.iter().enumerate() {
        let verses: Vec<String> = vec![format!("GEN.{}.{}", i + 1, 1)];
        events.push(Event {
            id: pair.a.into(),
            label: pair.a.into(),
            when: TimeRange::new(1, 1).unwrap(),
            order_key: 1,
            verses: verses.clone(),
            ..Default::default()
        });
        events.push(Event {
            id: pair.b.into(),
            label: pair.b.into(),
            when: TimeRange::new(2, 2).unwrap(), // disagreeing placement -- would fail if not exempt
            order_key: 2,
            verses,
            ..Default::default()
        });
    }
    let result = atlas_etl::validate::run_no_two_opinions(EVENT_DISTINCT_PAIRS, &events);
    assert!(result.is_ok(), "{:?}", result.err());
}

// ---------------------------------------------------------------------
// Batch HOTFIX-6 (graph-wide chronology audit): validate::run_chronology_anchors
// / run_chronology_windows / run_era_boundaries. The first two mirror the
// established quartet shape (run_place_merges/run_event_merges above)
// exactly. run_era_boundaries needs a full AtlasData (it reasons about
// GLOBAL TIMELINE POSITION, not raw years, unlike the other two) -- its own
// tests build one directly via AtlasData::new(...).finish().
// ---------------------------------------------------------------------

fn chronology_event(id: &str, label: &str, year: i32, book: &str, chapter: u16, verse: u16) -> Event {
    Event { id: id.into(), label: label.into(), when: TimeRange::new(year, year).unwrap(), verses: vec![format!("{book}.{chapter}.{verse}")], ..Default::default() }
}

fn anchor(id: &str, year: i32, event_id: Option<&str>, era_boundary: bool) -> ChronologyAnchor {
    ChronologyAnchor { id: id.into(), label: id.into(), year, event_id: event_id.map(String::from), era_boundary, source: "test".into(), note: None }
}

fn window(book: &str, from: i32, to: i32) -> BookNarrationWindow {
    BookNarrationWindow { book: book.into(), from_year: from, to_year: to, note: None }
}

#[test]
fn run_chronology_anchors_unknown_event_id_fails_naming_it() {
    let anchors = [anchor("a1", 100, Some("not-a-real-event"), false)];
    let events = vec![chronology_event("e1", "e1", 100, "MAT", 1, 1)];
    let err = atlas_etl::validate::run_chronology_anchors(&anchors, &events).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("not-a-real-event"), "{msg}");
    assert!(msg.contains("does not exist"), "{msg}");
}

#[test]
fn run_chronology_anchors_era_boundary_without_event_id_fails() {
    let anchors = [anchor("a1", 100, None, true)];
    let err = atlas_etl::validate::run_chronology_anchors(&anchors, &[]).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("a1"), "{msg}");
    assert!(msg.contains("era_boundary"), "{msg}");
}

#[test]
fn run_chronology_anchors_duplicate_id_fails() {
    let anchors = [anchor("a1", 100, None, false), anchor("a1", 200, None, false)];
    let err = atlas_etl::validate::run_chronology_anchors(&anchors, &[]).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("duplicate"), "{msg}");
    assert!(msg.contains("a1"), "{msg}");
}

#[test]
fn run_chronology_anchors_valid_table_passes() {
    let anchors = [anchor("a1", 100, Some("e1"), true)];
    let events = vec![chronology_event("e1", "e1", 100, "MAT", 1, 1)];
    let result = atlas_etl::validate::run_chronology_anchors(&anchors, &events);
    assert!(result.is_ok(), "{:?}", result.err());
}

// --- run_chronology_anchor_equality (fix round 2, review finding I-2) ---

#[test]
fn run_chronology_anchor_equality_matching_table_and_event_passes() {
    let anchors = [anchor("a1", 100, Some("e1"), false)];
    let events = vec![chronology_event("e1", "e1", 100, "MAT", 1, 1)];
    let result = atlas_etl::validate::run_chronology_anchor_equality(&anchors, &events);
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn run_chronology_anchor_equality_disagreeing_table_and_event_fails_naming_both_years() {
    let anchors = [anchor("nehemiah-wall", -454, Some("ret_jerusalem_wall"), true)];
    let events = vec![chronology_event("ret_jerusalem_wall", "Nehemiah completes Jerusalem's wall", -444, "NEH", 6, 15)];
    let err = atlas_etl::validate::run_chronology_anchor_equality(&anchors, &events).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("nehemiah-wall"), "{msg}");
    assert!(msg.contains("-454"), "{msg}");
    assert!(msg.contains("-444"), "{msg}");
}

#[test]
fn run_chronology_anchor_equality_is_a_noop_for_an_unbound_anchor() {
    let anchors = [anchor("exodus", -1491, None, false)];
    let result = atlas_etl::validate::run_chronology_anchor_equality(&anchors, &[]);
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn run_chronology_windows_event_outside_its_book_window_fails_naming_it() {
    // The owner's own case, synthetically reproduced: df_ramah's own
    // pre-fix -1014 outside 1SA's own -1171..-1055 window.
    let events = vec![chronology_event("df_ramah", "David flees to Ramah", -1014, "1SA", 19, 18)];
    let windows = [window("1SA", -1171, -1055)];
    let err = atlas_etl::validate::run_chronology_windows(&events, &windows).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("df_ramah"), "{msg}");
    assert!(msg.contains("1SA"), "{msg}");
}

#[test]
fn run_chronology_windows_missing_window_for_a_cited_book_fails_naming_it() {
    let events = vec![chronology_event("e1", "e1", 100, "ROM", 1, 1)];
    let err = atlas_etl::validate::run_chronology_windows(&events, &[]).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("ROM"), "{msg}");
    assert!(msg.contains("no BookNarrationWindow"), "{msg}");
}

#[test]
fn run_chronology_windows_recounting_chapter_citation_never_flags() {
    // theo-7-shaped: correctly dated deep in Genesis-era history, citing a
    // genealogy chapter (1 Chronicles 1) far outside 1 Chronicles's own
    // tight window -- must pass via RECOUNTING_CHAPTERS, not fail.
    let events = vec![chronology_event("theo-7", "Birth of Seth", -3874, "1CH", 1, 1)];
    let windows = [window("1CH", -1062, -1015)];
    let result = atlas_etl::validate::run_chronology_windows(&events, &windows);
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn run_chronology_windows_in_bounds_events_pass() {
    // df_ramah's own post-fix -1062, inside 1SA's own window.
    let events = vec![chronology_event("df_ramah", "David flees to Ramah", -1062, "1SA", 19, 18)];
    let windows = [window("1SA", -1171, -1055)];
    let result = atlas_etl::validate::run_chronology_windows(&events, &windows);
    assert!(result.is_ok(), "{:?}", result.err());
}

fn atlas_with_chronology(events: Vec<Event>, anchors: Vec<ChronologyAnchor>, windows: Vec<BookNarrationWindow>) -> AtlasData {
    let mut d = AtlasData::new(Canon::default(), vec![], events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
    d.chronology_anchors = anchors;
    d.book_narration_windows = windows;
    d
}

#[test]
fn run_era_boundaries_event_on_the_wrong_side_fails_naming_it_and_the_boundary() {
    // Boundary "conquest-begins" at year -1406 (bound to real event "b1");
    // "e1" is JDG-witnessed (JDG's own window entirely AFTER -1406) but its
    // OWN year (-1500) sorts BEFORE the boundary on the timeline -- the same
    // shape theo-124's own pre-exemption -1521 "Lifetime of Joshua" hit,
    // found live by this batch's own full audit.
    let events = vec![chronology_event("b1", "boundary event", -1406, "EXO", 1, 1), chronology_event("e1", "wrong-side event", -1500, "JDG", 3, 1)];
    let anchors = vec![anchor("conquest-begins", -1406, Some("b1"), true)];
    let windows = vec![window("EXO", -1600, -1400), window("JDG", -1400, -1100)];
    let d = atlas_with_chronology(events, anchors, windows);
    let err = atlas_etl::validate::run_era_boundaries(&d).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("e1"), "{msg}");
    assert!(msg.contains("conquest-begins"), "{msg}");
}

#[test]
fn run_era_boundaries_events_on_the_correct_side_pass() {
    let events = vec![chronology_event("b1", "boundary event", -1406, "EXO", 1, 1), chronology_event("e1", "correct-side event", -1300, "JDG", 3, 1)];
    let anchors = vec![anchor("conquest-begins", -1406, Some("b1"), true)];
    let windows = vec![window("EXO", -1600, -1400), window("JDG", -1400, -1100)];
    let d = atlas_with_chronology(events, anchors, windows);
    let result = atlas_etl::validate::run_era_boundaries(&d);
    assert!(result.is_ok(), "{:?}", result.err());
}
