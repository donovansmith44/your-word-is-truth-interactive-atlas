# DB-4c — Read Cutover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The server and `bibex` serve from the committed sections (`data/compiled/manifest.toml` + `sections/*.sqlite.zst`, through `CommittedZstdSource`) and no longer read `graph.bin` or the nine JSON sidecars; every observable response, scene hash, pact and fixture is byte-identical (spec §1.2), the §2.5 open sequence and §11 refusals are implemented, and §12's measurements are stated.

**Architecture:** Three moves. (1) **One fold correction first** (Task 1): the served `Event` wire takes `to_year`/`order_key` from the chronology's `source_meta` companion, which DB-4b's `event_date` does not carry — two columns are added, the root moves for the LAST time, pins are re-recorded once more by the DB-4a/4b recorders; after Task 1 nothing in this batch may move the root. (2) **`GraphService` over a `Snap`** (Tasks 2–3): its snapshot becomes an enum over `MemSnapshot` (the dev fallback and the compile keep it) and `Arc<SqliteSnapshot>`, where `SqliteSnapshot` now holds one connection per worker (round-robin `try_lock`, `mmap_size` = the sum of the attached files, capped) — handlers keep calling the port; the seven companions the handlers read as bare fields are loaded from the section tables at startup (they are small: 912 dates, ~3k headings, 2k red-letter verses, 13 legs, 21 provenance families) except `cross_refs_by_from` (344k rows), which retires into a per-request seek on `kjv.cross_refs`. (3) **`AtlasData` unfolds from core** (Task 4): the inverse of DB-4b's fold, proven equal to the JSON-loaded value on the real data, so `AppState` is built from the sections alone (Task 5). Task 6 is the proof and the measurements: the 25 scene hashes, both pacts, the 19 AQC fixtures and the CLI transcripts unchanged over the new path; startup under 4 s; peak RSS, cold start and a frontier-page latency distribution reported.

**Tech Stack:** Rust 1.97.1, `rusqlite 0.32` (bundled), zstd (DB-4b), axum 0.8 / tokio multi-thread; `graph-types` untouched (zero-dep).

**Spec:** `docs/superpowers/specs/2026-09-14-relational-artifact-design.md` — §1.2 (what does not change), §2.4–2.5 (source, open sequence), §3.6, §5.3 (`event_date`), §8 row 5, §9 (gates and bumps), §11 (errors), §12 (measurements). Prior plans: `2026-09-17-db4b-writer-completeness.md` (judgment call 4 is corrected here).

## Global Constraints

