use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Path as AxPath, State};
use axum::http::HeaderMap;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

use atlas_core::data::AtlasData;
use atlas_core::refs::ScriptureRef;
use atlas_core::scene::{compose_scripture_scene, compose_time_scene};
use atlas_core::time::TimeRange;
use atlas_graph::GraphService;
use atlas_contract::query::Contract;
use atlas_contract::reference::Reference;
use atlas_contract::{catechism, events, graph, map, places, reading};

fn repo_data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn load_real() -> (Arc<AtlasData>, Arc<GraphService>) {
    let compiled = repo_data_dir().join("compiled");
    let (graph, data, _sources) = GraphService::from_sections(&compiled)
        .expect("data/compiled/manifest.toml + sections/ must exist -- run atlas-graph-compile first (see README)");
    let data = data.finish();
    graph.scene_source(&data);
    (Arc::new(data), Arc::new(graph))
}

fn rt() -> Runtime {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
}

/// A reference as a route would have read it off the request. `unwrap` is the
/// benchmark saying that a reference written out here must be one this atlas
/// serves; a bench measuring a refusal would measure nothing.
fn asked_for<T: std::str::FromStr>(raw: &str) -> Reference<T> {
    Reference(raw.parse().unwrap_or_else(|_| panic!("a reference this benchmark names must be one this atlas reads")))
}

fn bench_scene_pure(c: &mut Criterion) {
    let (data, graph) = load_real();
    let source = graph.scene_source(&data);
    let mut group = c.benchmark_group("scene_pure");

    let windows: &[(&str, i32, i32)] = &[
        ("full_span", -4004, 100),
        ("patriarchs_era", -2166, -1877),
        ("nt_window", -5, 100),
        ("degenerate_1yr", -4004, -4004),
        ("exile_era", -586, -539),
    ];
    for (label, from, to) in windows {
        let w = TimeRange::new(*from, *to).unwrap();
        group.bench_function(*label, |b| b.iter(|| compose_time_scene(black_box(source), black_box(w))));
    }

    let chapter_ref = ScriptureRef::parse("JHN.3").unwrap();
    group.bench_function("scripture_chapter", |b| b.iter(|| compose_scripture_scene(black_box(source), black_box(&chapter_ref))));

    group.finish();
}

fn bench_handlers(c: &mut Criterion) {
    let (data, graph) = load_real();
    let rt = rt();
    let mut group = c.benchmark_group("handlers");

    group.bench_function("scene_time", |b| {
        b.iter(|| rt.block_on(map::scene_time(State(data.clone()), State(graph.clone()), Contract(map::SceneWindow { from: -5, to: 100 }))))
    });
    group.bench_function("scene_scripture", |b| {
        b.iter(|| rt.block_on(map::scene_scripture(State(data.clone()), State(graph.clone()), Contract(map::ScripturePassage { r#ref: "JHN.3".to_string() }))))
    });
    group.bench_function("books", |b| b.iter(|| rt.block_on(reading::books(State(data.clone())))));
    group.bench_function("eras", |b| b.iter(|| rt.block_on(map::eras(State(graph.clone())))));
    group.bench_function("narratives", |b| b.iter(|| rt.block_on(map::narratives(State(graph.clone())))));
    group.bench_function("landmarks", |b| b.iter(|| rt.block_on(map::landmarks(State(data.clone())))));
    group.bench_function("land_mask", |b| b.iter(|| rt.block_on(map::land_mask(State(data.clone())))));
    group.bench_function("polities", |b| {
        b.iter(|| rt.block_on(map::polities(State(graph.clone()), Contract(map::SceneWindow { from: -4004, to: 100 }))))
    });
    group.bench_function("chapter", |b| {
        b.iter(|| rt.block_on(reading::chapter(State(data.clone()), State(graph.clone()), asked_for("JHN.3"))))
    });
    group.bench_function("verse", |b| {
        b.iter(|| rt.block_on(reading::verse(State(data.clone()), State(graph.clone()), asked_for("JHN.3.16"))))
    });
    group.bench_function("xrefs", |b| b.iter(|| rt.block_on(reading::xrefs(State(graph.clone()), asked_for("JHN.3.16")))));
    group.bench_function("place", |b| {
        b.iter(|| rt.block_on(places::place(State(data.clone()), State(graph.clone()), AxPath("hebron".to_string()), Contract(places::PlacePeriod { from: None, to: None }))))
    });
    group.bench_function("event", |b| {
        b.iter(|| rt.block_on(events::event(State(data.clone()), State(graph.clone()), AxPath("ab_ur".to_string()))))
    });
    group.bench_function("narrative_event_positions", |b| {
        b.iter(|| rt.block_on(events::narrative_event_positions(State(data.clone()), State(graph.clone()), AxPath("ab_ur".to_string()))))
    });
    group.bench_function("catechism_for_span", |b| {
        b.iter(|| rt.block_on(catechism::catechism_for_span(State(data.clone()), State(graph.clone()), asked_for("EXO.20.3"))))
    });
    group.bench_function("catechism_item", |b| {
        b.iter(|| rt.block_on(catechism::catechism_item(State(data.clone()), State(graph.clone()), AxPath("commandment-1".to_string()))))
    });

    group.finish();
}

const CITES: atlas_graph_types::edge::EdgeKind = atlas_graph_types::edge::EdgeKind::Directed(atlas_graph_types::edge::RelationId::Cites, atlas_graph_types::edge::Direction::Forward);

fn bench_graph_handlers(c: &mut Criterion) {
    let (_data, graph) = load_real();
    let rt = rt();
    let mut group = c.benchmark_group("graph_handlers");

    group.bench_function("node_card", |b| {
        b.iter(|| rt.block_on(graph::node_card(State(graph.clone()), asked_for("text-unit:JHN.3.16"))))
    });
    group.bench_function("node_edges", |b| {
        b.iter(|| {
            rt.block_on(graph::node_edges(
                State(graph.clone()),
                asked_for("text-unit:JHN.3.16"),
                Contract(graph::EdgePageQuery { kind: CITES, cursor: Default::default(), limit: Default::default() }),
            ))
        })
    });
    group.bench_function("text_window", |b| {
        b.iter(|| {
            rt.block_on(graph::text_window(
                State(graph.clone()),
                HeaderMap::new(),
                Contract(graph::TextWindowQuery { r#ref: "JHN.3.16".to_string(), n: Default::default(), dir: None, scope: None, corpus: None }),
            ))
        })
    });

    group.finish();
}

fn bench_sections_open(c: &mut Criterion) {
    let mut group = c.benchmark_group("sections_open");
    group.sample_size(10);
    group.bench_function("full_startup_open", |b| b.iter(load_real));
    group.finish();
}

criterion_group!(scene_pure, bench_scene_pure);
criterion_group!(handlers_query, bench_handlers);
criterion_group!(graph_handlers_query, bench_graph_handlers);
criterion_group!(sections_open, bench_sections_open);
criterion_main!(scene_pure, handlers_query, graph_handlers_query, sections_open);
