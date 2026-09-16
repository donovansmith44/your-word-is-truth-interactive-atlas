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
//!
//! CONTENTION-1 (spec 2026-09-14-relational-artifact-design §8): the
//! timing gate(s) in this file are `#[ignore]`d in the default run and
//! executed by `scripts/timing-gates.sh` -- one cargo process per gate,
//! `--test-threads=1`, after the parallel suite -- because three
//! wall-clock ceilings were resolving BOTH WAYS on identical code under
//! parallel load (progress.md, 2026-09-08 through 2026-09-10). The
//! ceilings themselves are unchanged. `timing-gates.sh check` refuses
//! any #[ignore] in server/ that is not one of the listed gates.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use atlas_core::data::AtlasData;
use atlas_core::refs::ScriptureRef;
use atlas_core::scene::{compose_scripture_scene, compose_time_scene};
use atlas_core::time::TimeRange;
use atlas_graph::scene_source::GraphSceneSource;
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;

/// OVERLAY-1 Task 5: the SUBJECT of gates 3-5 below changed -- they now time
/// `compose_*_scene` over `GraphSceneSource` (what the handlers serve from)
/// instead of over an overlaid, `finish()`ed `AtlasData` (which no longer
/// exists on any serving path). The composed BYTES are identical (`tests/
/// scene_byte_identity.rs`, 25 pinned hashes) and no threshold moved; the
/// measured numbers are a new baseline for the same ceilings.
fn real_scene_source_and_graph() -> (Arc<GraphSceneSource>, Arc<GraphService>) {
    static CACHED: std::sync::OnceLock<(Arc<GraphSceneSource>, Arc<GraphService>)> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let compiled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled");
            let graph = GraphService::from_artifact(&compiled.join("graph.bin")).expect("data/compiled/graph.bin must exist -- run atlas-graph-compile first");
            let sidecars = AtlasData::load(&compiled).expect("data/compiled must exist");
            let source = GraphSceneSource::build(&graph, &sidecars);
            (Arc::new(source), Arc::new(graph))
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

/// `handlers::xrefs`'s own service-layer call
/// (`atlas_core::xrefs::aggregate_span_xrefs`) -- benched directly for the
/// same "no unrelated network/DNS noise" reason the module doc comment
/// gives; `graph.cross_refs_by_from` is the same companion index the real
/// handler reads, and `graph.verse_text_of` (OVERLAY-1 Task 2 -- the
/// retired `graph.verse_text` whole-spine companion's on-demand
/// replacement) is the same per-key preview-text closure the real handler
/// builds.
#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn xrefs_for_verse_completes_within_smoke_threshold() {
    let (_source, graph) = real_scene_source_and_graph();
    let span = ScriptureRef::parse("JHN.3.16").unwrap();
    let elapsed = median_of(7, || {
        let _ = atlas_core::xrefs::aggregate_span_xrefs(&span, &graph.cross_refs_by_from, |key| {
            let v = atlas_core::refs::VerseId::parse_canonical(key).ok()?;
            graph.verse_text_of(&atlas_graph_types::text::VerseRef { book: v.book.0, chapter: v.chapter, verse: v.verse })
        });
    });
    println!("PERF SMOKE {}: {elapsed:?} (gate {}ms)", "xrefs_for_verse_completes_within_smoke_threshold", 30);
    assert!(elapsed < Duration::from_millis(30), "aggregate_span_xrefs(JHN.3.16) took {elapsed:?}, over the 30ms smoke gate");
}

/// `graph_handlers::text_window`'s own service-layer call chain
/// (`window::window` + `window::render` per unit) -- a 20-verse onward
/// window from a real anchor, the same shape a Reader.razor chapter-scroll
/// fetch uses.
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
        for id in &ids {
            let _ = window::render(&snap, id);
        }
    });
    println!("PERF SMOKE {}: {elapsed:?} (gate {}ms)", "text_window_completes_within_smoke_threshold", 30);
    assert!(elapsed < Duration::from_millis(30), "text_window(JHN.3, n=20) took {elapsed:?}, over the 30ms smoke gate");
}

/// `handlers::chapter`'s own service-layer call chain (chapter_span +
/// window + per-verse place/person lookups) -- John 3 (JHN.3), 36 verses.
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
            for id in &ids {
                let _ = window::render(&snap, id);
            }
        }
    });
    println!("PERF SMOKE {}: {elapsed:?} (gate {}ms)", "chapter_window_completes_within_smoke_threshold", 50);
    assert!(elapsed < Duration::from_millis(50), "chapter(JHN.3) window took {elapsed:?}, over the 50ms smoke gate");
}