- **Byte-identical responses** (spec §1.2): the 25 scene hashes (`scene_byte_identity.rs`), both pacts, the 44/184 AQC scenarios and their 19 fixtures, `bibex`'s transcripts, `client.Tests`, Playwright — none loosened, none re-recorded EXCEPT (a) Task 1's root move and (b) `/api/contract`'s two ADDITIVE fields (Task 5).
- After Task 1, **the root does not move**: no DDL, no dump, no fold change in Tasks 2–6.
- `graph-types` is not edited. `graph.bin` is still WRITTEN by the compile (DB-5 deletes it) and still read by the compile's own admission and the artifact tests; the SERVING path (`atlas-server`, `bibex`) never opens it after this batch. The nine JSON sidecars are still written by the ETL and read by the COMPILE's fold; the serving path never opens them after this batch.
- Spec §2.5 open sequence, verbatim order: (1) read manifest, recompute root, refuse on mismatch; (2) resolve every section through the `SectionSource`, refuse a `required` one missing or failing its transport hash, record absent optional ones; (3) open `core` as `main`, ATTACH the rest in manifest order, `PRAGMA query_only = ON`, `PRAGMA mmap_size` = the sum of section sizes (capped), **one connection per worker thread**; (4) the per-connection TEMP union views; (5) serve.
- Spec §11 rows, verbatim behaviours: manifest root mismatch → refuse, print expected/actual; required missing → refuse, name + logical; transport mismatch → refuse, delete the partial cache file, print both hashes; `user_version` unknown → refuse like an old `graph.bin`; optional absent → start, log one line; cache dir unwritable → refuse with the path.
- Gates (§9), all existing, none loosened: perf_smoke 75/75/50/30/30/50 ms medians over the NEW path; `graph_conformance` 60 s (in-memory law, unchanged); load ceiling **4 s** — retargeted from `graph.bin` to the sections' startup (Task 6); gate 9 960 s; contract gate.
- Never `--release` while 8080 runs; never touch 8080; `RUST_TEST_THREADS=4` for the standing block; gcc on PATH for cargo builds (`$HOME/scoop/apps/gcc/current/bin`); no `git stash`; untracked `client.Tests/FrontierMatrix*.cs`, `publish-apphome/`, `tests/ux/frontier-matrix.spec.ts` never staged (**`git add` by explicit file, never `git add client.Tests`**); map-generator never edited.
- Commit trailers: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` / `Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W`.
- Recorders (from `server/`, in order): compile → `ATLAS_BLESS_PACT=1 cargo test -p atlas-server --test contract_pact` (red on purpose) then plain → `ATLAS_BLESS_PACT=1 cargo test -p atlas-cli --test contract_pact_cli` then plain → `"$RUNNER" run --replay contracts/pacts --exports data/exports --bless contracts/atlas-graph-contract` then `--results <file>` (RUNNER = `cd contracts/runner && cabal list-bin contract-runner`, `$HOME/.local/bin` on PATH) → `cargo run -p atlas-server --bin export_aqc_examples` → VERSION/CHANGELOG (five AQC copies: `server/atlas-server/src/contract.rs` MIN/MAX + its unit test, `contracts/atlas-query-contract/aqc.schema.json`, `features/versioning.feature`, `client/AqcContract.cs`, `contracts/pacts/http.json` via the recorder; plus `client.Tests/AqcContractTests.cs` InlineData, `server/atlas-server/tests/contract_api.rs`) → `bash scripts/contract-gate.sh` on a committed tree.

## Judgment calls

1. **`source_meta` joins `event_date` (Task 1).** DB-4b judgment call 4 dropped `order_key` as "a duplicate of `seq`"; it is not — `legacy::event_from_node` (the Event wire, the scene events) reads `chrono.source_meta[id].{to_year, order_key}`, the CURATED values, distinct from `resolved.date.to`/`seq`. Two nullable columns `meta_to_year INTEGER, order_key INTEGER` (NULL when the artifact's `source_meta` has no entry — `event_from_node` then substitutes `from_year`/`0`, and the loader reproduces that by leaving the map entry absent). Root moves; AQC 0.3.0 → **0.4.0**, AGC 0.5.0 → **0.6.0**. Spec §5.3's `event_date` note is amended again, honestly.
2. **A `Snap` enum, not a generic `GraphService`.** `GraphService<S: GraphSnapshot>` would ripple through `AppState`, `LoadedAtlas`, every handler signature and every test; an enum with two arms delegating the port keeps the blast radius to `service.rs`. The compile binary and the `--build-from-raw` fallback keep the `Mem` arm.
3. **Connection per worker, round-robin, no pool crate.** `Vec<Mutex<Connection>>` of `available_parallelism().min(8)` connections; a query takes the first `try_lock` success from a rotating start index, else blocks on its own slot. Handlers stay `async fn` calling the port synchronously: a request's queries are microseconds to low milliseconds (the DB-4c spike: `node` 93 µs, an edge page 77 µs, a 20-verse window 1.9 ms), the same order as the in-memory compute they block on today; disclosed, no `spawn_blocking`.
4. **Small companions load at startup; only cross-refs seek.** `chronology` (912 rows), `heading_index`, `red_letter_spans`, `narrative_legs`, the provenance families and the boot counters are loaded from the tables once by `from_sections` into the SAME field shapes the handlers read today, so the handlers do not change for them. `cross_refs_by_from` (344,040 rows, the one companion whose size is the artifact's) retires into `GraphService::cross_refs_for_span` — a seek on `xref_by_from`, the spec's own answer (§5.4, DB-3 judgment call 4).
5. **`AtlasData` is unfolded whole, then `finish()`ed.** The handlers read `canon`, `books_meta`, `landmarks`, `land_mask`, `place_history`, `place_name_aliases`, `catechism` (+ `finish()`'s indexes); `chronology_anchors` and `book_narration_windows` are loaded for parity; `places/events/narratives/eras/verses/cross_refs/polities` stay empty (never populated on the serving path since OVERLAY-1; `polities.json` retired at DB-5). The law: the unfolded `AtlasData` serialises (serde_json) identically to the JSON-loaded one on every live field, and `SourcesDocument` is `==`.
6. **`/api/contract` gains `manifest_schema: 1` and `section_schema_version: 14`** beside `artifact_format_version: 13` (kept: the compile still writes that artifact; DB-5 retires the field). Additive: pact and AGC `contract` fixture re-recorded (`wider` → PATCH under the 0.x rule: AGC 0.6.1 if Task 1 already took 0.6.0 — do both bumps in one CHANGELOG line, 0.6.0, since they land in one batch).
7. **The 4 s load gate retargets.** `artifact_conformance.rs`'s gate measured the serving path's startup; the serving path is now `GraphService::from_sections` + unfold + scene priming. A new gate `sections_startup.rs` (warm cache, same 4 s) replaces it in `scripts/timing-gates.sh` slot 1; the artifact test stays as a plain (non-timed) compile-side admission. Cold cache (unpack first) is measured and reported, not gated (a deploy unpacks once).
8. **"FQ-1 corpus" does not exist** (no `compose_frontier`, no `/api/frontier`); §12's frontier p50/p99 is reported as the `node_edges` first-page latency distribution over a fixed corpus of 1,000 positions (every kind, every relation, the same list before/after), disclosed as the stand-in.

## File structure

- `server/atlas-graph/src/sqlite/extras.rs` — MODIFY: `EVENT_DATE` spec (+2 columns), `graph_derived(g, &ChronologyDerivation, red_letter)`; `sqlite/ddl.rs` (+2 columns); `service.rs` (`assemble` passes `&chronology.chrono`); `bins/compile_graph.rs` (same); tests.
- `server/atlas-graph/src/sqlite/snapshot.rs` — MODIFY: connection per worker, `mmap_size`, `open_with_workers`.
- `server/atlas-graph/src/sqlite/serve.rs` — CREATE: the companion loaders and the cross-ref seek, over a `&Connection`.
- `server/atlas-graph/src/sqlite/sidecars.rs` — MODIFY: `unfold(conn) -> Result<(AtlasData, SourcesDocument)>`.
- `server/atlas-graph/src/provenance.rs` — MODIFY: `ProvenanceIndex::from_families`.
- `server/atlas-graph/src/service.rs` — MODIFY: `Snap`, `from_sections`, `cross_refs_for_span`, `cross_refs_by_from` retired.
- `server/atlas-server/src/{load.rs, main.rs, handlers.rs, contract.rs}` — MODIFY.
- `server/atlas-cli/src/load.rs` — MODIFY.
- Tests: `atlas-graph/tests/{sqlite_laws.rs, serve_real_data.rs (CREATE), sections_startup.rs (CREATE), sqlite_real_data.rs}`, `atlas-server/tests/{perf_smoke.rs, scene_byte_identity.rs, graph_api.rs, contract_api.rs, contract_pact.rs}`, `atlas-cli/tests/cli.rs`.
- `scripts/timing-gates.sh`, `scripts/measure-db4c.ps1` (CREATE), `server/BENCHMARKS.md`, spec notes, ledger, memory, contracts/client bumps.

---

### Task 1: The fold correction — `source_meta` in `event_date` (the last root move)

**Files:**
- Modify: `server/atlas-graph/src/sqlite/extras.rs` (`EVENT_DATE` spec; `graph_derived` signature and rows), `server/atlas-graph/src/sqlite/ddl.rs` (`EXTRA_DDL_CORE_GRAPH` `event_date`), `server/atlas-graph/src/service.rs:413` (`assemble`), `server/atlas-graph/src/bins/compile_graph.rs` (`extras_for_artifact` call), `server/atlas-graph/src/sqlite/extras.rs::extras_for_artifact` (takes `&ChronologyDerivation`), `server/atlas-graph/tests/{sqlite_laws.rs, sqlite_real_data.rs, exports_laws.rs, extras_real_data.rs, version_root_regression.rs}`, spec §5.3 `event_date` note.
- Then the recorders (Global Constraints) with AQC 0.4.0 / AGC 0.6.0.

**Interfaces:**
- Produces: `Extras::graph_derived(g: &Graph, chrono: &crate::event_world::ChronologyDerivation, red_letter: &HashMap<String, Vec<(usize,usize)>>) -> Result<Extras, SqliteError>`; `extras_for_artifact(g, chrono: &ChronologyDerivation, data_dir)`; `event_date` columns `["event_id","from_year","to_year","from_month","from_day","to_month","to_day","seq","basis","meta_to_year","order_key"]`.

- [ ] **Step 1: Failing law** in `sqlite_laws.rs`'s `the_graph_derived_extras_of_the_specimen_round_trip_and_agree_with_the_attached_dump`: build a `ChronologyDerivation { order: vec!["e1".into()], resolved, source_meta: [("e1", SourceEventMeta { to_year: -990, order_key: 7 })], placements: HashMap::new() }` and assert the `event_date` row is `[Text("e1"), Int(-1000), Int(-999), Int(3), Null, Null, Null, Int(0), Int(1), Int(-990), Int(7)]`; a second event `e2` in `resolved` but not in `source_meta` yields `…, Null, Null`. Update the spec/DDL law expectation (`table_info` columns).
- [ ] **Step 2: Run** → FAIL (signature, columns).
- [ ] **Step 3: Implement.** `EVENT_DATE.columns` += `"meta_to_year", "order_key"`; DDL `event_date` += `meta_to_year INTEGER, order_key INTEGER` after `basis`; `graph_derived` takes `chrono: &ChronologyDerivation`, uses `chrono.resolved` where it used `resolved`, builds `heading` with `&chrono.resolved`, and per event pushes `match chrono.source_meta.get(id) { Some(m) => [Col::Int(m.to_year as i64), Col::Int(m.order_key as i64)], None => [Col::Null, Col::Null] }`. `extras_for_artifact(g, chrono, data_dir)`. `assemble`: `Extras::graph_derived(&graph, &chronology.chrono, &red_letter_spans)`. Compile binary: `extras_for_artifact(&graph_a_indexed, &chronology.chrono, &data_dir)`. `sqlite_real_data.rs`: `extras_for_artifact(&graph, &chronology.chrono, …)`. `exports_laws.rs`: `graph_derived(&graph, &chronology.chrono, …)`. Spec §5.3 `event_date` note: "DB-4c: `meta_to_year`, `order_key` = the chronology's `source_meta` (the curated `to_year`/`order_key` the Event wire serves); DB-4b's 'order_key dropped' was wrong about which order_key the spec meant."
- [ ] **Step 4: Compile and re-record.** `rm -rf ../data/cache/sections && cargo run -p atlas-graph --bin atlas-graph-compile -- --data-dir ../data/compiled --out ../data/compiled/graph.bin` (core's logical hash and the root move; kjv/concord/kretzmann blobs are reused). `version_root_regression.rs` re-pins (from-sources harness; `source_meta` present there). Recorders in order; AQC 0.4.0 in its copies (versioning.feature accept `"0.4.0" through "0.4.0"`, reject window `"0.5.0" through "0.7.0"`, `"garbage" through "0.4.0"`; client InlineData accept ("0.4.0","0.4.0"), ("0.3.9","0.5.0"); reject ("0.5.0","0.7.0"), ("0.0.1","0.3.9")); AGC 0.6.0 (CHANGELOG: "the last root move: `event_date` gained the chronology's `source_meta` columns; ids unchanged"). Assert `git diff --stat contracts/` touches only root-bearing files and `contract.json`.
- [ ] **Step 5: Run** `cargo test -p atlas-graph --test sqlite_laws --test extras_real_data --test exports_laws --test version_root_regression`, `cargo test -p atlas-server --test contract_api --test aqc_cucumber`, client `AqcContract` 7/7 → green. Commit code + sections + exports; commit contracts/client/docs; `bash scripts/contract-gate.sh` → PASSED.

---

### Task 2: `SqliteSnapshot` with one connection per worker and `mmap_size`

**Files:**
- Modify: `server/atlas-graph/src/sqlite/snapshot.rs`, `server/atlas-graph/src/sqlite/mod.rs` (`open_read_only` unchanged), `server/atlas-graph/tests/sqlite_laws.rs`.

**Interfaces:**
- Produces: `SqliteSnapshot::open(manifest_path, source)` (= `open_with_workers(…, 1)`), `SqliteSnapshot::open_with_workers(manifest_path: &Path, source: &dyn SectionSource, workers: usize) -> Result<SqliteSnapshot, SqliteError>`, `pub fn workers(&self) -> usize`, `pub fn mmap_bytes(&self) -> u64`, `pub fn absent(&self) -> &[Section]`, `pub fn manifest(&self) -> &Manifest`, `pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T, SqliteError>) -> Result<T, SqliteError>` (made `pub`: the loaders in Task 3 run over a borrowed connection). `SqliteSnapshot: Send + Sync`.

- [ ] **Step 1: Failing laws** (`sqlite_laws.rs`):

```rust
#[test]
fn the_snapshot_opens_one_connection_per_worker_and_every_one_answers() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db4c-workers-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let layout = layout_under(&dir);
    write_sections(&g, &Extras::default(), "test", &layout).unwrap();
    let snap = SqliteSnapshot::open_with_workers(&layout.manifest_path(), &CommittedZstdSource { layout: layout.clone() }, 3).unwrap();
    assert_eq!(snap.workers(), 3);
    assert!(snap.mmap_bytes() > 0 && snap.mmap_bytes() == (0..4).map(|i| std::fs::metadata(layout.cache_path(&snap.manifest().sections[i].logical)).unwrap().len()).sum::<u64>());
    // every connection sees the views and the mmap pragma
    for _ in 0..6 {
        snap.with_conn(|c| {
            let n: i64 = c.query_row("SELECT COUNT(*) FROM all_node", [], |r| r.get(0))?;
            assert!(n > 0);
            let m: i64 = c.query_row("PRAGMA mmap_size", [], |r| r.get(0))?;
            assert_eq!(m as u64, snap.mmap_bytes());
            Ok(())
        }).unwrap();
    }
    // concurrent readers: 8 threads x 200 node lookups agree with the model
    let snap = std::sync::Arc::new(snap);
    let ids: Vec<_> = g.nodes.keys().cloned().collect();
    let handles: Vec<_> = (0..8).map(|_| { let s = snap.clone(); let ids = ids.clone(); let g = g.clone(); std::thread::spawn(move || { for _ in 0..200 { for id in &ids { assert_eq!(s.node(id).map(|n| n.id), g.node(id).map(|n| n.id)); } } }) }).collect();
    for h in handles { h.join().unwrap(); }
    assert_answers_match(&*snap, &g);
    assert!(SqliteSnapshot::open_with_workers(&layout.manifest_path(), &CommittedZstdSource { layout }, 0).is_err(), "zero workers is refused");
}
```

- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement.** Struct: `conns: Vec<Mutex<Connection>>, next: AtomicUsize, version, present: Vec<Section>, absent: Vec<Section>, manifest: Manifest, mmap_bytes: u64`. `open_with_workers`: resolve sections once (as today, collecting `absent`), compute `mmap_bytes = Σ metadata(path).len()` capped at `1 << 30` (`MMAP_CAP`), then for `w in 0..workers` build a connection with `open_read_only(core)`, ATTACH the rest, `PRAGMA query_only = OFF; PRAGMA mmap_size = {mmap_bytes}; <views>; PRAGMA query_only = ON;`. `with_conn`: `let start = self.next.fetch_add(1, Relaxed) % n; for i in 0..n { if let Ok(g) = self.conns[(start+i)%n].try_lock() { return f(&g) } } let g = self.conns[start].lock().unwrap_or_else(|e| e.into_inner()); f(&g)`. `open` = `open_with_workers(…, 1)`. Doc comment: spec §2.5 step 3 delivered; judgment call 3.
- [ ] **Step 4: Run** `cargo test -p atlas-graph --test sqlite_laws` → green.
- [ ] **Step 5: Commit** — `feat(graph): DB-4c -- SqliteSnapshot: one connection per worker (round-robin try_lock), mmap_size = the attached files' sum (spec 2.5 step 3)`.

