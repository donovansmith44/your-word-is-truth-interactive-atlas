mod common;

use common::{OptionalCorpora, RawSources};

use atlas_graph::exports;
use atlas_graph_types::store::{GraphPublisher, MemStore};

#[derive(Clone)]
struct Built {
    gazetteer: Vec<exports::GazetteerPlace>,
    events: Vec<exports::ChronologyEvent>,
    spans: Vec<exports::ChronologySpan>,
    anchors: Vec<exports::ChronologyAnchorRow>,
    order_len: usize,
    actual_hex: String,
    expected_hex: String,
}

fn built() -> Built {
    static CACHED: std::sync::OnceLock<Built> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let atlas = common::real_atlas();
            let sources = RawSources::read(OptionalCorpora { kretzmann: false, red_letter: false });

            let (mut graph, _stats, _ews, chrono) = sources.build_graph(&atlas.eras);
            graph.build_indexes();
            atlas_graph::event_world::add_justified_by(&mut graph);
            let chronology = atlas_graph::Chronology::from_derivation(chrono);
            let extras = atlas_graph::sqlite::extras::Extras::graph_derived(&graph, &chronology.chrono, &std::collections::HashMap::new())
                .expect("the real graph's projections encode");
            extras.attach(&mut graph);

            let gazetteer = exports::gazetteer_places(&graph);
            let events = exports::chronology_events(&graph, &chronology);
            let spans = exports::chronology_spans(&graph);
            let anchors = exports::chronology_anchors(&graph, &atlas.chronology_anchors);
            let order_len = chronology.chrono.order.len();

            let mut store = MemStore::default();
            let version = store.publish(graph).unwrap();
            let actual_hex = atlas_graph::version_hex(version);

            let svc = sources.build_service(&atlas.eras);
            let expected_hex = atlas_graph::version_hex(svc.version());

            Built { gazetteer, events, spans, anchors, order_len, actual_hex, expected_hex }
        })
        .clone()
}

#[test]
fn law_atlas_version_root_equals_the_live_graph_version() {
    let b = built();
    assert_eq!(b.actual_hex, b.expected_hex, "the exports' own version derivation (bare build + MemStore::publish) must agree byte-for-byte with GraphService's production path over the same real sources -- drift here means the exports could silently stamp a different root than the artifact reports");
}

#[test]
fn law_every_dated_events_placement_resolves_none_silently_dropped() {
    let b = built();
    assert_eq!(b.events.len(), b.order_len, "every id in chronology.chrono.order (the graph's own dated-event set) must produce exactly one exported row -- a mismatch means some real event's placement failed to resolve and was silently dropped rather than surfacing as a build failure");
    assert!(b.order_len > 0, "the real compiled data must have real dated events (a zero count would make this law vacuous)");
}

#[test]
fn law_creation_row_is_present_and_resolvable() {
    let b = built();
    let creation = b.anchors.iter().find(|a| a.id == "creation").expect("the 'creation' anchor row must be present -- the map system's own Anchor stand-in waits on it");
    assert_eq!(creation.at.year, -4004, "creation's own resolved year, Ussher's Annals of the World (1658)");
    assert!(creation.citation.contains("Ussher"), "citation must carry real source attribution, not a blank/placeholder string");
    assert_eq!(creation.label, "Creation of the world");
}

#[test]
fn law_alias_and_canonical_spot_checks_for_the_peers_binding_names() {
    let b = built();
    let kadesh = b.gazetteer.iter().find(|p| p.canonical == "Kadesh-barnea" || p.aliases.iter().any(|a| a == "Kadesh-barnea"));
    assert!(kadesh.is_some(), "\"Kadesh-barnea\" (the peer's own binding name) must be findable by canonical name or alias in the exported gazetteer");

    let en_rogel = b.gazetteer.iter().find(|p| p.canonical == "En-rogel" || p.aliases.iter().any(|a| a == "En-rogel"));
    assert!(en_rogel.is_some(), "\"En-rogel\" (the peer's own binding name) must be findable by canonical name or alias in the exported gazetteer");

    let hamath_entrance = b.gazetteer.iter().find(|p| p.canonical == "entrance of Hamath" || p.aliases.iter().any(|a| a == "entrance of Hamath"));
    assert!(hamath_entrance.is_some(), "\"entrance of Hamath\" (the peer's own binding name) must be findable by canonical name or alias in the exported gazetteer");
    assert_eq!(hamath_entrance.unwrap().id, "lebo-hamath", "\"entrance of Hamath\" must resolve onto lebo-hamath specifically -- the real-world location this traditional identification names");
}

