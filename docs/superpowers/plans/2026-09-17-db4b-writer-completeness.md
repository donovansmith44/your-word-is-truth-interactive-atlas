# DB-4b — Writer Completeness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The section writer writes EVERYTHING spec §5 lists for the four shipped sections — node projections, `event_date`, `heading_index`, `red_letter_span`, the folded sidecars — as zstd-compressed, committed blobs under `data/compiled/sections/` with `data/compiled/manifest.toml`, resolved at read time through `SectionSource` (`CommittedZstdSource` + the `data/cache/sections/` unpack cache), and verifiable offline by `bibex verify`.

**Architecture:** DB-4 (spec §8 row 5) is three sub-batches; DB-4a (identity) landed; **this is DB-4b**; DB-4c (the read cutover: server opens sections, `graph.bin` no longer read) follows. Three seams carry the batch. (1) **The graph carries the non-graph tables' logical lines.** `Graph` gains `extra_tables: BTreeMap<&'static str, Vec<Vec<u8>>>` — table name → canonical row bodies in primary-key order — and `sections::logical_dump_section` appends them after the graph-native tables, so the version root (`MemStore::publish`, `manifest.toml`, `SqliteSnapshot::version`) covers projections, resolved chronology, headings, red-letter spans and every folded sidecar with graph-types learning nothing but a map of bytes. (2) **One generic row shape for the 29 new tables.** `atlas_graph::sqlite::extras` defines `Col`/`ExtraTable` (name, columns, pk, rows); one INSERT path, one `SELECT … ORDER BY pk` read-back, one canonical encoder; per-table code is only "rows from the source struct". (3) **Committed blobs behind a source.** The writer writes the uncompressed file straight into the unpack cache (`data/cache/sections/<logical>.sqlite`), compresses it to `data/compiled/sections/<name>.<logical>.sqlite.zst`, records the blob hash; `SqliteSnapshot::open(manifest, &dyn SectionSource)` resolves every section through `CommittedZstdSource`, which verifies the blob hash and unpacks on a cache miss. The root MOVES ONCE MORE in this batch (the dump widens; spec §3.6's "once" was written before DB-4 was split) and every pin is re-recorded through the DB-4a recorders; DB-4c must then move nothing.

**Tech Stack:** Rust 1.97.1 (pinned), `rusqlite 0.32` (bundled), **`zstd` (bundled C, feature `zstdmt`) — new, in `atlas-graph` only**, `toml 0.8`, `serde_json`, `atlas-core` (sidecar structs), graph-types (zero-dep, `canon-ids` ON in the atlas workspace).

**Spec:** `docs/superpowers/specs/2026-09-14-relational-artifact-design.md` — §2.2 (manifest), §2.3 (files on disk), §2.4 (`SectionSource`), §2.5 (open sequence), §3.4 (logical dump), §3.5 (`bibex verify`), §3.6 (what moves), §5.0 (conventions), §5.3–5.6 (schema), §6.1 steps 3–4 (zstd, manifest, exports), §9 (bumps), §11 (errors), §12 (measurements). Prior plans: `2026-09-17-db2b-sqlite-writer.md` (judgment calls 3, 5 deferred here), `2026-09-17-db4a-identity-cutover.md`.

## Global Constraints

- `graph-types` has **zero dependencies** (spec §1.2, §3.4); `graph-types/tests` asserts the Cargo metadata. Nothing SQLite, zstd, serde or atlas-core enters it. Its version goes 0.2.0 → **0.2.1** (additive: one field, three functions; CDC-1 "additive = PATCH").
- `rusqlite` and `zstd` live in `atlas-graph` only (spec §8 row 3 precedent).
- Every section: `PRAGMA user_version = 14`, `application_id = 0x424C4741`, `page_size = 4096`, `encoding = 'UTF-8'`, `journal_mode = OFF` at build; `WITHOUT ROWID` unless `INTEGER PRIMARY KEY`; **no `FOREIGN KEY`, no `CHECK`, no triggers** (§5.0; `ddl.rs`'s unit test scans every DDL string).
- Node ids are `TEXT` in their `any_node_id_str` form (`"Place:jerusalem"`); positions `n:…`/`e:…`; hash columns 16-byte BLOBs.
- Logical dump line format is binding (§3.4, DB-2b): `<table>\t<canonical row JSON>\n`, tables in `logical_table_order(section)`, rows in primary-key order; canonical JSON = `graph-types::canon` (keys in byte order, no whitespace, floats by `f64` Display). The dump from the in-memory side and from the file **must agree**; the gate proves it.
- The version root = manifest root = `sha256_prefixed_128(DOMAIN_PREFIX, name|logical|14|required\n …)` over `Section::SHIPPED`.
- Every committed blob **< 104,857,600 bytes** compressed (§2.3, §12); the writer refuses a larger one.
- zstd level **19** (§6.1 step 3).
- Files: `data/compiled/manifest.toml`, `data/compiled/sections/<name>.<logical>.sqlite.zst`, cache `data/cache/sections/<logical>.sqlite` (gitignored, already in `.gitignore` as `data/cache/`). The loader trusts only the hash, never the name.
- Error behaviour (§11): manifest root mismatch → refuse; required section missing → refuse naming name + logical; transport hash mismatch → refuse, delete the partial cache file, print both hashes; optional absent → one line, uninhabited; `bibex verify` any mismatch → non-zero exit, per-section report.
- `graph.bin`, `FORMAT_VERSION = 13`, `artifact.rs`, and the nine JSON sidecars are **not touched** (DB-5 deletes them); the server still reads `graph.bin` (DB-4c cuts over). `polities.json` is retired at DB-5, not folded (§5.3 closing note).
- Never `--release` while port 8080 runs; never touch 8080; `RUST_TEST_THREADS=4` for the standing block; no `git stash`; untracked `client.Tests/FrontierMatrix*.cs`, `publish-apphome/`, `tests/ux/frontier-matrix.spec.ts` are never staged; map-generator never edited.
- Commit trailers: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` / `Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W`.
- Recorders, in order, from `server/`: compile → `ATLAS_BLESS_PACT=1 cargo test -p atlas-server --test contract_pact` (red on purpose) then plain → `ATLAS_BLESS_PACT=1 cargo test -p atlas-cli --test contract_pact_cli` then plain → `"$RUNNER" run --replay contracts/pacts --exports data/exports --bless contracts/atlas-graph-contract` then `--results` (RUNNER = `cd contracts/runner && cabal list-bin contract-runner`) → `cargo run -p atlas-server --bin export_aqc_examples` → VERSION/CHANGELOG → `bash scripts/contract-gate.sh` (needs a committed tree).

## Judgment calls (recorded so the executor and reviewer argue from the same text)

1. **Extras ride on `Graph`, as bytes.** `MemStore::publish(graph)` is the port's only way to a `GraphVersion`; for its root to equal the manifest root the graph must carry the non-graph tables' lines. A `BTreeMap<&'static str, Vec<Vec<u8>>>` (additive field, `Default`) keeps graph-types blind to sidecar types. Not serialized in `graph.bin`: it is attached at load from the same sidecar files by the same function (`extras::extras_for_artifact`), so compile and server agree by construction. Cost if wrong: a graph loaded without sidecars publishes a root that differs from the manifest — the from-sources dev path (`--build-from-raw`, `from_sources*` constructors) does exactly that and is DISCLOSED as graph-only; the served path (`from_artifact`) and the compile binary attach.
2. **Derived tables are IN the dump.** `place`/`era`/`polity_era`/`verse`/`concord_unit`/`event_date`/`heading_index` are recomputable, but they cost nothing to include (one encoder) and `bibex verify` then covers every table the server will read at DB-4c. `edge_index`, `justification`, `ground`, `meta` stay out (DB-2b's ruling, unchanged).
3. **Lossless fold, spec columns are a floor.** Where the loaded struct has more than spec §5.3's column list, the column is added rather than dropped, so DB-5 can delete the JSON: `chronology_anchor` + `event_id TEXT, era_boundary INTEGER NOT NULL, source TEXT NOT NULL, note TEXT`; `provenance_entry` + `locator TEXT`; `event_date` splits the spec's single `month, day` into `from_month, from_day, to_month, to_day` (both `TimePoint`s carry them). Where the data has LESS than the spec: `canon_book.testament` is derived (`BOOKS` index < 39 → `"OT"`, else `"NT"`; there is no source field); `land_mask_region.name`/`ref_note` are **nullable and NULL** — the compiled `land-mask.json` is already flattened rings (the ETL dropped region names; re-sourcing `data/curated/land-mask.toml` is ETL scope, not the writer's), one row per ring, `rings_json` = the ring as canonical JSON.
4. **`event_date.order_key` is dropped; `seq` is the total order (R-DB4a-1).** The spec's `order_key INTEGER NOT NULL -- TOTAL traversal order` predates the ruling that `seq` IS that order; a second column with the same value is a drift risk. `event_by_order` indexes `seq`.
5. **`heading_index` is keyed by verse, not event.** `heading::build_heading_index` returns `BTreeMap<dot-ref, HeadingEntry{event_id,title,kind,continuation}>` — several verses name one event, so the spec's `event_id TEXT PRIMARY KEY` cannot hold. PK is `(book, chapter, verse)` with `event_id TEXT NOT NULL`.
6. **`place_name_alias` gets `alias_ord`.** `AtlasData::place_name_aliases` is `HashMap<id, Vec<PlaceNameAlias>>` — one place (`lebo-hamath`) has two alias rows. PK `(place_id, alias_ord, translation)`; `place_name_alias_verse` PK `(place_id, alias_ord, ord)`.
7. **The unpack cache IS the writer's output directory.** The freshly written `.sqlite` is byte-identical to what unpacking its own blob yields (the gate's second-write law), so it is written to `data/cache/sections/<logical>.sqlite` directly and compressed from there; the first server start after a compile is a cache hit.
8. **Idempotent recompile.** `manifest.toml`'s `built` (informational, outside the root) is preserved from the existing manifest when root and every blob hash are unchanged; a blob whose `<name>.<logical>.sqlite.zst` already exists with a matching SHA-256 in the existing manifest is reused (no recompression). An idle recompile leaves `git status` clean — the same property `graph.bin` already has. `meta.built` is REMOVED from the section file (it would move the blob hash on every compile); `meta` keeps `section_name, schema_version, logical_hash, compiler, canon_version, hash_width`.
9. **The root moves once more, here.** Recorded pins, AQC fixtures, exports, `version_root_regression`, the CLI transcript root (if any) are re-recorded by the DB-4a recorders; `atlas-query-contract` 0.2.0 → **0.3.0**, `atlas-graph-contract` 0.4.0 → **0.5.0** (changed blessed fixtures = MINOR under the 0.x rule). Edge ids and pids do NOT move (they do not depend on the dump).
10. **Gate 9's ceiling is re-derived, not loosened.** It now measures compression too; the ceiling is the first measured run × 2 rounded up to the next 30 s (DB-2b's own rule), recorded in `scripts/timing-gates.sh`, the test, and `BENCHMARKS.md`.
11. **`bibex verify` gets exit code 6 (`integrity_failed`)** — a new `CliError` class; the taxonomy's existing five do not describe "the data on disk disagrees with its manifest". CONTRACT.md's exit-code table grows one row (additive).

## File structure

- `graph-types/src/graph.rs` — MODIFY: `Graph.extra_tables`.
- `graph-types/src/sections.rs` — MODIFY: `extra_tables_of`, `logical_table_order` widened, `logical_dump_section` appends extras, `extra_line_body`, laws.
- `graph-types/Cargo.toml` — MODIFY: 0.2.1.
- `server/atlas-graph/Cargo.toml` — MODIFY: `zstd`.
- `server/atlas-graph/src/sqlite/extras.rs` — CREATE: `Col`, `TableSpec`, `ExtraTable`, `Extras`, `Sidecars`, `extras_for_artifact`, generic insert/read/encode, the projections and derived tables.
- `server/atlas-graph/src/sqlite/sidecars.rs` — CREATE: `fold_sidecars(&AtlasData, &SourcesDocument) -> Vec<ExtraTable>` (the nine folds).
- `server/atlas-graph/src/sqlite/ddl.rs` — MODIFY: the 29 tables' DDL + indexes, `create_tables`/`create_indexes` extended.
- `server/atlas-graph/src/sqlite/logical.rs` — MODIFY: reads extra tables.
- `server/atlas-graph/src/sqlite/blob.rs` — CREATE: zstd compress/decompress + SHA-256 of a file, `BLOB_CEILING`.
- `server/atlas-graph/src/sqlite/source.rs` — CREATE: `SectionSource`, `SectionLayout`, `CommittedZstdSource`.
- `server/atlas-graph/src/sqlite/writer.rs` — MODIFY: `write_sections(g, extras, compiler, layout)`, cache + blob + manifest, idempotence, ceiling.
- `server/atlas-graph/src/sqlite/snapshot.rs` — MODIFY: `open(manifest_path, source)`.
- `server/atlas-graph/src/sqlite/mod.rs` — MODIFY: modules.
- `server/atlas-graph/src/service.rs` — MODIFY: `assemble` attaches extras; `from_artifact` loads sidecars.
- `server/atlas-graph/src/bins/compile_graph.rs` — MODIFY: sections to `data/compiled`, cache dir, extras, admission through the source.
- `server/atlas-graph/tests/sqlite_laws.rs`, `tests/sqlite_real_data.rs` — MODIFY; `tests/extras_real_data.rs` — CREATE.
- `server/atlas-cli/src/commands/verify.rs` — CREATE; `main.rs`, `commands/mod.rs`, `commands/help.rs`, `error.rs`, `CONTRACT.md`, `tests/cli.rs` — MODIFY.
- `.gitattributes` — MODIFY (blobs binary, manifest LF).
- `data/compiled/manifest.toml`, `data/compiled/sections/*.zst` — CREATE (committed).
- `scripts/timing-gates.sh`, `server/BENCHMARKS.md`, contracts (VERSION/CHANGELOG/fixtures/pacts), exports, client constants — MODIFY (Task 7).