---

### Task 3: The companions from the sections, and the cross-ref seek

**Files:**
- Create: `server/atlas-graph/src/sqlite/serve.rs`
- Modify: `server/atlas-graph/src/provenance.rs` (`from_families`), `server/atlas-graph/src/sqlite/mod.rs`
- Test: `server/atlas-graph/tests/serve_real_data.rs` (CREATE)

**Interfaces:**
- Consumes: Task 2's `with_conn`; `crate::event_world::{ChronologyDerivation, SourceEventMeta}`; `atlas_graph_types::chrono::{ResolvedPlacement, ResolvedDate, TimePoint, Year, SeqKey, PlacementBasis}`; `crate::heading::HeadingEntry`; `crate::kjv_adapter::dot_ref`; `atlas_core::data::CrossRef`; `atlas_core::refs::ScriptureRef`.
- Produces (all `pub fn … (conn: &rusqlite::Connection) -> Result<_, SqliteError>` unless noted):

```rust
pub fn load_chronology(conn) -> ChronologyDerivation      // order: event_id ORDER BY seq; resolved: every event_date row; source_meta: rows with meta_to_year NOT NULL; placements: empty (unused on the serving path)
pub fn load_heading_index(conn) -> BTreeMap<String, HeadingEntry>   // key = kjv_adapter::dot_ref(book, chapter, verse)
pub fn load_red_letter_spans(conn) -> HashMap<String, Vec<(usize, usize)>>   // kjv.red_letter_span ORDER BY book, chapter, verse, ord
pub fn load_narrative_legs(conn) -> BTreeMap<String, Vec<String>>   // succession JOIN succession_step ORDER BY id, ord
pub fn load_provenance_families(conn, present: &[Section]) -> BTreeMap<&'static str, BTreeSet<String>>   // SELECT DISTINCT provenance FROM <schema>.<family> for every family of every present section (RowFamily::name())
pub fn load_counters(conn, present: &[Section]) -> (BuildStats, EventWorldStats)   // COUNT(*) per table/kind; the boot log's numbers
pub fn cross_refs_for_span(conn, span: &ScriptureRef) -> HashMap<String, Vec<CrossRef>>   // kjv.cross_refs WHERE from_a=? AND from_b=? [AND from_c BETWEEN ? AND ?] ORDER BY ord; key = dot_ref(from_a, from_b, from_c); CrossRef { target: target_display, votes }
```

