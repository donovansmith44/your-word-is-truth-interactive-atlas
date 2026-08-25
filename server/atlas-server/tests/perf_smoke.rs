//! Batch PERF-2a Phase 1: the SMOKE-TIER regression gate -- a fast (single-
//! digit seconds), coarse subset of the full criterion suite
//! (`benches/queries.rs`), wired into the NORMAL test suite (counts toward
//! the standing canonical total) so a gross performance regression on the
//! owner's own named hot path (the timeline slider's `/api/scene` query)
//! fails LOUD in CI, not just when someone remembers to run `cargo bench`.
//!
//! LAW (brief, verbatim): "loud failure on gross regression, immune to
//! machine noise -- thresholds x3, not x1.05." Every threshold below is
//! AT LEAST 3x this batch's own measured baseline (see BENCHMARKS.md for
//! the exact numbers + machine context), most considerably more --
//! rounded up to a clean number, not shaved to the theoretical 3x floor,
//! so a normally-loaded dev machine or a slower CI runner never flakes this
//! red for reasons that have nothing to do with a real regression. This is
//! a FLOOR check (gross regression only): it does not replace
//! `benches/queries.rs`'s own fine-grained distribution, which is what a
//! real investigation should read.
//!
//! Deliberately measures the PURE `atlas_core`/`atlas_graph` functions
//! (never over real HTTP/loopback): this batch's own investigation found
//! this machine's "localhost" hostname resolution adds a large, unrelated,
//! ~200ms constant per request in several command-line tools (curl,
//! PowerShell's first call) -- see batch-perf2a-report.md. A smoke test
//! that measured over real sockets would inherit that unrelated noise
//! source and could flake on machine/tool-specific DNS behavior that has
//! nothing to do with this codebase's own performance. Calling the same
//! functions the handlers call, directly, measures exactly the thing this
//! batch's own compose-cost investigation and fix are about.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use atlas_core::data::AtlasData;
use atlas_core::refs::ScriptureRef;
use atlas_core::scene::{compose_scripture_scene, compose_time_scene};
use atlas_core::time::TimeRange;
use atlas_graph::window::{self, WindowDir};
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

/// Runs `f` `iters` times, returns the MEDIAN wall-clock duration --
/// resistant to one slow first-iteration (cache warmup) or one stray OS
/// scheduling hiccup, which a mean (or a single sample) is not; still cheap
/// (this whole file budgets well under a second of real work).
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

/// Owner's own named path (batch brief, verbatim: "the timeline slider
/// adjustments are LAGGY"): `compose_time_scene`, the full atlas span (the
/// most expensive window measured this batch, ~8ms via real HTTP/loopback
/// on this machine, sub-ms to a few ms pure-compute) and the documented
/// "NT window" (~205KB, ~4-5ms via HTTP). 75ms is >9x the HTTP-measured
/// worst case and ~15.6x the pure-compute worst case (4.80ms) -- the TARGET this batch's brief set
/// was <25ms; this gate is deliberately looser than the target itself
/// (a target is an engineering goal, not a noise-immune CI gate).
#[test]
fn scene_time_full_span_completes_within_smoke_threshold() {
    let (data, _graph) = real_data_and_graph();
    let w = TimeRange::new(-4004, 100).unwrap();
    let elapsed = median_of(7, || {
        let _ = compose_time_scene(&data, w);
    });
    assert!(elapsed < Duration::from_millis(75), "compose_time_scene(full span) took {elapsed:?}, over the 75ms smoke gate (baseline ~8ms HTTP / sub-ms pure-compute -- see BENCHMARKS.md)");
}

#[test]
fn scene_time_nt_window_completes_within_smoke_threshold() {
    let (data, _graph) = real_data_and_graph();
    let w = TimeRange::new(-5, 100).unwrap();
    let elapsed = median_of(7, || {
        let _ = compose_time_scene(&data, w);
    });
    assert!(elapsed < Duration::from_millis(75), "compose_time_scene(NT window) took {elapsed:?}, over the 75ms smoke gate (baseline ~4-5ms HTTP -- see BENCHMARKS.md)");
}

#[test]
fn scene_scripture_chapter_completes_within_smoke_threshold() {
    let (data, _graph) = real_data_and_graph();
    let r = ScriptureRef::parse("JHN.3").unwrap();
    let elapsed = median_of(7, || {
        let _ = compose_scripture_scene(&data, &r);
    });
    assert!(elapsed < Duration::from_millis(50), "compose_scripture_scene(JHN.3) took {elapsed:?}, over the 50ms smoke gate");
}

/// `handlers::xrefs`'s own service-layer call
/// (`atlas_core::xrefs::aggregate_span_xrefs`) -- benched directly for the
/// same "no unrelated network/DNS noise" reason the module doc comment
/// gives; `graph.cross_refs_by_from`/`graph.verse_text` are the exact
/// companion indexes the real handler reads.
#[test]
fn xrefs_for_verse_completes_within_smoke_threshold() {
    let (_data, graph) = real_data_and_graph();
    let span = ScriptureRef::parse("JHN.3.16").unwrap();
    let elapsed = median_of(7, || {
        let _ = atlas_core::xrefs::aggregate_span_xrefs(&span, &graph.cross_refs_by_from, &graph.verse_text);
    });
    assert!(elapsed < Duration::from_millis(30), "aggregate_span_xrefs(JHN.3.16) took {elapsed:?}, over the 30ms smoke gate");
}

/// `graph_handlers::text_window`'s own service-layer call chain
/// (`window::window` + `window::render` per unit) -- a 20-verse onward
/// window from a real anchor, the same shape a Reader.razor chapter-scroll
/// fetch uses.
#[test]
fn text_window_completes_within_smoke_threshold() {
    let (_data, graph) = real_data_and_graph();
    let snap = graph.snapshot();
    let anchor = match ScriptureRef::parse("JHN.3.1").unwrap() {
        ScriptureRef::Verse(v) => v,
        _ => unreachable!(),
    };
    let start = graph.position_of(anchor.book.0, anchor.chapter, anchor.verse).expect("JHN.3.1 must resolve");
    let elapsed = median_of(7, || {
        let ids = window::window(&snap, atlas_graph::kjv_adapter::BIBLE_CORPUS, start, 20, WindowDir::Onward);
        for id in &ids {
            let _ = window::render(&snap, id);
        }
    });
    assert!(elapsed < Duration::from_millis(30), "text_window(JHN.3, n=20) took {elapsed:?}, over the 30ms smoke gate");
}

/// `handlers::chapter`'s own service-layer call chain (chapter_span +
/// window + per-verse place/person lookups) -- John 3 (JHN.3), 36 verses.
#[test]
fn chapter_window_completes_within_smoke_threshold() {
    let (_data, graph) = real_data_and_graph();
    let snap = graph.snapshot();
    let book = match ScriptureRef::parse("JHN.3").unwrap() {
        ScriptureRef::Chapter { book, .. } => book,
        _ => unreachable!(),
    };
    let elapsed = median_of(7, || {
        if let Some((start, n)) = graph.chapter_span(book.0, 3) {
            let ids = window::window(&snap, atlas_graph::kjv_adapter::BIBLE_CORPUS, start, n, WindowDir::Onward);
            for id in &ids {
                let _ = window::render(&snap, id);
            }
        }
    });
    assert!(elapsed < Duration::from_millis(50), "chapter(JHN.3) window took {elapsed:?}, over the 50ms smoke gate");
}
