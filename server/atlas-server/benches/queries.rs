//! Batch PERF-2a Phase 1: the permanent criterion regression harness over
//! the REAL committed graph (`data/compiled/graph.bin`, loaded exactly the
//! way `atlas-server`'s own real startup loads it -- see `load_real` below,
//! which mirrors `main.rs`'s default artifact-load path field for field).
//!
//! Own bench target (`cargo bench -p atlas-server`, or `cargo bench` from
//! server/) -- NOT part of `cargo test`'s own count (server/Cargo.toml's own
//! STANDING COUNTING PROCEDURE comment). A fast SMOKE-TIER subset of these
//! same queries also lives in `tests/perf_smoke.rs`, wired into the normal
//! test suite with generous (x3-class) regression thresholds -- THAT is the
//! part that gates CI; this file is the deep, full-distribution instrument
//! a human runs by hand when investigating a specific query's performance.
//! See BENCHMARKS.md for recorded baselines + machine context, and
//! batch-perf2a-report.md for the investigation this batch ran to justify
//! (or rule out) a given query's own work.
//!
//! Coverage (brief's own enumeration): scene/time queries, verse/chapter
//! window, text window, xrefs for span, node card, node edges, catechism for
//! span, place/event/narrative lookups, eras/polities/landmarks, and the
//! full artifact load. "Admission" (`atlas_graph_types::store::
//! assert_answers_match`, comparing the artifact-loaded graph against a
//! from-raw-sources rebuild over the FULL graph) is deliberately NOT
//! duplicated here: it's already a committed, passing law
//! (`atlas-graph/tests/artifact_conformance.rs::
//! serialized_artifact_is_admitted_and_loads_under_the_committed_ceiling`),
//! self-documented there as "compile-time only... never run at server
//! startup" and ~15-40s per M-B's own prior report -- criterion's minimum
//! sample size (10) would cost 2.5-6.5+ minutes on ONE bench function alone
//! for a number that test already measures and prints
//! (`--nocapture`) every time it runs. Duplicating that heavy from-source
//! rebuild machinery here (four optional corpora, private loader helpers on
//! `GraphService`) to re-derive the same number under criterion's own
//! statistical harness was evaluated and rejected as the wrong tool for a
//! compile-time-only correctness gate; BENCHMARKS.md quotes that test's own
//! measured figure instead. The fast, real, runtime-hot artifact LOAD
//! (`GraphService::from_artifact` + `AtlasData::load` + overlay + `finish`
//! -- exactly `main.rs`'s default path, the one actually on the owner's
//! "make it fast" critical path) IS benched below, under `artifact_load`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Path as AxPath, Query as AxQuery, State};
use axum::http::HeaderMap;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

use atlas_core::data::AtlasData;
use atlas_core::refs::ScriptureRef;
use atlas_core::scene::{compose_scripture_scene, compose_time_scene};
use atlas_core::time::TimeRange;
use atlas_graph::GraphService;
use atlas_server::{graph_handlers, handlers};

fn repo_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

/// Field-for-field the SAME sequence `main.rs`'s default (non-`--build-from-raw`)
/// startup path runs: load the serialized graph artifact, load the ten
/// surviving compiled JSON files, overlay the graph-derived fields onto
/// `AtlasData`, `finish()`. This is what every query bench below queries
/// against, and what `bench_artifact_load` itself times end to end.
fn load_real() -> (Arc<AtlasData>, Arc<GraphService>) {
    let compiled = repo_data_dir().join("compiled");
    let graph = GraphService::from_artifact(&compiled.join("graph.bin"))
        .expect("data/compiled/graph.bin must exist -- run atlas-graph-compile first (see README)");
    let mut data = AtlasData::load(&compiled).expect("data/compiled must exist");
    let overlay = atlas_graph::legacy::atlas_data_overlay(&graph);
    data.events = overlay.events;
    data.places = overlay.places;
    data.narratives = overlay.narratives;
    data.verses = overlay.verses;
    let data = data.finish();
    (Arc::new(data), Arc::new(graph))
}

fn rt() -> Runtime {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
}