`ProvenanceIndex::from_families(by_family: BTreeMap<&'static str, BTreeSet<String>>) -> ProvenanceIndex`.

- [ ] **Step 1: Failing real-data laws** (`serve_real_data.rs`): load the artifact path ONCE (`GraphService::from_artifact` + `to_service_parts` for the raw chronology) and the sections ONCE (`SqliteSnapshot::open` over `SectionLayout::under(data/compiled)`), then:
  - `load_chronology`: `order == artifact.chronology.chrono.order`; `resolved == …resolved`; `source_meta == …source_meta` (HashMap equality; `SourceEventMeta: PartialEq` — add the derive if missing).
  - `load_heading_index == service.heading_index`.
  - `load_red_letter_spans == service.red_letter_spans`.
  - `load_narrative_legs == service.narrative_legs`.
  - `load_provenance_families`: for every family name `service.provenance.families()` lists, `by_family(name)` (a `Vec<String>`) equals the artifact index's; and no extra families.
  - `load_counters == (service.stats, service.event_world_stats)` — derive `PartialEq` on both if missing; if a counter is not derivable from the tables (`cites_dropped_negative_votes` is a compile-time count of DROPPED rows: not in any table), the loader returns 0 for it and the law asserts every OTHER field; disclose in the doc comment.
  - `cross_refs_for_span`: for `JHN.3.16`, `JHN.3`, `GEN.1.1-5` and 50 random verses, the map equals `{k: service.cross_refs_by_from[k]}` restricted to the span's keys (compare against the artifact companion, which still exists in this task).
