use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use atlas_contract::graph::bible_text_units;
use atlas_core::refs::ScriptureRef;
use atlas_core::scene::{compose_scripture_scene, compose_time_scene};
use atlas_core::time::TimeRange;
use atlas_graph::scene_source::GraphSceneSource;
use atlas_graph::window::{self, WindowDir};
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

fn median_of<F: FnMut()>(iters: usize, mut f: F) -> Duration {
    let mut samples: Vec<Duration> = (0..iters)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed()
        })
        .collect();
    samples.sort();
    samples[samples.len() / 2]
}

#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn scene_time_full_span_completes_within_smoke_threshold() {
    let (source, _graph) = real_scene_source_and_graph();
    let w = TimeRange::new(-4004, 100).unwrap();
    let elapsed = median_of(7, || {
        let _ = compose_time_scene(&*source, w);
    });
    println!("PERF SMOKE {}: {elapsed:?} (gate {}ms)", "scene_time_full_span_completes_within_smoke_threshold", 75);
    assert!(elapsed < Duration::from_millis(75), "compose_time_scene(full span) took {elapsed:?}, over the 75ms smoke gate (baseline ~8ms HTTP / sub-ms pure-compute -- see BENCHMARKS.md)");
}

#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn scene_time_nt_window_completes_within_smoke_threshold() {
    let (source, _graph) = real_scene_source_and_graph();
    let w = TimeRange::new(-5, 100).unwrap();
    let elapsed = median_of(7, || {
        let _ = compose_time_scene(&*source, w);
    });
    println!("PERF SMOKE {}: {elapsed:?} (gate {}ms)", "scene_time_nt_window_completes_within_smoke_threshold", 75);
    assert!(elapsed < Duration::from_millis(75), "compose_time_scene(NT window) took {elapsed:?}, over the 75ms smoke gate (baseline ~4-5ms HTTP -- see BENCHMARKS.md)");
}

#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn scene_scripture_chapter_completes_within_smoke_threshold() {
    let (source, _graph) = real_scene_source_and_graph();
    let r = ScriptureRef::parse("JHN.3").unwrap();
    let elapsed = median_of(7, || {
        let _ = compose_scripture_scene(&*source, &r);
    });
    println!("PERF SMOKE {}: {elapsed:?} (gate {}ms)", "scene_scripture_chapter_completes_within_smoke_threshold", 50);
    assert!(elapsed < Duration::from_millis(50), "compose_scripture_scene(JHN.3) took {elapsed:?}, over the 50ms smoke gate");
}

#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn xrefs_for_verse_completes_within_smoke_threshold() {
    let (_source, graph) = real_scene_source_and_graph();
    let span = ScriptureRef::parse("JHN.3.16").unwrap();
    let elapsed = median_of(7, || {
        let by_from = graph.cross_refs_for_span(&span);
        let _ = atlas_core::xrefs::aggregate_span_xrefs(&span, &by_from, |key| {
            let v = atlas_core::refs::VerseId::parse_canonical(key).ok()?;
            graph.verse_text_of(&atlas_graph_types::text::VerseRef { book: v.book.0, chapter: v.chapter, verse: v.verse })
        });
    });
    println!("PERF SMOKE {}: {elapsed:?} (gate {}ms)", "xrefs_for_verse_completes_within_smoke_threshold", 30);
    assert!(elapsed < Duration::from_millis(30), "aggregate_span_xrefs(JHN.3.16) took {elapsed:?}, over the 30ms smoke gate");
}

#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn text_window_completes_within_smoke_threshold() {
    let (_source, graph) = real_scene_source_and_graph();
    let snap = graph.snapshot();
    let anchor = match ScriptureRef::parse("JHN.3.1").unwrap() {
        ScriptureRef::Verse(v) => v,
        _ => unreachable!(),
    };
    let start = graph.position_of(anchor.book.0, anchor.chapter, anchor.verse).expect("JHN.3.1 must resolve");
    let elapsed = median_of(7, || {
        let ids = window::window(&snap, atlas_graph::kjv_adapter::BIBLE_CORPUS, start, 20, WindowDir::Onward);
        let _ = bible_text_units(&graph, &snap, &ids);
    });
    println!("PERF SMOKE {}: {elapsed:?} (gate {}ms)", "text_window_completes_within_smoke_threshold", 30);
    assert!(elapsed < Duration::from_millis(30), "text_window(JHN.3, n=20) took {elapsed:?}, over the 30ms smoke gate");
}

#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn chapter_window_completes_within_smoke_threshold() {
    let (_source, graph) = real_scene_source_and_graph();
    let snap = graph.snapshot();
    let book = match ScriptureRef::parse("JHN.3").unwrap() {
        ScriptureRef::Chapter { book, .. } => book,
        _ => unreachable!(),
    };
    let elapsed = median_of(7, || {
        if let Some((start, n)) = graph.chapter_span(book.0, 3) {
            let ids = window::window(&snap, atlas_graph::kjv_adapter::BIBLE_CORPUS, start, n, WindowDir::Onward);
            let _ = bible_text_units(&graph, &snap, &ids);
        }
    });
    println!("PERF SMOKE {}: {elapsed:?} (gate {}ms)", "chapter_window_completes_within_smoke_threshold", 50);
    assert!(elapsed < Duration::from_millis(50), "chapter(JHN.3) window took {elapsed:?}, over the 50ms smoke gate");
}

#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn adjacency_page_latency_corpus_over_both_arms() {
    use atlas_graph_types::adjacency::EdgeQuery;
    use atlas_graph_types::id::{NodeKind, Position};
    use atlas_graph_types::store::GraphQuery;
    use atlas_graph_types::store::{GraphPublisher, GraphStore, MemStore};
    let compiled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled");
    let (g, _) = atlas_graph::sqlite::reload::committed_graph(&compiled).expect("the sections read back");
    let mut store = MemStore::default();
    let v = store.publish(g);
    let mem_snap = store.open(v).expect("the published version opens");
    let (sql, _, _) = GraphService::from_sections(&compiled).expect("sections");
    let sql_snap = sql.snapshot();
    let corpus: Vec<Position> = {
        let snap = &mem_snap;
        let mut v = Vec::new();
        for kind in NodeKind::ALL {
            v.extend(snap.nodes_of_kind(kind, None, 100).ids.into_iter().map(Position::Node));
        }
        v
    };
    fn run<S: GraphQuery>(label: &str, snap: &S, corpus: &[Position]) -> Duration {
        let mut samples: Vec<Duration> = Vec::new();
        let mut pages = 0usize;
        for p in corpus {
            let summary = snap.edge_summary(p);
            for (kind, _) in summary.iter() {
                let t = Instant::now();
                let page = snap.edges_with_nodes(p, &EdgeQuery { kind: *kind, cursor: None, limit: 25 });
                samples.push(t.elapsed());
                pages += page.entries.len().min(1);
            }
        }
        samples.sort();
        let pct = |q: f64| samples[((samples.len() as f64 - 1.0) * q) as usize];
        println!("FRONTIER LATENCY [{label}]: {} positions, {} pages, p50 {:?}, p90 {:?}, p99 {:?}, max {:?}", corpus.len(), samples.len(), pct(0.5), pct(0.9), pct(0.99), samples.last().unwrap());
        let _ = pages;
        pct(0.99)
    }
    let _mem_p99 = run("mem (sections read back)", &mem_snap, &corpus);
    let sql_p99 = run("sqlite (sections)", &sql_snap, &corpus);
    assert!(sql_p99 < Duration::from_millis(100), "served frontier page p99 {sql_p99:?} over 100 ms (spec 12)");
}