---

### Task 1: The extras seam in graph-types

**Files:**
- Modify: `graph-types/src/graph.rs` (the `Graph` struct — add one field beside `reading`)
- Modify: `graph-types/src/sections.rs:283-350` (`logical_table_order`, `logical_dump_section`, laws)
- Modify: `graph-types/Cargo.toml:3` (version)

**Interfaces:**
- Produces: `Graph.extra_tables: BTreeMap<&'static str, Vec<Vec<u8>>>`; `sections::extra_tables_of(Section) -> &'static [&'static str]`; `sections::extra_line_body(cols: Vec<(&str, Value)>) -> Vec<u8>`; `logical_table_order` now ends with the extra tables; `logical_dump_section` emits them.

- [ ] **Step 1: Write the failing laws** in `graph-types/src/sections.rs`'s `mod laws`:

```rust
    #[test]
    fn extra_tables_follow_the_graph_native_tables_and_move_the_root() {
        assert_eq!(extra_tables_of(Section::Kjv), &["verse", "red_letter_span"]);
        assert_eq!(extra_tables_of(Section::Concord), &["concord_unit"]);
        assert!(extra_tables_of(Section::Kretzmann).is_empty());
        assert_eq!(extra_tables_of(Section::Core).len(), 26);
        let order = logical_table_order(Section::Core);
        assert_eq!(order.last().copied(), Some("provenance_entry"));
        assert!(order.iter().position(|t| *t == "place").unwrap() > order.iter().position(|t| *t == "analogue").unwrap());
        // the same name never appears in two sections' extras
        let mut all: Vec<&str> = Section::SHIPPED.iter().flat_map(|s| extra_tables_of(*s).iter().copied()).collect();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n, "extra table names are unique across sections");

        let g = fixture();
        let base = version_root(&g);
        let mut g2 = fixture();
        g2.extra_tables.insert("verse", vec![extra_line_body(vec![("book", Value::Int(0)), ("chapter", Value::Int(1)), ("node_id", str_value("TextUnit:bible/0.1.1")), ("verse", Value::Int(1))])]);
        assert_ne!(version_root(&g2), base, "an extra row moves the root");
        let kjv = String::from_utf8(logical_dump_section(&g2, Section::Kjv)).unwrap();
        assert!(kjv.ends_with("reading_spine\t{\"corpus\":\"bible\",\"node_id\":\"TextUnit:bible/1.1.2\",\"ord\":1}\nverse\t{\"book\":0,\"chapter\":1,\"node_id\":\"TextUnit:bible/0.1.1\",\"verse\":1}\n"), "extras come AFTER the spine, keys in byte order: {kjv}");
        assert_eq!(logical_dump_section(&g2, Section::Core), logical_dump_section(&g, Section::Core), "a kjv extra does not touch core");
        let mut g3 = fixture();
        g3.extra_tables.insert("not_a_table", vec![b"{}".to_vec()]);
        assert_eq!(version_root(&g3), base, "a table no section lists is not in any dump");
    }
```

- [ ] **Step 2: Run it** — `cd graph-types && cargo test --features canon-ids extra_tables_follow` → FAIL: no `extra_tables_of`, no field `extra_tables`.

- [ ] **Step 3: Implement.** In `graph.rs`, on `Graph` (beside `pub reading`):

```rust
    /// DB-4b: the canonical row bodies of the section tables that are NOT
    /// derived from `nodes`/rows/`reading` here — node projections, the
    /// resolved chronology, the heading index, red-letter spans, the folded
    /// sidecars — keyed by table name (`sections::extra_tables_of`), rows in
    /// primary-key order, each body the canonical JSON `sections::
    /// extra_line_body` spells. Supplied by the compiler
    /// (`atlas_graph::sqlite::extras`), attached again at artifact load from
    /// the same files, so `version_root` covers them on both sides. Not
    /// serialized in `graph.bin`. Empty for a graph nobody attached to.
    pub extra_tables: std::collections::BTreeMap<&'static str, Vec<Vec<u8>>>,
```

In `sections.rs`, after `spine_corpus`:

```rust
/// DB-4b: the tables a section carries beyond `node`, its row families and
/// `reading_spine` (spec §5.3–5.6, amended by the DB-4b plan's judgment
/// calls 3–6). Their rows reach the dump through `Graph::extra_tables`.
pub fn extra_tables_of(section: Section) -> &'static [&'static str] {
    match section {
        Section::Core => &[
            "place", "era", "polity_era", "event_date", "heading_index",
            "canon_book", "canon_chapter_verses", "book_meta", "chronology_anchor", "book_narration_window",
            "landmark", "land_mask_region",
            "catechism_part", "catechism_item", "catechism_item_verse", "catechism_question", "catechism_question_verse",
            "place_history", "place_history_name", "place_history_blurb", "place_history_verse",
            "place_name_alias", "place_name_alias_verse",
            "source_category", "source_entry", "provenance_entry",
        ],
        Section::Kjv => &["verse", "red_letter_span"],
        Section::Concord => &["concord_unit"],
        Section::Kretzmann | Section::Lexicon => &[],
    }
}

/// The ONE spelling of an extra table's row body, both sides: an object of
/// `(column, value)` pairs, keys in byte order, no whitespace.
pub fn extra_line_body(cols: Vec<(&str, Value)>) -> Vec<u8> {
    serialize(&obj(cols))
}
```

`logical_table_order`: after the spine push, `v.extend(extra_tables_of(section));`. `logical_dump_section`: after the spine loop:

```rust
    for table in extra_tables_of(section) {
        if let Some(bodies) = g.extra_tables.get(table) {
            for body in bodies {
                line(&mut out, table, body);
            }
        }
    }
```

Update the doc comment of `logical_table_order` ("…then `reading_spine` where present, then `extra_tables_of`"). `Cargo.toml`: `version = "0.2.1" # DB-4b: additive -- Graph::extra_tables, sections::extra_tables_of/extra_line_body.`

- [ ] **Step 4: Run** `cd graph-types && cargo test --features canon-ids && cargo test` → all green in both states (the OFF state has no `canon-ids` consumers of the new code; both compile).

- [ ] **Step 5: Commit** — `git add graph-types && git commit -m "feat(graph-types): DB-4b -- Graph::extra_tables and sections::extra_tables_of: the non-graph section tables' canonical rows ride the graph into the logical dump and the root (0.2.1, additive)"`.

---

### Task 2: The extras framework, the projections and the derived tables

**Files:**
- Create: `server/atlas-graph/src/sqlite/extras.rs`
- Modify: `server/atlas-graph/src/sqlite/ddl.rs` (new DDL consts, `create_tables`/`create_indexes`), `sqlite/logical.rs` (extra tables read back), `sqlite/writer.rs` (`insert_extras`; `write_sections` takes `&Extras` — layout change is Task 4), `sqlite/mod.rs` (`pub mod extras;`)
- Test: `server/atlas-graph/tests/sqlite_laws.rs`

**Interfaces:**
- Consumes: Task 1's `Graph.extra_tables`, `extra_tables_of`, `extra_line_body`; `atlas_graph_types::chrono::{ResolvedPlacement, PlacementBasis, TimePoint}`; `crate::heading::{build_heading_index, HeadingEntry}`; `crate::kjv_adapter::decode_text_unit`, `crate::concord_adapter::decode_text_unit`; `atlas_core::refs::ScriptureRef::parse` (dot-ref → `BookId(u8)`, chapter, verse).
- Produces:

```rust
pub enum Col { Null, Int(i64), Real(f64), Text(String) }
pub struct TableSpec { pub name: &'static str, pub columns: &'static [&'static str], pub pk: &'static [&'static str] }
pub struct ExtraTable { pub spec: &'static TableSpec, pub rows: Vec<Vec<Col>> }   // rows: one Col per spec.columns, any order until `sorted()`
pub struct Extras { pub tables: Vec<ExtraTable> }
impl Extras {
    pub fn graph_derived(g: &Graph, resolved: &HashMap<String, ResolvedPlacement>, red_letter: &HashMap<String, Vec<(usize, usize)>>) -> Result<Extras, SqliteError>;
    pub fn attach(&self, g: &mut Graph);                       // g.extra_tables[name] = canonical bodies in pk order (replaces)
    pub fn table(&self, name: &str) -> Option<&ExtraTable>;
    pub fn extend(&mut self, more: Vec<ExtraTable>);
}
pub fn table_specs_of(section: Section) -> &'static [&'static TableSpec];   // same names, same order as extra_tables_of
pub fn spec_named(name: &str) -> Option<&'static TableSpec>;
pub fn insert_table(tx: &Transaction, t: &ExtraTable) -> Result<(), SqliteError>;
pub fn read_table(conn: &Connection, spec: &TableSpec) -> Result<Vec<Vec<Col>>, SqliteError>;   // SELECT columns ORDER BY pk
pub fn row_body(spec: &TableSpec, row: &[Col]) -> Result<Vec<u8>, SqliteError>;               // extra_line_body over (column, value)
pub fn pk_cmp(spec: &TableSpec, a: &[Col], b: &[Col]) -> Ordering;                              // SQLite BINARY order per pk column
```

- [ ] **Step 1: Write the failing laws** in `tests/sqlite_laws.rs` (add imports `use atlas_graph::sqlite::extras::{Col, Extras, TableSpec, table_specs_of, spec_named, read_table, row_body};`):

```rust
#[test]
fn every_extra_table_spec_matches_its_ddl_and_graph_types_lists_it() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    for s in Section::SHIPPED {
        create_tables(&conn, s).unwrap();
        create_indexes(&conn, s).unwrap();
        let names: Vec<&str> = table_specs_of(s).iter().map(|t| t.name).collect();
        assert_eq!(names, atlas_graph_types::sections::extra_tables_of(s), "{s:?}: specs and graph-types agree on names and order");
        for spec in table_specs_of(s) {
            let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", spec.name)).unwrap();
            let info: Vec<(String, i64)> = stmt.query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, i64>(5)?))).unwrap().map(|r| r.unwrap()).collect();
            let cols: Vec<&str> = info.iter().map(|(n, _)| n.as_str()).collect();
            assert_eq!(cols, spec.columns, "{}: DDL columns == spec.columns, in order", spec.name);
            let mut pk: Vec<(i64, &str)> = info.iter().filter(|(_, k)| *k > 0).map(|(n, k)| (*k, n.as_str())).collect();
            pk.sort();
            let pk: Vec<&str> = pk.into_iter().map(|(_, n)| n).collect();
            assert_eq!(pk, spec.pk, "{}: DDL primary key == spec.pk, in order", spec.name);
        }
        conn.execute_batch("PRAGMA writable_schema = OFF;").unwrap();
        // fresh connection per section: the DDL names collide across sections
        drop(stmt_guard(&conn));
    }
    assert!(spec_named("verse").is_some() && spec_named("nope").is_none());
}
```

(Replace the `stmt_guard` line with opening a new in-memory connection per section — `let conn = Connection::open_in_memory()` inside the loop. Use that.)

```rust
#[test]
fn the_graph_derived_extras_of_the_specimen_round_trip_and_agree_with_the_attached_dump() {
    let mut g = specimen_graph();
    // one dated event, one heading, one red-letter span for the specimen
    let mut resolved = std::collections::HashMap::new();
    resolved.insert("e1".to_string(), ResolvedPlacement {
        date: ResolvedDate { from: TimePoint { year: -1000, month: Some(3), day: None }, to: TimePoint { year: -999, month: None, day: None } },
        seq: SeqKey(0),
        basis: PlacementBasis::Traditional,
    });
    let mut red = std::collections::HashMap::new();
    red.insert("GEN.1.1".to_string(), vec![(0usize, 5usize), (10, 12)]);
    let extras = Extras::graph_derived(&g, &resolved, &red).unwrap();
    let verse = extras.table("verse").unwrap();
    assert!(verse.rows.iter().any(|r| r == &vec![Col::Text("TextUnit:bible/0.1.1".into()), Col::Int(0), Col::Int(1), Col::Int(1)]));
    let ed = extras.table("event_date").unwrap();
    assert_eq!(ed.rows[0], vec![Col::Text("e1".into()), Col::Int(-1000), Col::Int(-999), Col::Int(3), Col::Null, Col::Null, Col::Null, Col::Int(0), Col::Int(1)]);
    let rl = extras.table("red_letter_span").unwrap();
    assert_eq!(rl.rows, vec![vec![Col::Int(0), Col::Int(1), Col::Int(1), Col::Int(0), Col::Int(0), Col::Int(5)], vec![Col::Int(0), Col::Int(1), Col::Int(1), Col::Int(1), Col::Int(10), Col::Int(12)]]);
    let place = extras.table("place").unwrap();
    assert_eq!(place.spec.columns, &["node_id", "canonical", "lat", "lon"]);

    extras.attach(&mut g);
    let dir = tempdir();
    let (_m, written) = write_sections(&g, &extras, "test", &layout_under(&dir)).unwrap();
    for w in &written {
        let conn = open_read_only(&w.path).unwrap();
        for spec in table_specs_of(w.section) {
            let back = read_table(&conn, spec).unwrap();
            let expected: Vec<Vec<u8>> = g.extra_tables.get(spec.name).cloned().unwrap_or_default();
            let got: Vec<Vec<u8>> = back.iter().map(|r| row_body(spec, r).unwrap()).collect();
            assert_eq!(got, expected, "{}: SELECT … ORDER BY pk re-encodes to the attached bodies", spec.name);
        }
        assert_eq!(logical_hash(&logical_dump_of_db(&conn, w.section).unwrap()), w.logical);
    }
}
```