- [ ] **Step 2: Run** → FAIL (module missing).
- [ ] **Step 3: Implement** `serve.rs` with the SQL named in the interfaces; `ResolvedPlacement` rebuilt as `ResolvedPlacement { date: ResolvedDate { from: TimePoint { year: Year::new(from_year)?, month, day }, to: … }, seq: SeqKey(seq as u32), basis: match basis { 0 => Textual, 1 => Traditional } }` (`Year::new` refuses 0 — a `YearError` becomes `SqliteError`). `load_counters`: `kjv_verses` = `SELECT COUNT(*) FROM kjv.verse`; `cites_rows` = `COUNT(*) FROM kjv.cross_refs`; `places/events/narratives/anchors` = `COUNT(*) FROM all_node WHERE kind = ?` (via `node_kind_ordinal`); `dated_events` = `COUNT(*) FROM event_date`; `succession_rows/attests_rows/located_at_rows/dated_by_rows` = `COUNT(*)` of those tables; the ATTEST-1 counter (`event_mentions_rows` or whatever the struct names) = `COUNT(*) FROM mentions WHERE entity_kind = 3`. Read `EventWorldStats`'s full field list at `event_world.rs:445-470` and cover every field. `sqlite/mod.rs`: `pub mod serve;`.
- [ ] **Step 4: Run** `cargo test -p atlas-graph --test serve_real_data` → green.
- [ ] **Step 5: Commit** — `feat(graph): DB-4c -- the serving companions loaded from the section tables (chronology, headings, red-letter spans, legs, provenance families, boot counters) and cross_refs_for_span as a seek on xref_by_from; each proven equal to the artifact path on the real data`.

---

### Task 4: `AtlasData` and `SourcesDocument` unfold from core

**Files:**
- Modify: `server/atlas-graph/src/sqlite/sidecars.rs` (`unfold`), test `server/atlas-graph/tests/extras_real_data.rs`.

**Interfaces:**
- Produces: `pub fn unfold(conn: &Connection) -> Result<(AtlasData, SourcesDocument), SqliteError>` — `AtlasData` with `canon, books_meta, landmarks, land_mask, place_history, place_name_aliases, catechism, chronology_anchors, book_narration_windows` filled from the 21 tables (the inverse of `fold_sidecars`, reading with `extras::read_table` over each `TableSpec`), every other field `Default`; NOT `finish()`ed (the caller does, exactly as `load.rs` does today).

- [ ] **Step 1: Failing law** (`extras_real_data.rs`):

