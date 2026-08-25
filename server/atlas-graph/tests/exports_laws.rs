//! C2C3-EXPORT laws (design doc `.superpowers/sdd/2026-08-17-bible-
//! atlas-m1/c2c3-export-design.md`, section 4): round-trip parse (also
//! unit-tested in `exports.rs` itself over synthetic fixtures -- this
//! file adds ONE real-data round trip for extra confidence), every
//! dated event's placement resolves (count asserted, not assumed), the
//! embedded `atlas_version_root` equals the live `GraphVersion`
//! (drift-impossible by construction, proven here rather than merely
//! architected), the peer's own alias/canonical spot-checks
//! ("Kadesh-barnea", "En-rogel" resolve by CANONICAL name; "entrance of
//! Hamath" resolves by curated ALIAS on `lebo-hamath` as of Batch
//! GAZ-1-R1 -- see `exports.rs`'s own header comment for the full
//! citation set), the creation row is present and resolvable, and every
//! span interval is well-formed (`from <= to`).
//!
//! Real committed `data/raw` + `data/curated`, same pattern `tests/
//! version_root_regression.rs` already established (duplicated helper,
//! per that file's own convention -- every real-data integration test
//! file in this crate keeps its own copy).

use std::path::Path;

use atlas_graph::exports;
use atlas_graph_types::store::{GraphPublisher, MemStore};

fn real_atlas_data() -> atlas_core::data::AtlasData {
    static CACHED: std::sync::OnceLock<atlas_core::data::AtlasData> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
                .data
        })
        .clone()
}

#[derive(Clone)]
struct Built {
    gazetteer: Vec<exports::GazetteerPlace>,
    events: Vec<exports::ChronologyEvent>,
    spans: Vec<exports::ChronologySpan>,
    anchors: Vec<exports::ChronologyAnchorRow>,
    order_len: usize,
    /// This test's own bare-build + `MemStore::publish` version (the SAME
    /// sequence `bins/compile_graph.rs` uses to stamp the exports).
    actual_hex: String,
    /// `GraphService`'s own production construction path, independently
    /// built from the SAME real sources -- law 3 (atlas_version_root
    /// equals the live GraphVersion) is `actual_hex == expected_hex`.
    expected_hex: String,
}

/// Builds everything ONCE per test binary (a full real compile + two
/// independent graph builds is real wall-clock time -- `compile_graph.rs`'s
/// own doc comment measures ~3-15s for ONE admission-shaped build) and
/// hands every `#[test]` fn a clone of small, owned row data.
fn built() -> Built {
    static CACHED: std::sync::OnceLock<Built> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
            let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
            let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
            let atlas = real_atlas_data();
            let brainfuel = atlas_etl::brainfuel::read_all(&dir.join("brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step first");
            // CORP-2a: the real vendored Concord data joins the root
            // computation, the SAME "otherwise this harness proves a
            // DIFFERENT graph than atlas-graph-compile actually produces"
            // reasoning CORP-1a's own brainfuel threading already
            // established here.
            let concord_corpus = atlas_etl::concord::read_all(&dir.join("concord")).expect("data/raw/concord must exist -- run data/fetch-raw.ps1 first");
            let sc_overlap_text = std::fs::read_to_string(dir.parent().unwrap().join("curated/concord-sc-overlap.toml")).expect("data/curated/concord-sc-overlap.toml must exist");
            let sc_overlap = atlas_etl::concord::parse_sc_overlap(&sc_overlap_text).expect("concord-sc-overlap.toml must parse");
            let concord_bundle = atlas_graph::concord_adapter::ConcordBundle { corpus: concord_corpus, sc_overlap };

            let (mut graph, _stats, _ews, chrono) =
                atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord(&kjv_json, &xrefs_tsv, &atlas, &atlas.eras, Some(&brainfuel), Some(&concord_bundle))
                    .expect("the real committed sources must build");
            graph.build_indexes();
            atlas_graph::event_world::add_justified_by(&mut graph);
            let chronology = atlas_graph::Chronology::from_derivation(chrono);

            let gazetteer = exports::gazetteer_places(&graph);
            let events = exports::chronology_events(&graph, &chronology);
            let spans = exports::chronology_spans(&graph);
            let anchors = exports::chronology_anchors(&graph, &atlas.chronology_anchors);
            let order_len = chronology.chrono.order.len();

            let mut store = MemStore::default();
            let version = store.publish(graph);
            let actual_hex = atlas_graph::version_hex(version);

            let svc = atlas_graph::GraphService::from_sources_with_eras_and_brainfuel_and_concord(&kjv_json, &xrefs_tsv, &atlas, &atlas.eras, Some(&brainfuel), Some(&concord_bundle))
                .expect("GraphService must build the same real sources");
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
    // "Kadesh-barnea" and "En-rogel" both resolve as CANONICAL place
    // names (not KJV aliases) -- see exports.rs's own header comment.
    let kadesh = b.gazetteer.iter().find(|p| p.canonical == "Kadesh-barnea" || p.aliases.iter().any(|a| a == "Kadesh-barnea"));
    assert!(kadesh.is_some(), "\"Kadesh-barnea\" (the peer's own binding name) must be findable by canonical name or alias in the exported gazetteer");

    let en_rogel = b.gazetteer.iter().find(|p| p.canonical == "En-rogel" || p.aliases.iter().any(|a| a == "En-rogel"));
    assert!(en_rogel.is_some(), "\"En-rogel\" (the peer's own binding name) must be findable by canonical name or alias in the exported gazetteer");

    // Batch GAZ-1-R1: "entrance of Hamath" (the peer's third named binding
    // example, previously disclosed as unresolvable) now resolves as a
    // curated KJV ALIAS on `lebo-hamath` -- checked both ways (findable at
    // all, AND specifically on the right place id, not merely findable
    // somewhere) so a future accidental re-homing onto the wrong place
    // fails loud rather than silently passing this law.
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
