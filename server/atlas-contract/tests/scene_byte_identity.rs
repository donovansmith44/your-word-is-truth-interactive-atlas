use std::path::Path;
use std::sync::Arc;

use atlas_core::refs::ScriptureRef;
use atlas_core::scene::{compose_scripture_scene, compose_time_scene};
use atlas_core::time::TimeRange;
use atlas_graph::scene_source::GraphSceneSource;
use atlas_graph::GraphService;

fn real_scene_source_and_graph() -> (Arc<GraphSceneSource>, Arc<GraphService>) {
    static CACHED: std::sync::OnceLock<(Arc<GraphSceneSource>, Arc<GraphService>)> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let compiled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled");
            let (graph, sidecars, _sources) = GraphService::from_sections(&compiled).expect("data/compiled/manifest.toml + sections/ must exist -- run atlas-graph-compile first");
            let sidecars = sidecars.finish();
            let source = GraphSceneSource::build(&graph, &sidecars);
            (Arc::new(source), Arc::new(graph))
        })
        .clone()
}

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

fn time_windows() -> Vec<(&'static str, i32, i32, u64, usize)> {
    vec![
        ("era_primeval", -4004, -2167, 0xf083d261b01e935a, 42120),
        ("era_patriarchs", -2166, -1877, 0x59ecfcd13bcebd01, 57985),
        ("era_egypt_exodus", -1876, -1407, 0x8f5d981e925b89ba, 120747),
        ("era_conquest_judges", -1406, -1051, 0x932459077bbc8c39, 78475),
        ("era_united_kingdom", -1050, -932, 0x923579cf5fde045f, 70412),
        ("era_divided_kingdom", -931, -587, 0x15ae0f8e972530f4, 93093),
        ("era_exile", -586, -539, 0x0ebc0e8f4cec1584, 50336),
        ("era_return", -538, -6, 0xda1acb8a571a5e0f, 67867),
        ("era_gospels", -5, 29, 0x20d87a9456c5abab, 46272),
        ("era_early_church", 30, 100, 0x4c3e76f329914f59, 231156),
        ("full_span", -4004, 100, 0xb7de18e5c890fd79, 519798),
        ("nt_window_gospels_plus_church", -5, 100, 0xcaa40f01528abafc, 240023),
        ("degenerate_start_year", -4004, -4004, 0x0faa6c1b6634cd05, 37395),
        ("degenerate_end_year", 100, 100, 0x5007dff97712f0e0, 37389),
        ("degenerate_mid_year", -1000, -1000, 0xbac0c5ec382429de, 37396),
        ("straddle_primeval_patriarchs", -2200, -2100, 0x6147a0a68aa56486, 37682),
        ("straddle_gospels_early_church", 25, 35, 0x196dba35f373439a, 154735),
        ("straddle_exile_return", -600, -500, 0xd32066036d81a5f4, 78788),
        ("narrow_conquest", -1407, -1406, 0xc36b7c1746189285, 40291),
        ("wide_kingdom_era", -1051, -539, 0x761ee3d1480ee74b, 138487),
    ]
}

fn scripture_refs() -> Vec<(&'static str, &'static str, u64, usize)> {
    vec![
        ("scripture_gen1", "GEN.1", 0xb7c306586ac67917, 92),
        ("scripture_jhn316", "JHN.3.16", 0x9b95681aef7d1441, 902),
        ("scripture_psa23", "PSA.23", 0x0867e662452f6d17, 93),
        ("scripture_exo20", "EXO.20", 0xfe2ffa5c868afca3, 1396),
        ("scripture_rev22", "REV.22", 0xb337c277a4229dd3, 93),
    ]
}

#[test]
fn scene_responses_are_byte_identical_to_the_pinned_base_captures() {
    let (source, _graph) = real_scene_source_and_graph();

    let mut failures = Vec::new();
    for (label, from, to, expected_hash, expected_len) in time_windows() {
        let w = TimeRange::new(from, to).unwrap();
        let scene = compose_time_scene(&*source, w);
        let bytes = serde_json::to_vec(&scene).unwrap();
        let hash = fnv1a(&bytes);
        println!("{label} -> hash {hash:#018x} ({} bytes)", bytes.len());
        if expected_hash == 0 && expected_len == 0 {
            continue;
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
        let scene = compose_scripture_scene(&*source, &r);
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

    assert!(failures.is_empty(), "scene response(s) changed since the pinned baseline (BASE 7c32200 for PERF-2a; re-pinned by Batch CHRON-1's own recompile, see this file's own module doc) -- if this batch is NOT a deliberate data/behavior change, the zero-behavior-change law is broken:\n{}", failures.join("\n"));
}

#[test]
fn a_window_this_file_composes_with_is_the_one_the_route_would_read() {
    // Arrange
    let asked = "from=-5&to=100";
    // Act
    let read: atlas_contract::map::SceneWindow = serde_urlencoded::from_str(asked).expect("the route's own parameters read");
    // Assert
    assert_eq!(read.span().expect("a span of real years"), TimeRange::new(-5, 100).unwrap());
}
