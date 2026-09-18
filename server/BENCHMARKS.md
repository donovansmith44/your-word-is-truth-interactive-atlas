# Server performance benchmarks

Batch PERF-2a (2026-08-25) established this file and the criterion harness
it reports on. Owner order (2026-08-25, verbatim): "make every graph query
four times faster with zero regression." Phase 0 of that order was the
owner's own named lagging interaction ("the timeline slider adjustments are
LAGGY"); Phase 1 (this file + `benches/queries.rs` + `tests/perf_smoke.rs`)
is the permanent harness that makes speed claims and regressions measurable
at all. Full investigation, before/after tables, and what was tried and
rejected: `.superpowers/sdd/2026-08-17-bible-atlas-m1/batch-perf2a-report.md`.

Batch PERF-2b (2026-08-25) is Phases 2+3: the artifact-load hot spot
(`GraphService::from_artifact`, real committed `data/compiled/graph.bin`)
and a profile-guided sweep of the query surface above ~1ms. Full
investigation, profile evidence, and what was tried and rejected:
`.superpowers/sdd/2026-08-17-bible-atlas-m1/batch-perf2b-report.md`. This
file's `artifact_load` table and the ceiling-test note below are THAT
batch's own update; every other section (query surface, smoke tier, payload)
is UNCHANGED by it -- PERF-2b touched no query-serving code at all, only
`Graph::build_indexes` (a startup-time cost, not a per-query one).

## Machine context

- Windows 11 Home 10.0.26200, quiet at measurement time (no concurrent
  batch/build in flight -- verified before every timing run below).
- Rust: release profile (`cargo build --release`) for every server-side
  timing number below unless marked otherwise; criterion's own `bench`
  profile (opt+debuginfo, same optimization level as release) for the
  `cargo bench` numbers.
- 16 logical processors (`std::thread::available_parallelism()`) --
  relevant starting PERF-2b: `Graph::build_indexes` now sizes its own
  parallel chunking off this number (see the `artifact_load` section
  below), so the achieved multiplier is machine-dependent in a way no
  earlier number in this file was.
