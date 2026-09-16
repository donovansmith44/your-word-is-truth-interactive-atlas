//! Fix round 1 (I-3; batch-mc2-review.md's own Lens 4 deep-dive, which
//! confirms this as the implementer's own self-disclosed Concern 2, right-
//! sized): `atlas_graph::legacy::atlas_data_overlay` is called exactly
//! once, on the server's real DEFAULT startup path, and had NO dedicated
//! test anywhere -- its four composed pieces (`event_from_node`/
//! `place_from_node`/`narrative_from_node`/`verses_from_graph`) are each
//! individually exercised with real data by other tests (`graph_api.rs`,
//! `handlers::verse`/`xrefs`'s own tests), and the Playwright E2E suite
//! drives the real running server through the NOT-yet-migrated surfaces
//! this overlay feeds -- but nothing ever checked the BULK,
//! whole-graph-enumeration property: does the overlay correctly
//! reconstruct EVERY event/place/narrative/verse, not just the handful of
//! ids any spot-check test happens to name.
//!
//! Same `graph_equivalence`-style pattern `server/atlas-server/tests/
//! graph_equivalence.rs` already established (old-path vs. new-path,
//! independently re-derived, over real committed data): the OLD/reference
//! side is a fresh `atlas_etl::compile::compile` (the same real ETL-
//! equivalent path `--build-from-raw` uses, never touching the graph at
//! all); the NEW/under-test side is `atlas_data_overlay` fed by
//! `GraphService::from_artifact` loading the REAL committed
//! `data/compiled/graph.bin` -- byte-for-byte the production sequence
//! `atlas-server/src/main.rs`'s own default startup path runs (artifact
//! load, then overlay), not a from-sources approximation of it.
//! `Event`/`Place`/`Narrative` already derive `PartialEq` (+ `Debug`), so
//! each comparison is a sorted-by-id `assert_eq!` over the whole
//! collection -- no new infrastructure, per the review's own "right-sized"
//! confirmation.
//!
//! KNOWN DIFFERENCE (OVERLAY-1 Task 1, controller-ruled): compile-from-
//! sources and the overlay AGREE on the whole-collection, sorted-by-id
//! comparisons above, but DISAGREE on the events' own post-`finish()`
//! ORDER -- `AtlasData::finish()`'s `events.sort_by_key(|e| e.when.
//! from_year)` (`data.rs:1533`) is a stable sort with no secondary key, so
//! ties (same `from_year`, the overwhelming majority of the 1,711 real
//! events) keep whichever pre-sort insertion order each path started
//! with, and the two paths' insertion orders are unrelated (graph
//! traversal vs. raw/curated file parse order). Measured: 1,364 of 1,711
//! positions differ once both paths are pushed through `finish()` (full
//! verbatim capture: `.superpowers/sdd/2026-09-16-overlay1/
//! task-1-event-order-diff.txt`). This is a real, ledgered finding about
//! `finish()` itself, queued as its own future owner batch -- NOT fixed
//! here (no secondary sort key added; no production code touched beyond
//! the one stale-comment correction in `legacy.rs`), because fixing it
//! would change served order and the 25 hashes `scene_byte_identity.rs`
//! pinned. For OVERLAY-1, the OVERLAY path's order is authoritative (the
//! 25 hashes were pinned against it) -- so the order property this file
//! proves below is DETERMINISM of the overlay path across independent
//! builds, the exact property Task 4's graph-backed `SceneSource` must
//! reproduce, not agreement with compile-from-sources order.

use std::path::Path;

use atlas_core::data::AtlasData;
use atlas_graph::legacy::atlas_data_overlay;
use atlas_graph::GraphService;

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

/// Batch KJV-CASE: the real vendored brain-fuel corpus, needed to
/// reproduce the SAME case-restoration `build::build_graph_from_sources_
/// with_eras_and_brainfuel` applied when it built the real committed
/// `graph.bin` this file's own `real_overlay()` loads.
fn real_brainfuel() -> atlas_etl::brainfuel::BrainFuelCorpus {
    atlas_etl::brainfuel::read_all(&data_dir().join("raw/brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step first")
}

/// The independent reference -- cached (`OnceLock`) since a real compile
/// is expensive and this file's own several assertions would otherwise
/// each pay for it again.
fn expected() -> &'static AtlasData {
    static CACHED: std::sync::OnceLock<AtlasData> = std::sync::OnceLock::new();
    CACHED.get_or_init(|| {
        atlas_etl::compile::compile(&data_dir().join("raw"), &data_dir().join("curated"))
            .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
            .data
    })
}