fn qmap(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// Pure `atlas_core::scene` composition, no HTTP/JSON/axum machinery at all
/// -- isolates the exact function the Phase 0 investigation profiled
/// (`compose_time_scene`/`compose_scripture_scene`), so a regression here
/// can never be masked or falsely blamed on handler-layer overhead.
fn bench_scene_pure(c: &mut Criterion) {
    let (data, _graph) = load_real();
    let mut group = c.benchmark_group("scene_pure");

    // Same five windows batch-perf2a-report.md's before/after table uses --
    // spans an era, an exact era, the full atlas span, the documented
    // ~208KB "NT window," and a one-year degenerate window (all-quiet).
    let windows: &[(&str, i32, i32)] = &[
        ("full_span", -4004, 100),
        ("patriarchs_era", -2166, -1877),
        ("nt_window", -5, 100),
        ("degenerate_1yr", -4004, -4004),
        ("exile_era", -586, -539),
    ];
    for (label, from, to) in windows {
        let w = TimeRange::new(*from, *to).unwrap();
        group.bench_function(*label, |b| b.iter(|| compose_time_scene(black_box(&data), black_box(w))));
    }

    let chapter_ref = ScriptureRef::parse("JHN.3").unwrap();
    group.bench_function("scripture_chapter", |b| b.iter(|| compose_scripture_scene(black_box(&data), black_box(&chapter_ref))));

    group.finish();
}

/// The real axum handlers (`atlas_server::handlers`/`graph_handlers`) called
/// directly with hand-built extractors (`State`/`Path`/`Query` are public
/// tuple structs -- this is the standard way to bench/unit-test an axum
/// handler without a socket or even a `Router`), over real, valid,
/// committed-data ids/refs. Covers the brief's full enumerated surface:
/// scene/time queries, verse/chapter window, xrefs for span, catechism for
/// span, place/event/narrative lookups, eras/polities/landmarks.
fn bench_handlers(c: &mut Criterion) {
    let (data, graph) = load_real();
    let rt = rt();
    let mut group = c.benchmark_group("handlers");

    group.bench_function("scene_time", |b| {
        b.iter(|| rt.block_on(handlers::scene_time(State(data.clone()), AxQuery(qmap(&[("from", "-5"), ("to", "100")])))))
    });
    group.bench_function("scene_scripture", |b| {
        b.iter(|| rt.block_on(handlers::scene_scripture(State(data.clone()), AxQuery(qmap(&[("ref", "JHN.3")])))))
    });
    group.bench_function("books", |b| b.iter(|| rt.block_on(handlers::books(State(data.clone())))));
    group.bench_function("eras", |b| b.iter(|| rt.block_on(handlers::eras(State(graph.clone())))));
    group.bench_function("narratives", |b| b.iter(|| rt.block_on(handlers::narratives(State(graph.clone())))));
    group.bench_function("landmarks", |b| b.iter(|| rt.block_on(handlers::landmarks(State(data.clone())))));
    group.bench_function("land_mask", |b| b.iter(|| rt.block_on(handlers::land_mask(State(data.clone())))));
    group.bench_function("polities", |b| {
        b.iter(|| rt.block_on(handlers::polities(State(graph.clone()), AxQuery(qmap(&[("from", "-4004"), ("to", "100")])))))
    });
    group.bench_function("chapter", |b| {
        b.iter(|| rt.block_on(handlers::chapter(State(data.clone()), State(graph.clone()), AxPath("JHN.3".to_string()))))
    });
    group.bench_function("verse", |b| {
        b.iter(|| rt.block_on(handlers::verse(State(data.clone()), State(graph.clone()), AxPath("JHN.3.16".to_string()))))
    });
    group.bench_function("xrefs", |b| b.iter(|| rt.block_on(handlers::xrefs(State(graph.clone()), AxPath("JHN.3.16".to_string())))));
    group.bench_function("place", |b| {
        b.iter(|| rt.block_on(handlers::place(State(data.clone()), State(graph.clone()), AxPath("hebron".to_string()), AxQuery(HashMap::new()))))
    });
    group.bench_function("event", |b| {
        b.iter(|| rt.block_on(handlers::event(State(data.clone()), State(graph.clone()), AxPath("ab_ur".to_string()))))
    });
    group.bench_function("narrative_event_positions", |b| {
        b.iter(|| rt.block_on(handlers::narrative_event_positions(State(data.clone()), State(graph.clone()), AxPath("ab_ur".to_string()))))
    });
    group.bench_function("catechism_for_span", |b| {
        b.iter(|| rt.block_on(handlers::catechism_for_span(State(data.clone()), AxPath("EXO.20.3".to_string()))))
    });
    group.bench_function("catechism_item", |b| {
        b.iter(|| rt.block_on(handlers::catechism_item(State(data.clone()), AxPath("commandment-1".to_string()))))
    });

    group.finish();
}

/// The generic typed-graph endpoints (design doc §5/§6): node card, node
/// edges, text window -- `atlas_server::graph_handlers`, the newer surface
/// the REFOUNDED typed-edge graph serves directly (not through `AtlasData`
/// at all).
fn bench_graph_handlers(c: &mut Criterion) {
    let (_data, graph) = load_real();
    let rt = rt();
    let mut group = c.benchmark_group("graph_handlers");

    group.bench_function("node_card", |b| {
        b.iter(|| rt.block_on(graph_handlers::node_card(State(graph.clone()), AxPath("text-unit:JHN.3.16".to_string()))))
    });
    group.bench_function("node_edges", |b| {
        b.iter(|| {
            rt.block_on(graph_handlers::node_edges(
                State(graph.clone()),
                AxPath("text-unit:JHN.3.16".to_string()),
                AxQuery(qmap(&[("kind", "cites")])),
            ))
        })
    });
    group.bench_function("text_window", |b| {
        b.iter(|| rt.block_on(graph_handlers::text_window(State(graph.clone()), HeaderMap::new(), AxQuery(qmap(&[("ref", "JHN.3.16")])))))
    });

    group.finish();
}

/// The full artifact-load path (`GraphService::from_artifact` + `AtlasData::
/// load` + overlay + `finish`) -- exactly `main.rs`'s default startup, timed
/// end to end, freshly, on EVERY sample (not just once in setup, unlike
/// every other group above). Sample size dropped to criterion's own minimum
/// (10): at ~1.7s/load (server startup log, this batch's own re-baseline),
/// even 10 samples cost ~20-30s, and this is the one bench in this file
/// whose whole POINT is the real end-to-end cost of the thing every other
/// group's `load_real()` call pays once and amortizes away.
fn bench_artifact_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("artifact_load");
    group.sample_size(10);
    group.bench_function("full_startup_load", |b| b.iter(load_real));
    group.finish();
}

criterion_group!(scene_pure, bench_scene_pure);
criterion_group!(handlers_query, bench_handlers);
criterion_group!(graph_handlers_query, bench_graph_handlers);
criterion_group!(artifact_load, bench_artifact_load);
criterion_main!(scene_pure, handlers_query, graph_handlers_query, artifact_load);