```rust
#[test]
fn unfold_is_the_inverse_of_fold_on_the_real_sidecars() {
    let sc = Sidecars::load(&data_dir()).unwrap().unwrap();
    let layout = SectionLayout::under(&data_dir());
    let snap = SqliteSnapshot::open(&layout.manifest_path(), &CommittedZstdSource { layout }).unwrap();
    let (atlas, sources) = snap.with_conn(|c| atlas_graph::sqlite::sidecars::unfold(c)).unwrap();
    assert_eq!(sources, sc.sources);
    let j = |a: &AtlasData| serde_json::to_value(a).unwrap();
    let (got, want) = (j(&atlas), j(&sc.atlas));
    for field in ["canon", "books_meta"] { assert_eq!(got[field], want[field], "{field}"); }
    // #[serde(skip)] fields: compare directly
    assert_eq!(atlas.landmarks, sc.atlas.landmarks);
    assert_eq!(atlas.land_mask, sc.atlas.land_mask);
    assert_eq!(atlas.catechism, sc.atlas.catechism);
    assert_eq!(atlas.chronology_anchors, sc.atlas.chronology_anchors);
    assert_eq!(atlas.book_narration_windows, sc.atlas.book_narration_windows);
    assert_eq!(atlas.place_history, sc.atlas.place_history);
    assert_eq!(atlas.place_name_aliases, sc.atlas.place_name_aliases);
    // and the derived indexes agree after finish()
    let (a, b) = (atlas.finish(), sc.atlas.clone().finish());
    assert_eq!(a.catechism_items_for_span(&atlas_core::refs::ScriptureRef::parse("JHN.3.16").unwrap()).len(), b.catechism_items_for_span(&atlas_core::refs::ScriptureRef::parse("JHN.3.16").unwrap()).len());
}
```

(`sc.atlas` is already `finish()`ed by `Sidecars::load`; comparing pre-`finish` fields is unaffected. Every compared struct derives `PartialEq` per the DB-4b survey; `AtlasData` itself does not, hence field-wise.)

- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement** `unfold`: read each table through `extras::read_table(conn, &SPEC)`, rebuild the structs in `ord` order (rows come back in pk order; `canon_book` by `ord`, `canon_chapter_verses` grouped by `book_ord`, catechism items grouped by `part_id` then `ord`, questions/verses likewise; `place_history` map keyed by `place_id` with names/blurbs by `ord` and verses by `(owner_kind, owner_ord, ord)`; `place_name_aliases` grouped by `place_id` then `alias_ord`, `translations` as a `HashMap`; `land_mask` from `rings_json` via `serde_json::from_str::<Vec<(f64,f64)>>`). `AtlasData { canon, books_meta, …, ..Default::default() }`.
- [ ] **Step 4: Run** → green. **Step 5: Commit** — `feat(graph): DB-4c -- AtlasData and SourcesDocument unfold from core (the inverse of the fold, proven on the real sidecars)`.

---

### Task 5: `GraphService::from_sections`, the `Snap`, and the cutover of the two load paths

**Files:**
- Modify: `server/atlas-graph/src/service.rs`, `server/atlas-server/src/load.rs`, `server/atlas-server/src/main.rs`, `server/atlas-server/src/handlers.rs` (`verse` ~1178, `xrefs` ~1970), `server/atlas-server/src/contract.rs`, `server/atlas-server/tests/perf_smoke.rs:144`, `server/atlas-cli/src/load.rs`, `server/atlas-server/tests/contract_api.rs`.

**Interfaces:**
- Produces:

```rust
#[derive(Clone)]
pub enum Snap { Mem(MemSnapshot), Sqlite(std::sync::Arc<SqliteSnapshot>) }
impl GraphQuery for Snap { /* every method delegates */ }
impl GraphSnapshot for Snap { fn version(&self) -> GraphVersion }
pub struct GraphService { snapshot: Snap, /* fields as today minus cross_refs_by_from */ }
impl GraphService {
    pub fn snapshot(&self) -> Snap;
    pub fn from_sections(data_dir: &Path) -> anyhow::Result<(GraphService, AtlasData, SourcesDocument)>;   // spec 2.5; AtlasData NOT finish()ed
    pub fn cross_refs_for_span(&self, span: &ScriptureRef) -> HashMap<String, Vec<CrossRef>>;   // Sqlite arm: serve::cross_refs_for_span; Mem arm: the retained in-memory map filtered to the span's keys (the Mem arm keeps a private `cross_refs_by_from` built at assemble, as today)
    pub fn absent_sections(&self) -> &[Section];
}
```