#[test]
fn law_every_span_interval_is_well_formed() {
    let b = built();
    assert!(!b.spans.is_empty(), "the real compiled data must have real Era spans (a zero count would make this law vacuous)");
    for s in &b.spans {
        assert!(s.from <= s.to, "span '{}' ({}) has an inverted interval: from {} > to {}", s.id, s.label, s.from, s.to);
    }
}

#[test]
fn export_hash_1_atlas_version_root_does_not_change_when_only_a_dated_events_own_resolved_placement_does() {
    let (graph, _stats, _ews, chrono) = common::kjv_and_atlas_build(&common::real_atlas().eras);
    let chronology = atlas_graph::Chronology::from_derivation(chrono);

    let event_id = chronology.chrono.order.first().cloned().expect("the real compiled data must have at least one dated event");
    let mut mutated = atlas_graph::Chronology::from_derivation(chronology.chrono.clone());
    let placement = mutated.chrono.resolved.get_mut(&event_id).expect("the chosen event id must resolve in its own derivation's resolved map");
    let shifted_year = atlas_graph_types::chrono::Year::new(placement.date.from.year.get() + 1).expect("a valid shifted year");
    placement.date.from.year = shifted_year;
    placement.date.to.year = shifted_year;

    let events_before = exports::chronology_events(&graph, &chronology);
    let events_after = exports::chronology_events(&graph, &mutated);
    assert_ne!(events_before, events_after, "the mutated resolved placement must produce a genuinely different exported row -- otherwise this test proves nothing");

    let mut store = MemStore::default();
    let version = store.publish(graph).unwrap();
    let root_hex = atlas_graph::version_hex(version);

    let export_before = exports::ChronologyExport { format_version: exports::CHRONOLOGY_FORMAT_VERSION, atlas_version_root: root_hex.clone(), events: events_before, spans: vec![], anchors: vec![] };
    let export_after = exports::ChronologyExport { format_version: exports::CHRONOLOGY_FORMAT_VERSION, atlas_version_root: root_hex, events: events_after, spans: vec![], anchors: vec![] };

    assert_eq!(
        export_before.atlas_version_root, export_after.atlas_version_root,
        "EXPORT-HASH-1 (documented, not fixed this batch -- a content_hash field rides the NEXT deliberate format_version bump): two exports with genuinely DIFFERENT content embed the IDENTICAL atlas_version_root -- the root alone cannot detect this class of drift"
    );
    assert_ne!(export_before, export_after, "sanity: the two exports must differ in some field (their own `events`) despite sharing a root -- otherwise the assertion above would be vacuous");
}

#[test]
fn real_data_export_round_trips_through_json() {
    let b = built();
    let gazetteer = exports::GazetteerExport { format_version: exports::GAZETTEER_FORMAT_VERSION, atlas_version_root: b.actual_hex.clone(), places: b.gazetteer.clone() };
    let chronology = exports::ChronologyExport { format_version: exports::CHRONOLOGY_FORMAT_VERSION, atlas_version_root: b.actual_hex.clone(), events: b.events.clone(), spans: b.spans.clone(), anchors: b.anchors.clone() };

    let gazetteer_json = serde_json::to_string(&gazetteer).expect("gazetteer must serialize");
    let gazetteer_back: exports::GazetteerExport = serde_json::from_str(&gazetteer_json).expect("gazetteer must deserialize");
    assert_eq!(gazetteer_back, gazetteer, "gazetteer round-trip must be lossless over real data");

    let chronology_json = serde_json::to_string(&chronology).expect("chronology must serialize");
    let chronology_back: exports::ChronologyExport = serde_json::from_str(&chronology_json).expect("chronology must deserialize");
    assert_eq!(chronology_back, chronology, "chronology round-trip must be lossless over real data");
}
