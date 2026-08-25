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
    // CORP-1a: real vendored brain-fuel data joins the root computation --
    // this test's own real_atlas_data()/kjv_json/xrefs_tsv reads are
    // already "over the real committed sources"; leaving brainfuel out
    // here would test a DIFFERENT (KJV-only) graph than the one
    // atlas-graph-compile actually produces.
    let brainfuel = atlas_etl::brainfuel::read_all(&dir.join("brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step first");

    let svc = GraphService::from_sources_with_eras_and_brainfuel(&kjv_json, &xrefs_tsv, &atlas, &[], Some(&brainfuel)).expect("the real committed sources must build");
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
// MOVED (deliberately -- ENT-1 writer window, 2026-08-24): every Place/
// Person payload gained `description: Option<String>` (None until the
// Easton's adapter fills it), which changes each node's canonical
// bytes -- the root moving IS the widening being real.
// MOVED AGAIN (deliberately -- ENT-1a, 2026-08-24): `description_adapter::
// fill_descriptions` (new MERGE/ALIAS call) now actually FILLS that field
// with real Easton's Bible Dictionary (1897, PD) prose over the real
// committed data -- 2,344/3,067 persons (76.4%) and 837/1,373 places
// (61.0%) go from `None` to `Some(text)`, changing those nodes' own
// canonical bytes; this is the widening becoming real content, the exact
// same class of move the ENT-1 crate commit's own note above already
// named as expected once a data batch landed.
// MOVED AGAIN (deliberately -- PG-1a, 2026-08-24): 38 new PeopleGroup
// nodes (23 Theographic `peopleGroups.json` records + 6 curated nation
// seeds + 9 reclassified Gen-10 gentilics), 27 new `Mentions(PeopleGroup)`
// rows (the reclassified nine's own real `verse_links`), and 18 new
// `named_after` rows all feed the content hash; the nine reclassified
// slugs' own Person nodes/mentions are GONE (person_adapter's own
// exclusion), so Person-side content shrinks by exactly that much too --
// both are the expected effect of a real reclassification, not a bug.
// `description_adapter::fill_descriptions` also newly fills 8/38 of the
// new PeopleGroup nodes' own descriptions (tier c only) and 2,337/3,058
// persons (down from 2,344/3,067 -- seven of the nine reclassified had a
// filled description as a Person; both numbers are real, honest
// consequences of the same reclassification). New captured value:
// "8855af2b0742cc31".
//
// MOVED AGAIN (deliberately -- Batch CORP-1a, "brain-fuel editions: the
// ingestion half", 2026-08-24): every existing KJV TextUnit node's own
// `renderings` LayerMap gains up to five new non-KJV entries (Clementine
// Vulgate/Westminster Leningrad Codex/Douay-Rheims/Biblia 1776/Karl XII:s
// Bibel, whichever apply to that verse's own testament -- 31,092/23,145/
// 23,132/31,102/31,099 verses respectively, `brainfuel_adapter.rs`'s own
// module doc comment), changing every one of those TextUnit nodes' own
// canonical bytes; 6 new `Translation` nodes are authored (one per
// ingested edition -- NOT for KJV itself, no pre-existing pattern was
// found to follow, disclosed in this batch's own report). This test's own
// real-data source now threads the real vendored `data/raw/brain-fuel-
// bible/` corpus through `GraphService::from_sources_with_eras_and_
// brainfuel` (previously plain `from_sources`) -- otherwise this harness
// would keep proving a DIFFERENT, KJV-only graph than the one
// `atlas-graph-compile` actually produces. New captured value:
// "5753f377e4bbcfe9".
//
// MOVED AGAIN (deliberately -- Batch KJV-CASE, owner ruling "KJV-CASE-1
// APPROVED... 3. yes", 2026-08-24): the Tetragrammaton LORD/Lord
// case-restoration pass (`atlas_etl::brainfuel::restore_kjv_case`, wired
// into `build::build_graph_from_sources_with_eras_and_brainfuel`) changes
// 5,473 KJV `TextUnit` nodes' own canonical bytes -- every position where
// our source's casing disagreed with brain-fuel's own `king_james` column
// under ASCII case-folding (`the LORD`/`Lord`/`GOD`/`JAH` Tetragrammaton
// convention our own `data/raw/kjv.json` had lost) adopts brain-fuel's
// casing, characters otherwise byte-identical (the case-only law,
// verified over all 31,102 real positions in `atlas-etl`'s own
// `brainfuel_real_data.rs`). This is real, deliberate, KJV-INERRANCY-
// DIRECTIVE-compliant restoration content, not a bug -- the root moving
// IS the restoration being real. New captured value (this test's own
// eras-less `GraphService::from_sources_with_eras_and_brainfuel(..., &[],
// ...)` build, NOT `atlas-graph-compile`'s own with-eras root -- those two
// have always differed, per this file's own historical note above):
// "d6779aa183ec6ea7".
//
// MOVED AGAIN (deliberately -- Batch GAZ-1-R1, 2026-08-24): `lebo-hamath`
// gains 6 curated KJV aliases (was 0) -- `event_world::place_node`'s own
// `aliases: Vec<String>` payload field (part of `NodePayload::Place`,
// hashed into the version root) changes for this one node, from `[]` to
// the 6-element list -- real, deliberate content, not a bug. (The CHRON-FIX
// commit immediately before this one, theo-87's date correction, did NOT
// move this root: `NodePayload::Event` dropped from_year/to_year/order_key
// from its own payload back at Batch M-D3, so a `THEO_DATE_OVERRIDES`
// correction only ever touches a `DatedBy` EDGE, never a node payload --
// confirmed by that commit's own report, not assumed here.) New captured
// value: "d8d7d4acea171fd5".
const EXPECTED_VERSION_HEX: &str = "d8d7d4acea171fd5";
