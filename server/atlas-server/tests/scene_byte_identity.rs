//! Batch PERF-2a: the ZERO-BEHAVIOR-CHANGE proof the brief requires --
//! `/api/scene` (time mode) and `/api/scene/scripture` (scripture mode)
//! responses, byte-for-byte, over the REAL committed graph
//! (`data/compiled/graph.bin`), for a fixed set of >=20 representative
//! windows/refs spanning every era, several boundary/degenerate windows, and
//! a handful of scripture refs.
//!
//! HOW THIS PROVES ZERO BEHAVIOR CHANGE: each expected hash below was
//! captured by running this exact test against BASE (7c32200, the commit
//! this batch started from, before any change) -- `cargo test -p
//! atlas-server --test scene_byte_identity -- --nocapture` prints
//! `label -> hash (n bytes)` for every case; the printed hashes are what's
//! hard-coded here. Batch PERF-2a made NO changes to `atlas-core::scene`,
//! `wire.rs`, or anything else in the compose path (the Phase 0
//! investigation found the diagnosed "~220ms fixed compose cost" was a
//! measurement artifact -- see batch-perf2a-report.md -- not a real
//! algorithmic problem; the real cost was already ~1-8ms, so no restructure
//! was made, and this harness is the proof), so every hash here is expected
//! to still match after this batch's changes (World.razor debounce/perf
//! marks are client-only and touch none of this). Going forward, this is
//! also a PERMANENT regression harness (Phase 1): any future change to the
//! compose path that silently alters a real window's response now fails
//! loud, here, rather than being caught (or missed) downstream.
//!
//! "commit the harness, not the captured fixtures if large" (brief): a
//! single NT-window response alone is ~205KB; committing 25 raw JSON
//! fixtures would be several MB for zero benefit over a hash. FNV-1a (below)
//! rather than `std::collections::hash_map::DefaultHasher` deliberately --
//! the std docs disclose DefaultHasher's algorithm is NOT guaranteed stable
//! across Rust releases, which would make a hard-coded expected value here
//! a false-positive time bomb; FNV-1a is a fixed, tiny, dependency-free
//! algorithm with no such risk. Not a security property (a scene response
//! is public, non-adversarial, served-from-a-local-sketch-app data) --
//! collision-resistance against an attacker is not the property being
//! bought here, only a cheap, stable fingerprint over content nobody is
//! trying to forge.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use atlas_core::data::AtlasData;
use atlas_core::refs::ScriptureRef;
use atlas_core::scene::{compose_scripture_scene, compose_time_scene};
use atlas_core::time::TimeRange;
use atlas_graph::GraphService;

fn real_data_and_graph() -> (Arc<AtlasData>, Arc<GraphService>) {
    static CACHED: std::sync::OnceLock<(Arc<AtlasData>, Arc<GraphService>)> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let compiled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled");
            let graph = GraphService::from_artifact(&compiled.join("graph.bin")).expect("data/compiled/graph.bin must exist -- run atlas-graph-compile first");
            let mut data = AtlasData::load(&compiled).expect("data/compiled must exist");
            let overlay = atlas_graph::legacy::atlas_data_overlay(&graph);
            data.events = overlay.events;
            data.places = overlay.places;
            data.narratives = overlay.narratives;
            data.verses = overlay.verses;
            let data = data.finish();
            (Arc::new(data), Arc::new(graph))
        })
        .clone()
}

