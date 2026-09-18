# DB-5 — The Deletions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `artifact.rs` (the bincode encoder/decoder and `FORMAT_VERSION`), `graph.bin`, `polities.json`, the nine folded JSON sidecars, `red-letter-spans.json` and `report.txt` leave the repository and the build; the compile folds the ETL's in-memory `AtlasData` straight into the sections; every real-data test gets its in-memory graph by reading the sections back; the contract's artifact wall becomes the manifest's; the repository size delta is stated (spec §8 row 6, §9, §12).

**Architecture:** Three moves. (1) **A sections → `Graph` reader** (`sqlite::reload::graph_from_sections`), the inverse of the writer: nodes from `all_node`, every row family from its table, the two spines, the extra tables' canonical bodies re-attached — proven by `version_root(&g) == manifest.root` and `assert_answers_match`. It is what the eleven real-data tests and the benches load instead of `graph.bin`, and it is a strong round-trip law in its own right. (2) **The compile folds from memory.** `atlas_etl::compile::compile` already yields the `AtlasData` the ETL used to serialise into the nine JSONs; the fold takes it (plus `sources.json`, kept) directly, red-letter spans stay in memory, and nothing is written but the exports and the sections. The ETL binary stops writing the JSONs and `report.txt` (it prints the report). (3) **The deletions**, then the contract's identity field: the graph vocabulary pins `manifest_schema` and `section_schema_version` instead of `artifact_format_version` (spec §9: "identity moves to the manifest"), which is one AGC MINOR (0.7.0) with the runner's projection updated and rebuilt.

**Tech Stack:** Rust 1.97.1; `rusqlite` (bundled); `bincode` REMOVED from `atlas-graph`; cabal/GHC for `contracts/runner`.

**Spec:** `docs/superpowers/specs/2026-09-14-relational-artifact-design.md` — §8 row 6, §9 (artifact row), §12 (repository size delta), §6.3 (the ETL keeps the in-memory `Graph`), §1.2. Owner ruling 2026-09-18: the "two green releases" gate is met.

## Global Constraints