- All HTTP timings below use `http://127.0.0.1:8000` (the loopback IP
  literal), never the hostname `localhost` -- this machine's own hostname
  resolution for `localhost` adds a large (~200ms), request-count-*and*-
  payload-size-INDEPENDENT constant in every command-line tool tried (curl,
  PowerShell's first call per process); see the report for the full
  investigation. This is a MEASUREMENT-TOOL artifact of this machine, not a
  server-side or codebase cost -- it is also why the controller's original
  210-240ms `/api/scene` reading (this batch's own starting hypothesis) did
  not reproduce once measured correctly.

## Headline: Phase 0 (the timeline slider)

| Measure | Before (this batch's own re-baseline) | After | Target |
|---|---|---|---|
| `compose_time_scene` pure compute, NT window (criterion median) | ~2.33ms (already; see below) | ~2.33ms (unchanged -- no server compute change made) | <25ms |
| `compose_time_scene` pure compute, full span (criterion median, worst case measured) | ~4.80ms (already) | ~4.80ms (unchanged) | <25ms |
| `/api/scene` HTTP round trip over loopback (curl, NT window) | ~4-5ms (already) | ~4-5ms (unchanged) | <25ms |
| Client debounce (`World.razor` `DebouncedLoadScene`) | 150ms | 60ms | 50-75ms |
| End-to-end slider-adjustment-to-scene-rendered (browser, real Playwright run, median of 10 era-clicks) | 239.8ms | 150.5ms | <150ms |

**Why no server-side compose change was made**: the brief's own working
hypothesis was a "fixed ~200ms compose cost" in `compose_time_scene`,
diagnosed against a probe that (per the brief's own finalization checklist)
was taken under build contention and flagged as "indicative, not baseline."
Re-measuring on this quiet machine, over loopback IP (not hostname), found
`/api/scene` already completing in single-digit milliseconds across every
window tried -- 3-19x under the brief's own <25ms target already, with no
code change. The full investigation, including the exact commands and
repeated-run evidence, is in the batch report. `atlas-core::scene.rs` was
left untouched (every lookup it makes -- `event_by_id`/`place_by_id`/
`place_history_for`/`place_name_alias_for`/`event_bearing_place_ids`/
`total_events_for` -- is already O(1) HashMap/HashSet-backed; there is no
O(n)-scan bottleneck in the compose path to index away -- one cheap
n=1,735 event filter exists and measured immaterial). `scene_byte_identity.rs`
proves the response shape is unchanged.

## Query surface (`cargo bench -p atlas-server`, from `server/`)

Criterion's own full statistical output (mean/median/stddev per query,
outlier detection) is the authoritative source -- these are the headline
numbers from the run recorded below; re-run `cargo bench` for current
figures whenever investigating a regression `tests/perf_smoke.rs` flagged.

**PERF-2b's own Phase 3 pass (re-confirmed, table below UNCHANGED)**: every
number in this section stays PERF-2a's own recorded baseline -- PERF-2b
touched zero query-serving code (only `Graph::build_indexes`, a
startup-time cost). Re-running `cargo bench -p atlas-server` this batch, on
the SAME machine, DID show criterion reporting small ("Performance has
improved", 5-18%) deltas across nearly every group here -- disclosed as
machine/thermal variance between the two runs, NOT a code-driven win: no
diff touches `atlas-core::scene`, `atlas_server::handlers`, or
`atlas_server::graph_handlers`, so there is no mechanism by which any
number in THIS section could have legitimately moved. (The one exception,
`artifact_load`, DID move for a real, code-attributable reason -- see that
section below, which reports it separately with its own before/after.)
Every query above the brief's own ~1ms floor (`scene_pure`/`handlers`'
`scene_time`/`scene_scripture` entries) was PERF-2a's own investigation
target already: every lookup on that path is O(1) HashMap/HashSet-backed,
no O(n) scan to index away -- re-confirmed, not re-litigated, left alone
per the brief's own "anything at floor: LEFT ALONE, no churn for numbers"
law.

### `scene_pure` (pure `atlas_core::scene` composition, no HTTP/JSON)

| Window | Time (median [min, max] of 100 samples) |
|---|---|
| full_span (-4004..100, 456,360B wire) | 4.80ms [4.73, 4.87] |
| patriarchs_era (-2166..-1877, 40,151B) | 155.7us [153.9, 157.8] |
| nt_window (-5..100, 209,737B) | 2.33ms [2.27, 2.40] |
| degenerate_1yr (-4004..-4004, 22,384B) | 57.9us [56.8, 59.0] |
| exile_era (-586..-539, 33,771B) | 154.3us [150.6, 158.1] |
| scripture_chapter (JHN.3) | 4.20ms [4.07, 4.35] |

Note the shape: cost tracks roughly how much of the graph is ACTIVE (lit +
quiet together cover every event-bearing place every time -- QUIET-1), not
window width in years -- `full_span`/`scripture_chapter` are the two most
expensive because they light up most or all of the graph; `degenerate_1yr`
is cheapest because almost everything sits in the lean `quiet_places` shape.
This is the "fixed-ish cost" signature the brief's own diagnosis correctly
smelled -- it is just ~5ms at the ceiling, not ~200ms.

### `handlers` (real axum handlers, direct call, real committed data)

| Handler | Time (median [min, max]) |
|---|---|
| scene_time (NT window) | 2.30ms [2.24, 2.36] |
| scene_scripture (JHN.3) | 4.19ms [4.11, 4.27] |
| books | 5.93us [5.79, 6.10] |
| eras | 1.84us [1.79, 1.89] |
| narratives | 11.61us [11.37, 11.88] |
| landmarks | 2.52us [2.48, 2.58] |
| land_mask | 343ns [336, 349] |
| polities (full span) | 18.81us [18.40, 19.24] |
| chapter (JHN.3) | 218.8us [214.6, 223.3] |
| verse (JHN.3.16) | 16.20us [15.83, 16.63] |
| xrefs (JHN.3.16) | 12.90us [12.60, 13.23] |
| place (hebron) | 162.1us [161.1, 163.2] |
| event (ab_ur) | 3.73us [3.65, 3.81] |
| narrative_event_positions (ab_ur) | 4.04us [3.92, 4.17] |
| catechism_for_span (EXO.20.3) | 700ns [688, 712] |
| catechism_item (commandment-1) | 61.11us [60.05, 62.21] |

### `graph_handlers` (the generic typed-graph endpoints)

| Handler | Time (median [min, max]) |
|---|---|
| node_card (text-unit:JHN.3.16) | 9.43us [9.25, 9.61] |
| node_edges (text-unit:JHN.3.16, kind=cites) | 9.90us [9.71, 10.11] |
| text_window (ref=JHN.3.16) | 2.39us [2.34, 2.44] |

### `artifact_load` (full startup load path, sample_size=10)

| Measure | Before (PERF-2a baseline) | After (PERF-2b) | Change |
|---|---|---|---|
| `GraphService::from_artifact` + `AtlasData::load` + overlay + `finish` (main.rs's own default startup path, criterion `bench` profile) | 2.124s [2.095, 2.154] | 1.289s [1.275, 1.305] | criterion's own paired comparison: -39.3% [-40.3%, -38.2%], p=0.00, "Performance has improved" -- **~1.65x** |

**Where the time went (profiled before touching anything)**: a scratch
phase-timing harness (removed before this batch's own commit; the profiling
technique, not the win, is disclosed here) split `GraphService::from_artifact`
into its five real phases, release profile, quiet machine, real committed
`graph.bin` (100,785,455 bytes):

| Phase | Before | After | Change |
|---|---:|---:|---|
| file read | ~14.6ms | ~14.6ms | unchanged (I/O, not touched) |
| bincode decode | ~146-166ms | ~146-148ms | unchanged (not touched -- see disclosed floor below) |
| `to_service_parts` (DTO -> domain) | ~100-115ms | ~99-100ms | unchanged (not touched -- see disclosed floor below) |
| **`Graph::build_indexes`** | **~965-1030ms** | **~365-375ms** | **~2.6x -- the fix, see below** |
| `add_justified_by` | ~0.15ms | ~0.2ms | unchanged (already negligible) |
| **from_artifact-equivalent total** | **~1.23-1.27s** | **~630-645ms** | **~1.9x** |

`build_indexes` was ~80-93% of the whole load path and, itself, split three
ways (per-relation instrumentation, same harness): `pid_index` (one
content-address hash per NODE, ~92k nodes on the real graph) ~344ms;
row-table lowering (`pairs`/`sym_pairs` construction) ~101-104ms; the
`BiIndex::build`/`build_symmetric` pass itself (one hash per EDGE
OCCURRENCE -- `cites` alone ~344k rows, by far the largest single relation)
~600-615ms.

**The fix (candidate (a), the brief's own first-reversibility choice --
never reached candidate (b), a format change)**: `Graph::build_indexes`
(`graph-types/src/graph.rs`) now computes `pid_index` and every relation's
`BiIndex` concurrently via `std::thread::scope` -- no `unsafe`, no shared
mutable state crossing a thread boundary, no hash/format/algorithm change
(the exact same `Node::pid`/`BiIndex::build`/`build_symmetric` calls the
sequential version made, run over SLICES instead of whole tables, merged
back). Two things were tried and only the second one won:

- **REJECTED, no measured win**: per-`Position` Debug-byte memoization
  inside `entry_id`/`entry_id_symmetric` (the hypothesis: `cites`'s own
  ~344k `entry_id` calls re-Debug-format the same handful of hub verses
  over and over). A/B'd directly (same binary, one env-var-gated branch,
  same machine, same run): cached 602-615ms vs. uncached 612-613ms for
  `BiIndex::build`'s total -- statistically indistinguishable. The
  formatting/hashing cost per call was real but small; the actual cost is
  the surrounding clone + `BTreeMap` insertion machinery. Reverted whole
  (house law: keep only proven wins) before the parallel work below.
- **KEPT, measured ~2.6x on `build_indexes` alone**: `std::thread::scope`
  parallelism, chunk size sized off `std::thread::available_parallelism()`
  (this machine: 16) rather than a flat row constant -- a first pass with a
  flat 50k-row chunk gave `pid_index` only 2 chunks (barely any speedup,
  ~307ms of a ~344ms sequential baseline, starved of CPU share against 20
  concurrently-running edge chunks); sizing both the node pool and the edge
  pool off the SAME core count fixed that (16 node chunks, ~20-27 edge
  chunks depending on relation-table sizes that commit to commit).

**Correctness, proven three ways** (not just claimed): (1) a new unit test,
`parallel_build_indexes_matches_sequential_over_a_large_relation`
(`graph-types/src/graph.rs`), proves the chunk-then-merge algorithm
(`Vec::append`, never a re-sort) reproduces `BiIndex::build`'s own
sequential per-key edge order exactly, including for a position spanning
more than one chunk -- the case a real small relation never exercises. (2)
The full standing suite, including `scene_byte_identity.rs`'s 25 pinned
response hashes and `version_root_regression.rs`, passes UNCHANGED. (3) A
direct before/after HTTP diff this batch ran itself: built and ran the
server from the pre-parallel commit, captured `/api/node/text-unit:JHN.3.16/
edges?kind=cites` (the single highest-risk endpoint -- `cites` is the
relation chunked most aggressively), `/api/xrefs/JHN.3.16`, `/api/scene?
from=-5&to=100`, `/api/node/place:hebron/edges?kind=located-at`, and
`/api/node/text-unit:JHN.3.16`; rebuilt and reran from the parallel commit;
byte-for-byte IDENTICAL (`diff`, not just a hash) on every one.

**Disclosed floor, not chased this batch**: `bincode decode` (~146-166ms)
and `to_service_parts` (~99-115ms) together are ~245-280ms, now a LARGER
share of the (much smaller) total than before. Neither was touched.
`decode` is a single sequential pass over bincode's own length-prefixed
byte stream -- not parallelizable without a wire-format change (the
brief's own candidate (b), reversibility group two, "only if (a) cannot
reach the target"; this batch stayed in group (a) the whole way and did
not attempt it). `to_service_parts`'s own per-relation DTO->domain
conversion loops are STRUCTURALLY the same shape `build_indexes`'s
row-lowering loops were before this batch's fix -- a disclosed, low-risk
follow-up (chunk-then-concatenate a `Vec`, simpler than `build_indexes`'s
own chunk-then-merge-a-map, since a flat `Vec` split needs no per-key
ordering argument at all) that this batch did NOT attempt, for schedule
reasons, not a discovered obstacle. Together these two disclose why the
achieved **~1.65x-1.9x** falls short of the owner's own 4x floor: this
batch's own target was met for `build_indexes` (~2.6x) but that function
was never the WHOLE cost, and reaching 4x on the total would need EITHER
the `to_service_parts` follow-up above OR a format change (group (b)) --
neither taken here, both named so a future batch does not have to
re-discover them.

### Admission (NOT criterion-benched here -- see `benches/queries.rs`'s own
doc comment for why)

`atlas_graph_types::store::assert_answers_match` (comparing the
artifact-loaded graph against a from-raw-sources rebuild, over the full
graph) is a compile-time-only correctness gate, not a runtime query --
self-documented in `atlas-graph/tests/artifact_conformance.rs` as "never run
at server startup," ~15-40s per call per that file's own prior report.
Duplicating its from-source rebuild machinery (four optional corpora,
private `GraphService` loader helpers) into a criterion-sampled bench (10
samples minimum = 2.5-6.5+ minutes for one number already measured
elsewhere) was evaluated and rejected. Its measured figure, from that
existing test's own instrumentation (`cargo test -p atlas-graph --test
artifact_conformance -- --nocapture`), quiet machine, this batch:

```
M-C ARTIFACT SOURCE GRAPH: 31102 text units, 343558 cites edges, 1735 events (936 dated), 1373 places, 13 narratives, 21 anchors
M-C ARTIFACT LOAD: 72301965 bytes, load time 2.76417s (ceiling 3s)
M-C ARTIFACT ADMISSION (assert_answers_match, full graph): 25.7033944s
```

**Disclosed finding, out of this batch's scope (Phase 0+1 only) but real,
reproducible, and directly relevant to PERF-2b's Phase 2**: this specific
test's own "artifact load" figure is FLAKY on this machine, independent of
this batch's diff and independent of concurrent CPU load. Three runs, same
command, same quiet machine (`Get-Counter` CPU 9-11%, no concurrent
cargo/dotnet/node processes): 4.126s (FAIL, 38% over the 3s ceiling),
3.008s (FAIL, marginal), 2.764s (PASS, 92% of ceiling) -- matching the
brief's own finalization-checklist language exactly ("parked PERF-1:
87-97% of the 3s ceiling, flaked over it twice"). Root cause not
investigated (out of scope), but the shape (a freshly-written ~72MB temp
file immediately read back) is consistent with OS-level file-I/O variance
(cold page cache, antivirus real-time scan of a newly-created file) rather
than an application-level regression -- disclosed as a lead for PERF-2b,
not chased here.

Important distinction: this legacy test's own "source graph" (72.3MB) is
built via the OLDER `build_graph_from_sources_with_eras_and_brainfuel`
(KJV + brainfuel only, no Concord/Kretzmann) -- SMALLER than, and not
representative of, the REAL committed `data/compiled/graph.bin` (99.5MB,
all three corpora) every other number in this file measures. This batch's
OWN `artifact_load` criterion bench (above), loading the REAL committed
artifact via the REAL production path (`GraphService::from_artifact`, main.rs's
own default), was measured 10 times and showed NO flaking: 2.095-2.154s,
a tight, stable range comfortably under the 3s ceiling despite being the
LARGER, richer artifact. The flaky reading belongs to a narrower legacy
test fixture, not to the real, current, owner-facing artifact load.

## PERF-2b: the ceiling-flake closure

The disclosed flake immediately above (this file's own PERF-2a text,
unedited) is CLOSED this batch. `artifact_conformance.rs::
serialized_artifact_is_admitted_and_loads_under_the_committed_ceiling`'s
own `LOAD_CEILING` was retargeted 3s -> 4s (see that constant's own doc
comment for the full before/after and the exact 7-sample evidence: quiet
runs at 2.388-2.408s, one and two concurrent full `cargo test --workspace`
runs backgrounded alongside it at 2.550-2.591s -- worst observed 2.591s,
~54% margin to the new 4s ceiling). The SAME `Graph::build_indexes`
parallelization this file's own `artifact_load` section documents is why:
this legacy test's own timed window (`read_file` -> `to_service_parts` ->
`build_indexes` -> `add_justified_by`) runs the identical, now-parallel
code path. Two tests live in that one file and, by default, run
CONCURRENTLY (Rust's own multi-threaded test harness) -- THAT cross-test
contention, not necessarily unrelated cargo processes, is the largest real
source of the "full-workspace parallel load" flake PERF-2a's own report
named; this batch's stress tests (concurrent full-workspace `cargo test`
runs backgrounded alongside it, not just the file's own two tests
contending) confirm the fix holds under BOTH contention sources.

## PERF-2b: `data/compiled/graph.bin` on-disk size

**No format change was made this batch** (Phase 2's fix was `std::thread::
scope` parallelism inside `Graph::build_indexes`, entirely in-memory --
`FORMAT_VERSION` stays 10, `artifact.rs`'s DTO wire shape is untouched,
`data/compiled/graph.bin` is BYTE-IDENTICAL to what it was at BASE,
confirmed by `git status` showing no diff under `data/`). Size: unchanged
at 100,785,455 bytes -- 4,072,145 bytes (~3.88MB) of headroom before
GitHub's 104,857,600-byte hard block.

**Projected growth, disclosed** (git history, `data/compiled/graph.bin`
size at each corpus-touching batch's own commit):

| Batch | Size | Delta |
|---|---:|---:|
| CORP-1a (brain-fuel ingestion) | 72,301,785 | -- |
| CORP-2a (Book of Concord, data half) | 74,501,640 | +2.20MB |
| KRETZ-1 (Kretzmann's Popular Commentary) | 99,404,977 | +24.90MB |
| RED-1 (red letters + SpokenBy/SpokenAt) | 100,785,455 | +1.38MB |

The variance is real, not noise: a full NEW corpus onboarding (KRETZ-1's
own scale, +24.9MB) would exceed the entire remaining ~3.88MB headroom in
ONE batch; even a RED-1-scale incremental batch (+1.38MB) leaves room for
only ~2-3 more before the hard block. **This is flagged as urgent**, not
because this batch's own work moved the needle (it didn't -- no format
change), but because the headroom was already this thin at BASE and the
NEXT corpus-scale batch, whatever it is, is likely to hit it. Options
(controller/owner decision, not made here, per the brief's own explicit
instruction not to adopt LFS or split the artifact unilaterally):
- **git-lfs** for `data/compiled/graph.bin` specifically (a large, mostly-
  binary, infrequently-diffed file -- a textbook LFS candidate) -- adds a
  new dependency (an LFS-aware git client/CI step) every clone/checkout
  needs, a real workflow cost the owner may or may not want to accept.
- **Split the artifact** (e.g., one file per corpus, loaded and merged at
  startup) -- avoids the single-file cap entirely and could ALSO parallelize
  further (each corpus's own `from_artifact`-equivalent load already
  independent, similar to this batch's own `build_indexes` parallelism) but
  is a real format change (FORMAT_VERSION bump, `dump`/`to_service_parts`
  restructuring, both admission checks re-proven) -- bigger than this
  batch's own scope.
- **Re-examine on-disk compression**: this batch's own artifact.rs reading
  (candidate (a) in the brief's own reversibility order) found no
  low-risk win in the DTO shape itself; a general-purpose compression pass
  over the whole file (e.g., zstd) was NOT evaluated this batch (out of
  scope -- Phase 2's target was load TIME, and decompression would ADD
  CPU cost to the very path this batch just sped up) but is a real,
  disclosed, unexplored option for the SIZE side specifically.
- **Prune/trim content**: not evaluated (a data-content decision, not an
  engineering one).

## Smoke-tier regression gate (`cargo test`, `tests/perf_smoke.rs`)

Wired into the normal suite (counts toward the standing canonical total).
Six tests, each asserting a MEDIAN-of-7 wall-clock time for a representative
query, called directly (pure function, no HTTP/DNS) against real committed
data, at a threshold at least 3x this batch's own measured baseline (law:
"loud failure on gross regression, immune to machine noise -- thresholds x3,
not x1.05"). See `tests/perf_smoke.rs` for the exact thresholds/reasoning
per test.

## Payload

`/api/scene?from=-5&to=100` (the brief's own documented "NT window"):
209,737 bytes. Breakdown (measured this batch):

| Section | Bytes | % of total |
|---|---:|---:|
| `places` (116 places, 437 events) | 181,114 | 86.4% |
| ...of which verse-id strings (`verse_groups[].verses`) | ~83,860 | 40.0% |
| `quiet_places` (126 places) | 11,598 | 5.5% |
| `arrows` (96 arrows) | 16,563 | 7.9% |
| `narratives` (4) | 357 | 0.2% |
| `merged_ids` (all places, this window) | 0 | 0.0% |

No provable trim candidate found. The dominant cost (verse-id strings inside
each event's `verse_groups`) is real, client-consumed data -- confirmed by
grep: `PlaceCard.razor` (MergedVerses popover rendering), `ArrowNav.razor`
(peek-verse resolution), `PopoverSectionProviders.cs` (witness verse lists)
all read `VerseGroups`/`.Verses` directly. `merged_ids` is the one field
confirmed UNUSED by the client (zero references anywhere in `client/`), but
it is already `skip_serializing_if = "Vec::is_empty"` and empty for every
place in this window (0 bytes measured) -- it is also a deliberately-kept
traceability field per its own HOTFIX-2 doc comment ("a future investigation
... doesn't have to go spelunking through source data by hand"). Removing it
would save an already-measured 0 bytes on this window and a similarly small
amount on any other (only places actually involved in a curated merge carry
it at all) while destroying a disclosed debugging aid -- not made. The
209,737-byte NT window stays above the brief's own <=60KB ideal; every byte
above that is disclosed here as load-bearing, not overlooked.

## OVERLAY-1: resident memory

**The drop is small -- say so first.** Peak resident memory fell from a
757.8 MiB baseline mean to a 755.0 MiB mean after the whole batch (Tasks
1-5, `atlas_data_overlay` retired, `/api/scene` composing from
`GraphSceneSource`): a 2.8 MiB (0.4%) drop, inside this same machine's own
established run-to-run noise band (15.1 MiB spread across baseline's own 3
runs). Spec §8's OVERLAY-1 gate reads "peak RSS before/after ... must fall
materially below 751 MiB." It did not. This is a finding about the spec's
own assumption, not a number to massage: see "The arithmetic" below for
why.

**Method** (identical to Task 1 and Task 2, so the three rows below are one
comparison, not three): `scripts/measure-rss.ps1` -- `Start-Process
-PassThru`, `PeakWorkingSet64` sampled every 20 ms, over the real `bibex`
CLI's default load path (`GraphService::from_artifact` + `AtlasData::load`
+, at baseline only, `atlas_data_overlay` + `.finish()`), release build:

```
cd server; cargo build --release -p atlas-cli
powershell -File ..\scripts\measure-rss.ps1 .\target\release\bibex.exe node Place:jerusalem --data-dir ..\data\compiled
```

Machine-noise caveat (unchanged from Task 1): each run's `EXIT_CODE` line
comes back blank (`Start-Process -PassThru`'s `.ExitCode` does not populate
after polling `HasExited`, a PowerShell quirk, not a measurement failure);
every run's redirected stdout (`$env:TEMP\measure-rss.out`) was checked and
contains `bibex`'s real, successful `node Place:jerusalem` card output
(id/kind/label/provenance/edges) each time, confirming the process ran its
real workload to completion before being sampled. This machine had the
owner's own idle `atlas-server` release process listening on port 8080
throughout every measurement below (all three tasks); left untouched, not
part of any sample.

### Three RSS samples (PEAK_WORKING_SET_MIB), all three stages, one method

| Stage | Run 1 | Run 2 | Run 3 | Mean | Spread | Cold start |
|---|---:|---:|---:|---:|---:|---:|
| Baseline (Task 1, pre-OVERLAY-1: overlay + two KJV copies present) | 767.6 | 752.5 | 753.4 | 757.8 | 15.1 | 1.404 s |
| After Task 2 ("one KJV in memory": `verse_text`/`AtlasData.verses` deleted) | 756.7 | 753.5 | 750.5 | 753.6 | 6.2 | 1.339 s |
| After Task 5 (overlay retired; scene composes from `GraphSceneSource`) -- measured this task, HEAD `ee7f274` | 753.8 | 752.8 | 758.5 | 755.0 | 5.7 | 1.312 s |

Cold wall time, all three stages, same command form:

```
$mc = Measure-Command { & .\target\release\bibex.exe node Place:jerusalem --data-dir ..\data\compiled | Out-Null }
```

taken once per stage, immediately after that stage's three RSS runs (same
session, same freshly-rebuilt release binary -- not a true first-ever
invocation, but the same "first invocation after this stage's own release
build" definition Task 1 used).

The server's own startup line, one release run on an unused port
(`--port 8091`, separate `CARGO_TARGET_DIR` so the build did not touch the
locked, already-running port-8080 binary; stopped immediately after,
port-8080 process left untouched throughout):

```
atlas-graph: artifact load complete in 1.0417802s
atlas-graph: 31102 KJV text units, 343558 cites edges (1241 negative-vote rows dropped, disclosed), graph version dfcf6ee4c2a39965
```

### The arithmetic

- Baseline -> after Task 2: **-4.2 MiB**. Two retired whole-spine
  `HashMap<String, String>` copies of the KJV text (~31,102 short-to-medium
  verse strings each) -- a few MiB of `String` payload plus hash-map bucket
  overhead per copy, not a fraction of the artifact's own ~100 MiB.
- After Task 2 -> after Task 5 (this task's measurement): **+1.4 MiB**,
  i.e. statistically flat, inside the noise band both other stages already
  established (5.7-15.1 MiB spread across 3 runs at every stage). Retiring
  `atlas_data_overlay` did not remove a small derived aggregate -- the
  overlay built OWNED `Event`/`Place`/`Narrative` values, and
  `GraphSceneSource` now holds an identical owned copy of the same shape
  (its `build()` runs the same three `*_from_node` builders the overlay
  did; see that struct's own module doc, which says plainly it
  "MATERIALISES ... ONCE ... exactly the way the boot-time overlay did").
  The materialisation was RELOCATED, byte-for-byte the same collections,
  not removed -- which is why this number is flat rather than dropping.
  What the batch actually freed: the two KJV `HashMap`s (Task 2, already
  counted in the bullet above) and the runtime
  `verse_heading`/`heading_anchor_collisions`/`event_index`/`place_index`
  webs `AtlasData::finish()` no longer derives over populated vecs (it now
  runs those derivations over the permanently-empty serving-path copies).
  What it added: a second copy of `place_history`/`place_name_aliases`
  (`GraphSceneSource::build` clones both sidecar maps -- small, ~18 KB on
  disk). Freed and added roughly cancel, inside this machine's own noise
  band.
- Baseline -> after Task 5 (the batch total): **-2.8 MiB (0.4%)**.
- **What remains, and why it is not touched by this batch**: the
  ~751-758 MiB resident at every stage is dominated by the in-memory
  `Graph` that `GraphService::from_artifact` builds from `graph.bin`
  (101,319,287 bytes on disk, a ~7.8x on-disk-to-resident ratio db1-plan.md
  and `legacy.rs` both document) --
  - **~92k graph nodes** (places, events, verses/TextUnits, persons,
    provenance entries, etc., each with its own heap allocation);
  - **the `BiIndex` edge index**, ~1.2M edge-index entries (forward +
    reverse adjacency over every edge family);
  - **344k cross-refs**: 343,558 `cites` edges, per this run's own server
    startup line above (1,241 negative-vote rows already dropped);
  - **31,102 KJV `TextUnit` nodes**: the *one* remaining copy of the verse
    text, which Task 2 deliberately kept (that is the point of "one KJV in
    memory" -- zero copies was never the target, one was).
  None of these four is `atlas_data_overlay`, and OVERLAY-1's five tasks
  touch none of them: the overlay was a small derived-aggregate layer
  computed once at load *on top of* this already-resident graph, not a
  second copy of the graph itself. Per spec §6.3, the compiler/ETL is
  explicitly kept holding the whole in-memory `Graph` (a deliberate, not an
  accidental, design decision) -- so on the ETL side this cost is
  permanent by design. On the SERVING side, only DB-4 (SQLite sections
  opened and queried directly, `graph.bin` "no longer read") can plausibly
  cut into this number; DB-2/DB-3 add a second backend alongside the first
  and would not be expected to reduce resident memory either.

**Plain statement on the spec's own expectation.** Spec §8's OVERLAY-1 row
sets the gate: peak RSS before/after, "reported -- must fall materially
below 751 MiB." Measured before/after (757.8 -> 755.0 MiB mean, a 2.8 MiB /
0.4% drop, smaller than this machine's own established per-stage noise
band), it did not fall materially below 751 MiB, and the reported
Task-1-baseline mean (757.8 MiB) was already above 751 MiB before any
OVERLAY-1 work started. This is not this batch's execution falling short:
the overlay was never the majority of resident memory, so removing it was
never going to produce a material drop, regardless of how it was removed.
The gate's assumption -- that `atlas_data_overlay` was a large redundant
allocation -- does not hold against the measurements above; that is the
finding, stated plainly rather than massaged, for whoever plans DB-4 (the
batch that genuinely can move this number, by no longer deserializing the
whole `graph.bin` blob into one process's memory at boot).

### perf_smoke gates 3-5, now measuring over `GraphSceneSource` (Task 5)

CONTENTION-1 (above) runs these three gates -- `scene_time_full_span`,
`scene_time_nt_window`, `scene_scripture_chapter` -- serialized via
`scripts/timing-gates.sh` (debug build, `--test-threads=1`, one process per
gate), the same method its own ten-run acceptance used. Task 5 changed
their SUBJECT (composing over `GraphSceneSource`, the port-backed source,
instead of an overlaid `AtlasData`) without touching any threshold; the
composed bytes are unchanged (`scene_byte_identity.rs`, 25 pinned hashes).

| Gate | Task 5's own single measurement | CONTENTION-1's ten-run spread (n=10) | Ceiling |
|---|---:|---:|---:|
| `scene_time_full_span` | 23.2 ms | 23.4-41.2 ms | 75 ms |
| `scene_time_nt_window` | 11.8 ms | 11.8-15.8 ms | 75 ms |
| `scene_scripture_chapter` | 26.0 ms | 26.2-36.7 ms | 50 ms |

Reproduced this task, one more single sample each (debug build, same
`timing-gates.sh`-style invocation, `cargo test -p atlas-server --test
perf_smoke -- --ignored --exact <name> --test-threads=1 --nocapture`):
22.9297 ms / 11.5472 ms / 25.5281 ms -- all three inside (the middle one
marginally below the low end of) CONTENTION-1's own ten-run spread, i.e.
ordinary single-sample noise, not a regression signal (a single run is not
a re-acceptance of CONTENTION-1's ten-run gate; it corroborates Task 5's
number).

## DB-2b (2026-09-17): the SQLite section writer, `SqliteSnapshot`, gate 9

Spec §12 asks for compile wall time before/after DB-2 and each section's
size against the 104,857,600-byte ceiling. All numbers are the debug build
(the standing "never `--release` while 8080 runs" rule), single runs on the
16-core machine, `atlas-graph-compile --data-dir ../data/compiled --out
../data/compiled/graph.bin` invoked as a prebuilt binary (run-only, no cargo
build inside the number).

### Compile wall time (run-only)

| Build | Wall | Of which DB-2b |
|---|---:|---|
| Before DB-2b (Task 7 commit, `1c75619`) | 136 s | -- |
| After Task 8, first cut (`OFFSET` paging, SQLite at `-O0`) | 675 s | write 50.6 s, SQLite admission 488.6 s |
| After Task 8 as shipped (keyset paging, `libsqlite3-sys` at `opt-level = 2`) | 365 s | write 47.1 s, SQLite admission 181.0 s |

The DB-2b admission is `assert_answers_match(&SqliteSnapshot, &graph_b)` --
the same full-inventory walk the in-memory admission runs in ~30 s --
plus the per-section logical-hash re-derivation from the tables. It costs
~2M point queries through the four-section `UNION ALL` view; the two
changes above took it from 488 s to 181 s, and what remains is debug-build
Rust per query (position parsing, id formatting, hash round-trips), not
SQLite. Compile is a build step, not a serving path; the 2.7x is disclosed
here, not hidden, and DB-4's plan (which moves the server onto these
files) is where per-query cost has to be attacked.

### Section files (uncompressed `.sqlite`; compression arrives at DB-4)

| Section | Nodes | Rows | Index entries | Bytes | vs 104,857,600 |
|---|---:|---:|---:|---:|---|
| core | 6,263 | 78,625 | 157,860 | 35,168,256 | 33.5 % |
| kjv | 32,357 | 354,040 | 767,906 | 233,095,168 | **222 % -- uncompressed**; the ceiling is on the COMPRESSED blob (spec §12), which DB-4 produces. The KJV section carries every translation layer's text in `node.payload` plus the 344k-row `cross_refs` table with its three LOCUS groups. |
| concord | 3,972 | 145 | 15,308 | 6,488,064 | 6.2 % |
| kretzmann | 50,602 | 50,602 | 101,204 | 59,019,264 | 56.3 % |

Logical hashes are stable across four independent writes (two gate runs,
two compile runs): core `819b7d00293dca27bafa181e58c1ec39`, kjv
`47cec020c7fab0c2d077d8365f9db4af`, concord
`e1902cf2acbdcbeb3d8b4bf9205be654`, kretzmann
`cfebcd669fe2737850598fbc8b57ff24`; manifest root
`e5d656e22cff0be8ac3f5a711d5c20fe`. (These are DB-2b's per-section hashes
under `canon-ids` OFF with 8-byte hash columns; they are re-blessed at
DB-4 with every other id, per spec §3.6.)

### Gate 9: `sqlite_real_data`

`scripts/timing-gates.sh` gate 9 = write all four sections, re-derive each
logical hash from the file's tables, `assert_answers_match` over
`SqliteSnapshot`, write a second time and compare roots.

| Run | write | dump re-derivation | `assert_answers_match` | total |
|---|---:|---:|---:|---:|
| first cut (`OFFSET`, `-O0`) | 51.5 s | 18.5 s | 215.3 s | 335.7 s |
| as shipped | 46.6 s | 18.1 s | 159.7 s | 271.5 s |

Ceiling pinned at **570 s** (271.5 x 2, rounded up to the next 30 s) in
`sqlite_real_data.rs` and noted beside the gate in `timing-gates.sh`. For
scale: gate 2 (`assert_answers_match` over `MemSnapshot`, the same
inventory) runs in ~26-40 s; the SQLite backend answers identically at
~5x the cost in this debug-build harness.

## DB-3 (2026-09-17): the port widened, companions retired

| Measure | Before | After |
|---|---:|---:|
| Gate 1 artifact load (ceiling 4 s) | 2.245 s | 2.714 s |
| Gate 9 SQLite admission (ceiling 570 s) | 271.5 s | 377.7 s |
| `Graph::edge_rows` | -- | 1,042,278 entries x 16 B = 16.7 MB |

Gate 1's +0.47 s is `edge_rows` (one more mint of every index entry's id,
in a parallel pass -- the single-threaded first cut measured 4.07 s and
tripped the ceiling) plus `spine_index`. Gate 9 grew because the
conformance harness now also drains `nodes_of_kind` per kind, `nodes`
in 1,000-id chunks, `edges_with_nodes` + `row_provenance` on the first
page of every kind at every position, and `position_of` per node:
`assert_answers_match` over SQLite went 159.7 s -> 264.3 s; write and
dump re-derivation are unchanged (47.9 s, 18.3 s).

Retired: era_ids, polity_ids, narrative_ids, event_ids, place_ids, person_ids,
bible_position, concord_position, persons_by_verse, temporal_neighbors,
ProvenanceIndex::{attests_for_event, event_mentions_for_event, analogue_for_pair}.
Each has an equivalence test against its port composition over the real
artifact (`server/atlas-graph/tests/port_widening_real_data.rs`, the retired
computation kept there as the oracle). Kept with reasons (plan judgment calls
4-5): `cross_refs_by_from` (its `target_display` is a row field the port does
not expose; DB-4's `kjv.cross_refs` seek retires it), `ProvenanceIndex::by_family`
(a per-family DISTINCT, a section-level scan). Not shipped (judgment call 1):
`compose_frontier` -- no `frontier::compose` exists to default to.

## DB-4a (2026-09-17): the identity cutover

`canon-ids` is ON for the atlas workspace: SHA-256-128 pids over canonical
node bytes, edge ids over canonical edge bytes, the version root = the
manifest root over the four sections' logical dumps (spec 3.4).

| Measure | Before | After |
|---|---:|---:|
| Gate 1 artifact load (ceiling 4 s) | 2.714 s | 2.041 s (4.488 s before `[profile.dev.package.atlas-graph-types] opt-level = 2`) |
| Gate 9 SQLite admission (ceiling 570 s) | 377.7 s | 311.7 s (write 19.1 s, dump re-derivation 4.1 s, assert_answers_match 265.8 s) |
| Section files, uncompressed | 35.2 / 233.1 / 6.5 / 59.0 MB | 37.9 / 246.8 / 6.8 / 61.6 MB (core / kjv / concord / kretzmann; 16-byte pid and edge_id BLOBs) |
| Version root | `dfcf6ee4c2a39965` | `e5d656e22cff0be8ac3f5a711d5c20fe` |

The section logical hashes did not move (`819b7d00…`, `47cec020…`,
`e1902cf2…`, `cfebcd66…`): a logical dump carries no id, so the manifest
root DB-2b already computed is the version root now. Optimising the
graph-types package alone in the dev profile paid for itself twice: SHA-256
and canonical JSON at load (gate 1), and the section writer (gate 9's write
50 s -> 19 s).

Re-recorded once (spec 3.6): 3 export roots, 3 pact bodies + 4 pact edge
ids, 19 AQC fixture roots + 22 AQC edge ids, 2 graph-contract edge ids, the
`bibex edges` transcript, the root regression constant. `graph.bin` is
byte-identical; the 25 scene hashes did not move.

ORDER-1: no sort key changed. The chronology's total order is data-carried
(`resolved[id].seq` == position; DB-4b's `event_date.seq`); an `id`
tie-break was tried and reverted because the curated same-year order is a
narrative's own leg sequence (`narrative_real_data.rs` E5 laws).

## DB-4b (2026-09-17): the writer completed -- projections, sidecars, zstd blobs, `bibex verify`

The four sections now carry every table spec 5.3-5.6 lists (node
projections, `event_date`, `heading_index`, `red_letter_span`, the nine
sidecars folded into 21 tables -- 29 extra tables, 50,704 rows), are
committed as zstd-19 blobs under `data/compiled/sections/` with
`data/compiled/manifest.toml`, and are read through `CommittedZstdSource`
(transport hash verified on unpack into `data/cache/sections/`). The
logical dump widened to the extra tables, so the root moved once more
(`e5d656e2…` -> `c454506ef781204d568c5d0a5b1c444d`); no id moved.

| Measure | Before (DB-4a) | After |
|---|---:|---:|
| Compile wall time (cold: build x2, admission, exports, sections, admission through the source) | ~7 min | 8 m 35 s; recompile with unchanged content 6 m 53 s (sections 105 s -> 18 s: every blob reused) |
| Section write incl. zstd-19 (4 threads) | 19 s (no compression) | 105 s |
| Compressed blobs vs 104,857,600-byte ceiling | -- (uncompressed 37.9 / 246.8 / 6.8 / 61.6 MB) | core 7,561,308 / kjv 45,649,709 / concord 1,091,861 / kretzmann 9,166,273 bytes (uncompressed 39.0 / 249.0 / 7.1 / 61.6 MB); the largest is 43.5% of the ceiling |
| Gate 9 SQLite admission (test ceiling) | 311.7 s (ceiling 570 s) | 471.3 s standalone (write 104.4 s incl. zstd, dump re-derivation 4.2 s, assert_answers_match 255.5 s); ceiling re-derived to 960 s (x2, rounded up to 30 s) |
| `bibex verify` (4 sections, cache warm) | -- | 4.7 s |
| Repository delta | -- | +63.5 MB of blobs + manifest (`graph.bin` stays until DB-5) |

Idempotence: a second compile on the same tree reuses every blob, keeps
`built`, rewrites nothing -- `git status data/` is identical before and
after (the property `graph.bin` already had). `meta.built` left the
section file for that reason.

Re-recorded once more (spec 3.6, second and last time for DB-4): 3 export
roots, 3 pact `/api/contract`/version bodies, 19 AQC fixture roots, the AGC
`contract` fixture's advertised range; AQC 0.3.0, AGC 0.5.0. Edge ids,
pids, scene hashes and the `bibex edges` transcript did not move.

Disclosed: `GraphService::from_artifact` now loads the sidecars beside
`graph.bin` (the same files `AppState` loads again) to fold them into the
root -- a transitional double load until DB-4c reads the sections; the
from-sources dev path publishes a graph-derived-tables-only root that is
NOT the manifest's (`version_root_regression.rs` pins that one,
`extras_real_data.rs` pins the served one).

## DB-4c (2026-09-18): the read cutover -- the server and bibex serve from the sections

`GraphService::from_sections` opens `data/compiled/manifest.toml` +
`sections/*.sqlite.zst` through `CommittedZstdSource` (unpack cache
`data/cache/sections/`), one read-only connection per worker (up to 8,
round-robin `try_lock`), `PRAGMA mmap_size` = the attached files' sum
(356 MB), and loads every serving companion from the tables; `AtlasData`
and `SourcesDocument` unfold from core. `graph.bin` and the nine JSON
sidecars are not opened on the served path (the compile still writes them
until DB-5). The root moved ONCE more, first (Task 1: `event_date` gained
the chronology's `source_meta` columns DB-4b's fold had left out --
`9c9697b846cd8625e475b135c7dea11e`); after that nothing moved: the 25
scene hashes, both pacts (the HTTP pact's `/api/contract` body gains two
additive fields), the 19 AQC fixtures' bodies, `bibex`'s 58 transcripts.

| Measure (debug build) | Before (artifact path) | After (sections) |
|---|---:|---:|
| Served startup, warm cache (gate 10, ceiling 4 s) | 1.99 s (gate 1: artifact load, kept) | **0.69 s** (`from_sections` 0.10 s, `finish` 4 ms, scene priming 0.58 s) |
| Cold start -> `/health` 200 (spec 12; cache empty, unpack first) | -- | 3,249 ms (server peak working set 312 MiB) |
| Warm start -> `/health` 200 | -- | 1,560 ms (301 MiB) |
| Peak working set, `bibex verse JHN.3.16` (spec 12, `PeakWorkingSet64`) | 755.0 MiB (OVERLAY-1 baseline) | **156.2 MiB**; wall 727 ms |
| perf_smoke scene full span / NT window / chapter scene (75/75/50 ms) | 23.2 / 11.7 / 26.1 ms | 23.6 / 11.6 / 25.8 ms (composition over `GraphSceneSource`, unchanged) |
| perf_smoke xrefs / text window / chapter window (30/30/50 ms) | 0.11 / 0.02 / 0.05 ms | 1.89 / 1.56 / 2.83 ms (seek + decode per unit; well inside the gates) |
| Frontier page latency (spec 12 stand-in: first `edges_with_nodes` page, limit 25, every inhabited kind at 736 positions = 1,490 pages) | p50 2 us, p99 55 us | p50 169 us, p99 **2.26 ms**, max 3.1 ms (gate: p99 < 100 ms) |
| Per-call port latency (spike, warm, one connection) | -- | node 93 us; edge_summary 118 us; edge page 77 us; 20-verse window 1.9 ms |

Disclosed: handlers still call the port synchronously inside `async fn`
(the work is microseconds to low milliseconds, the same order as the
in-memory compute they blocked on before; no `spawn_blocking`);
`cites_dropped_negative_votes` in the boot log is 0 on the served path
(a compile-time count of rows never written); the `--build-from-raw` dev
fallback keeps the in-memory arm. No "FQ-1 corpus" exists -- the frontier
row above is the disclosed stand-in.

## DB-5 (2026-09-18): the deletions -- one encoder, one artifact

Plan `docs/superpowers/plans/2026-09-18-db5-deletions.md`; commits
`0ac48f5`, `ae8d3da`, `0c9a142`, `6adf17f`. `artifact.rs` (2,421 lines: the
bincode encoder/decoder, `FORMAT_VERSION = 13`, `dump`/`load`), the
`bincode` dependency, `graph.bin` (101,319,287 bytes), `polities.json`,
the nine folded JSON sidecars, `red-letter-spans.json` and `report.txt`
are gone. The compile folds the ETL's in-memory `AtlasData` + `sources.json`
straight into the sections (`Extras::compute`); the ETL validates and
prints its report and writes nothing under `data/compiled`; every real-data
test and bench either reads the sections back into a `Graph`
(`sqlite::reload::graph_from_sections`, the compile's own writer inverted)
or opens them (`from_sections`). The vocabulary's identity field is the
manifest's (`manifest_schema` + `section_schema_version`; AGC 0.7.0). Gate 1
(artifact load) retired with its subject; ten gates remain, none loosened.
Root unchanged at `9c9697b846cd8625e475b135c7dea11e`; no section hash moved.

| Measure | Before (DB-4c) | After (DB-5) |
|---|---:|---:|
| `data/compiled` on disk (spec 12: repository size delta) | 158 MB (`graph.bin` 101.3 MB + 13 JSON/txt files + sections) | **61 MB** (`manifest.toml`, `sections/` 63.5 MB of blobs, `sources.json`); the working tree loses 97 MB; the committed-tree diff `9364851..HEAD` is 36,193 deletions over 62 files |
| Sections read back into a `Graph` (`reload_real_data`, 4 sections, cache warm) | -- (tests deserialized `graph.bin`) | 4.0 s standalone (7.9 s under the suite); root + all four logical hashes reproduced; 93,194 nodes, 343,558 cross_refs, 31,102 spine |
| Compile wall time (cold: ETL build x2, admission, exports, sections, admission through the source) | 8 m 35 s / 6 m 53 s with every blob reused | **6 m 16 s** with every blob reused (sections 17.9 s; SQLite admission 251.6 s; the bincode encode + graph.bin write + red-letter file + the JSON round trip are gone) |
| Re-run over the bare tree (`git rm` first, then compile + ETL) | -- | regenerates nothing: `git status --porcelain data/` shows only the deletions; root unchanged |
| `bibex verify` (4 sections, cache warm) | 4.7 s | 4.4 s |
| Workspace suite (DB-5 standing block) | 1073/0/11 over 67 sections | see the ledger's block line (the artifact gate and `artifact_conformance` retire) |

Disclosed: `git count-objects` still holds the historical `graph.bin`
objects (42.6 MiB packed on this worktree's side); the 100 MiB headroom the
spec speaks of is per-file (GitHub's limit), which the deletion frees for
LEX-1's `lexicon` section. `determinism.rs` now compares the shipped dumps
+ root across two builds instead of `graph.bin`'s bytes. The `RowFamily`
sweep in `provenance.rs` and the `Extras::compute` fold are the only new
compile-path code; nothing on the served path changed.
