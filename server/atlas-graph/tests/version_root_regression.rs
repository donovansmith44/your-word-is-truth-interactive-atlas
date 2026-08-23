//! Batch M-C, controller decision 3 (the compiler pipeline contract):
//! "the existing M-A/M-B build logic restructures INTO passes --
//! behavior-identical, proven by the graph version root being UNCHANGED
//! for identical sources (content addressing is your regression harness)."
//!
//! `GraphVersion` (`atlas_graph_types::store::GraphVersion`) is a content
//! hash over every node's id+payload -- two builds of "the same" graph
//! content are equal iff their roots are equal (design doc §9b). This test
//! pins the version root over the REAL committed sources (`data/raw/kjv.json`
//! + `data/raw/xrefs/cross_references.txt` + the real compiled `AtlasData`)
//! to an EXACT, hardcoded hex value, captured BEFORE the pipeline
//! restructuring (commit history: this file was added and its expected
//! value captured against the pre-pipeline `build_graph_from_sources`/
//! `event_world::populate`/`Graph::build_indexes`/`add_justified_by` call
//! chain, then the pipeline commit landed on top with this test unchanged
//! and still green) -- so a divergence here means the restructuring changed
//! what gets built, not just how it's built, which is exactly the failure
//! mode "passes as data" is supposed to make impossible to introduce
//! silently.
//!
//! NOTE: this baseline value moves whenever a LATER M-C commit adds real
//! data to the graph (new adapters -- places/eras/polities/catechism/
//! mentions all change node/edge content, which is supposed to change the
//! version root; that is the root doing its job). Each such commit updates
//! the constant below directly in the same commit that changes what the
//! graph contains, with a one-line note of why, so this file stays a live
//! regression harness rather than a one-time snapshot.

use std::path::Path;

use atlas_graph::GraphService;
use atlas_graph_types::store::GraphSnapshot as _;

// M-C2 DELETION EVENT: `AtlasData::load`'s own five retiring-file reads
// (places/events/narratives/verses-kjv/cross-refs.json) return empty now
// -- `atlas_etl::compile::compile` is this crate's own real-data source
// for every test needing a fully-populated `AtlasData` from here on
// (already a normal dependency, no layering concern). Cached so this
// binary's own multiple `#[test]`s (if any) share one real compile.
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

#[test]
fn version_root_matches_the_captured_pre_pipeline_baseline() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv =
        std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    let atlas = real_atlas_data();

    let svc = GraphService::from_sources(&kjv_json, &xrefs_tsv, &atlas).expect("the real committed sources must build");
    let hex = atlas_graph::version_hex(svc.snapshot().version());

    // Captured once, before the M-C pipeline restructuring (controller
    // decision 3's own regression harness) -- update alongside a commit
    // that deliberately changes graph content, never silently.
    assert_eq!(
        hex, EXPECTED_VERSION_HEX,
        "graph version root diverged from the captured baseline -- if this build genuinely changed graph \
         content on purpose, update EXPECTED_VERSION_HEX in this same commit with a one-line reason; if not, \
         this is exactly the regression this test exists to catch"
    );
}

// Captured 2026-08-22, pre-pipeline-restructuring, base commit ddb4b73
// (EdgeMeta fix + EventWorld narrative retirement -- no adapter/graph
// content change), over the real committed data/raw + data/compiled:
// "4ad17fd0f68660f7".
//
// MOVED (deliberately -- controller decision 2, new adapters): the place/
// polity/catechism adapters now build real payload (Place gains lat/lon +
// KJV aliases, was `canonical` only; Polity/CatechismItem nodes newly
// exist) and new rows (named/mentions/catechism-link) over the SAME real
// `data/compiled/*.json` this test's own `real_atlas_data()` loads --
// this test itself calls the plain (eras-less) `GraphService::from_sources`,
// so eras stay absent from THIS particular root; the place/polity/
// catechism content change alone is what moved it: "ea2276b34f44d29f".
//
// MOVED AGAIN (deliberately -- map migration, controller decision 7):
// `PolityEraPayload.transition`/`.fall` widened from a collapsed display
// string to the fully structured `PolityDeltaPayload { event, verses,
// ref_note }` (graph-types/src/node.rs), so `/api/polities`'s own wire
// response (`PolityDeltaOut`) reconstructs losslessly from the graph
// payload alone instead of a lossy formatted summary -- real content
// change, not a bug.
//
// MOVED AGAIN (deliberately -- M-C2, re-homing handlers::place/event/
// verse/xrefs/narratives onto the graph): `NodePayload::Event` widened
// from `{ label }` to carry kind/from_year/to_year/order_key/verses/
// witnesses/robertson_section/acts_section/atlas_section/
// kjv_superscription/ref_note (real payload, not a stub -- same precedent
// as Place/Polity); `NodePayload::Narrative` widened from `{ label }` to
// `{ label, color }`; `event_world.rs`'s own `attests` row construction
// changed from one row per witness VERSE GROUP to one row per witness
// VERSE (a real gap fix -- a verse cited only in the interior of a group,
// e.g. MAT.26.6, now resolves back to its event through `attested-in`,
// matching `AtlasData::events_for_verse`'s own pre-existing witness-verse
// union) -- both are real content changes to the graph's own node
// payloads and row tables, not a bug.
//
// MOVED AGAIN (deliberately -- Batch M-D1, the HOTFIX-5 remaining-
// duplicates rectification): three Event nodes retired from the compiled
// graph entirely (`atlas_core::event_merge::EVENT_MERGE_PAIRS` gains
// `pr_rome`/`theo-384`, `theo-338`/`theo-337`,
// `ezr_altar_and_foundation`/`ret_jerusalem_altar` -- each entry's own
// reason has the full derivation) -- three fewer node ids feeding the
// content hash is exactly the expected effect of an identity-only merge,
// not a bug. New captured value: "eeb356047aa500f5".
//
// MOVED AGAIN (deliberately -- Batch P, the extensibility proof):
// person_adapter.rs newly runs in both NORMALIZE and MERGE/ALIAS, building
// 3,067 real Person nodes (widened payload: label/gender/birth_year/
// death_year/also_called) plus one `mentions` row per resolved, canon-
// sorted verse link (28,240 raw refs, all resolved) over the SAME real
// data/raw + data/compiled this test's own real_atlas_data() loads --
// thousands of new node ids and edge rows feeding the content hash is
// exactly the expected effect of a new adapter shipping real data, not a
// bug. New captured value: "d004fd6f72196214".
//
// MOVED AGAIN (deliberately -- Batch M-D3, owner rulings R1+R2): every
// `NodePayload::Event` node's own content hash changes (from_year/to_year/
// order_key dropped from the payload -- owner R1: "only keep narrative");
// every `Named` "row" that used to feed `graph.named` is gone from the
// content hash entirely (owner R2 -- the relation retired whole, and those
// rows never lowered into the port's own indexes in the first place, so
// this is a real, if narrow, "the hash used to include phantom weight"
// correction, not a data loss -- `NodePayload::Place.aliases` carries the
// SAME alias content, and Place node hashes are unaffected). Real,
// deliberate content changes to node payloads and a retired relation, not
// a bug. New captured value: "b668c9886baff013".
const EXPECTED_VERSION_HEX: &str = "b668c9886baff013";
