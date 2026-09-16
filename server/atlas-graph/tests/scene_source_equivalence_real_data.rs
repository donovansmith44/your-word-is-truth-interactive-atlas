//! OVERLAY-1 Task 4: the graph-backed `SceneSource` must compose
//! BYTE-IDENTICAL scenes to the overlay-backed `AtlasData` path, on the
//! real committed `data/compiled/graph.bin`, for every one of the 25
//! inputs `server/atlas-server/tests/scene_byte_identity.rs` pins
//! (`:157-190`: 20 time windows + 5 scripture refs, copied verbatim
//! below so this file is self-contained and a drift in either list is
//! visible as a plain diff rather than a hidden coupling).
//!
//! WHY BYTES, NOT STRUCTS: `scene_byte_identity.rs` pins the SERIALIZED
//! response, so that is the property that actually protects the wire.
//! Comparing `serde_json::to_vec` of both sides catches everything a
//! struct comparison would plus field/array ORDER, which is the whole
//! game here -- `AtlasData::finish()`'s `events.sort_by_key(|e| e.when.
//! from_year)` (`data.rs:1533`) is a STABLE sort with no secondary key,
//! so the pre-sort insertion order of the 1,711 real events (the
//! overwhelming majority of which tie on `from_year`) survives into the
//! served order. Task 1 measured that the compile-from-sources path and
//! the overlay path disagree on 1,364 of those 1,711 positions and ruled
//! the OVERLAY order authoritative (the 25 hashes were pinned against
//! it). `GraphSceneSource` therefore reproduces the OVERLAY path's build
//! order exactly -- same source of ids (`gs.event_ids`/`gs.place_ids`),
//! same per-node builders, same two merge passes, same sort -- and this
//! file is the proof.
//!
//! The reference side is built the way the real server boots (artifact
//! load + `AtlasData::load` sidecars + `atlas_data_overlay` + `finish()`),
//! copied from `overlay_equivalence.rs`'s own `finished_overlay_atlas_data`
//! helper so the two files agree on what "the current way" means.

use std::path::Path;

use atlas_core::data::AtlasData;
use atlas_core::refs::ScriptureRef;
use atlas_core::scene::{compose_scripture_scene, compose_time_scene};
use atlas_core::scene_source::SceneSource;
use atlas_core::time::TimeRange;
use atlas_graph::legacy::atlas_data_overlay;
use atlas_graph::scene_source::GraphSceneSource;
use atlas_graph::GraphService;

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

/// Both sides, from ONE artifact load (the expensive part, ~750ms under
/// the committed load ceiling) and one sidecar load: the reference
/// `AtlasData` (overlay + `finish()`) and the `GraphSceneSource` under
/// test. Cached, because every test below wants both.
fn both_sources() -> &'static (AtlasData, GraphSceneSource) {
    static CACHED: std::sync::OnceLock<(AtlasData, GraphSceneSource)> = std::sync::OnceLock::new();
    CACHED.get_or_init(|| {
        let graph = GraphService::from_artifact(&data_dir().join("compiled/graph.bin")).expect("the real committed data/compiled/graph.bin must load");
        let sidecars = AtlasData::load(&data_dir().join("compiled")).expect("data/compiled sidecars must load (canon.json, books-meta.json, etc.)");

        // The reference: exactly `overlay_equivalence.rs`'s own
        // `finished_overlay_atlas_data`, which is exactly the real server
        // startup path (`atlas-server/src/load.rs:103-108`). OVERLAY-1
        // Task 2 retired `AtlasData.verses`, so verse text is not
        // materialized on either side.
        let mut data = sidecars.clone();
        let overlay = atlas_data_overlay(&graph);
        data.events = overlay.events;
        data.places = overlay.places;
        data.narratives = overlay.narratives;
        let data = data.finish();

        // The path under test: same graph, same sidecars, no overlay, no
        // `AtlasData::finish()`.
        let graph_src = GraphSceneSource::build(&graph, &sidecars);

        (data, graph_src)
    })
}

/// Copied verbatim from `scene_byte_identity.rs:157-179` (label, from,
/// to). The pinned hashes/byte-lengths are deliberately NOT copied: this
/// file's property is EQUALITY BETWEEN THE TWO SOURCES, not agreement
/// with a pinned constant -- that is `scene_byte_identity.rs`'s own job,
/// and duplicating the constants here would only create a second place to
/// re-pin.
fn time_windows() -> Vec<(&'static str, i32, i32)> {
    vec![
        ("era_primeval", -4004, -2167),
        ("era_patriarchs", -2166, -1877),
        ("era_egypt_exodus", -1876, -1407),
        ("era_conquest_judges", -1406, -1051),
        ("era_united_kingdom", -1050, -932),
        ("era_divided_kingdom", -931, -587),
        ("era_exile", -586, -539),
        ("era_return", -538, -6),
        ("era_gospels", -5, 29),
        ("era_early_church", 30, 100),
        ("full_span", -4004, 100),
        ("nt_window_gospels_plus_church", -5, 100),
        ("degenerate_start_year", -4004, -4004),
        ("degenerate_end_year", 100, 100),
        ("degenerate_mid_year", -1000, -1000),
        ("straddle_primeval_patriarchs", -2200, -2100),
        ("straddle_gospels_early_church", 25, 35),
        ("straddle_exile_return", -600, -500),
        ("narrow_conquest", -1407, -1406),
        ("wide_kingdom_era", -1051, -539),
    ]
}

/// Copied verbatim from `scene_byte_identity.rs:183-190` (label, ref).
fn scripture_refs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("scripture_gen1", "GEN.1"),
        ("scripture_jhn316", "JHN.3.16"),
        ("scripture_psa23", "PSA.23"),
        ("scripture_exo20", "EXO.20"),
        ("scripture_rev22", "REV.22"),
    ]
}

