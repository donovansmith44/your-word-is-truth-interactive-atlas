use std::collections::HashMap;

use atlas_core::data::{AtlasData, BookMeta, Canon, CrossRef, Era, Event, Narrative, Place};
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
        Event { id: "e1".into(), label: "E1".into(), when: TimeRange::new(-5, -5).unwrap(), places: vec!["p".into()], verses: vec![] },
        Event { id: "e2".into(), label: "E2".into(), when: TimeRange::new(-9, -9).unwrap(), places: vec!["p".into()], verses: vec![] },
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
        Event { id: "dup".into(), label: "A".into(), when: TimeRange::new(-5, -5).unwrap(), places: vec!["p".into()], verses: vec![] },
        Event { id: "dup".into(), label: "B".into(), when: TimeRange::new(-3, -3).unwrap(), places: vec!["p".into()], verses: vec![] },
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
        Event { id: "e1".into(), label: "E1".into(), when: TimeRange::new(-1406, -1406).unwrap(), places: vec!["gilgal".into()], verses: vec!["JOS.4.19".into()] },
        Event { id: "e2".into(), label: "E2".into(), when: TimeRange::new(-1405, -1405).unwrap(), places: vec!["jericho".into()], verses: vec!["JOS.6.1".into()] },
    ];
    let narratives = vec![Narrative { id: "n".into(), name: "N".into(), color: "#fff".into(), legs: vec!["e1".into(), "e2".into()] }];
    let books_meta = vec![BookMeta { book: "GEN".into(), author: "Moses".into(), write_place: None, write_from: None, write_to: None }];
    let mut verses = HashMap::new();
    verses.insert("GEN.1.1".to_string(), "In the beginning...".to_string());

    let data = AtlasData::new(Canon { books: vec![] }, places, events, narratives, full_eras(), books_meta, verses, HashMap::new()).finish();
    assert!(atlas_etl::validate::run(&data).is_ok(), "{:?}", atlas_etl::validate::run(&data).err());
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
        },
        pct_events_dated: 80.0,
        pct_verses_geocoded: 12.5,
        narrative_leg_counts: vec![("exodus".to_string(), 10)],
        slug_collisions: vec!["antioch -> antioch, antioch-2".to_string()],
        warnings: vec!["books.toml: unknown write_place 'nope' on GEN, dropped".to_string()],
        xref_dropped_unparseable: 1,
        xref_dropped_self: 1,
        xref_dropped_missing_first_verse: 2,
    };
    let text = atlas_etl::report::write(&report);
    assert!(text.contains("66"), "{text}");
    assert!(text.contains("exodus"), "{text}");
    assert!(text.contains("antioch-2"), "{text}");
    assert!(text.contains("unknown write_place"), "{text}");
}