/// The path under test -- the real committed artifact, loaded exactly the
/// way `atlas-server/src/main.rs`'s own default startup path does, then
/// overlaid. Not cached: `GraphService`/`LegacyAtlasFields` hold real data
/// (~40MB), and this file's own handful of tests are cheap enough
/// (sub-second overlay reconstruction, per `atlas_data_overlay`'s own doc
/// comment) that a second load is not worth the `OnceLock<Arc<...>>`
/// plumbing -- confirmed via `time cargo test` during this fix round: the
/// artifact load itself (the expensive part) still sits at ~750ms per the
/// committed load-ceiling law, an order of magnitude under this test
/// file's own patience.
fn real_overlay() -> atlas_graph::legacy::LegacyAtlasFields {
    let graph = GraphService::from_artifact(&data_dir().join("compiled/graph.bin")).expect("the real committed data/compiled/graph.bin must load");
    atlas_data_overlay(&graph)
}

#[test]
fn every_reconstructed_event_equals_compiles_own_real_event_field_for_field() {
    let mut want = expected().events.clone();
    let mut got = real_overlay().events;
    want.sort_by(|a, b| a.id.cmp(&b.id));
    got.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(got.len(), want.len(), "event count must match exactly (real data: 1,711 as of Batch CHRON-1's own 24-pair triage merge, DUPLICATE_JACCARD_THRESHOLD lowered 0.8 -> 0.5; this test compares the COMMITTED data/compiled/graph.bin against a fresh recompile of source -- a mismatch here means graph.bin needs regenerating, `cargo run -p atlas-graph --bin atlas-graph-compile`, not that this count is wrong)");
    assert_eq!(got, want, "every ID must reconstruct to an Event equal in EVERY field to compile()'s own real Event -- a diff here means atlas_data_overlay silently drops or corrupts data for at least one real event");
}

#[test]
fn every_reconstructed_place_equals_compiles_own_real_place_field_for_field() {
    let mut want = expected().places.clone();
    let mut got = real_overlay().places;
    want.sort_by(|a, b| a.id.cmp(&b.id));
    got.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(got.len(), want.len(), "place count must match exactly (real data: 1,373)");
    assert_eq!(got, want, "every ID must reconstruct to a Place equal in EVERY field to compile()'s own real Place");
}

#[test]
fn every_reconstructed_narrative_equals_compiles_own_real_narrative_field_for_field() {
    let mut want = expected().narratives.clone();
    let mut got = real_overlay().narratives;
    want.sort_by(|a, b| a.id.cmp(&b.id));
    got.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(got.len(), want.len(), "narrative count must match exactly (real data: 13)");
    assert_eq!(got, want, "every ID must reconstruct to a Narrative equal in EVERY field to compile()'s own real Narrative, including its own succession-derived `legs` order");
}