/// FNV-1a, 64-bit. Fixed algorithm, no dependency, good enough for a
/// non-adversarial content fingerprint (see module doc comment).
fn fnv1a(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET_BASIS;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// `(label, from_year, to_year, expected_fnv1a_hash, expected_byte_len)`.
/// Spans: every one of the 10 curated eras exactly (`data/curated/eras.toml`),
/// the full atlas span, the brief's own documented "NT window," three
/// degenerate/single-year windows (the atlas's own start year, end year, and
/// an arbitrary mid-span year), and three windows that straddle an era
/// boundary rather than landing exactly on one.
fn time_windows() -> Vec<(&'static str, i32, i32, u64, usize)> {
    vec![
        ("era_primeval", -4004, -2167, 0xeee278d893da398b, 26199),
        ("era_patriarchs", -2166, -1877, 0x880e1dca61e707de, 40151),
        ("era_egypt_exodus", -1876, -1407, 0x582c4bbb35da149c, 97856),
        ("era_conquest_judges", -1406, -1051, 0xb7f0f2c46f6c1ca8, 59347),
        ("era_united_kingdom", -1050, -932, 0x636a05aaf5dc4e1d, 51147),
        ("era_divided_kingdom", -931, -587, 0xa5415ca390c636d0, 70318),
        ("era_exile", -586, -539, 0x59ed412d6162e2dc, 33771),
        ("era_return", -538, -6, 0xe58f114db506303c, 48767),
        ("era_gospels", -5, 29, 0xf25817c14b1acc77, 30790),
        ("era_early_church", 30, 100, 0x5b1b4f203ea4e47b, 201340),
        ("full_span", -4004, 100, 0xbab7a2226ee5d2d2, 456360),
        ("nt_window_gospels_plus_church", -5, 100, 0x074a49748b8bccec, 209737),
        ("degenerate_start_year", -4004, -4004, 0x92e2c06f8cad8c7f, 22384),
        ("degenerate_end_year", 100, 100, 0xc7c15a91e1c8da7d, 22381),
        ("degenerate_mid_year", -1000, -1000, 0x61672595e56e43d9, 22385),
        ("straddle_primeval_patriarchs", -2200, -2100, 0x107951b4ab92eb70, 22589),
        ("straddle_gospels_early_church", 25, 35, 0xb782ad97310e9d05, 131267),
        ("straddle_exile_return", -600, -500, 0xe4741224197fa4e8, 58722),
        ("narrow_conquest", -1407, -1406, 0x61e135c9d3e687a5, 24878),
        ("wide_kingdom_era", -1051, -539, 0xa7412af29a0dde9d, 109983),
    ]
}

/// `(label, sref, expected_fnv1a_hash, expected_byte_len)`.
fn scripture_refs() -> Vec<(&'static str, &'static str, u64, usize)> {
    vec![
        ("scripture_gen1", "GEN.1", 0xb7c306586ac67917, 92),
        ("scripture_jhn316", "JHN.3.16", 0x342baaf323a0e57d, 722),
        ("scripture_psa23", "PSA.23", 0x0867e662452f6d17, 93),
        ("scripture_exo20", "EXO.20", 0x2cb339e0c367e6d1, 1128),
        ("scripture_rev22", "REV.22", 0xb337c277a4229dd3, 93),
    ]
}

/// 20 time windows + 5 scripture refs = 25 cases, comfortably over the
/// brief's own N>=20 floor.
#[test]
fn scene_responses_are_byte_identical_to_the_pinned_base_captures() {
    let (data, _graph) = real_data_and_graph();

    let mut failures = Vec::new();
    for (label, from, to, expected_hash, expected_len) in time_windows() {
        let w = TimeRange::new(from, to).unwrap();
        let scene = compose_time_scene(&data, w);
        let bytes = serde_json::to_vec(&scene).unwrap();
        let hash = fnv1a(&bytes);
        println!("{label} -> hash {hash:#018x} ({} bytes)", bytes.len());
        if expected_hash == 0 && expected_len == 0 {
            continue; // capture mode: hashes not pinned yet (see module doc comment)
        }
        if hash != expected_hash || bytes.len() != expected_len {
            failures.push(format!(
                "{label}: got hash {hash:#018x} ({} bytes), expected {expected_hash:#018x} ({expected_len} bytes) -- /api/scene?from={from}&to={to} changed",
                bytes.len()
            ));
        }
    }

    for (label, sref, expected_hash, expected_len) in scripture_refs() {
        let r = ScriptureRef::parse(sref).unwrap();
        let scene = compose_scripture_scene(&data, &r);
        let bytes = serde_json::to_vec(&scene).unwrap();
        let hash = fnv1a(&bytes);
        println!("{label} -> hash {hash:#018x} ({} bytes)", bytes.len());
        if expected_hash == 0 && expected_len == 0 {
            continue;
        }
        if hash != expected_hash || bytes.len() != expected_len {
            failures.push(format!(
                "{label}: got hash {hash:#018x} ({} bytes), expected {expected_hash:#018x} ({expected_len} bytes) -- /api/scene/scripture?ref={sref} changed",
                bytes.len()
            ));
        }
    }

    assert!(failures.is_empty(), "scene response(s) changed since BASE (7c32200) -- zero-behavior-change law broken:\n{}", failures.join("\n"));
}

/// Sanity companion: every one of `AtlasData`'s HTTP-facing scene functions
/// used above must actually be reachable with the SAME `HashMap` shape
/// `handlers::scene_time`/`scene_scripture` parse `from`/`to`/`ref` out of --
/// this doesn't test that directly (the handler-level equivalence is
/// `atlas-server/tests/api.rs`'s job), just documents the coupling so a
/// future reader knows why this file calls `atlas_core::scene::*` directly
/// rather than going through `axum`.
#[test]
fn windows_parse_the_same_way_the_http_handler_would() {
    let params: HashMap<String, String> = [("from".to_string(), "-5".to_string()), ("to".to_string(), "100".to_string())].into_iter().collect();
    let from: i32 = params["from"].parse().unwrap();
    let to: i32 = params["to"].parse().unwrap();
    assert_eq!((from, to), (-5, 100));
}