/// The brief's own debugging requirement: on a mismatch, print the FIRST
/// differing byte offset and the surrounding JSON of both sides, so the
/// real ORDER or MERGE fact is findable -- never relax the comparison.
fn describe_first_difference(label: &str, want: &[u8], got: &[u8]) -> String {
    let at = want.iter().zip(got).position(|(a, b)| a != b).unwrap_or(want.len().min(got.len()));
    let lo = at.saturating_sub(200);
    let want_hi = (at + 200).min(want.len());
    let got_hi = (at + 200).min(got.len());
    format!(
        "{label}: scenes differ.\n  atlas_data len {} vs graph len {}\n  first differing byte offset: {at}\n  atlas_data[{lo}..{want_hi}]: {}\n  graph     [{lo}..{got_hi}]: {}",
        want.len(),
        got.len(),
        String::from_utf8_lossy(&want[lo..want_hi]),
        String::from_utf8_lossy(&got[lo..got_hi]),
    )
}

#[test]
fn graph_scene_source_composes_byte_identical_time_scenes_for_every_pinned_window() {
    let (data, graph_src) = both_sources();

    let mut failures = Vec::new();
    for (label, from, to) in time_windows() {
        let w = TimeRange::new(from, to).unwrap();
        let want = serde_json::to_vec(&compose_time_scene(data, w)).unwrap();
        let got = serde_json::to_vec(&compose_time_scene(graph_src, w)).unwrap();
        if want != got {
            failures.push(describe_first_difference(label, &want, &got));
        } else {
            println!("{label} -> {} bytes, byte-identical", want.len());
        }
    }

    assert!(
        failures.is_empty(),
        "the graph-backed SceneSource must compose byte-identical time scenes to the overlay-backed AtlasData path -- a difference here is an ORDER or MERGE fact to find, never a reason to relax this comparison:\n{}",
        failures.join("\n")
    );
}

#[test]
fn graph_scene_source_composes_byte_identical_scripture_scenes_for_every_pinned_ref() {
    let (data, graph_src) = both_sources();

    let mut failures = Vec::new();
    for (label, sref) in scripture_refs() {
        let r = ScriptureRef::parse(sref).unwrap();
        let want = serde_json::to_vec(&compose_scripture_scene(data, &r)).unwrap();
        let got = serde_json::to_vec(&compose_scripture_scene(graph_src, &r)).unwrap();
        if want != got {
            failures.push(describe_first_difference(label, &want, &got));
        } else {
            println!("{label} -> {} bytes, byte-identical", want.len());
        }
    }

    assert!(
        failures.is_empty(),
        "the graph-backed SceneSource must compose byte-identical scripture scenes to the overlay-backed AtlasData path:\n{}",
        failures.join("\n")
    );
}

/// The three `SceneSource` reads that are DERIVED rather than composed
/// (`finish()` builds the latter two at `data.rs:1592-1599`), checked
/// directly as well as through the composed bytes above -- a scene only
/// exercises the subset of places/events its own window touches, so a
/// derivation bug outside every pinned window would otherwise hide.
#[test]
fn graph_scene_source_agrees_on_events_in_window_and_the_finish_derived_aggregates() {
    let (data, graph_src) = both_sources();

    for (label, from, to) in time_windows() {
        let w = TimeRange::new(from, to).unwrap();
        let want: Vec<&str> = data.events_in_window(&w).into_iter().map(|e| e.id.as_str()).collect();
        let got: Vec<&str> = graph_src.events_in_window(&w).into_iter().map(|e| e.id.as_str()).collect();
        assert_eq!(got, want, "{label}: events_in_window must agree in CONTENT AND ORDER (the scene's own event order is this order)");
    }

    for (label, sref) in scripture_refs() {
        let r = ScriptureRef::parse(sref).unwrap();
        let want: Vec<&str> = data.events_matching_ref(&r).into_iter().map(|e| e.id.as_str()).collect();
        let got: Vec<&str> = graph_src.events_matching_ref(&r).into_iter().map(|e| e.id.as_str()).collect();
        assert_eq!(got, want, "{label}: events_matching_ref must agree in content and order");
    }

    assert_eq!(
        graph_src.event_bearing_place_ids(),
        data.event_bearing_place_ids(),
        "event_bearing_place_ids must be the identical set (finish() derives it at data.rs:1592-1599 from every event's every place)"
    );

    let mut checked = 0usize;
    for pid in data.event_bearing_place_ids() {
        assert_eq!(graph_src.total_events_for(pid), data.total_events_for(pid), "total_events_for({pid}) must agree (quiet_places carries this all-time count verbatim)");
        checked += 1;
    }
    assert!(checked > 0, "event_bearing_place_ids must not be empty on real data");
    println!("event_bearing_place_ids: {checked} places, all totals agree");

    // The whole-collection reads the composer makes, plus the anchor-order
    // law: `places[0]` is the true anchor, load-bearing for
    // `scene::build_arrows`'s own arrow endpoints.
    assert_eq!(graph_src.places(), data.places(), "places() must be identical in content AND order");
    assert_eq!(graph_src.narratives(), data.narratives(), "narratives() must be identical in content AND order");
    for e in data.events_in_window(&TimeRange::new(-4004, 100).unwrap()) {
        let mirror = graph_src.event_by_id(&e.id).unwrap_or_else(|| panic!("event {} must exist in the graph-backed source", e.id));
        assert_eq!(mirror.places, e.places, "event {}'s places vector (places[0] = anchor) must be identical in order", e.id);
        assert_eq!(mirror, e, "event {} must be equal field for field", e.id);
    }
}
