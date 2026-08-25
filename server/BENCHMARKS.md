# Server performance benchmarks

Batch PERF-2a (2026-08-25) established this file and the criterion harness
it reports on. Owner order (2026-08-25, verbatim): "make every graph query
four times faster with zero regression." Phase 0 of that order was the
owner's own named lagging interaction ("the timeline slider adjustments are
LAGGY"); Phase 1 (this file + `benches/queries.rs` + `tests/perf_smoke.rs`)
is the permanent harness that makes speed claims and regressions measurable
at all. Full investigation, before/after tables, and what was tried and
rejected: `.superpowers/sdd/2026-08-17-bible-atlas-m1/batch-perf2a-report.md`.

## Machine context

- Windows 11 Home 10.0.26200, quiet at measurement time (no concurrent
  batch/build in flight -- verified before every timing run below).
- Rust: release profile (`cargo build --release`) for every server-side
  timing number below unless marked otherwise; criterion's own `bench`
  profile (opt+debuginfo, same optimization level as release) for the
  `cargo bench` numbers.
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

| Measure | Time (median [min, max] of 10 samples) |
|---|---|
| `GraphService::from_artifact` + `AtlasData::load` + overlay + `finish` (main.rs's own default startup path) | 2.124s [2.095, 2.154] (server startup log's own single reading: 1.697s -- both comfortably under the committed 3s ceiling; criterion's slightly higher median is consistent with `bench` profile overhead vs. a bare `release` binary, not a regression) |

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