/// Batch KJV-CASE (owner ruling; batch-kjv-case-brief.md) moved this
/// test's own "want": `expected().verses` is `atlas_etl::compile::
/// compile`'s own output, which is (by design -- see `atlas_graph::build`'s
/// own module doc comment) NEVER case-restored; the graph `real_overlay()`
/// reads FROM (the real committed `data/compiled/graph.bin`) IS -- that
/// restoration happens once, at graph-compile time
/// (`build::build_graph_from_sources_with_eras_and_brainfuel`), and
/// `AtlasData.verses` deliberately stays outside its scope (it is a
/// validation-only sidecar in the ETL step -- no display surface ever
/// reads its string VALUES, only key existence; see that module's own doc
/// comment for the full reasoning). So "the real compiled KJV text" this
/// test's own name promises is now, honestly, "compile()'s own verses,
/// case-restored the SAME way the graph was" -- applying
/// `atlas_etl::brainfuel::restore_kjv_case` to `want` here is not
/// weakening this test, it is keeping its OWN stated promise (bijection +
/// content fidelity between the overlay and the true compiled source) once
/// that source is honestly two-staged (raw-compile, then case-restore).
#[test]
fn every_reconstructed_verse_text_equals_the_real_compiled_kjv_text_exactly() {
    let brainfuel = real_brainfuel();
    let (want, _report) = atlas_etl::brainfuel::restore_kjv_case(&brainfuel, &expected().verses);
    let got = real_overlay().verses;
    assert_eq!(got.len(), want.len(), "verse count must match exactly (real data: 31,102)");
    let mut mismatches: Vec<String> = Vec::new();
    for (k, want_text) in &want {
        match got.get(k) {
            Some(got_text) if got_text == want_text => {}
            Some(got_text) => mismatches.push(format!("{k}: text differs (want {want_text:?}, got {got_text:?})")),
            None => mismatches.push(format!("{k}: missing from the overlay entirely")),
        }
    }
    assert!(mismatches.is_empty(), "verse text mismatches:\n{}", mismatches.join("\n"));
}

// --- OVERLAY-1 Task 1 additions -----------------------------------------
//
// The four tests above compare the two paths SORTED BY ID, before
// `finish()` -- collection-level equivalence only. Two increments this
// batch needs, both because Task 4 must reproduce them without the
// overlay:
//
//   (a) ORDER: each event's `places` vector equal IN ORDER (`places[0]` is
//       the anchor place -- legacy.rs:52-58, load-bearing for
//       `scene::build_arrows` endpoints and `handlers::event`), checked
//       compile-vs-overlay, sorted by id (pre-`finish()`, matching the
//       four tests above); and the OVERLAY path's own post-`finish()`
//       event order (`data.rs:1533`, `events.sort_by_key(|e| e.when.
//       from_year)`, stable) -- exactly what `scene.rs`'s time-mode
//       composition iterates `d.events` in -- checked for DETERMINISM
//       across two independent overlay builds, NOT against compile-from-
//       sources order (see this file's own module-doc "KNOWN DIFFERENCE"
//       paragraph above: that comparison is a real, ledgered disagreement,
//       queued as an owner batch, not something this test asserts).
//   (b) POST-`finish()` AGGREGATES: the derived indexes `scene::
//       quiet_places` reads -- `event_bearing_place_ids()` and
//       `total_events_for(id)` for every place (`data.rs:1823,1833`) --
//       must agree between the two paths. These only exist after
//       `finish()` runs, so both paths below are built as full, finished
//       `AtlasData` (unlike `real_overlay()`/`expected()` above, which
//       compare pre-`finish()`).

/// Full `AtlasData`, overlay-sourced and finished -- byte-for-byte the
/// server's own real default startup call sequence (`atlas-server/src/
/// load.rs`'s `load_graph_and_data`: `GraphService::from_artifact` ->
/// `AtlasData::load` -> `atlas_data_overlay` -> assign the four retiring
/// fields -> `finish()`). Not cached, same reasoning as `real_overlay()`
/// above (cheap; this file's own tests already pay for a fresh load each).
fn finished_overlay_atlas_data() -> AtlasData {
    let graph = GraphService::from_artifact(&data_dir().join("compiled/graph.bin")).expect("the real committed data/compiled/graph.bin must load");
    let mut data = AtlasData::load(&data_dir().join("compiled")).expect("data/compiled sidecars must load (canon.json, books-meta.json, etc.)");
    let overlay = atlas_data_overlay(&graph);
    data.events = overlay.events;
    data.places = overlay.places;
    data.narratives = overlay.narratives;
    data.verses = overlay.verses;
    data.finish()
}

/// Full `AtlasData`, compiled-from-sources and finished -- the same
/// independent reference `expected()` above caches (pre-`finish()`),
/// cloned and finished fresh here so this helper's own `finish()`-derived
/// indexes never leak into `expected()`'s cached copy, which the four
/// tests above deliberately compare pre-`finish()`.
fn finished_sources_atlas_data() -> AtlasData {
    expected().clone().finish()
}