(`layout_under`/`tempdir` are the test file's existing helpers adjusted in Task 4; until then use the existing `write_sections(&g, &extras, "test", &dir)` shape this task introduces — the signature grows `extras` here and `layout` in Task 4.) The specimen has a Place, an Era, a Polity and bible/concord TextUnits — check `specimen_graph()` at `sqlite_laws.rs:233` and add a `Polity` node with two eras if none exists so `polity_era` is exercised.

- [ ] **Step 2: Run** → FAIL: module `extras` missing.

- [ ] **Step 3: Implement `extras.rs`.** Skeleton (fill every arm; nothing left to the reader):

```rust
//! DB-4b: the section tables that are not graph-native (spec §5.3–5.6):
//! one generic row shape, one INSERT, one `SELECT … ORDER BY pk`, one
//! canonical encoder (`sections::extra_line_body`), and the per-table
//! builders. `Graph::extra_tables` carries the encoded bodies so the
//! version root covers them; the writer inserts the typed rows.
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use atlas_graph_types::canon::ids::any_node_id_str;
use atlas_graph_types::canon::Value;
use atlas_graph_types::chrono::{PlacementBasis, ResolvedPlacement};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::sections::{extra_line_body, extra_tables_of, Section};
use rusqlite::{Connection, Transaction};
use super::SqliteError;

#[derive(Clone, Debug, PartialEq)]
pub enum Col { Null, Int(i64), Real(f64), Text(String) }

pub struct TableSpec { pub name: &'static str, pub columns: &'static [&'static str], pub pk: &'static [&'static str] }

pub struct ExtraTable { pub spec: &'static TableSpec, pub rows: Vec<Vec<Col>> }

#[derive(Default)]
pub struct Extras { pub tables: Vec<ExtraTable> }

pub static PLACE: TableSpec = TableSpec { name: "place", columns: &["node_id", "canonical", "lat", "lon"], pk: &["node_id"] };
pub static ERA: TableSpec = TableSpec { name: "era", columns: &["node_id", "label", "from_year", "to_year"], pk: &["node_id"] };
pub static POLITY_ERA: TableSpec = TableSpec { name: "polity_era", columns: &["node_id", "ord", "name", "from_year", "to_year"], pk: &["node_id", "ord"] };
pub static EVENT_DATE: TableSpec = TableSpec { name: "event_date", columns: &["event_id", "from_year", "to_year", "from_month", "from_day", "to_month", "to_day", "seq", "basis"], pk: &["event_id"] };
pub static HEADING_INDEX: TableSpec = TableSpec { name: "heading_index", columns: &["book", "chapter", "verse", "event_id", "title", "kind", "continuation"], pk: &["book", "chapter", "verse"] };
pub static VERSE: TableSpec = TableSpec { name: "verse", columns: &["node_id", "book", "chapter", "verse"], pk: &["node_id"] };
pub static RED_LETTER_SPAN: TableSpec = TableSpec { name: "red_letter_span", columns: &["book", "chapter", "verse", "ord", "start", "end_"], pk: &["book", "chapter", "verse", "ord"] };
pub static CONCORD_UNIT: TableSpec = TableSpec { name: "concord_unit", columns: &["node_id", "part", "article", "paragraph"], pk: &["node_id"] };
// sidecar specs are declared in sidecars.rs (Task 3) and listed here:
pub fn table_specs_of(section: Section) -> &'static [&'static TableSpec] {
    match section {
        Section::Core => &[&PLACE, &ERA, &POLITY_ERA, &EVENT_DATE, &HEADING_INDEX, /* Task 3 appends the 21 sidecar specs here, in extra_tables_of order */],
        Section::Kjv => &[&VERSE, &RED_LETTER_SPAN],
        Section::Concord => &[&CONCORD_UNIT],
        Section::Kretzmann | Section::Lexicon => &[],
    }
}
pub fn spec_named(name: &str) -> Option<&'static TableSpec> {
    Section::SHIPPED.iter().flat_map(|s| table_specs_of(*s).iter().copied()).find(|t| t.name == name)
}

fn col_value(c: &Col) -> Result<Value, SqliteError> {
    Ok(match c {
        Col::Null => Value::Null,
        Col::Int(i) => Value::Int(*i),
        Col::Real(f) => Value::float(*f)?,
        Col::Text(s) => Value::Str(s.clone()),
    })
}
pub fn row_body(spec: &TableSpec, row: &[Col]) -> Result<Vec<u8>, SqliteError> {
    if row.len() != spec.columns.len() { return Err(SqliteError(format!("{}: row has {} cols, spec {}", spec.name, row.len(), spec.columns.len()))); }
    let mut cols = Vec::with_capacity(row.len());
    for (name, c) in spec.columns.iter().zip(row) { cols.push((*name, col_value(c)?)); }
    Ok(extra_line_body(cols))
}
fn col_cmp(a: &Col, b: &Col) -> Ordering {
    // SQLite BINARY collation order within one column's type class: NULL < numeric < TEXT
    fn class(c: &Col) -> u8 { match c { Col::Null => 0, Col::Int(_) | Col::Real(_) => 1, Col::Text(_) => 2 } }
    match (a, b) {
        (Col::Int(x), Col::Int(y)) => x.cmp(y),
        (Col::Real(x), Col::Real(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (Col::Int(x), Col::Real(y)) => (*x as f64).partial_cmp(y).unwrap_or(Ordering::Equal),
        (Col::Real(x), Col::Int(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(Ordering::Equal),
        (Col::Text(x), Col::Text(y)) => x.as_bytes().cmp(y.as_bytes()),
        _ => class(a).cmp(&class(b)),
    }
}
pub fn pk_cmp(spec: &TableSpec, a: &[Col], b: &[Col]) -> Ordering {
    for k in spec.pk {
        let i = spec.columns.iter().position(|c| c == k).expect("pk column is a column");
        match col_cmp(&a[i], &b[i]) { Ordering::Equal => continue, o => return o }
    }
    Ordering::Equal
}
impl ExtraTable {
    pub fn sorted(mut self) -> Self { let spec = self.spec; self.rows.sort_by(|a, b| pk_cmp(spec, a, b)); self }
}
impl Extras {
    pub fn table(&self, name: &str) -> Option<&ExtraTable> { self.tables.iter().find(|t| t.spec.name == name) }
    pub fn extend(&mut self, more: Vec<ExtraTable>) { self.tables.extend(more.into_iter().map(ExtraTable::sorted)); }
    pub fn attach(&self, g: &mut Graph) {
        for t in &self.tables {
            let bodies: Vec<Vec<u8>> = t.rows.iter().map(|r| row_body(t.spec, r).expect("attach: every row encodes")).collect();
            g.extra_tables.insert(t.spec.name, bodies);
        }
    }
    pub fn graph_derived(g: &Graph, resolved: &HashMap<String, ResolvedPlacement>, red_letter: &HashMap<String, Vec<(usize, usize)>>) -> Result<Extras, SqliteError> {
        let (mut place, mut era, mut polity_era, mut verse, mut concord) = (vec![], vec![], vec![], vec![], vec![]);
        for n in g.nodes.values() {
            let id = any_node_id_str(&n.id);
            match &n.payload {
                NodePayload::Place { canonical, lat, lon, .. } => place.push(vec![Col::Text(id), Col::Text(canonical.clone()), Col::Real(*lat), Col::Real(*lon)]),
                NodePayload::Era { label, from_year, to_year } => era.push(vec![Col::Text(id), Col::Text(label.clone()), Col::Int(*from_year as i64), Col::Int(*to_year as i64)]),
                NodePayload::Polity { eras, .. } => for (i, e) in eras.iter().enumerate() {
                    polity_era.push(vec![Col::Text(id.clone()), Col::Int(i as i64), Col::Text(e.name.clone()), Col::Int(e.from_year as i64), Col::Int(e.to_year as i64)]);
                },
                NodePayload::TextUnit { .. } => {
                    if let Some((b, c, v)) = crate::kjv_adapter::decode_text_unit(&n.id) {
                        verse.push(vec![Col::Text(id), Col::Int(b as i64), Col::Int(c as i64), Col::Int(v as i64)]);
                    } else if let Some((p, a, par)) = crate::concord_adapter::decode_text_unit(&n.id) {
                        concord.push(vec![Col::Text(id), Col::Int(p as i64), Col::Int(a as i64), Col::Int(par as i64)]);
                    } else {
                        return Err(SqliteError(format!("TextUnit {id} is neither a bible nor a concord unit")));
                    }
                }
                _ => {}
            }
        }
        let event_date = resolved.iter().map(|(id, r)| vec![
            Col::Text(id.clone()), Col::Int(r.date.from.year as i64), Col::Int(r.date.to.year as i64),
            opt_u8(r.date.from.month), opt_u8(r.date.from.day), opt_u8(r.date.to.month), opt_u8(r.date.to.day),
            Col::Int(r.seq.0 as i64), Col::Int(match r.basis { PlacementBasis::Textual => 0, PlacementBasis::Traditional => 1 }),
        ]).collect();
        let heading = crate::heading::build_heading_index(g, resolved);
        let mut heading_rows = Vec::with_capacity(heading.len());
        for (sref, h) in &heading {
            let (b, c, v) = verse_triple(sref)?;
            heading_rows.push(vec![Col::Int(b), Col::Int(c), Col::Int(v), Col::Text(h.event_id.clone()), Col::Text(h.title.clone()), Col::Text(h.kind.clone()), Col::Int(h.continuation as i64)]);
        }
        let mut red = Vec::new();
        for (sref, spans) in red_letter {
            let (b, c, v) = verse_triple(sref)?;
            for (i, (s, e)) in spans.iter().enumerate() {
                red.push(vec![Col::Int(b), Col::Int(c), Col::Int(v), Col::Int(i as i64), Col::Int(*s as i64), Col::Int(*e as i64)]);
            }
        }
        let mut out = Extras::default();
        out.extend(vec![
            ExtraTable { spec: &PLACE, rows: place }, ExtraTable { spec: &ERA, rows: era }, ExtraTable { spec: &POLITY_ERA, rows: polity_era },
            ExtraTable { spec: &EVENT_DATE, rows: event_date }, ExtraTable { spec: &HEADING_INDEX, rows: heading_rows },
            ExtraTable { spec: &VERSE, rows: verse }, ExtraTable { spec: &RED_LETTER_SPAN, rows: red }, ExtraTable { spec: &CONCORD_UNIT, rows: concord },
        ]);
        Ok(out)
    }
}
fn opt_u8(v: Option<u8>) -> Col { v.map(|x| Col::Int(x as i64)).unwrap_or(Col::Null) }
/// `"GEN.1.1"` -> `(book index, chapter, verse)` through `atlas_core::refs::ScriptureRef::parse` (its `Verse` arm).
pub fn verse_triple(sref: &str) -> Result<(i64, i64, i64), SqliteError> {
    match atlas_core::refs::ScriptureRef::parse(sref) {
        Ok(atlas_core::refs::ScriptureRef::Verse { book, chapter, verse }) => Ok((book.0 as i64, chapter as i64, verse as i64)),
        other => Err(SqliteError(format!("{sref} is not a single-verse dot-ref: {other:?}"))),
    }
}
pub fn insert_table(tx: &Transaction, t: &ExtraTable) -> Result<(), SqliteError> {
    let sql = format!("INSERT INTO {} ({}) VALUES ({})", t.spec.name, t.spec.columns.join(", "), vec!["?"; t.spec.columns.len()].join(", "));
    let mut stmt = tx.prepare_cached(&sql)?;
    for row in &t.rows {
        let params: Vec<rusqlite::types::Value> = row.iter().map(|c| match c {
            Col::Null => rusqlite::types::Value::Null, Col::Int(i) => rusqlite::types::Value::Integer(*i),
            Col::Real(f) => rusqlite::types::Value::Real(*f), Col::Text(s) => rusqlite::types::Value::Text(s.clone()),
        }).collect();
        stmt.execute(rusqlite::params_from_iter(params))?;
    }
    Ok(())
}
pub fn read_table(conn: &Connection, spec: &TableSpec) -> Result<Vec<Vec<Col>>, SqliteError> {
    let sql = format!("SELECT {} FROM {} ORDER BY {}", spec.columns.join(", "), spec.name, spec.pk.join(", "));
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        let mut cols = Vec::with_capacity(spec.columns.len());
        for i in 0..spec.columns.len() {
            cols.push(match r.get_ref(i)? {
                rusqlite::types::ValueRef::Null => Col::Null,
                rusqlite::types::ValueRef::Integer(i) => Col::Int(i),
                rusqlite::types::ValueRef::Real(f) => Col::Real(f),
                rusqlite::types::ValueRef::Text(t) => Col::Text(String::from_utf8(t.to_vec()).map_err(|e| SqliteError(format!("{}: {e}", spec.name)))?),
                rusqlite::types::ValueRef::Blob(_) => return Err(SqliteError(format!("{}: BLOB in an extra table", spec.name))),
            });
        }
        out.push(cols);
    }
    Ok(out)
}
```

Check `ScriptureRef`'s verse variant name and fields at `server/atlas-core/src/refs.rs:80-95` and use them exactly (the parse code above shows `[_, ch, v]` producing a single-verse variant when `v` has no `-`). Verify `ScriptureRef` derives `Debug`; if not, format the error without `{other:?}`.

**DDL** (`ddl.rs`) — add, verbatim, and wire into `create_tables` (after the family tables) and `create_indexes`:

```sql
-- EXTRA_DDL_CORE_GRAPH (Task 2; Task 3 adds EXTRA_DDL_CORE_SIDECARS)
CREATE TABLE place (
  node_id TEXT PRIMARY KEY, canonical TEXT NOT NULL, lat REAL NOT NULL, lon REAL NOT NULL
) WITHOUT ROWID;
CREATE TABLE era (
  node_id TEXT PRIMARY KEY, label TEXT NOT NULL, from_year INTEGER NOT NULL, to_year INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE polity_era (
  node_id TEXT NOT NULL, ord INTEGER NOT NULL, name TEXT NOT NULL,
  from_year INTEGER NOT NULL, to_year INTEGER NOT NULL,
  PRIMARY KEY (node_id, ord)
) WITHOUT ROWID;
CREATE TABLE event_date (
  event_id  TEXT PRIMARY KEY,
  from_year INTEGER NOT NULL, to_year INTEGER NOT NULL,
  from_month INTEGER, from_day INTEGER, to_month INTEGER, to_day INTEGER,
  seq       INTEGER NOT NULL,
  basis     INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE heading_index (
  book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL,
  event_id TEXT NOT NULL, title TEXT NOT NULL, kind TEXT NOT NULL, continuation INTEGER NOT NULL,
  PRIMARY KEY (book, chapter, verse)
) WITHOUT ROWID;
-- EXTRA_INDEX_DDL_CORE_GRAPH
CREATE INDEX polity_era_by_span ON polity_era (from_year, to_year);
CREATE INDEX event_by_span ON event_date (from_year, to_year);
CREATE INDEX event_by_order ON event_date (seq);
CREATE INDEX heading_by_event ON heading_index (event_id);
-- EXTRA_DDL_KJV
CREATE TABLE verse (
  node_id TEXT PRIMARY KEY, book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL
) WITHOUT ROWID;
CREATE TABLE red_letter_span (
  book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL, ord INTEGER NOT NULL,
  start INTEGER NOT NULL, end_ INTEGER NOT NULL,
  PRIMARY KEY (book, chapter, verse, ord)
) WITHOUT ROWID;
-- EXTRA_INDEX_DDL_KJV
CREATE UNIQUE INDEX verse_by_ref ON verse (book, chapter, verse);
-- EXTRA_DDL_CONCORD
CREATE TABLE concord_unit (
  node_id TEXT PRIMARY KEY, part INTEGER NOT NULL, article INTEGER NOT NULL, paragraph INTEGER NOT NULL
) WITHOUT ROWID;
-- EXTRA_INDEX_DDL_CONCORD
CREATE UNIQUE INDEX concord_by_ref ON concord_unit (part, article, paragraph);
```

Add `pub fn extra_ddl(section: Section) -> &'static [&'static str]` and `extra_index_ddl(section)` returning the constant lists; `create_tables` runs each, `create_indexes` likewise. Extend the existing "no constraint machinery" unit test to scan them (it iterates DDL strings — add the new constants to its list). Update the module doc's scope sentence ("…DB-4 arrives" → "the node projections, `event_date`, `heading_index`, `red_letter_span` (Task 2) and the folded sidecars (Task 3) are `extra_ddl`").

**Writer** (`writer.rs`): `write_one(g, p, extras, …)` — after the spine insert, inside the same transaction:

```rust
        for spec in super::extras::table_specs_of(p.section) {
            if let Some(t) = extras.table(spec.name) {
                super::extras::insert_table(&tx, t)?;
            }
        }
```

and `pub fn write_sections(g: &Graph, extras: &Extras, compiler: &str, out_dir: &Path)` (layout in Task 4). `WrittenSection` gains `pub extra_row_count: usize`. Remove `("built", built.to_string())` from `insert_meta` (judgment call 8) and the `built` parameter of `write_one`; `now_rfc3339()` stays for the manifest.

**Logical read-back** (`logical.rs`): in the `match table` add, before the `family_table` fallback arm:

```rust
            extra if super::extras::spec_named(extra).is_some() => {
                let spec = super::extras::spec_named(extra).unwrap();
                for row in super::extras::read_table(conn, spec)? {
                    line(&mut out, extra, &super::extras::row_body(spec, &row)?);
                }
            }
```

(Rust match guards on a binding: write `t if spec_named(t).is_some() => …`.) Update the module doc's line-format paragraph with the extras clause.

Every existing call of `write_sections` (compile binary, `sqlite_laws.rs`, `sqlite_real_data.rs`) passes `&Extras::default()` for now — Task 6 attaches the real ones; the laws in this task build them for the specimen.

- [ ] **Step 4: Run** `cd server && cargo test -p atlas-graph --test sqlite_laws` → green (17 + 2). `cargo build -p atlas-graph --bins` compiles.

- [ ] **Step 5: Commit** — `git add server/atlas-graph && git commit -m "feat(graph): DB-4b -- extras: one row shape for the non-graph tables; place/era/polity_era/event_date/heading_index/verse/red_letter_span/concord_unit written, read back in pk order, and in the dump"`.

---

### Task 3: The folded sidecars

**Files:**
- Create: `server/atlas-graph/src/sqlite/sidecars.rs`
- Modify: `sqlite/extras.rs` (`table_specs_of(Core)` lists the 21 sidecar specs; `Sidecars`, `extras_for_artifact`), `sqlite/ddl.rs` (`EXTRA_DDL_CORE_SIDECARS` + indexes), `sqlite/mod.rs`
- Test: `server/atlas-graph/tests/extras_real_data.rs` (CREATE), `tests/sqlite_laws.rs` (spec/DDL law already covers the new tables once listed)

**Interfaces:**
- Consumes: `atlas_core::data::{AtlasData, CanonBook, BookMeta, ChronologyAnchor, BookNarrationWindow, Landmark, CatechismPart, CatechismItem, CatechismQuestion, PlaceHistory, PlaceNameEntry, PlaceBlurbEntry, PlaceDateClaim, PlaceNameAlias}` (`server/atlas-core/src/data.rs`, lines in the DB-4b survey: 11-21, 580-587, 637-664, 671-679, 617-624, 976-981, 916-946, 964-969, 728-737, 686-691, 701-706, 716-721, 760-764); `atlas_core::sources::{SourcesDocument, SourceCategory, SourceEntry, ProvenanceEntry}` (`sources.rs:21-25, 37-48, 56-66, 96-113`); `atlas_core::canon::BOOKS`; Task 2's `Col`, `TableSpec`, `ExtraTable`.
- Produces:

```rust
pub struct Sidecars { pub atlas: AtlasData, pub sources: SourcesDocument }
impl Sidecars {
    /// `None` when `<data_dir>/canon.json` is absent (a fixture dir); every other missing file is an error.
    pub fn load(data_dir: &Path) -> anyhow::Result<Option<Sidecars>>;
}
pub fn fold_sidecars(atlas: &AtlasData, sources: &SourcesDocument) -> Result<Vec<ExtraTable>, SqliteError>;
/// The whole composition every artifact reader uses: graph-derived + sidecars from `data_dir`
/// (red-letter-spans.json read here too). `Ok(extras)` with only the graph-derived tables when canon.json is absent.
pub fn extras_for_artifact(g: &Graph, resolved: &HashMap<String, ResolvedPlacement>, data_dir: &Path) -> anyhow::Result<Extras>;
```

- [ ] **Step 1: Write the failing real-data test** `tests/extras_real_data.rs`:

```rust
//! DB-4b: the nine folded sidecars over the REAL data/compiled files: every
//! loaded struct's every field lands in a table (lossless -- DB-5 deletes the
//! JSONs), row counts match the sources, and the fold is deterministic.
use std::path::Path;
use atlas_graph::sqlite::extras::{Col, Extras, table_specs_of};
use atlas_graph::sqlite::sidecars::{fold_sidecars, Sidecars};
use atlas_graph_types::sections::Section;

fn data_dir() -> std::path::PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled") }

#[test]
fn the_real_sidecars_fold_losslessly_into_twenty_one_tables() {
    let sc = Sidecars::load(&data_dir()).unwrap().expect("data/compiled has canon.json");
    let tables = fold_sidecars(&sc.atlas, &sc.sources).unwrap();
    let mut ex = Extras::default();
    ex.extend(tables);
    let names: Vec<&str> = ex.tables.iter().map(|t| t.spec.name).collect();
    let expected: Vec<&str> = table_specs_of(Section::Core).iter().map(|s| s.name).skip(5).collect();
    assert_eq!(names, expected, "the fold produces exactly the sidecar tables, in extra_tables_of order");
    let n = |t: &str| ex.table(t).unwrap().rows.len();
    assert_eq!(n("canon_book"), 66);
    assert_eq!(n("canon_chapter_verses"), sc.atlas.canon.books.iter().map(|b| b.chapters.len()).sum::<usize>());
    assert_eq!(n("book_meta"), sc.atlas.books_meta.len());
    assert_eq!(n("chronology_anchor"), sc.atlas.chronology_anchors.len());
    assert_eq!(n("book_narration_window"), sc.atlas.book_narration_windows.len());
    assert_eq!(n("landmark"), sc.atlas.landmarks.len());
    assert_eq!(n("land_mask_region"), sc.atlas.land_mask.len());
    assert_eq!(n("catechism_part"), sc.atlas.catechism.len());
    assert_eq!(n("catechism_item"), sc.atlas.catechism.iter().map(|p| p.items.len()).sum::<usize>());
    assert_eq!(n("catechism_item_verse"), sc.atlas.catechism.iter().flat_map(|p| &p.items).map(|i| i.verses.len()).sum::<usize>());
    assert_eq!(n("catechism_question"), sc.atlas.catechism.iter().flat_map(|p| &p.items).map(|i| i.questions.len()).sum::<usize>());
    assert_eq!(n("catechism_question_verse"), sc.atlas.catechism.iter().flat_map(|p| &p.items).flat_map(|i| &i.questions).map(|q| q.verses.len()).sum::<usize>());
    assert_eq!(n("place_history"), sc.atlas.place_history.len());
    assert_eq!(n("place_history_name"), sc.atlas.place_history.values().map(|h| h.names.len()).sum::<usize>());
    assert_eq!(n("place_history_blurb"), sc.atlas.place_history.values().map(|h| h.blurbs.len()).sum::<usize>());
    let claim_verses = |c: &Option<atlas_core::data::PlaceDateClaim>| c.as_ref().map(|c| c.verses.len()).unwrap_or(0);
    assert_eq!(n("place_history_verse"), sc.atlas.place_history.values().map(|h| h.names.iter().map(|x| x.verses.len()).sum::<usize>() + claim_verses(&h.established) + claim_verses(&h.destroyed)).sum::<usize>());
    assert_eq!(n("place_name_alias"), sc.atlas.place_name_aliases.values().flatten().map(|a| a.translations.len()).sum::<usize>());
    assert_eq!(n("place_name_alias_verse"), sc.atlas.place_name_aliases.values().flatten().map(|a| a.verses.len()).sum::<usize>());
    assert_eq!(n("source_category"), sc.sources.categories.len());
    assert_eq!(n("source_entry"), sc.sources.sources.len());
    assert_eq!(n("provenance_entry"), sc.sources.provenances.len());
    // testament is derived from BOOKS order (survey: no source field)
    let cb = ex.table("canon_book").unwrap();
    assert_eq!(cb.rows[0], vec![Col::Int(0), Col::Text("GEN".into()), Col::Text("Genesis".into()), Col::Text("OT".into()), Col::Int(50)]);
    assert_eq!(cb.rows[39][3], Col::Text("NT".into()));
    // lebo-hamath has two alias rows (judgment call 6)
    assert!(ex.table("place_name_alias").unwrap().rows.iter().any(|r| r[0] == Col::Text("lebo-hamath".into()) && r[1] == Col::Int(1)));
    // deterministic
    let again = fold_sidecars(&sc.atlas, &sc.sources).unwrap();
    let mut ex2 = Extras::default();
    ex2.extend(again);
    for (a, b) in ex.tables.iter().zip(&ex2.tables) { assert_eq!(a.rows, b.rows, "{}", a.spec.name); }
}

#[test]
fn a_fixture_directory_without_canon_json_yields_no_sidecars() {
    let dir = std::env::temp_dir().join(format!("db4b-nosidecars-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    assert!(Sidecars::load(&dir).unwrap().is_none());
}
```

If `lebo-hamath` is not the duplicated id in the committed data, find the id with two rows (`place_name_aliases.values().find(|v| v.len() > 1)`) and assert on that one; keep the assertion that SOME place has `alias_ord = 1`. Check `AtlasData` field visibility (`pub canon`, `pub books_meta`, …) — every field the test reads is `pub` per the survey; `land_mask: Vec<Vec<(f64,f64)>>`.

- [ ] **Step 2: Run** `cargo test -p atlas-graph --test extras_real_data` → FAIL: no `sidecars` module.

- [ ] **Step 3: Implement `sidecars.rs`.** Specs (columns ARE the DDL order; pk as written):

```rust
pub static CANON_BOOK: TableSpec = TableSpec { name: "canon_book", columns: &["ord", "code", "name", "testament", "chapters"], pk: &["ord"] };
pub static CANON_CHAPTER_VERSES: TableSpec = TableSpec { name: "canon_chapter_verses", columns: &["book_ord", "chapter", "verses"], pk: &["book_ord", "chapter"] };
pub static BOOK_META: TableSpec = TableSpec { name: "book_meta", columns: &["book", "author", "write_place", "write_from", "write_to"], pk: &["book"] };
pub static CHRONOLOGY_ANCHOR: TableSpec = TableSpec { name: "chronology_anchor", columns: &["id", "ord", "label", "year", "event_id", "era_boundary", "source", "note"], pk: &["id"] };
pub static BOOK_NARRATION_WINDOW: TableSpec = TableSpec { name: "book_narration_window", columns: &["book", "from_year", "to_year", "note"], pk: &["book"] };
pub static LANDMARK: TableSpec = TableSpec { name: "landmark", columns: &["ord", "name", "kind", "lat", "lon", "size"], pk: &["ord"] };
pub static LAND_MASK_REGION: TableSpec = TableSpec { name: "land_mask_region", columns: &["ord", "name", "ref_note", "rings_json"], pk: &["ord"] };
pub static CATECHISM_PART: TableSpec = TableSpec { name: "catechism_part", columns: &["id", "ord", "title"], pk: &["id"] };
pub static CATECHISM_ITEM: TableSpec = TableSpec { name: "catechism_item", columns: &["id", "part_id", "ord", "name", "text", "explanation_heading", "explanation", "where_written", "ref_note"], pk: &["id"] };
pub static CATECHISM_ITEM_VERSE: TableSpec = TableSpec { name: "catechism_item_verse", columns: &["item_id", "ord", "sref"], pk: &["item_id", "ord"] };
pub static CATECHISM_QUESTION: TableSpec = TableSpec { name: "catechism_question", columns: &["item_id", "ord", "title", "source"], pk: &["item_id", "ord"] };
pub static CATECHISM_QUESTION_VERSE: TableSpec = TableSpec { name: "catechism_question_verse", columns: &["item_id", "question_ord", "ord", "sref"], pk: &["item_id", "question_ord", "ord"] };
pub static PLACE_HISTORY: TableSpec = TableSpec { name: "place_history", columns: &["place_id", "est_from", "est_to", "est_note", "dest_from", "dest_to", "dest_note"], pk: &["place_id"] };
pub static PLACE_HISTORY_NAME: TableSpec = TableSpec { name: "place_history_name", columns: &["place_id", "ord", "name", "from_year", "to_year"], pk: &["place_id", "ord"] };
pub static PLACE_HISTORY_BLURB: TableSpec = TableSpec { name: "place_history_blurb", columns: &["place_id", "ord", "text", "from_year", "to_year", "breadth"], pk: &["place_id", "ord"] };
pub static PLACE_HISTORY_VERSE: TableSpec = TableSpec { name: "place_history_verse", columns: &["place_id", "owner_kind", "owner_ord", "ord", "sref"], pk: &["place_id", "owner_kind", "owner_ord", "ord"] };
pub static PLACE_NAME_ALIAS: TableSpec = TableSpec { name: "place_name_alias", columns: &["place_id", "alias_ord", "translation", "name"], pk: &["place_id", "alias_ord", "translation"] };
pub static PLACE_NAME_ALIAS_VERSE: TableSpec = TableSpec { name: "place_name_alias_verse", columns: &["place_id", "alias_ord", "ord", "sref"], pk: &["place_id", "alias_ord", "ord"] };
pub static SOURCE_CATEGORY: TableSpec = TableSpec { name: "source_category", columns: &["id", "ord", "label"], pk: &["id"] };
pub static SOURCE_ENTRY: TableSpec = TableSpec { name: "source_entry", columns: &["id", "ord", "category", "title", "what_it_is", "what_we_built", "license", "link", "licenses_row_key"], pk: &["id"] };
pub static PROVENANCE_ENTRY: TableSpec = TableSpec { name: "provenance_entry", columns: &["id", "ord", "source", "confidence", "locator"], pk: &["id"] };
pub static SIDECAR_SPECS: [&TableSpec; 21] = [&CANON_BOOK, &CANON_CHAPTER_VERSES, &BOOK_META, &CHRONOLOGY_ANCHOR, &BOOK_NARRATION_WINDOW, &LANDMARK, &LAND_MASK_REGION, &CATECHISM_PART, &CATECHISM_ITEM, &CATECHISM_ITEM_VERSE, &CATECHISM_QUESTION, &CATECHISM_QUESTION_VERSE, &PLACE_HISTORY, &PLACE_HISTORY_NAME, &PLACE_HISTORY_BLURB, &PLACE_HISTORY_VERSE, &PLACE_NAME_ALIAS, &PLACE_NAME_ALIAS_VERSE, &SOURCE_CATEGORY, &SOURCE_ENTRY, &PROVENANCE_ENTRY];
```

Builders — helpers `t(s: &str) -> Col`, `ot(o: &Option<String>) -> Col`, `oi(o: Option<i32>) -> Col`, `i(x: impl Into<i64>) -> Col`:

- `canon_book`: `books.iter().enumerate()` → `[i(ord), t(code), t(name), t(if ord < 39 {"OT"} else {"NT"}), i(chapters.len())]`; `canon_chapter_verses`: for each book ord, `chapters.iter().enumerate()` → `[i(ord), i(ch+1), i(verses)]`.
- `book_meta`: `[t(book), t(author), ot(write_place), oi(write_from), oi(write_to)]`.
- `chronology_anchor`: `enumerate()` → `[t(id), i(ord), t(label), i(year), ot(event_id), i(era_boundary as i64), t(source), ot(note)]`.
- `book_narration_window`: `[t(book), i(from_year), i(to_year), ot(note)]`.
- `landmark`: `enumerate()` → `[i(ord), t(name), t(kind), Real(lat), Real(lon), ot(size)]`.
- `land_mask_region`: `land_mask.iter().enumerate()` → `[i(ord), Null, Null, Text(rings_json)]` where `rings_json` = `String::from_utf8(extra_line_body(...))`? No — rings are an ARRAY: build `Value::Arr(ring.iter().map(|(lat,lon)| Value::Arr(vec![Value::float(*lat)?, Value::float(*lon)?])).collect())` and spell it with `atlas_graph_types::canon::json::serialize` (pub; if the path is not re-exported, add `pub use json::serialize` to `canon/mod.rs` — check `graph-types/src/canon/mod.rs` exports first; `sections.rs` imports `serialize` from `crate::canon`, so it is already reachable as `atlas_graph_types::canon::serialize`).
- `catechism_*`: parts `enumerate()` → part row `[t(id), i(ord), t(title)]`; items `enumerate()` → `[t(id), t(part.id), i(ord), t(name), ot(text), t(explanation_heading), t(explanation), ot(where_written), ot(ref_note)]`; item verses `[t(item.id), i(ord), t(sref)]`; questions `[t(item.id), i(qord), t(title), t(source)]`; question verses `[t(item.id), i(qord), i(ord), t(sref)]`.
- `place_history`: iterate `place_history` values sorted by id (a HashMap — sort first for a stable `ord` where one is assigned; PK order is applied by `sorted()` anyway) → `[t(id), oi(est.when.from_year), oi(est.when.to_year), ot(est.note), oi(dest…), …]` with `est`/`dest` = `established`/`destroyed` (`Option<PlaceDateClaim{when: TimeRange{from_year,to_year}, verses, note}>`); names `enumerate()` → `[t(id), i(ord), t(name), i(when.from_year), i(when.to_year)]`; blurbs → `[t(id), i(ord), t(text), i(from_year), i(to_year), t(breadth)]`; verses: for each name entry `(owner_kind 0, owner_ord = name ord, ord, sref)`, established `(1, 0, ord, sref)`, destroyed `(2, 0, ord, sref)`.
- `place_name_alias`: for `(id, aliases)` in `place_name_aliases`, `aliases.iter().enumerate()` → for each `(translation, name)` in `translations` (a HashMap: rows sorted by PK afterwards) `[t(id), i(alias_ord), t(translation), t(name)]`; verses `[t(id), i(alias_ord), i(ord), t(sref)]`.
- `source_category`: `enumerate()` → `[t(id), i(ord), t(label)]`; `source_entry` → `[t(id), i(ord), t(category), t(title), t(what_it_is), t(what_we_built), t(license), ot(link), t(licenses_row_key)]`; `provenance_entry` → `[t(id), i(ord), t(source), t(confidence), ot(locator)]`.

`Sidecars::load(data_dir)`: `if !data_dir.join("canon.json").is_file() { return Ok(None) }`; `let atlas = AtlasData::load(data_dir)?.finish();` (`finish()` is what the server calls; harmless here); `let sources: SourcesDocument = serde_json::from_str(&std::fs::read_to_string(data_dir.join("sources.json"))?)?;`.

`extras_for_artifact(g, resolved, data_dir)`: read `data_dir/red-letter-spans.json` through `crate::red_letter_spans::read_file` (`Option` → empty map), `Extras::graph_derived(g, resolved, &spans)`, then `if let Some(sc) = Sidecars::load(data_dir)? { ex.extend(fold_sidecars(&sc.atlas, &sc.sources)?) }`. Put it in `extras.rs` (it composes both modules).

`table_specs_of(Section::Core)`: append `SIDECAR_SPECS` (a `static CORE_SPECS: [&TableSpec; 26]` listing the five graph-derived then the 21 — the Task 2 law pins the names/order against graph-types).

**DDL** (`EXTRA_DDL_CORE_SIDECARS`, verbatim, spec §5.3 amended per judgment calls 3 and 6):

```sql
CREATE TABLE canon_book (
  ord INTEGER PRIMARY KEY, code TEXT NOT NULL, name TEXT NOT NULL, testament TEXT NOT NULL,
  chapters INTEGER NOT NULL
);
CREATE TABLE canon_chapter_verses (
  book_ord INTEGER NOT NULL, chapter INTEGER NOT NULL, verses INTEGER NOT NULL,
  PRIMARY KEY (book_ord, chapter)
) WITHOUT ROWID;
CREATE TABLE book_meta (
  book TEXT PRIMARY KEY, author TEXT NOT NULL, write_place TEXT, write_from INTEGER, write_to INTEGER
) WITHOUT ROWID;
CREATE TABLE chronology_anchor (
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, label TEXT NOT NULL, year INTEGER NOT NULL,
  event_id TEXT, era_boundary INTEGER NOT NULL, source TEXT NOT NULL, note TEXT
) WITHOUT ROWID;
CREATE TABLE book_narration_window (
  book TEXT PRIMARY KEY, from_year INTEGER NOT NULL, to_year INTEGER NOT NULL, note TEXT
) WITHOUT ROWID;
CREATE TABLE landmark (
  ord INTEGER PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL, lat REAL NOT NULL, lon REAL NOT NULL, size TEXT
);
CREATE TABLE land_mask_region (
  ord INTEGER PRIMARY KEY, name TEXT, ref_note TEXT, rings_json TEXT NOT NULL
);
CREATE TABLE catechism_part (
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, title TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE catechism_item (
  id TEXT PRIMARY KEY, part_id TEXT NOT NULL, ord INTEGER NOT NULL, name TEXT NOT NULL,
  text TEXT, explanation_heading TEXT NOT NULL, explanation TEXT NOT NULL,
  where_written TEXT, ref_note TEXT
) WITHOUT ROWID;
CREATE TABLE catechism_item_verse (
  item_id TEXT NOT NULL, ord INTEGER NOT NULL, sref TEXT NOT NULL,
  PRIMARY KEY (item_id, ord)
) WITHOUT ROWID;
CREATE TABLE catechism_question (
  item_id TEXT NOT NULL, ord INTEGER NOT NULL, title TEXT NOT NULL, source TEXT NOT NULL,
  PRIMARY KEY (item_id, ord)
) WITHOUT ROWID;
CREATE TABLE catechism_question_verse (
  item_id TEXT NOT NULL, question_ord INTEGER NOT NULL, ord INTEGER NOT NULL, sref TEXT NOT NULL,
  PRIMARY KEY (item_id, question_ord, ord)
) WITHOUT ROWID;
CREATE TABLE place_history (
  place_id TEXT PRIMARY KEY,
  est_from INTEGER, est_to INTEGER, est_note TEXT,
  dest_from INTEGER, dest_to INTEGER, dest_note TEXT
) WITHOUT ROWID;
CREATE TABLE place_history_name (
  place_id TEXT NOT NULL, ord INTEGER NOT NULL, name TEXT NOT NULL, from_year INTEGER NOT NULL, to_year INTEGER NOT NULL,
  PRIMARY KEY (place_id, ord)
) WITHOUT ROWID;
CREATE TABLE place_history_blurb (
  place_id TEXT NOT NULL, ord INTEGER NOT NULL, text TEXT NOT NULL, from_year INTEGER NOT NULL, to_year INTEGER NOT NULL, breadth TEXT NOT NULL,
  PRIMARY KEY (place_id, ord)
) WITHOUT ROWID;
CREATE TABLE place_history_verse (
  place_id TEXT NOT NULL, owner_kind INTEGER NOT NULL, owner_ord INTEGER NOT NULL, ord INTEGER NOT NULL, sref TEXT NOT NULL,
  PRIMARY KEY (place_id, owner_kind, owner_ord, ord)
) WITHOUT ROWID;
CREATE TABLE place_name_alias (
  place_id TEXT NOT NULL, alias_ord INTEGER NOT NULL, translation TEXT NOT NULL, name TEXT NOT NULL,
  PRIMARY KEY (place_id, alias_ord, translation)
) WITHOUT ROWID;
CREATE TABLE place_name_alias_verse (
  place_id TEXT NOT NULL, alias_ord INTEGER NOT NULL, ord INTEGER NOT NULL, sref TEXT NOT NULL,
  PRIMARY KEY (place_id, alias_ord, ord)
) WITHOUT ROWID;
CREATE TABLE source_category (
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, label TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE source_entry (
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, category TEXT NOT NULL, title TEXT NOT NULL,
  what_it_is TEXT NOT NULL, what_we_built TEXT NOT NULL, license TEXT NOT NULL, link TEXT,
  licenses_row_key TEXT NOT NULL
) WITHOUT ROWID;
CREATE TABLE provenance_entry (
  id TEXT PRIMARY KEY, ord INTEGER NOT NULL, source TEXT NOT NULL, confidence TEXT NOT NULL, locator TEXT
) WITHOUT ROWID;
-- EXTRA_INDEX_DDL_CORE_SIDECARS
CREATE INDEX catechism_item_by_part ON catechism_item (part_id, ord);
CREATE INDEX catechism_item_verse_by_sref ON catechism_item_verse (sref);
CREATE INDEX catechism_question_verse_by_sref ON catechism_question_verse (sref);
CREATE INDEX provenance_by_source ON provenance_entry (source);
```

(The three `_by_sref` / `_by_part` indexes are the write-time materialisation of `AtlasData::finish()`'s `verse_to_catechism` join — survey item 7; DB-4c's `/api/catechism/{sref}` seeks them.)

**`PRAGMA table_info` pk order note:** for `INTEGER PRIMARY KEY` tables the pk column reports `pk = 1` — the law handles it.

- [ ] **Step 4: Run** `cargo test -p atlas-graph --test extras_real_data --test sqlite_laws` → green.

- [ ] **Step 5: Commit** — `git add server/atlas-graph && git commit -m "feat(graph): DB-4b -- the nine sidecars folded (21 tables, lossless: anchors/provenance keep every field, aliases keep alias_ord, land-mask rings honestly unnamed); Sidecars::load and extras_for_artifact"`.

---

### Task 4: zstd blobs, the section source and the cache

**Files:**
- Modify: `server/atlas-graph/Cargo.toml` (`zstd = { version = "0.13", features = ["zstdmt"] }` — use the newest 0.x `cargo search zstd` reports that has the `zstdmt` feature; the survey saw 0.14.0)
- Create: `server/atlas-graph/src/sqlite/blob.rs`, `server/atlas-graph/src/sqlite/source.rs`
- Modify: `sqlite/writer.rs`, `sqlite/snapshot.rs`, `sqlite/mod.rs`, `.gitattributes`
- Test: `server/atlas-graph/tests/sqlite_laws.rs`

**Interfaces:**
- Produces:

```rust
// blob.rs
pub const BLOB_CEILING: u64 = 104_857_600;
pub const ZSTD_LEVEL: i32 = 19;
pub fn sha256_hex_of_file(path: &Path) -> Result<String, SqliteError>;                       // 64 lowercase hex
pub fn compress_file(src: &Path, dst: &Path) -> Result<(String, u64), SqliteError>;          // writes dst.tmp, renames; returns (sha256 hex of dst, dst bytes)
pub fn decompress_verified(blob: &Path, expected_sha256: &str, dst: &Path) -> Result<u64, SqliteError>; // hashes the blob first; mismatch -> Err naming both, nothing written; writes dst.tmp then renames; returns dst bytes
// source.rs
pub type SectionError = SqliteError;
pub trait SectionSource { fn resolve(&self, entry: &ManifestSection) -> Result<PathBuf, SectionError>; }
#[derive(Clone, Debug)]
pub struct SectionLayout { pub compiled_dir: PathBuf, pub cache_dir: PathBuf }
impl SectionLayout {
    pub fn under(data_dir: &Path) -> SectionLayout;   // compiled_dir = data_dir; cache_dir = data_dir/../cache/sections
    pub fn manifest_path(&self) -> PathBuf;           // compiled_dir/manifest.toml
    pub fn blob_path(&self, name: &str, logical: &str) -> PathBuf;   // compiled_dir/sections/<name>.<logical>.sqlite.zst
    pub fn cache_path(&self, logical: &str) -> PathBuf;              // cache_dir/<logical>.sqlite
}
pub struct CommittedZstdSource { pub layout: SectionLayout }
impl SectionSource for CommittedZstdSource { … }   // cache hit -> path; else blob must exist (Err "section <name> (<logical>) has no blob at <path>") -> decompress_verified into cache_path -> path
// writer.rs
pub fn write_sections(g: &Graph, extras: &Extras, compiler: &str, layout: &SectionLayout) -> Result<(Manifest, Vec<WrittenSection>), SqliteError>;
// WrittenSection: path = the CACHE file, blob_path, logical, blob (sha256 of the .zst), bytes (compressed), uncompressed_bytes, reused_blob: bool, …
// snapshot.rs
impl SqliteSnapshot { pub fn open(manifest_path: &Path, source: &dyn SectionSource) -> Result<SqliteSnapshot, SqliteError>; }
```

- [ ] **Step 1: Write the failing laws** in `sqlite_laws.rs` (rewrite the file's `write_sections` calls to the layout form; add a helper `fn layout_under(dir: &Path) -> SectionLayout { SectionLayout { compiled_dir: dir.join("compiled"), cache_dir: dir.join("cache/sections") } }` and `fn open_written(layout) -> SqliteSnapshot { SqliteSnapshot::open(&layout.manifest_path(), &CommittedZstdSource { layout: layout.clone() }).unwrap() }`):

```rust
#[test]
fn the_writer_lands_cache_files_blobs_and_a_manifest_and_the_source_resolves_by_hash() {
    let g = specimen_graph();
    let dir = tempdir();
    let layout = layout_under(&dir);
    let (m, written) = write_sections(&g, &Extras::default(), "test", &layout).unwrap();
    assert_eq!(written.len(), 4);
    for w in &written {
        assert_eq!(w.path, layout.cache_path(&w.logical));
        assert_eq!(w.blob_path, layout.blob_path(w.section.name(), &w.logical));
        assert!(w.blob_path.is_file() && w.path.is_file());
        assert_eq!(sha256_hex_of_file(&w.blob_path).unwrap(), w.blob);
        assert_eq!(std::fs::metadata(&w.blob_path).unwrap().len(), w.bytes);
        assert!(w.bytes < w.uncompressed_bytes, "{}: zstd shrinks a sqlite file", w.section.name());
        assert!(!w.reused_blob);
        let ms = m.sections.iter().find(|s| s.name == w.section.name()).unwrap();
        assert_eq!((ms.blob.as_str(), ms.bytes), (w.blob.as_str(), w.bytes));
    }
    // a cold cache: delete it, resolve through the source, get a byte-identical file back
    let core_cache = layout.cache_path(&written[0].logical);
    let before = std::fs::read(&core_cache).unwrap();
    std::fs::remove_file(&core_cache).unwrap();
    let src = CommittedZstdSource { layout: layout.clone() };
    let resolved = src.resolve(&m.sections[0]).unwrap();
    assert_eq!(resolved, core_cache);
    assert_eq!(std::fs::read(&resolved).unwrap(), before, "unpacking the blob reproduces the written file byte for byte");
    // a tampered blob is refused, both hashes named, and nothing lands in the cache
    std::fs::remove_file(&core_cache).unwrap();
    let mut bytes = std::fs::read(&written[0].blob_path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x55;
    std::fs::write(&written[0].blob_path, &bytes).unwrap();
    let err = src.resolve(&m.sections[0]).unwrap_err().to_string();
    assert!(err.contains(&m.sections[0].blob) && err.contains("transport hash"), "{err}");
    assert!(!core_cache.exists() && !layout.cache_dir.join(format!("{}.sqlite.tmp", written[0].logical)).exists());
    // a missing blob for an optional section is recorded absent by the snapshot; a required one refuses by name + logical
    std::fs::write(&written[0].blob_path, std::fs::read(&written[0].blob_path).map(|_| Vec::<u8>::new()).unwrap()).ok(); // restore below via re-write
}
```

Split the last two checks into their own test (`a_required_blob_missing_refuses_and_an_optional_one_is_absent`): write the specimen, delete the concord blob AND its cache file → `open` succeeds and `present()` lacks `Concord`; delete the kjv blob and cache → `open` errors with a message containing `"kjv"` and the kjv logical hash. Fix the tampered-blob restoration by rewriting the sections (a second `write_sections` call) instead of the placeholder line above.

```rust
#[test]
fn a_recompile_is_idempotent_and_reuses_unchanged_blobs() {
    let g = specimen_graph();
    let dir = tempdir();
    let layout = layout_under(&dir);
    let (m1, w1) = write_sections(&g, &Extras::default(), "test", &layout).unwrap();
    let text1 = std::fs::read_to_string(layout.manifest_path()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let (m2, w2) = write_sections(&g, &Extras::default(), "test", &layout).unwrap();
    assert_eq!(m1, m2, "root, blobs, bytes AND built are unchanged");
    assert_eq!(text1, std::fs::read_to_string(layout.manifest_path()).unwrap());
    assert!(w2.iter().all(|w| w.reused_blob) && w1.iter().all(|w| !w.reused_blob));
    let mut g2 = specimen_graph();
    g2.analogue[0].provenance = "moved".into();
    let (m3, w3) = write_sections(&g2, &Extras::default(), "test", &layout).unwrap();
    assert_ne!(m3.root, m2.root);
    assert!(!w3[0].reused_blob && w3[1].reused_blob, "core changed and was recompressed; kjv did not and was reused");
    let stale: Vec<_> = std::fs::read_dir(layout.compiled_dir.join("sections")).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).filter(|n| n.starts_with("core.")).collect();
    assert_eq!(stale.len(), 1, "the stale core blob was deleted: {stale:?}");
}

#[test]
fn a_blob_over_the_ceiling_is_refused() {
    // exercised through the constant, not a 100 MiB file: the writer compares `bytes > BLOB_CEILING`
    assert_eq!(atlas_graph::sqlite::blob::BLOB_CEILING, 104_857_600);
    assert_eq!(atlas_graph::sqlite::blob::ZSTD_LEVEL, 19);
}
```

Also update the existing tests: `the_writer_produces_four_files_named_by_logical_hash_and_a_manifest_in_order` (names now `<name>.<logical>.sqlite.zst` under `compiled/sections`, cache `<logical>.sqlite`), `a_changed_row_changes_the_logical_hash_and_a_changed_timestamp_does_not` (the timestamp half now asserts the BLOB is unchanged too — `meta.built` is gone), every `SqliteSnapshot::open(&dir.join("manifest.toml"))` → `open_written(&layout)`, `the_sqlite_snapshots_version_is_the_manifest_root…`, `mem_store_stamps_the_same_root…` (unchanged semantics).

- [ ] **Step 2: Run** → FAIL to compile (no `blob`, `source`, new signatures).

- [ ] **Step 3: Implement.**

`blob.rs`:

```rust
use std::io::{Read, Write};
pub const BLOB_CEILING: u64 = 104_857_600;
pub const ZSTD_LEVEL: i32 = 19;
fn hex(b: &[u8]) -> String { b.iter().map(|x| format!("{x:02x}")).collect() }
pub fn sha256_hex_of_file(path: &Path) -> Result<String, SqliteError> {
    Ok(hex(&atlas_graph_types::sha256::sha256(&std::fs::read(path)?)))
}
fn threads() -> u32 { std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1).min(4) }
pub fn compress_file(src: &Path, dst: &Path) -> Result<(String, u64), SqliteError> {
    if let Some(parent) = dst.parent() { std::fs::create_dir_all(parent)?; }
    let tmp = dst.with_extension("zst.tmp");
    {
        let mut input = std::io::BufReader::new(std::fs::File::open(src)?);
        let out = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
        let mut enc = zstd::stream::Encoder::new(out, ZSTD_LEVEL).map_err(|e| SqliteError(format!("zstd: {e}")))?;
        enc.multithread(threads()).map_err(|e| SqliteError(format!("zstd: {e}")))?;
        std::io::copy(&mut input, &mut enc)?;
        enc.finish().map_err(|e| SqliteError(format!("zstd finish: {e}")))?.flush()?;
    }
    let _ = std::fs::remove_file(dst);
    std::fs::rename(&tmp, dst)?;
    let bytes = std::fs::metadata(dst)?.len();
    Ok((sha256_hex_of_file(dst)?, bytes))
}
pub fn decompress_verified(blob: &Path, expected_sha256: &str, dst: &Path) -> Result<u64, SqliteError> {
    let compressed = std::fs::read(blob)?;
    let actual = hex(&atlas_graph_types::sha256::sha256(&compressed));
    if actual != expected_sha256 {
        return Err(SqliteError(format!("transport hash mismatch for {}: manifest {expected_sha256}, file {actual}", blob.display())));
    }
    if let Some(parent) = dst.parent() { std::fs::create_dir_all(parent)?; }
    let tmp = dst.with_extension("sqlite.tmp");
    let result = (|| -> Result<(), SqliteError> {
        let mut dec = zstd::stream::Decoder::new(&compressed[..]).map_err(|e| SqliteError(format!("zstd: {e}")))?;
        let mut out = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
        std::io::copy(&mut dec, &mut out)?;
        out.flush()?;
        Ok(())
    })();
    if let Err(e) = result { let _ = std::fs::remove_file(&tmp); return Err(e); }
    let _ = std::fs::remove_file(dst);
    std::fs::rename(&tmp, dst)?;
    Ok(std::fs::metadata(dst)?.len())
}
```

(Level 19 with `multithread` needs the `zstdmt` feature; if the crate version chosen lacks it, drop the `multithread` line, keep single-threaded level 19, and record the measured compile cost in Task 7's BENCHMARKS entry.)

`source.rs`: as the interface block; `resolve`: `let cache = self.layout.cache_path(&entry.logical); if cache.is_file() { return Ok(cache) } let blob = self.layout.blob_path(&entry.name, &entry.logical); if !blob.is_file() { return Err(SqliteError(format!("section {} ({}) has no blob at {}", entry.name, entry.logical, blob.display()))) } decompress_verified(&blob, &entry.blob, &cache)?; Ok(cache)`.

`writer.rs` `write_one(g, p, extras, compiler, layout)`: compute `logical`; `let cache = layout.cache_path(&logical); let tmp = layout.cache_dir.join(format!("{}.build.tmp", name));` write the sqlite into `tmp` exactly as today (nodes, rows, edges, spine, extras, indexes, meta without `built`, VACUUM); `uncompressed_bytes`; rename `tmp` → `cache` (replace). Blob: `let blob_path = layout.blob_path(name, &logical); let (blob, bytes, reused) = match previous.and_then(|m| m.sections.iter().find(|s| s.name == name && s.logical == logical)) { Some(prev) if blob_path.is_file() && sha256_hex_of_file(&blob_path)? == prev.blob => (prev.blob.clone(), prev.bytes, true), _ => { let (h, b) = compress_file(&cache, &blob_path)?; (h, b, false) } };` `if bytes > BLOB_CEILING { return Err(SqliteError(format!("section {name} compressed to {bytes} bytes, over the {BLOB_CEILING}-byte ceiling (spec 2.3)"))) }`. `previous: Option<&Manifest>` = `read_manifest(&layout.manifest_path()).ok()` computed once in `write_sections` (a manifest that fails its own root check is treated as absent). Stale blobs: after all sections are written, delete every `compiled_dir/sections/<name>.*.sqlite.zst` whose `<logical>` is not the one just written (keep the cache: it is content-addressed; no deletion). Manifest: `built = match &previous { Some(prev) if prev.root == root && prev.sections.iter().map(|s| (&s.name, &s.blob)).eq(sections.iter().map(|s| (&s.name, &s.blob))) => prev.built.clone(), _ => now_rfc3339() }`. Write `manifest.toml` only if its text differs from the existing file (keeps mtime stable too).

`snapshot.rs` `open(manifest_path, source)`: replace the `dir.join(...)`/`is_file` block with `match source.resolve(ms) { Ok(path) => present.push((section, path)), Err(e) if ms.required => return Err(SqliteError(format!("required section {} ({}) unavailable: {e}", ms.name, ms.logical))), Err(_) => {} }` — an optional section whose blob is MISSING is absent; an optional section whose blob is PRESENT but corrupt must still be loud: distinguish by checking `source` errors that name a transport mismatch — simplest: `CommittedZstdSource::resolve` returns a typed enum `Resolve::{Path(PathBuf), Missing}` internally… keep the trait as spec'd (`Result<PathBuf, SectionError>`) and add `pub fn is_missing(&SectionError) -> bool` in `source.rs` that tests for the `"has no blob at"` prefix; `open` refuses on any error that is not missing, for required OR optional. Pin that with a law: tamper the concord blob (optional), delete its cache → `open` errs.

`.gitattributes` append:

```
# DB-4b: committed section blobs are binary; the manifest is LF like the pacts (its root is over parsed values, the blob hashes are over the .zst bytes).
data/compiled/sections/*.sqlite.zst binary
data/compiled/manifest.toml text eol=lf
```

`sqlite/mod.rs`: `pub mod blob; pub mod source;`.

Update callers to compile: `bins/compile_graph.rs` (Task 6 does the real wiring; for now `SectionLayout::under(&sections_out)` and `SqliteSnapshot::open(&layout.manifest_path(), &CommittedZstdSource{layout})`), `tests/sqlite_real_data.rs` (same shape, `Extras::default()`).

- [ ] **Step 4: Run** `cargo test -p atlas-graph --test sqlite_laws` → green; `cargo build -p atlas-graph --bins`.

- [ ] **Step 5: Commit** — `git add .gitattributes server/atlas-graph server/Cargo.lock && git commit -m "feat(graph): DB-4b -- zstd-19 blobs under data/compiled/sections, the unpack cache is the writer's output, CommittedZstdSource verifies the transport hash before unpacking, idempotent recompile (built preserved, unchanged blobs reused, stale blobs removed), 100 MiB ceiling refused"`.

---

### Task 5: `bibex verify`

**Files:**
- Create: `server/atlas-cli/src/commands/verify.rs`
- Modify: `server/atlas-cli/src/main.rs:110-150, 169-214` (dispatch, both forms), `commands/mod.rs`, `commands/help.rs` (one line in the command list), `error.rs` (`IntegrityFailed`, code `"integrity_failed"`, exit 6), `server/atlas-cli/CONTRACT.md` (command vocabulary; exit-code table row 6)
- Test: `server/atlas-cli/tests/cli.rs`

**Interfaces:**
- Consumes: `atlas_graph::sqlite::{manifest::read_manifest, source::{SectionLayout, CommittedZstdSource, SectionSource, is_missing}, blob::sha256_hex_of_file, logical::{logical_dump_of_db, logical_hash}, open_read_only}`; `atlas_graph::sections::Section`.
- Produces: `commands::verify::run(data_dir: &Path, section: Option<&str>) -> Result<String, CliError>` and `run_json(...) -> Result<serde_json::Value, CliError>`; `CliError::IntegrityFailed { what, why, do_ }`.

Report shape (plain), one line per manifest section then the root:

```
core       logical 819b7d00…  OK   transport OK   37,949,440 -> 11,510,415 bytes
kjv        logical 47cec020…  OK   transport OK   246,849,536 -> 61,383,932 bytes
concord    logical e1902cf2…  OK   transport OK   6,815,744 -> 1,626,620 bytes
kretzmann  logical cfebcd66…  OK   transport OK   61,587,456 -> 14,131,688 bytes
root e5d656e22cff0be8ac3f5a711d5c20fe OK (recomputed from 4 section lines)
```

(full 32-hex logical in the real output; the ellipsis here is prose). Per section: `transport` = `MISSING` (blob absent; `absent (optional)` when `required = false` and the cache is absent too — not a failure) | `MISMATCH manifest <blob> file <actual>` | `OK`; `logical` = resolve through the source (unpacks on a miss) → `open_read_only` → `PRAGMA user_version` must equal `schema_version` (else `schema MISMATCH file <v>`) → `logical_hash(logical_dump_of_db(conn, section))` equals `logical` → `OK` | `MISMATCH file <hex>`. Any `MISMATCH`, any required `MISSING`, an unreadable manifest → `Err(CliError::IntegrityFailed { what: "<n> of <m> checks failed", why: "<the failing lines joined by '; '>", do_: "recompile (cargo run -p atlas-graph --bin atlas-graph-compile) or restore data/compiled from git; a tampered or truncated blob must not be served" })`. `--section <name>` limits the per-section checks to that section (unknown name → `BadUsage`); the root line always prints. `--json`: `{"root": {"manifest": …, "recomputed": …, "ok": bool}, "sections": [{"name","required","logical","blob","bytes","transport":"ok|missing|mismatch|absent","logical_check":"ok|mismatch|skipped","schema_version":14, "uncompressed_bytes": n|null}]}`; on failure the JSON error envelope the CLI already uses (`error.rs`'s `to_json`).

`read_manifest` failing (root mismatch) is itself `IntegrityFailed` (why = the manifest error text); a missing manifest is `DataLoadFailed` (nothing to verify — the data dir predates DB-4b).

- [ ] **Step 1: Write the failing tests** in `tests/cli.rs` (use the file's `run`/`run_json`/`stdout`/`run_with_data_dir` helpers; the real committed `data/compiled` is the default data dir for these tests):

```rust
// DB-4b: `bibex verify` (spec 3.5).
#[test]
fn verify_passes_on_the_committed_sections_and_names_every_section_and_the_root() {
    let o = run(&["verify"]);
    assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));
    let text = stdout(&o);
    for name in ["core", "kjv", "concord", "kretzmann"] {
        let line = text.lines().find(|l| l.starts_with(name)).unwrap_or_else(|| panic!("no line for {name}: {text}"));
        assert!(line.contains("logical") && line.contains(" OK ") && line.contains("transport OK"), "{line}");
    }
    assert!(text.lines().last().unwrap().starts_with("root ") && text.trim_end().ends_with("section lines)"), "{text}");
    let (o, v) = run_json(&["verify"]);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(v["root"]["ok"], true);
    assert_eq!(v["sections"].as_array().unwrap().len(), 4);
    assert!(v["sections"].as_array().unwrap().iter().all(|s| s["transport"] == "ok" && s["logical_check"] == "ok"));
    let o = run(&["verify", "--section", "concord"]);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(stdout(&o).lines().count(), 2, "one section line + the root line");
    let o = run(&["verify", "--section", "nope"]);
    assert_eq!(o.status.code(), Some(4));
}

#[test]
fn verify_fails_with_exit_6_on_a_tampered_blob_and_names_both_hashes() {
    // a private copy: manifest + blobs, no cache, so the tampered blob must be unpacked and refused
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled");
    let root = std::env::temp_dir().join(format!("bibex-verify-{}", std::process::id()));
    let data = root.join("compiled");
    std::fs::create_dir_all(data.join("sections")).unwrap();
    std::fs::copy(src.join("manifest.toml"), data.join("manifest.toml")).unwrap();
    for e in std::fs::read_dir(src.join("sections")).unwrap() {
        let e = e.unwrap();
        std::fs::copy(e.path(), data.join("sections").join(e.file_name())).unwrap();
    }
    let concord = std::fs::read_dir(data.join("sections")).unwrap().map(|e| e.unwrap().path()).find(|p| p.file_name().unwrap().to_string_lossy().starts_with("concord.")).unwrap();
    let mut bytes = std::fs::read(&concord).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0xFF;
    std::fs::write(&concord, &bytes).unwrap();
    let o = run_with_data_dir_at(&data, &["verify"]);
    assert_eq!(o.status.code(), Some(6), "{}", String::from_utf8_lossy(&o.stderr));
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("integrity_failed") && err.contains("concord") && err.contains("transport"), "{err}");
    let (o, v) = run_json_with_data_dir_at(&data, &["verify"]);
    assert_eq!(o.status.code(), Some(6));
    assert_eq!(v["error"]["code"], "integrity_failed");
    // the other three still verify, and the tampered section is the only failure named
    let o = run_with_data_dir_at(&data, &["verify", "--section", "core"]);
    assert_eq!(o.status.code(), Some(0));
    std::fs::remove_dir_all(&root).ok();
}
```

Add `run_with_data_dir_at(dir: &Path, args)` / `run_json_with_data_dir_at` helpers beside the existing `run_with_data_dir` (which points at the committed dir) — same `Command` construction with `--data-dir <dir>`. Check the JSON error envelope's field path in `error.rs` (`to_json`) and use its exact keys.

Also extend `help_command_matches_bare_invocation`'s neighbour test that lists commands (search `cli.rs` for `"kinds"` in a help assertion) so `verify` appears in the help text, and `json_help_is_bad_usage` stays.

- [ ] **Step 2: Run** `cargo test -p atlas-cli --test cli verify_` → FAIL (exit 4, unrecognized subcommand).

- [ ] **Step 3: Implement.** `error.rs`: add `IntegrityFailed { what: String, why: String, do_: String }` to the enum, `code()` → `"integrity_failed"`, `exit_code()` → `6`, `parts()` arm, and a `pub fn integrity_failed(what, why, do_) -> CliError` constructor mirroring `bad_usage`. `verify.rs`:

```rust
pub struct SectionReport { pub name: String, pub required: bool, pub logical: String, pub blob: String, pub bytes: u64, pub transport: String, pub logical_check: String, pub schema_version: u32, pub uncompressed_bytes: Option<u64>, pub failures: Vec<String> }
pub fn check(data_dir: &Path, only: Option<&str>) -> Result<(Manifest, Vec<SectionReport>, /*root recomputed*/ String), CliError> { … }
pub fn run(data_dir: &Path, only: Option<&str>) -> Result<String, CliError> { let (m, reports, root) = check(data_dir, only)?; render; if any failures -> Err(integrity_failed(..)) }
pub fn run_json(data_dir: &Path, only: Option<&str>) -> Result<serde_json::Value, CliError> { … }
```

`check`: `let layout = SectionLayout::under(data_dir); let m = read_manifest(&layout.manifest_path()).map_err(|e| if manifest missing { DataLoadFailed } else { integrity_failed("manifest", e.to_string(), …) })?;` root recomputed = `atlas_graph::sqlite::manifest::root_of(&m.sections)` (already equal by `read_manifest`, printed anyway). Per section (filtered by `only`, validated against manifest names → `BadUsage` "unknown section"): blob path → `transport`: `!is_file()` → if cache also absent: required → failure `"kjv: required blob MISSING at <path>"`, transport `"missing"`; optional → transport `"absent"`, logical `"skipped"`; else `sha256_hex_of_file` vs `blob` → `"ok"`/`"mismatch"` (failure line names both). Logical: `source.resolve(ms)` (unpacks a cache miss; a transport failure here is already recorded — skip when transport mismatched: `logical_check = "skipped"`) → `open_read_only` → `PRAGMA user_version` → `logical_dump_of_db` → compare. `main.rs`: `"verify" => { let only = parse_verify_args(&rest)?; commands::verify::run(&data_dir, only.as_deref()) }` in both `run` and `run_json` (the JSON form returns the value through the existing JSON envelope path — mirror how `"kinds"` is handled in `run_json`); `parse_verify_args`: accepts nothing or `--section <name>`; anything else → `bad_usage`. Update both "'atlas' only knows …" strings to include `verify`. `help.rs`: add `  verify [--section <name>]   recompute every section's logical hash, blob hash and the root against manifest.toml (exit 6 on any mismatch)` in the command list, and the tutorial's closing "every command" step if it enumerates them (`tutorial.rs` — search for `kinds`). `CONTRACT.md`: add `verify` to the vocabulary section with the report shape above, and row `6 | integrity_failed | the data on disk disagrees with its manifest (a hash mismatch, a required section missing)` to the exit-code table.

- [ ] **Step 4: Run** `cargo test -p atlas-cli` (the whole crate: help/tutorial pins) → green. Note the test needs the committed sections — they land in Task 6; run this task's tests after Task 6's compile if executing in order, or compile first (Task 6 Step 3 is independent of Task 5).

- [ ] **Step 5: Commit** — `git add server/atlas-cli && git commit -m "feat(cli): DB-4b -- bibex verify: logical hash, transport hash and root recomputed against manifest.toml, per-section report, exit 6 integrity_failed (spec 3.5, 11)"`.

---

### Task 6: The compile binary, the artifact path, the committed sections, and the gates

**Files:**
- Modify: `server/atlas-graph/src/bins/compile_graph.rs` (args, extras, layout, admission), `server/atlas-graph/src/service.rs:374-430` (`assemble` attaches extras; `from_artifact` computes them), `server/atlas-graph/tests/sqlite_real_data.rs` (extras, layout, ceiling), `tests/version_root_regression.rs` (re-pin if its graph is artifact-based), `tests/sqlite_laws.rs` (if any root literal), `scripts/timing-gates.sh` (gate 9 ceiling note)
- Create (committed): `data/compiled/manifest.toml`, `data/compiled/sections/{core,kjv,concord,kretzmann}.<logical>.sqlite.zst`
- Delete: `data/cache/sections-build/` (gitignored leftovers; `rm -rf`)

**Interfaces:**
- Consumes: `extras_for_artifact(g, resolved, data_dir)`, `SectionLayout::under(data_dir)`, `CommittedZstdSource`, `write_sections(g, &extras, compiler, &layout)`, `SqliteSnapshot::open(manifest, &source)`.
- Produces: `GraphService::from_artifact` publishes the manifest root; the compile binary writes committed sections; `atlas-graph-compile --data-dir <compiled> [--out <graph.bin>] [--sections-cache <dir>]` (`--sections-out` removed: the sections' home is `--data-dir`).

- [ ] **Step 1: `service.rs`.** `assemble(mut graph, stats, ews, chronology, red_letter_spans, sidecars: Option<&crate::sqlite::sidecars::Sidecars>)`: after `heading_index` is built (line ~388) and before `provenance`/`publish`:

```rust
        // DB-4b: the non-graph section tables ride the graph into the root
        // (graph-types `Graph::extra_tables`); computed from the same values
        // the writer folds, so this service's version IS manifest.toml's root
        // when `sidecars` is the compiled dir's own (from_artifact). The
        // from-sources paths pass None: graph-derived tables only, a root
        // that is DISCLOSED as not the manifest's (the dev fallback).
        let mut extras = crate::sqlite::extras::Extras::graph_derived(&graph, &chronology.chrono.resolved, &red_letter_spans)
            .expect("assemble: the graph's projections encode");
        if let Some(sc) = sidecars {
            extras.extend(crate::sqlite::sidecars::fold_sidecars(&sc.atlas, &sc.sources).expect("assemble: the sidecars fold"));
        }
        extras.attach(&mut graph);
```

(`heading_index` is built twice on this path — once for the service, once inside `graph_derived`; 1,711 events, milliseconds; disclosed in the doc comment.) `from_artifact`: `let data_dir = path.parent()…; let sidecars = crate::sqlite::sidecars::Sidecars::load(data_dir)?; Ok(Self::assemble(…, sidecars.as_ref()))`; the two from-sources constructors pass `None`.

- [ ] **Step 2: `compile_graph.rs`.** `parse_args` → `(data_dir, out, sections_cache: Option<PathBuf>)`; `let layout = match sections_cache { Some(c) => SectionLayout { compiled_dir: data_dir.clone(), cache_dir: c }, None => SectionLayout::under(&data_dir) };`. In the DB-2b block (after the red-letter file and exports are written — they are inputs now): `let resolved = &chronology.chrono.resolved; let extras = atlas_graph::sqlite::extras::extras_for_artifact(&graph_b, resolved, &data_dir)?; extras.attach(&mut graph_b);` — and ALSO attach to `graph_a_indexed` BEFORE `version_store.publish(graph_a_indexed)` (line ~317; the exports' `atlas_version_root` must be the manifest root): move the extras computation up to just before the exports block, computed over `graph_a_indexed` (same content), attach to both. Rename the block's messages `DB-2b` → `DB-4b`. `write_sections(&graph_b, &extras, &compiler, &layout)`; print per section `name, nodes, rows, extra rows, edges, uncompressed -> compressed bytes (reused/compressed), logical`; open through `CommittedZstdSource { layout: layout.clone() }`; the per-section logical recompute loop opens `w.path` (the cache file) as today; add `anyhow::ensure!(manifest.root == version_hex, "DB-4b: manifest root {} != the published version {}", …)` right after the write — THE root-equality proof on the real graph, at compile. Update the module doc comment (the three outputs: `graph.bin`, `red-letter-spans.json` + exports, and now `manifest.toml` + `sections/`).

- [ ] **Step 3: Compile and commit the sections.** From `server/`: `rm -rf ../data/cache/sections-build && time cargo run -p atlas-graph --bin atlas-graph-compile -- --data-dir ../data/compiled --out ../data/compiled/graph.bin 2>&1 | tee "$L/db4b-compile.log"` (`$L` = the batch's absolute log dir under `.superpowers/sdd/2026-08-17-bible-atlas-m1/logs-db4b/`). Expect: `graph.bin` byte-identical (`git status` shows it unchanged), `data/exports/*.json` roots moved, `data/compiled/manifest.toml` + four blobs new, cache populated. Record: compile wall time, per-section uncompressed → compressed bytes (each < 104,857,600), the new root. Run it a SECOND time: `git status --porcelain data/` must show the same set (no churn: `built` preserved, blobs reused) — that is judgment call 8 on the real data.

- [ ] **Step 4: Gate 9 and the real-data laws.** `sqlite_real_data.rs`: `committed_graph()` → after `add_justified_by`, `let extras = extras_for_artifact(&graph, &chronology.chrono.resolved, &data_dir)?; extras.attach(&mut graph);` and keep `extras` beside the graph (return `(Graph, Extras)` from the `OnceLock`); the test uses `layout_under(&tempdir)`, passes `&extras`, opens through the source, and adds: `assert_eq!(m1.root, GraphSnapshot::version(&MemSnapshot-of-committed_graph).hex())` — use the existing `mem_store_stamps_the_same_root` pattern — and `assert_eq!(m1.root, read_manifest(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled/manifest.toml")).unwrap().root, "the committed manifest is what this graph + these sidecars produce")`. Measure; set `CEILING_SECS` = measured × 2 rounded up to the next 30 s; update the doc comment with the measurement and `scripts/timing-gates.sh`'s gate-9 note. Add to `extras_real_data.rs`: `the_committed_manifest_root_recomputes_from_graph_bin_plus_the_sidecars` (from_artifact → `GraphService::version()` hex == manifest root) — the server-path proof.
  `version_root_regression.rs`: run it; if its graph comes from the artifact its `EXPECTED_VERSION_HEX` moves — re-pin with a DB-4b paragraph naming why (the dump widened: extras); if it is a fixture graph with no extras, it stays.

- [ ] **Step 5: Run** `cargo test -p atlas-graph` (the whole crate incl. `sqlite_real_data` — ~6–8 min) and `cargo test -p atlas-cli` (Task 5's verify tests now have sections) → green. `cargo test -p atlas-server --test graph_conformance` (gate 2) → green (no version comparison, but the load path changed).

- [ ] **Step 6: Commit** in two: `git add server data/compiled/manifest.toml data/compiled/sections scripts/timing-gates.sh && git commit -m "feat(graph, compile): DB-4b -- the compile writes committed zstd sections + manifest under data/compiled; from_artifact attaches the extras so the served version is the manifest root; gate 9 measures the full writer"` then `git add data/exports && git commit -m "chore(exports): DB-4b -- atlas_version_root moved once with the widened dump (spec 3.6; the manifest root now covers projections and sidecars)"`. (`graph.bin` unchanged, so not staged.)

---

### Task 7: Pins re-recorded, contract bumps, docs, standing block, push

**Files:**
- Modify: `contracts/pacts/http.json`, `contracts/pacts/cli.json` (recorders), `contracts/atlas-graph-contract/{VERSION → 0.5.0, CHANGELOG.md, fixtures/*}` (runner `--bless`), `contracts/atlas-query-contract/{VERSION → 0.3.0, CHANGELOG.md, aqc.schema.json version, features/versioning.feature (accept "0.3.0"… reject window "0.4.0" through "0.6.0"), fixtures/*}` (`export_aqc_examples`), `server/atlas-server/src/contract.rs` (MIN/MAX_SUPPORTED_VERSION 0.3.0), `server/atlas-server/tests/contract_api.rs`, `client/AqcContract.cs::ClientVersion "0.3.0"`, `client.Tests/AqcContractTests.cs` InlineData (accept ("0.3.0","0.3.0"), ("0.2.9","0.4.0"); reject ("0.4.0","0.6.0"), ("0.0.1","0.2.9")), `contracts/atlas-graph-contract/fixtures/contract.json` (re-blessed to the advertised range), `server/atlas-cli/tests/cli.rs` (only if a transcript embeds the root — `grep -n "e5d656e2" server/atlas-cli/tests/cli.rs`), `server/BENCHMARKS.md`, `docs/superpowers/specs/2026-09-14-relational-artifact-design.md` (§5.3 `event_date`/`heading_index`/`chronology_anchor`/`land_mask_region`/`place_name_alias`/`provenance_entry` rows amended with a "DB-4b:" note each; §3.6 "once" footnote: DB-4a moved ids and root, DB-4b moved the root once more when the dump widened), `.superpowers/sdd/2026-08-17-bible-atlas-m1/progress.md`, memory `relational-artifact-decisions.md`.

- [ ] **Step 1: Recorders**, in the Global Constraints order. Expect the AQC 19 root-bearing fixtures, the three pact `/api/contract`/version bodies, the AGC `version-root` fixture to change; NO edge id or pid changes (assert with `git diff --stat contracts/ | grep -c edges-` — only files carrying the root move; if an id moved, STOP: the identity did not stop moving and the batch has a defect).
- [ ] **Step 2: Bumps.** Five AQC copies + client cases + `contract.json`; AGC VERSION/CHANGELOG (`0.5.0 — DB-4b: the version root moved once more (the logical dump widened to projections and folded sidecars); ids unchanged`). `bash scripts/contract-gate.sh` after committing → PASSED (both graded "declared minor = major under the 0.x rule"). `dotnet test client.Tests --filter AqcContract` with `DOTNET_ROOT=$HOME/AppData/Local/Microsoft/dotnet` → green; `cargo test -p atlas-server --test aqc_cucumber` → 44/44.
- [ ] **Step 3: Docs.** `BENCHMARKS.md` `## DB-4b (2026-09-17)`: compile wall time before (DB-4a's) / after; per-section uncompressed → compressed bytes vs 104,857,600; zstd threads used; gate 9 measured and ceiling; `bibex verify` wall time; recompile idempotence verified; `GraphService` startup delta from loading sidecars twice. Spec amendments as listed. Ledger `## DB-4b PLAN/EXECUTION` with rulings `R-DB4b-1..` (judgment calls that changed in execution) and the standing-block line. Memory line under DB-4b: root moved (new hex), AQC 0.3.0, AGC 0.5.0, NEXT: DB-4c.
- [ ] **Step 4: Standing block** — copy `logs-db3-standing/run.sh` to `logs-db4b-standing/run.sh`, launch detached (`Start-Process bash.exe`), sum `^test result:` lines: workspace all green, graph-types ON and OFF green, timing gates 9/9 with gate 9 under its new ceiling. Contract gate PASSED.
- [ ] **Step 5: Commit and push** — `git add` the contract/client/docs/ledger paths (never the three untracked frontier files or `publish-apphome/`), commit `chore(contracts, client, docs): DB-4b -- pins re-recorded once (root moved with the widened dump): AQC 0.3.0, AGC 0.5.0; BENCHMARKS, spec 5.3 amendments, ledger`, then `git push origin worktree-bible-atlas-m1`.

---

## Self-review

**Spec coverage.** §2.2 manifest fields ✓ (Task 4: `bytes` = compressed size, `built` outside the root and preserved). §2.3 files on disk ✓ (Task 4 layout; ceiling refused; loader trusts the hash — `resolve` never parses the name). §2.4 `SectionSource` + `CommittedZstdSource` ✓ (Task 4: verify blob SHA-256, unpack to `data/cache/sections/<logical>.sqlite` if absent, cache hits skip). §2.5 steps 1–4 ✓ (Task 4 `open` through the source; step 3's mmap/connection-per-worker → DB-4c). §3.4 logical dump over the widened table list ✓ (Tasks 1–3), recomputable from the file ✓ (`logical_dump_of_db` extras arm). §3.5 `bibex verify` ✓ (Task 5, both forms, exit 6). §3.6 root moved once more, pins re-recorded ✓ (Task 7, judgment call 9). §5.0 conventions ✓ (DDL scan law covers the new strings; `WITHOUT ROWID`/`INTEGER PRIMARY KEY` per table as spec'd). §5.3 projections/`event_date`/`heading_index`/sidecars ✓ (Tasks 2–3; amendments: judgment calls 3–6; `polities.json` retired not folded). §5.4 `verse`, `red_letter_span` ✓; §5.5 `concord_unit` ✓; §5.6 nothing new ✓. §6.1 step 3 (VACUUM, zstd 19, name by logical, transport hash) ✓, step 4 (manifest, root, exports stamped — the exports carry the same root because both graphs carry the extras) ✓. §6.2 gate over the compile binary's own graph ✓ (Task 6: `SqliteSnapshot` through the source + per-section logical recompute + root equality). §9 bumps: the artifact wall → DB-4c; AQC/AGC MINOR here ✓. §11 error rows ✓ (Task 4/5: mismatch names both hashes and leaves no partial file; required missing names name+logical; optional absent recorded). §12: compile wall time, compressed sizes vs ceiling ✓ (Task 7); peak RSS / cold start / frontier → DB-4c.

**Placeholder scan.** No TBD/TODO. Every DDL string is written out. The one "check the exact name" instruction (Task 2's `ScriptureRef` verse variant) names the file and lines to read.

**Type consistency.** `Extras::graph_derived(&Graph, &HashMap<String, ResolvedPlacement>, &HashMap<String, Vec<(usize,usize)>>)` used identically in Tasks 2, 3 (`extras_for_artifact`) and 6 (`assemble`); `red_letter_spans` in `assemble` is already a `HashMap` (service.rs:374); `extras_for_artifact` reads the `BTreeMap` from the file and converts. `write_sections(&Graph, &Extras, &str, &SectionLayout)` in Tasks 4–6. `SqliteSnapshot::open(&Path, &dyn SectionSource)` in Tasks 4–6. `table_specs_of`/`extra_tables_of` name parity is a law (Task 2) and Task 3 appends to both lists in the same order. `Col` variants `Null/Int/Real/Text` everywhere. `WrittenSection.path` = cache file, `.blob_path` = blob, in Tasks 4 and 6.
