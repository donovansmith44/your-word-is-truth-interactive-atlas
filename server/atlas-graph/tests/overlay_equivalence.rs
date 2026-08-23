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

use std::path::Path;

use atlas_core::data::AtlasData;
use atlas_graph::legacy::atlas_data_overlay;
use atlas_graph::GraphService;

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
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
    assert_eq!(got.len(), want.len(), "event count must match exactly (real data: 1,738)");
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

#[test]
fn every_reconstructed_verse_text_equals_the_real_compiled_kjv_text_exactly() {
    let want = &expected().verses;
    let got = real_overlay().verses;
    assert_eq!(got.len(), want.len(), "verse count must match exactly (real data: 31,102)");
    let mut mismatches: Vec<String> = Vec::new();
    for (k, want_text) in want {
        match got.get(k) {
            Some(got_text) if got_text == want_text => {}
            Some(got_text) => mismatches.push(format!("{k}: text differs (want {want_text:?}, got {got_text:?})")),
            None => mismatches.push(format!("{k}: missing from the overlay entirely")),
        }
    }
    assert!(mismatches.is_empty(), "verse text mismatches:\n{}", mismatches.join("\n"));
}
