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
