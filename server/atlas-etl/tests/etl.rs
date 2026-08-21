use std::collections::{HashMap, HashSet};

use atlas_core::data::{AtlasData, BookMeta, Canon, CrossRef, Era, Event, LandMaskRegion, Narrative, Place, PlaceBlurbEntry, PlaceDateClaim, PlaceHistory, PlaceNameEntry, Polity, PolityEra};
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
        eras: vec![PolityEra { name: name.into(), from, to, ref_note: "fixture".into(), rings: vec![ring] }],
    }
}

#[test]
fn polities_valid_toml_parses_and_validates() {
    let polity = atlas_etl::curated::parse_polity(include_str!("fixtures/polities-sample.toml")).unwrap();
    assert!(atlas_etl::validate::run_polities(&[polity], &test_bbox()).is_ok());
}

#[test]
fn polities_zero_year_fails_validation() {
    let polity = one_era_polity("z", "Z", 0, 100, square_ring(10.0, 10.0, 20.0, 20.0));
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox()).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("zero"), "{err}");
}

#[test]
fn polities_inverted_range_fails_validation() {
    let polity = one_era_polity("i", "I", 100, -100, square_ring(10.0, 10.0, 20.0, 20.0));
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox()).unwrap_err();
    assert!(err.to_string().to_lowercase().contains("inverted"), "{err}");
}

#[test]
fn polities_year_outside_atlas_span_fails_validation() {
    let polity = one_era_polity("o", "O", -5000, -4500, square_ring(10.0, 10.0, 20.0, 20.0));
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox()).unwrap_err();
    assert!(err.to_string().contains("outside [-4004,100]"), "{err}");
}

#[test]
fn polities_overlapping_eras_fail_validation() {
    let polity = Polity {
        id: "overlap".into(),
        color_key: 0,
        eras: vec![
            PolityEra { name: "A".into(), from: -1000, to: -500, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)] },
            PolityEra { name: "B".into(), from: -600, to: -100, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)] },
        ],
    };
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox()).unwrap_err();
    assert!(err.to_string().contains("overlaps"), "{err}");
}

#[test]
fn polities_adjacent_non_overlapping_eras_pass_validation() {
    let polity = Polity {
        id: "adjacent".into(),
        color_key: 0,
        eras: vec![
            PolityEra { name: "A".into(), from: -1000, to: -500, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)] },
            PolityEra { name: "B".into(), from: -499, to: -100, ref_note: "fixture".into(), rings: vec![square_ring(10.0, 10.0, 20.0, 20.0)] },
        ],
    };
    assert!(atlas_etl::validate::run_polities(&[polity], &test_bbox()).is_ok());
}

#[test]
fn polities_unclosed_ring_fails_validation() {
    let mut ring = square_ring(10.0, 10.0, 20.0, 20.0);
    ring.pop(); // drop the closing repeat -- no longer a closed ring
    let polity = one_era_polity("u", "U", -1000, -500, ring);
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox()).unwrap_err();
    assert!(err.to_string().contains("not a closed ring"), "{err}");
}

#[test]
fn polities_self_intersecting_ring_fails_validation() {
    // The same "bowtie" shape polities.rs's own ring_is_simple unit test
    // uses -- verifying the validator actually WIRES that checker in, not
    // just that the checker itself works in isolation.
    let bowtie = vec![(10.0, 10.0), (20.0, 20.0), (20.0, 10.0), (10.0, 20.0), (10.0, 10.0)];
    let polity = one_era_polity("b", "B", -1000, -500, bowtie);
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox()).unwrap_err();
    assert!(err.to_string().contains("self-intersects"), "{err}");
}

#[test]
fn polities_out_of_bbox_point_fails_validation() {
    let polity = one_era_polity("far", "Far", -1000, -500, square_ring(60.0, 60.0, 70.0, 70.0)); // outside test_bbox()'s 0..50
    let err = atlas_etl::validate::run_polities(&[polity], &test_bbox()).unwrap_err();
    assert!(err.to_string().contains("outside the clip bbox"), "{err}");
}