- **The root does not move.** No DDL, dump or fold change. `manifest.toml`, the four blobs, the exports, the pacts' bodies, the 25 scene hashes, the AQC fixtures and the CLI transcripts are byte-identical after this batch, EXCEPT the graph vocabulary's identity field (Task 4, disclosed, AGC 0.7.0).
- The served path (`GraphService::from_sections`, DB-4c) is not touched. `--build-from-raw` (in-memory `Graph` from raw + curated) stays.
- `sources.json` and `data/exports/*.json` stay (not in the deletion list); `gen_sources` stays.
- `graph-types` is not edited.
- Gates: the artifact load gate (gate 1) retires WITH its subject; gates 2–11 renumber to 1–10, none loosened; the workspace suite must not get slower than it is by more than the sections read (a few seconds per real-data binary).
- Never `--release` while 8080 runs; never touch 8080; `RUST_TEST_THREADS=4` for the standing block; gcc on PATH; no `git stash`; untracked `client.Tests/FrontierMatrix*.cs`, `publish-apphome/`, `tests/ux/frontier-matrix.spec.ts` never staged (`git add` by explicit path); map-generator never edited.
- Commit trailers: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` / `Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W`.
- Recorders (from `server/`): `ATLAS_BLESS_PACT=1 cargo test -p atlas-server --test contract_pact` then plain; `ATLAS_BLESS_PACT=1 cargo test -p atlas-cli --test contract_pact_cli` then plain; `"$RUNNER" run --replay contracts/pacts --exports data/exports --bless contracts/atlas-graph-contract` then `--results <file>`; `bash scripts/contract-gate.sh` on a committed tree. The runner is rebuilt with `cd contracts/runner && cabal build` after Task 4's `Proj.hs` change.

## Judgment calls

1. **Tests read the sections back rather than rebuild from raw.** A from-raw build is 60–130 s per test binary (eleven binaries); reading the sections back is seconds and proves the writer round-trips. `graph_from_sections` is the "scan port" DB-1 OQ-8 deferred "until a second consumer wants one" — the tests are that consumer; it lives in `sqlite::reload`, test-and-tooling grade, never on the served path.
2. **`determinism.rs` retargets to the root and the dumps.** Two independent builds from identical raw sources must produce byte-identical per-section logical dumps (stronger than the root, which is a hash of them) — the same law the artifact bytes carried, now over the thing that ships.
3. **`artifact_conformance.rs` is deleted, not retargeted.** Its two subjects were the artifact's load ceiling (succeeded by gate 10, `sections_startup`, DB-4c) and the artifact's own admission (the compile's in-memory admission plus the sections admission cover it). `kjv_case_artifact_real_data.rs` (reads five verses back from the shipped file) retargets to the sections through `from_sections` — its purpose, "the shipped file, not a fresh build", survives intact.
4. **The vocabulary's identity field is swapped, not dropped.** `artifact_format_version: 13` → `manifest_schema: 1` + `section_schema_version: 14` in the graph vocabulary body, the fixture, the pact and the runner projection. AGC 0.7.0 (MAJOR class → MINOR under 0.x). The HTTP pact's `/api/contract` body already carries both since DB-4c.
5. **The ETL binary keeps running** (`compile` + `validate` + the printed report): it is the curated data's validation gate. It writes nothing under `data/compiled/` any more; `gen_sources` still writes `sources.json`.
6. **`AtlasData::load` and `read_json` leave atlas-core** (their only callers were `Sidecars::load` and the bench): nothing reads the JSONs after this batch, and a loader with no files to load is a trap.
7. **Historical prose stays historical.** `BENCHMARKS.md` entries, `server/Cargo.toml`'s batch log comments, old plans and CHANGELOG lines keep naming `graph.bin`; README's compile instructions, `LICENSES.md`'s "compiled into" cells, the CLI help/`CONTRACT.md`, `data/raw/README.md` and the doc comments on live code are updated.

## File structure

- `server/atlas-graph/src/sqlite/reload.rs` — CREATE: `graph_from_sections`.
- `server/atlas-graph/src/sqlite/extras.rs` — MODIFY: `Extras::compute(g, chrono, red_letter, atlas, sources)`; `extras_for_artifact` deleted.
- `server/atlas-graph/src/sqlite/sidecars.rs` — MODIFY: `Sidecars`/`Sidecars::load` deleted (`fold_sidecars` + `unfold` stay).
- `server/atlas-graph/src/service.rs` — MODIFY: `from_artifact` deleted; `assemble(…, sidecars: Option<(&AtlasData, &SourcesDocument)>)`.
- `server/atlas-graph/src/red_letter_spans.rs` — MODIFY: `read_file`/`write_file` deleted (spans_by_dot_ref stays).
- `server/atlas-graph/src/bins/compile_graph.rs` — MODIFY: no dump/encode/decode/graph.bin/red-letter file; folds from memory; `--out` retired.
- `server/atlas-graph/src/artifact.rs` — DELETE; `lib.rs` (`pub mod artifact;`), `Cargo.toml` (`bincode`), `build.rs`/`event_world.rs` (serde derives).
- `server/atlas-core/src/data.rs` — MODIFY: `AtlasData::load`, `read_json` deleted.
- `server/atlas-etl/src/main.rs` — MODIFY: the ten `write_json` calls and the `report.txt` write deleted.
- Tests: `artifact_conformance.rs` DELETE; `determinism.rs`, `canon_real_data.rs`, `port_widening_real_data.rs`, `provenance_registry_real_data.rs`, `sections_real_data.rs`, `sqlite_real_data.rs`, `serve_real_data.rs`, `extras_real_data.rs`, `kjv_case_artifact_real_data.rs`, `atlas-server/tests/{scene_byte_identity.rs, perf_smoke.rs, contract_pact.rs}`, `benches/queries.rs`, `graph_api.rs:1462` (message) — MODIFY.
- Contracts: `contracts/runner/src/Proj.hs`, `contracts/atlas-graph-contract/{VERSION, CHANGELOG.md, fixtures/graph-vocabulary.json}`, `contracts/pacts/http.json` (recorded), `server/atlas-server/src/graph_handlers.rs` or wherever the vocabulary body is assembled (search `artifact_format_version`).
- `scripts/timing-gates.sh`, `README.md`, `LICENSES.md`, `data/raw/README.md`, `server/atlas-cli/{CONTRACT.md, src/commands/help.rs, src/error.rs, src/load.rs}`, `server/atlas-server/src/{main.rs, load.rs, app.rs}` doc comments, `server/BENCHMARKS.md` (DB-5 entry), ledger, memory.
- `data/compiled/{graph.bin, polities.json, canon.json, books-meta.json, chronology-anchors.json, book-narration-windows.json, landmarks.json, land-mask.json, catechism.json, place-history.json, place-names-kjv.json, red-letter-spans.json, report.txt}` — `git rm`.

---

### Task 1: `graph_from_sections` — the sections read back into a `Graph`

**Files:**
- Create: `server/atlas-graph/src/sqlite/reload.rs`; modify `sqlite/mod.rs`.
- Test: `server/atlas-graph/tests/reload_real_data.rs` (CREATE).

**Interfaces:**
- Consumes: `SqliteSnapshot::{with_conn, present, manifest}`, `rows::read_rows(conn, family) -> Vec<(i64, RowOwned)>`, `extras::{table_specs_of, read_table, row_body}`, `sections::{row_tables_of, spine_corpus, extra_tables_of}`, `Node::decode`, `Graph::build_indexes`, `event_world::add_justified_by`.
- Produces: `pub fn graph_from_sections(snap: &SqliteSnapshot) -> Result<Graph, SqliteError>` — every node of every present section, every row family in `ord` order, both spines, `extra_tables` re-attached; indexes built and `justified-by` added, exactly the shape `from_artifact` used to return. And `pub fn committed_graph(data_dir: &Path) -> anyhow::Result<(Graph, SqliteSnapshot)>` (opens the layout under `data_dir` with one worker and reads it back) for the tests.

- [ ] **Step 1: Failing law** (`reload_real_data.rs`):

```rust
//! DB-5: the sections read back into a Graph -- what every real-data test
//! loads now that graph.bin is gone, and the writer's round-trip law.
use std::path::Path;
use atlas_graph::sqlite::reload::committed_graph;
use atlas_graph_types::store::assert_answers_match;