- [ ] **Step 1: Failing tests.** `extras_real_data.rs` (rename the file's purpose in its doc): `from_sections_serves_the_same_version_and_companions_as_from_artifact` — `let (s, atlas, sources) = GraphService::from_sections(&data_dir())?; let a = GraphService::from_artifact(&data_dir().join("graph.bin"))?; assert_eq!(s.version(), a.version()); assert_eq!(s.heading_index, a.heading_index); assert_eq!(s.red_letter_spans, a.red_letter_spans); assert_eq!(s.narrative_legs, a.narrative_legs); assert_eq!(s.chronology.chrono.order, a.chronology.chrono.order); assert_eq!(s.cross_refs_for_span(&ScriptureRef::parse("JHN.3.16")?), a.cross_refs_for_span(…)); assert!(s.absent_sections().is_empty());` plus `assert_answers_match(&s.snapshot(), &<the artifact graph>)` (the port over the pool answers like the model — reuse `sqlite_real_data`'s `committed_graph` helper by copying its 12 lines). `contract_api.rs`: `/api/contract` body has `manifest_schema == 1`, `section_schema_version == 14`, `artifact_format_version == 13`. A refusal test in `sqlite_laws.rs`: a tempdir with the specimen sections whose manifest `root` line is edited → `from_sections`-equivalent `SqliteSnapshot::open` errors with a message containing both hashes (already `read_manifest`'s behaviour — assert it names "expected"/"actual" style: extend `read_manifest`'s message to `root {stated} does not recompute from its sections (expected {recomputed})` if it doesn't already name both — it does).
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement.** `Snap` in `service.rs` (delegating impls for node, derive, edge_summary, edges, reading_window, nodes_of_kind, nodes, edges_with_nodes, row_provenance, rows_behind, position_of, version). `GraphService.snapshot: Snap`; `assemble` wraps `Snap::Mem(store.open(version)…)`; `snapshot()` returns `self.snapshot.clone()`. `cross_refs_by_from` → private `mem_cross_refs: Option<HashMap<…>>` (Some on the Mem arm); `cross_refs_for_span` as in the interface (Mem arm: iterate the span's verse keys via `ScriptureRef` → `dot_ref` per verse, using `data.canon`-free logic: for `Chapter`, the keys are every `BOOK.ch.v` present in the map with that prefix — collect by filtering the map's keys with `starts_with(&format!("{code}.{ch}."))`, exactly what `aggregate_span_xrefs` does internally today; read `atlas-core/src/xrefs.rs:109-160` and mirror its key selection so both arms feed it the same keys). `from_sections(data_dir)`: `let layout = SectionLayout::under(data_dir); std::fs::create_dir_all(&layout.cache_dir).map_err(|e| anyhow!("cache directory {} is not writable: {e}", …))?;` (§11 row 6) → `let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).min(8); let snap = SqliteSnapshot::open_with_workers(&layout.manifest_path(), &CommittedZstdSource { layout }, workers)?;` → `snap.with_conn(|c| { chronology, heading_index, red_letter_spans, narrative_legs, provenance families, counters, unfold })` → for each absent optional section `eprintln!("atlas: optional section {} absent -- its kinds are uninhabited", s.name())` (§11 row 5) → `GraphService { snapshot: Snap::Sqlite(Arc::new(snap)), … , scene_source: OnceLock::new() }`. `user_version` check: `SqliteSnapshot::open_with_workers` verifies `PRAGMA user_version` of every attached section equals its manifest `schema_version` and `SECTION_SCHEMA_VERSION`, refusing otherwise (`"section {} user_version {} unsupported (this build understands 14)"` — the artifact wall's wording, §11 row 4) — add that to Task 2's open if not already there (put it in Task 2; the law: a tampered `user_version` on the specimen is refused).
  `atlas-server/src/load.rs`: `load_graph_and_data` → `let (graph, data, _sources) = GraphService::from_sections(data_dir)?; let data = data.finish(); graph.scene_source(&data); Ok((graph, data))`; `load_sources` → returns the unfolded `SourcesDocument` (restructure `load_from_data_dir` so `from_sections` is called ONCE: `pub fn load_from_data_dir(data_dir) -> Result<LoadedAtlas> { let (graph, data, sources) = GraphService::from_sections(data_dir)?; let data = data.finish(); graph.scene_source(&data); Ok(LoadedAtlas::new(data, graph, sources)) }`, and `load_graph_and_data`/`load_sources` become thin wrappers or are deleted if `main.rs` is the only caller — read `main.rs:170-200` and update its two-halves usage to the one call, keeping its startup println of `stats`/`event_world_stats`). `atlas-cli/src/load.rs::load`: the same one call. `handlers.rs` `verse`: `let by_from = graph.cross_refs_for_span(&ScriptureRef::Verse(vid)); … aggregate_span_xrefs(&sref, &by_from, …)`; `xrefs`: `graph.cross_refs_for_span(&span)`. `perf_smoke.rs:144`: `&graph.cross_refs_for_span(&sref)`. `contract.rs`: `ContractOut` + `manifest_schema: u32` (= `atlas_graph::sqlite::manifest::MANIFEST_SCHEMA`) and `section_schema_version: u32` (= `atlas_graph::sections::SECTION_SCHEMA_VERSION`); unit test updated. `main.rs` doc comment: the served path is the sections; `--build-from-raw` unchanged.
- [ ] **Step 4: Run** `cargo test -p atlas-graph --test extras_real_data --test sqlite_laws`, `cargo test -p atlas-server --test contract_api --test graph_api`, `cargo test -p atlas-cli --test cli` → green (the CLI transcripts unchanged: `cli.rs:578` byte-identical).
- [ ] **Step 5: Commit** — `feat(graph, server, cli): DB-4c -- GraphService::from_sections: the server and bibex serve from the committed sections (spec 2.5 open sequence, 11 refusals); Snap over Mem|Sqlite; cross_refs_by_from retired into a seek; /api/contract advertises manifest_schema and section_schema_version`.

---

### Task 6: The proof, the gates, the measurements, the record

**Files:**
- Modify: `server/atlas-server/tests/{scene_byte_identity.rs, perf_smoke.rs, contract_pact.rs, graph_api.rs}` (they go through `load_from_data_dir`/`from_sections` — most already do via `LoadedAtlas`; make sure none calls `from_artifact` for the served surface), `server/atlas-graph/tests/artifact_conformance.rs` (the timing assertion moves), `scripts/timing-gates.sh` (slot 1 → `sections_startup`), `server/BENCHMARKS.md`, spec §9 note, ledger, memory, `contracts/pacts/http.json` (recorder: `/api/contract` only), `contracts/atlas-graph-contract/fixtures/contract.json` (bless), CHANGELOG lines.
- Create: `server/atlas-graph/tests/sections_startup.rs`, `scripts/measure-db4c.ps1`.

- [ ] **Step 1: The invariance proof.** From `server/`: `cargo test -p atlas-server --test scene_byte_identity` (25 hashes unchanged — if ANY moved, stop: the cutover changed a response); `cargo test -p atlas-server --test contract_pact` plain (must PASS without re-recording EXCEPT `/api/contract` — run the recorder, then `git diff contracts/pacts/http.json` must touch only the `/api/contract` entry); `cargo test -p atlas-cli --test contract_pact_cli` plain (no change); `cargo test -p atlas-server --test aqc_cucumber` 44/184 with NO fixture change (`git status contracts/atlas-query-contract` clean); AGC `--bless` then `--results`: only `contract.json` changes (two added fields); `dotnet test client.Tests` (all), Playwright if the 8080 demo is not in use (`npx playwright test` from `tests/ux` — skip and disclose if 8080 is busy: never touch it).
- [ ] **Step 2: `sections_startup.rs`** (gate, `#[ignore]`, ceiling 4 s, warm cache): times `GraphService::from_sections` + `finish()` + `scene_source` priming (the exact `load_from_data_dir` sequence) — `const STARTUP_CEILING: Duration = Duration::from_secs(4)`; prints `DB-4c STARTUP: from_sections {a}, finish {b}, scene priming {c}, total {t} (ceiling 4 s)`. Move `artifact_conformance.rs`'s `LOAD_CEILING` assertion out (keep the admission, drop the timing; doc comment says why); `scripts/timing-gates.sh` slot 1 → `atlas-graph|sections_startup|the_served_path_starts_under_the_ceiling`. perf_smoke's six gates run unchanged over the new path (their fixture `real_scene_source_and_graph()` goes through `load_from_data_dir`).
- [ ] **Step 3: `scripts/measure-db4c.ps1`** (§12): (a) peak RSS: `Start-Process bibex verse JHN.3.16 -PassThru`, sample `PeakWorkingSet64` every 20 ms until exit (the OVERLAY-1 method), before (checkout `29f06b9`'s binary? no — run `bibex --data-dir` against the artifact path is gone; report AFTER only against OVERLAY-1's recorded 755 MiB baseline); (b) cold start: `Remove-Item data/cache/sections/*`, `Start-Process atlas-server` on port **8000**, poll `http://localhost:8000/health` every 50 ms, report first-200 time; then warm (cache present) the same; stop the server; (c) frontier stand-in: a small ignored Rust test `frontier_latency_corpus` in `atlas-server/tests/perf_smoke.rs` that, over 1,000 positions (the first 100 ids of each of ten kinds, fixed order), times `snap.edges_with_nodes` (limit 25) for each inhabited kind at that position and prints p50/p99 — run before (at `29f06b9`, the artifact path: `git stash` is forbidden, so run it FIRST in this task before Task 5's commit? No: Task 6 runs after; instead run it once on the `Mem` arm via `GraphService::from_artifact` and once on the `Sqlite` arm via `from_sections`, both in the same test, and print both distributions).
- [ ] **Step 4: Standing block** (`logs-db4c-standing/run.sh`, `RUST_TEST_THREADS=4`) → workspace green, graph-types OFF/ON, gates 10/10 (the new slot 1 + the nine). Contract gate on the committed tree → PASSED.
- [ ] **Step 5: Docs and record.** `BENCHMARKS.md` `## DB-4c`: startup warm/cold, peak RSS vs 755 MiB, perf_smoke medians before/after (from the gate logs), frontier p50/p99 both arms, per-request spike numbers. Spec §9 table: artifact row "as shipped: `/api/contract` advertises manifest schema 1 + section user_version 14 beside the artifact field until DB-5". Ledger `## DB-4c EXECUTION` + standing line; memory `relational-artifact-decisions.md` DB-4c line (NEXT: DB-5). Commit; push `origin/worktree-bible-atlas-m1`.

---

## Self-review

**Spec coverage.** §2.5 steps 1–5 → Task 2 (steps 3–4: workers, mmap, views, `user_version` wall) + Task 5 (steps 1–2 through `read_manifest`/`CommittedZstdSource`; step 5). §11 all seven rows → Tasks 2/5 (root mismatch, required missing, transport mismatch, user_version, optional absent one line, cache unwritable, `bibex verify` from DB-4b). §8 row 5 "graph.bin no longer read" → Task 5 (both load paths); "AQC through the one path" → Task 6 Step 1 (recorder through `load_from_data_dir`). §9 gates → Task 6 (scene hashes, AGC both transports, AQC, client, Playwright-or-disclosed, load ceiling retargeted not loosened, perf_smoke unchanged); bumps: AQC/AGC in Task 1 (the fold correction) + the additive contract fields (Task 5/6); artifact wall → manifest schema advertised (Task 5). §12 → Task 6 Step 3 (peak RSS, cold start, frontier stand-in; compile wall time unchanged by this batch; repo delta at DB-5). §1.2 byte-identity → Task 6 Step 1 is the proof; Task 1 is the one disclosed exception.

**Placeholder scan.** Task 3's `load_counters` names the one field that cannot be derived and rules it; Task 5's Mem-arm key selection points at the exact lines to mirror. No TBDs.

**Type consistency.** `Extras::graph_derived(&Graph, &ChronologyDerivation, &HashMap)` (Task 1) is what `assemble`, the compile, `sqlite_real_data`, `exports_laws` call. `SqliteSnapshot::open_with_workers(&Path, &dyn SectionSource, usize)` (Task 2) is what `from_sections` calls (Task 5). `with_conn` is `pub` (Task 2) and the loaders take `&Connection` (Tasks 3–4). `Snap` (Task 5) is what `snapshot()` returns and what `perf_smoke`/handlers pass to `window::render(&snap, …)` (which take `&impl GraphQuery`). `cross_refs_for_span(&ScriptureRef) -> HashMap<String, Vec<CrossRef>>` in Tasks 3 (free fn over a connection) and 5 (method on the service).