/// Step 1(a), first half, named explicitly: `places[0]` is the event's
/// ANCHOR place. This is already implicitly proven by
/// `every_reconstructed_event_equals_compiles_own_real_event_field_for_field`
/// above (`Event`'s derived `PartialEq` compares `places: Vec<String>`
/// element-wise, in order, as part of the whole-struct comparison) -- this
/// test asserts it on its own, with an anchor-specific failure message, so
/// a real anchor-order regression is never buried in a generic whole-event
/// diff.
#[test]
fn overlay_and_compiled_events_agree_on_places_vector_order_including_anchor() {
    let mut want = expected().events.clone();
    let mut got = real_overlay().events;
    want.sort_by(|a, b| a.id.cmp(&b.id));
    got.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(got.len(), want.len(), "event count");
    for (g, w) in got.iter().zip(&want) {
        assert_eq!(g.id, w.id, "sorted-by-id iteration must line up");
        assert_eq!(
            g.places, w.places,
            "event {}'s places vector must match IN ORDER -- places[0] is the anchor place (legacy.rs:52-58), load-bearing for scene::build_arrows endpoints and handlers::event",
            g.id
        );
    }
}

/// Step 1(a), second half -- RESHAPED per the controller's ruling on this
/// file's own module-doc "KNOWN DIFFERENCE" paragraph above: compile-vs-
/// overlay post-`finish()` event order is a real, ledgered disagreement
/// (queued as an owner batch, not fixed here), so this test does NOT
/// compare compile-from-sources order against the overlay. Instead it
/// proves the property OVERLAY-1 actually needs: the OVERLAY path's own
/// post-`finish()` event order (and each event's `places` order,
/// `places[0]` the anchor) is DETERMINISTIC across two INDEPENDENT builds
/// (two full `GraphService::from_artifact` + `AtlasData::load` + overlay +
/// `finish()` sequences -- not a clone of one build) -- exactly the
/// property Task 4's graph-backed `SceneSource` must reproduce, since the
/// 25 `scene_byte_identity.rs` hashes were pinned against this path's own
/// order.
#[test]
fn overlay_path_event_order_and_places_order_are_deterministic_across_independent_builds() {
    let a = finished_overlay_atlas_data();
    let b = finished_overlay_atlas_data();
    let a_order: Vec<&str> = a.events.iter().map(|e| e.id.as_str()).collect();
    let b_order: Vec<&str> = b.events.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(
        a_order, b_order,
        "two independent builds of the overlay path must produce IDENTICAL finish()-derived event order -- this is the order Task 4's graph-backed source must reproduce"
    );
    assert_eq!(a.events.len(), b.events.len(), "event count must match across independent overlay builds");
    for (x, y) in a.events.iter().zip(&b.events) {
        assert_eq!(x.id, y.id, "event order differs at {} vs {}", x.id, y.id);
        assert_eq!(
            x.places, y.places,
            "event {}'s places vector (places[0] = anchor) must be identical in order across independent overlay builds",
            x.id
        );
    }
}

/// Step 1(b): the `finish()`-derived indexes `scene::quiet_places` reads
/// (`event_bearing_place_ids()` and `total_events_for(id)` -- data.rs:1823,
/// 1833) must agree between the two paths for EVERY place, since Task 4
/// must reproduce them without the overlay.
#[test]
fn overlay_and_compiled_agree_on_event_bearing_place_ids_and_totals_after_finish() {
    let overlay = finished_overlay_atlas_data();
    let sources = finished_sources_atlas_data();

    let mut overlay_ids: Vec<&String> = overlay.event_bearing_place_ids().iter().collect();
    let mut sources_ids: Vec<&String> = sources.event_bearing_place_ids().iter().collect();
    overlay_ids.sort();
    sources_ids.sort();
    assert_eq!(overlay_ids, sources_ids, "event_bearing_place_ids() must be the identical set on both paths");

    let mut mismatches: Vec<String> = Vec::new();
    for id in &sources_ids {
        let want = sources.total_events_for(id);
        let got = overlay.total_events_for(id);
        if got != want {
            mismatches.push(format!("{id}: overlay total_events_for={got} sources total_events_for={want}"));
        }
    }
    assert!(mismatches.is_empty(), "total_events_for(id) mismatches:\n{}", mismatches.join("\n"));
}