#[test]
fn polities_duplicate_id_fails_validation() {
    let a = one_era_polity("dup", "A", -1000, -500, square_ring(10.0, 10.0, 20.0, 20.0));
    let b = one_era_polity("dup", "B", -400, -100, square_ring(10.0, 10.0, 20.0, 20.0));
    let err = atlas_etl::validate::run_polities(&[a, b], &test_bbox()).unwrap_err();
    assert!(err.to_string().contains("duplicate"), "{err}");
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
        atlas_core::data::CatechismPart { id: "dup".into(), title: "A".into(), items: vec![catechism_item("a1", "A1", &[])] },
        atlas_core::data::CatechismPart { id: "dup".into(), title: "B".into(), items: vec![catechism_item("b1", "B1", &[])] },
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
        atlas_core::data::CatechismPart { id: "p1".into(), title: "P1".into(), items: vec![catechism_item("shared", "A", &[])] },
        atlas_core::data::CatechismPart { id: "p2".into(), title: "P2".into(), items: vec![catechism_item("shared", "B", &[])] },
    ];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("duplicate catechism item"), "{err}");
}

#[test]
fn catechism_empty_part_fails_validation() {
    let parts = vec![atlas_core::data::CatechismPart { id: "empty".into(), title: "Empty".into(), items: vec![] }];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("has no items"), "{err}");
}

#[test]
fn catechism_verse_missing_from_compiled_kjv_text_fails_validation() {
    let parts = vec![atlas_core::data::CatechismPart {
        id: "p".into(),
        title: "P".into(),
        items: vec![catechism_item("i1", "I1", &["GEN.99.99"])],
    }];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("does not exist in the compiled KJV text"), "{err}");
}

#[test]
fn catechism_non_canonical_verse_fails_validation() {
    let parts = vec![atlas_core::data::CatechismPart {
        id: "p".into(),
        title: "P".into(),
        items: vec![catechism_item("i1", "I1", &["NOT.A.VERSE"])],
    }];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("not a canonical single-verse ref"), "{err}");
}

// --- Batch F2: run_catechism's own question-level checks --------------

#[test]
fn catechism_question_with_verse_missing_from_compiled_text_fails_validation() {
    let mut item = catechism_item("i1", "I1", &[]);
    item.questions = vec![catechism_question("Q1", &["GEN.99.99"], "brain-fuel/catechism")];
    let parts = vec![atlas_core::data::CatechismPart { id: "p".into(), title: "P".into(), items: vec![item] }];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("does not exist in the compiled KJV text"), "{err}");
    assert!(err.to_string().contains("Q1"), "{err}");
}

#[test]
fn catechism_question_with_non_canonical_verse_fails_validation() {
    let mut item = catechism_item("i1", "I1", &[]);
    item.questions = vec![catechism_question("Q1", &["NOT.A.VERSE"], "brain-fuel/catechism")];
    let parts = vec![atlas_core::data::CatechismPart { id: "p".into(), title: "P".into(), items: vec![item] }];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("not a canonical single-verse ref"), "{err}");
}

#[test]
fn catechism_question_with_zero_verses_fails_validation() {
    let mut item = catechism_item("i1", "I1", &[]);
    item.questions = vec![catechism_question("Q1", &[], "brain-fuel/catechism")];
    let parts = vec![atlas_core::data::CatechismPart { id: "p".into(), title: "P".into(), items: vec![item] }];
    let err = atlas_etl::validate::run_catechism(&parts, &HashMap::new()).unwrap_err();
    assert!(err.to_string().contains("has zero verses"), "{err}");
}

#[test]
fn catechism_valid_questions_pass_validation() {
    let mut verses = HashMap::new();
    verses.insert("MAT.28.19".to_string(), "text".to_string());
    let mut item = catechism_item("i1", "I1", &[]);
    item.questions = vec![catechism_question("Q1", &["MAT.28.19"], "brain-fuel/catechism")];
    let parts = vec![atlas_core::data::CatechismPart { id: "p".into(), title: "P".into(), items: vec![item] }];
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