fn data_dir() -> std::path::PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled") }

#[test]
fn the_sections_read_back_into_the_graph_that_wrote_them() {
    let (g, snap) = committed_graph(&data_dir()).unwrap();
    assert_eq!(atlas_graph_types::sections::version_root(&g).hex(), snap.manifest().root, "the read-back graph publishes the manifest root");
    for s in atlas_graph_types::sections::Section::SHIPPED {
        let dump = atlas_graph_types::sections::logical_dump_section(&g, s);
        let hash = atlas_graph::sqlite::logical::logical_hash(&dump);
        let ms = snap.manifest().sections.iter().find(|m| m.name == s.name()).unwrap();
        assert_eq!(hash, ms.logical, "{s:?}: the read-back dump is the section's own");
    }
    assert_eq!(g.nodes.len(), 6263 + 32357 + 3972 + 50602);
    assert_eq!(g.cross_refs.len(), 343558);
    assert_answers_match(&snap, &g);
}
```

- [ ] **Step 2: Run** → FAIL (module missing).
- [ ] **Step 3: Implement `reload.rs`:**

```rust
pub fn graph_from_sections(snap: &SqliteSnapshot) -> Result<Graph, SqliteError> {
    snap.with_conn(|conn| {
        let mut g = Graph::default();
        // nodes: every present section's `node` table (the union view), payload decoded
        let mut stmt = conn.prepare("SELECT payload FROM all_node")?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            let payload: Vec<u8> = r.get(0)?;
            let node = Node::decode(&payload).map_err(|e| SqliteError(format!("node payload: {e}")))?;
            g.nodes.insert(node.id.clone(), node);
        }
        // rows: each family from each present section's table, in ord order (ContainsBible from both core and kjv, merged by ord)
        for section in snap.present() {
            let schema = if *section == Section::Core { "main" } else { section.name() };
            for family in row_tables_of(*section) {
                for (ord, row) in read_rows_in(conn, schema, *family)? {   // read_rows over `<schema>.<table>`
                    push_row(&mut g, *family, ord, row);                  // one match over RowOwned -> the Graph Vec
                }
            }
        }
        sort_contains_bible_by_ord(&mut g);   // the two halves interleave by their global ord
        // spines
        for section in snap.present() {
            if let Some(corpus) = spine_corpus(*section) { … SELECT node_id FROM <schema>.reading_spine ORDER BY ord → g.reading.insert(corpus, ReadingSpine { order }) }
        }
        // extras: the canonical bodies straight from the tables (pk order)
        for section in snap.present() {
            for spec in table_specs_of(*section) {
                let bodies = read_table_in(conn, schema, spec)?.iter().map(|r| row_body(spec, r)).collect::<Result<Vec<_>, _>>()?;
                g.extra_tables.insert(spec.name, bodies);
            }
        }
        Ok(g)
    })
    .map(|mut g| { g.build_indexes(); crate::event_world::add_justified_by(&mut g); g })
}
```

`read_rows` today prepares `SELECT … FROM {table}` unqualified; add `read_rows_in(conn, schema, family)` in `rows/mod.rs` that qualifies the table (`{schema}.{table}`) and make `read_rows` call it with `"main"`; likewise `read_table_in` in `extras.rs`. `push_row`: a `match row { RowOwned::Attests(r) => g.attests.push(r), … }` over all 21 arms (the `ord` from the table IS the Vec index; assert `g.<family>.len() == ord as usize` before each push to catch a gap). `ContainsBible` rows come from core and kjv with a shared global ord space: collect `(ord, row)` pairs from both, sort by ord, then push (the assert holds). `committed_graph(data_dir)`: `SectionLayout::under(data_dir)` → `SqliteSnapshot::open` → `graph_from_sections` → `(g, snap)`.

- [ ] **Step 4: Run** `cargo test -p atlas-graph --test reload_real_data` → green (and note its wall time for BENCHMARKS).
- [ ] **Step 5: Commit** — `feat(graph): DB-5 -- sqlite::reload::graph_from_sections: the sections read back into the Graph that wrote them (root and every logical hash reproduced; the tests' in-memory graph now that graph.bin goes)`.

---

### Task 2: The compile folds from memory; the ETL stops writing the JSONs

**Files:**
- Modify: `server/atlas-graph/src/sqlite/extras.rs` (`Extras::compute`, delete `extras_for_artifact`), `sqlite/sidecars.rs` (delete `Sidecars`), `service.rs` (`assemble` sidecar parameter; delete `from_artifact`), `red_letter_spans.rs` (delete `read_file`/`write_file`, `FORMAT_VERSION`, `RedLetterSpansFile`), `bins/compile_graph.rs`, `server/atlas-etl/src/main.rs`, `server/atlas-core/src/data.rs` (delete `load`, `read_json`), `server/atlas-server/src/main.rs` doc comment.
- Test: `extras_real_data.rs`, `sqlite_real_data.rs`, `serve_real_data.rs` (switch to Task 1's reader; Task 3 finishes the rest).

**Interfaces:**
- Produces: `Extras::compute(g: &Graph, chrono: &ChronologyDerivation, red_letter: &HashMap<String, Vec<(usize,usize)>>, atlas: &AtlasData, sources: &SourcesDocument) -> Result<Extras, SqliteError>` = `graph_derived` + `fold_sidecars`; `GraphService::assemble(…, sidecars: Option<(&AtlasData, &SourcesDocument)>)`.
- The compile binary: `atlas-graph-compile --data-dir <compiled> [--sections-cache <dir>]` (`--out` accepted and ignored with a printed line for one release: scripts and README pass it today); it reads `data/raw`, `data/curated`, `<compiled>/sources.json`; writes `data/exports/*.json`, `<compiled>/manifest.toml`, `<compiled>/sections/`.

- [ ] **Step 1: Failing tests.** `extras_real_data.rs`: replace `Sidecars::load(&data_dir())` with the ETL's in-memory value — `let out = atlas_etl::compile::compile(&raw_dir, &curated_dir).unwrap(); let atlas = out.data.finish(); let sources: SourcesDocument = serde_json::from_str(&read_to_string(data_dir().join("sources.json"))).unwrap();` (raw/curated dirs = `data_dir().parent()/raw|curated`; `atlas-etl` is already a dev-dependency of atlas-graph? It is a normal dependency of atlas-graph — check `Cargo.toml`; yes: `atlas-etl = { path = "../atlas-etl" }`); the unfold law then compares against THAT `atlas` (identical fields: the JSONs were its serialisation). `sqlite_real_data.rs`'s `committed_graph()` → Task 1's `reload::committed_graph` + `Extras::compute(...)` with the same in-memory atlas (the extras are already attached by the reader — `write_sections` needs the typed rows, so compute them from `atlas` + the graph's chronology: the chronology now comes from `serve::load_chronology` over the snapshot; red-letter from `serve::load_red_letter_spans`). `serve_real_data.rs`: the "artifact" side becomes `GraphService::from_sections` (DB-4c's own path) — the laws then say "loaded twice, equal", plus the counters/companions against `reload`'s graph where a graph is needed.
- [ ] **Step 2: Run** → FAIL (signatures).
- [ ] **Step 3: Implement.** `Extras::compute` as the interface says; delete `extras_for_artifact` and `Sidecars`; `assemble` takes `Option<(&AtlasData, &SourcesDocument)>` (the from-sources constructors pass `None`; `from_artifact` deleted with its doc); `red_letter_spans.rs` keeps only `spans_by_dot_ref` (+ `char_offset`). Compile binary: delete the `dump`/`encode`/`decode`/`to_service_parts`/`write graph.bin` block and the red-letter `write_file`; the in-memory admission `assert_answers_match(&graph_a_indexed, &graph_b)` stays; `let sources: SourcesDocument = serde_json::from_str(&std::fs::read_to_string(data_dir.join("sources.json"))?)?;` `let extras = Extras::compute(&graph_a_indexed, &chronology.chrono, &red_letter_spans_map, &atlas, &sources)?;` (`red_letter_spans` is the `BTreeMap` from `spans_by_dot_ref` → `.into_iter().collect()` into the `HashMap`; `atlas` is the in-memory `AtlasData` the binary already holds — confirm it is `finish()`ed: `atlas_etl::compile::compile(..)?.data` then `.finish()` if the binary does not already). `parse_args`: `--out` parsed and ignored with `println!("atlas-graph-compile: --out is retired (DB-5: graph.bin is no longer written); the sections under --data-dir are the artifact")`. ETL `main.rs`: delete the ten `write_json` lines and the `report.txt` write; keep `print!("{text}")`; delete `write_json` if unused; doc comment: the ETL validates and reports, the compile writes. `AtlasData::load` + `read_json` deleted from `data.rs` (grep for any remaining caller first; the survey found none but `Sidecars::load` and the bench, both gone by Task 3). `atlas-server/src/main.rs`: doc comments naming graph.bin updated ("the sections under --data-dir").
- [ ] **Step 4: Run** `cargo build --workspace --bins --tests` (Task 3's files will not compile yet if they still call `from_artifact` — do Task 3's mechanical retargets in the same working set if needed, but commit separately) and `cargo test -p atlas-graph --test extras_real_data --test serve_real_data --test reload_real_data` → green; `cargo run -p atlas-graph --bin atlas-graph-compile -- --data-dir ../data/compiled` → the manifest and blobs unchanged (`git status data/` shows nothing; the exports unchanged), no `graph.bin`/`red-letter-spans.json` rewritten (delete them first in Task 5; here assert the binary no longer touches them: their mtimes unchanged).
- [ ] **Step 5: Commit** — `feat(graph, etl, compile): DB-5 -- the compile folds the ETL's in-memory AtlasData + sources.json straight into the sections (no graph.bin, no red-letter file, no JSON round trip); the ETL validates and reports without writing data/compiled; AtlasData::load retired`.

---

### Task 3: `artifact.rs` deleted; every test and bench on the sections

**Files:**
- Delete: `server/atlas-graph/src/artifact.rs`, `server/atlas-graph/tests/artifact_conformance.rs`.
- Modify: `lib.rs`, `Cargo.toml` (`bincode` removed; `cargo update -p bincode`? no — just remove the line and let the lockfile prune), `build.rs`/`event_world.rs` (drop `serde::Serialize, serde::Deserialize` from `BuildStats`/`EventWorldStats`), `service.rs` (`use crate::artifact` gone), `exports.rs:12,142` doc comments; tests `determinism.rs`, `canon_real_data.rs`, `port_widening_real_data.rs`, `provenance_registry_real_data.rs`, `sections_real_data.rs`, `kjv_case_artifact_real_data.rs`, `atlas-server/tests/{scene_byte_identity.rs, perf_smoke.rs}`, `benches/queries.rs`, `graph_api.rs:1462`; `scripts/timing-gates.sh` (gate 1 retired: the list starts at `graph_conformance`, comments renumbered 1–10).

- [ ] **Step 1: Retarget the tests** (each is a mechanical swap of the loader):
  - `canon_real_data.rs`, `port_widening_real_data.rs`, `provenance_registry_real_data.rs`, `sections_real_data.rs`: their `committed_graph()` helper → `atlas_graph::sqlite::reload::committed_graph(&data_dir()).unwrap().0` (keep the `OnceLock`). `provenance_registry_real_data.rs` also counts `pub provenance:` declarations across files — `artifact.rs`'s DTOs carried some: re-pin its expected counts after the deletion (the test prints the tally it found; the DTO structs were `provenance: String` fields in artifact.rs — subtract them).
  - `kjv_case_artifact_real_data.rs`: `GraphService::from_artifact(...)` → `GraphService::from_sections(&compiled).unwrap().0`; module doc: "the SHIPPED SECTIONS, the actual thing atlas-server opens".
  - `determinism.rs`: `build_and_encode` → `build_and_dump`: build the graph, `build_indexes`, `add_justified_by`, attach `Extras::compute(&g, &chrono, &HashMap::new(), &atlas, &sources)` — hmm, the two independent builds must be compared on what ships: return `(version_root(&g).hex(), Section::SHIPPED.iter().map(|s| logical_dump_section(&g, *s)).collect::<Vec<_>>())`; assert both equal across the two builds; the doc comment says why this is at least as strong as the bytes were.
  - `scene_byte_identity.rs:74`: `from_sections` (+ `.finish()` on the returned `AtlasData`, the scene source primed as `load_all` does).
  - `perf_smoke.rs` gate 11: the Mem arm = `reload::committed_graph(&compiled)` published into a `MemStore` (`store.publish(g); store.open(v)`) wrapped as `GraphService`? The test only needs a `GraphQuery`: use the `MemSnapshot` directly for the mem arm and `from_sections`'s `snapshot()` for the sqlite arm; labels unchanged.
  - `benches/queries.rs`: `load_real()` → `load_all`-shaped: `GraphService::from_sections` + `finish` + `scene_source`; `bench_artifact_load` → `bench_sections_open` (same `sample_size(10)`).
  - `graph_api.rs:1462`: the error string → "data/compiled/manifest.toml + sections/ must exist".
- [ ] **Step 2: Delete** `artifact.rs`, `artifact_conformance.rs`, `pub mod artifact;`, the `bincode` line, the two serde derive pairs; `scripts/timing-gates.sh` gate list minus gate 1 (comment: "DB-5: the artifact load gate retired with its subject; the served startup gate (`sections_startup`) is its successor"); renumber the log prefixes if the script numbers them (it prints `gate N/M` from the array — automatic).
- [ ] **Step 3: Run** `cargo build --workspace --bins --tests --benches` → clean, no `bincode` in `Cargo.lock` for atlas-graph (`grep -c bincode server/Cargo.lock` may still list it if another dependency pulls it: report). `cargo test -p atlas-graph` (whole crate, minus ignored) and `cargo test -p atlas-server --test scene_byte_identity --test graph_api` → green.
- [ ] **Step 4: Commit** — `refactor(graph, server): DB-5 -- artifact.rs, the bincode encoder and FORMAT_VERSION deleted; every real-data test and bench reads the sections back (reload) or opens them (from_sections); the artifact load gate retires with its subject`.

---

### Task 4: The identity field of the graph vocabulary becomes the manifest's (AGC 0.7.0)

**Files:**
- Modify: the server's vocabulary assembly (search `artifact_format_version` in `server/atlas-server/src/` — the DB-4c survey placed it in `graph_handlers.rs` or `contract_pact.rs:158`; the pact recorder reads the LIVE endpoint, so the field is produced by the endpoint), `server/atlas-server/tests/contract_pact.rs:158`, `contracts/runner/src/Proj.hs:184-189`, `contracts/atlas-graph-contract/fixtures/graph-vocabulary.json:2`, `contracts/atlas-graph-contract/{VERSION → 0.7.0, CHANGELOG.md}`, `contracts/pacts/http.json` (recorded).

- [ ] **Step 1:** In the vocabulary body: replace `artifact_format_version: artifact_format_version()` with `manifest_schema: MANIFEST_SCHEMA` and `section_schema_version: SECTION_SCHEMA_VERSION` (both `u32`). `Proj.hs`: `field1 "manifest_schema" Keep, field1 "section_schema_version" Keep` in place of the `artifact_format_version` line, comment updated ("the manifest wall: an unknown section user_version is refused at open, exactly as an old graph.bin was"). Rebuild: `cd contracts/runner && cabal build` (the `RUNNER` path is `cabal list-bin contract-runner`). `graph-vocabulary.json`: re-blessed by the runner (Step 2). AGC `VERSION` 0.7.0, CHANGELOG: "**0.7.0** (Batch DB-5) — MINOR under the 0.x policy (MAJOR class: a pinned field renamed). `artifact_format_version` leaves the vocabulary with the artifact; `manifest_schema` (1) and `section_schema_version` (14) pin the served identity instead (spec §9). No id, no root, no other fixture moved."
- [ ] **Step 2: Recorders** (Global Constraints order): the HTTP pact's vocabulary body changes (one field out, two in), nothing else; the CLI pact unchanged; AGC bless changes `graph-vocabulary.json` only; AQC unchanged (no root move — `git status contracts/atlas-query-contract` clean; do NOT run `export_aqc_examples`... run it anyway to prove it rewrites nothing).
- [ ] **Step 3:** `cargo test -p atlas-server --test contract_pact --test contract_api --test aqc_cucumber`, `cargo test -p atlas-cli --test contract_pact_cli` → green; commit; `bash scripts/contract-gate.sh` → PASSED (AGC 0.7.0 graded "declared minor = major under the 0.x rule"; AQC unchanged in range).
- [ ] **Step 4: Commit** — `chore(contracts, server): DB-5 -- the graph vocabulary pins manifest_schema + section_schema_version in place of artifact_format_version (AGC 0.7.0); runner projection rebuilt; HTTP pact re-recorded (vocabulary body only)`.

---

### Task 5: The deletions, the docs, the size delta, the record

**Files:**
- `git rm`: `data/compiled/graph.bin`, `polities.json`, `canon.json`, `books-meta.json`, `chronology-anchors.json`, `book-narration-windows.json`, `landmarks.json`, `land-mask.json`, `catechism.json`, `place-history.json`, `place-names-kjv.json`, `red-letter-spans.json`, `report.txt`.
- Modify: `README.md` (the compile command: `cargo run -p atlas-graph --bin atlas-graph-compile -- --data-dir ../data/compiled`; the "Data" section: what is committed = `manifest.toml`, `sections/`, `sources.json`, `exports/`), `LICENSES.md` (every "compiled into `<x>.json`" / "graph artifact (`graph.bin`)" cell → "compiled into the sections (`data/compiled/sections/`, the `core`/`kjv`/… section)"), `data/raw/README.md` (any "compiled into graph.bin"), `server/atlas-cli/{CONTRACT.md, src/commands/help.rs (`--data-dir` line), src/error.rs:20, src/load.rs:5-38}`, `server/atlas-server/src/{load.rs:80,122, app.rs:50}` doc comments, `.gitignore` (no change), `server/BENCHMARKS.md` (`## DB-5`), spec §8 row 6 "as shipped" note, ledger, memory.

- [ ] **Step 1:** `git rm` the thirteen files; run the compile once more (`--data-dir ../data/compiled`) and assert `git status --porcelain data/` is EMPTY (nothing regenerated, nothing moved); run the ETL binary (`cargo run -p atlas-etl` — check its args in `atlas-etl/src/main.rs`) and assert the same; `bibex verify` → OK; `cargo test -p atlas-cli --test cli` → 58/58 (transcripts unchanged).
- [ ] **Step 2: Size delta** (§12): before/after `du -sh data/compiled` (158 M → ~62 M), `git count-objects -vH` after `git gc --prune=now`? NO — never run gc on the shared worktree repo without the owner; report the working-tree delta and the pack size as observed (`git count-objects -vH`), and state that history retains the artifact.
- [ ] **Step 3: Docs** as listed; BENCHMARKS `## DB-5 (2026-09-18)`: the deletion list, the working-tree delta, `reload_real_data` wall time, the workspace suite's wall time before/after (from the standing-block logs: `1-workspace.log`'s last `finished in` lines are per binary — sum them or use the wrapper's timestamps).
- [ ] **Step 4: Standing block** (`logs-db5-standing/run.sh`, `RUST_TEST_THREADS=4`): workspace green, graph-types OFF/ON, gates 10/10. Contract gate PASSED (Task 4). Commit; push `origin/worktree-bible-atlas-m1`.

---

## Self-review

**Spec coverage.** §8 row 6: `artifact.rs` (Task 3), `graph.bin` (Task 5), the bincode encoder (Task 3), `polities.json` and the nine JSONs (Task 5; the ETL stops writing them, Task 2); "the commit that finally frees the 100 MiB headroom" (Task 5 Step 2 states it). §9 artifact row: identity in the manifest — the vocabulary swap (Task 4) and the `user_version` wall already at open (DB-4c). §12 repository size delta (Task 5). §6.3: the ETL keeps the in-memory `Graph`; the compile still builds and admits it in memory (Task 2 keeps `assert_answers_match(graph_a, graph_b)`). §1.2 byte-identity: nothing on the wire changes but the vocabulary's identity field, disclosed.

**Placeholder scan.** Task 2 Step 1's `serve_real_data.rs` rewrite is described precisely enough (the "artifact side" becomes `from_sections`); Task 3 Step 1 names each file's swap. No TBDs.

**Type consistency.** `reload::committed_graph(&Path) -> anyhow::Result<(Graph, SqliteSnapshot)>` (Task 1) is what Tasks 2–3 call; `Extras::compute(&Graph, &ChronologyDerivation, &HashMap<String, Vec<(usize,usize)>>, &AtlasData, &SourcesDocument)` (Task 2) is what the compile, `determinism.rs` and `sqlite_real_data.rs` call; `assemble(…, Option<(&AtlasData, &SourcesDocument)>)` (Task 2) has no external callers but `from_sources*`.
