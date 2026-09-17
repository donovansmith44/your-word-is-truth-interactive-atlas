# DB-2b: SQLite Section Writer, SqliteSnapshot and the Admission Gate — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Write the four per-corpus SQLite sections (+ manifest) from the compiled `Graph`, open them through a `SqliteSnapshot` that implements the existing read port, and prove — on the real graph — that the SQLite backend answers every port question exactly as the in-memory graph does and that the logical hash computed from the Graph equals the one recomputed from the SQLite tables.

**Architecture:** Nothing observable changes: the server keeps serving `MemSnapshot` from `graph.bin`; the new writer runs in `atlas-graph-compile` beside `artifact::dump`, into a gitignored directory, and is proven by `assert_answers_match(&SqliteSnapshot, &graph)` plus per-section logical-hash equality. `graph-types` gains ONE public seam, `Graph::row_edges()`, which factors the private row→edge pairing out of `build_indexes` so the writer (and `add_justified_by`) can name the row behind every index entry — the `edge_index.row_family/row_id` columns and DB-2a's RELMAP-1 finding. Everything SQLite lives in `atlas-graph` under `src/sqlite/`; `rusqlite` (bundled) is a dependency of `atlas-graph` only.

**Tech Stack:** Rust 1.97.1 (pinned, `x86_64-pc-windows-gnu`); `rusqlite = { version = "0.32", features = ["bundled"] }` and `toml = "0.8"` in `atlas-graph`; DB-2a's `atlas_graph_types::canon` (encode/decode, `RowFamily`, ids) and `sha256`; `sections.rs` (DB-2a Task 4).

**Spec:** `docs/superpowers/specs/2026-09-14-relational-artifact-design.md` — §2 (artifact), §3.4 (identity hierarchy), §5 (schema), §6.1–6.2 (writer + gate), §8 row 3 (this batch), §11, §12. Survey: `.superpowers/sdd/2026-09-16-db2-survey.md`. DB-2a final review (plan inputs EDGE-ID-1, ROOT-1, RELMAP-1, VALUE-INT-1): `.superpowers/sdd/2026-08-17-bible-atlas-m1/db2a-final-review-report.md`.

## Global Constraints

- **`graph-types` stays zero-dependency** (`[dependencies]` and `[dev-dependencies]` empty; `graph-types/tests/zero_deps.rs` enforces it). `rusqlite`, `toml` and every SQLite line of code live in `server/atlas-graph` only (spec §8 row 3).
- **Nothing observable changes** (spec §8 row 3: "version bump none — an unused file is not a contract"): the server still loads `graph.bin`; `GraphService::snapshot()` still returns `MemSnapshot`; version root at startup stays `dfcf6ee4c2a39965`; `version_root_regression.rs`'s `59c6f7dc236f6f23`, the 25 scene hashes, every `contracts/` fixture and `atlas-cli/tests/cli.rs:577` stay untouched. The `canon-ids` feature stays OFF; hash columns are **8 bytes** (`ContentHash(u64)` big-endian) in this batch — `meta.hash_width = "8"` records it (spec §5.0).
- **Section files are build outputs, not repository content, in this batch:** the compile binary writes them under `data/cache/sections-build/` (gitignored: add `data/cache/` to `.gitignore`). No `.sqlite` or `manifest.toml` is committed before DB-4.
- **Cursor order is proven, never assumed** (spec §5.2): `edge_index.ord` is the entry's global position within the in-memory `(subject, rel, dir)` list, so `ORDER BY ord` reproduces today's order exactly across sections; `assert_answers_match` is the proof.
- **Logical dump line format (spec §3.4), binding:** for each table in `LOGICAL_TABLE_ORDER`, for each row in primary-key order, `<table>\t<canonical row JSON>\n`, where the canonical JSON is `Node::encode()` for `node`, `encode_row_in_family(family, row.to_value())` for a row table, and `{"corpus":"bible","ord":N,"node_id":"TextUnit:bible/1.1.1"}` for `reading_spine` (keys in byte order, no whitespace — write it through `canon::obj`/`serialize`, never by string formatting). Logical hash = `sha256_prefixed_128(DOMAIN_PREFIX, dump)` rendered as 32 lowercase hex. Root = `sha256_prefixed_128(DOMAIN_PREFIX, lines)` where lines = for each section in manifest order `name|logical|schema_version|required\n` (`required` as `true`/`false`).
- **Positions on the wire are DB-2a's canonical strings:** `canon::ids::position_str` / `parse_position` (`n:<Kind>:<raw>`, `e:<EdgeId string>`); node ids `any_node_id_str` / `parse_any_node_id` (`Kind:raw`). Never a second spelling.
- **`section_of_family(RowFamily::ContainsBible)` panics by design (DB-2a R17):** every ContainsBible row routes through `section_of_contains_bible(&row)`; justified-by entries through `section_of_justified_by(family, container_raw)`.
- No `#[ignore]` except the ONE new wall-clock gate added to `scripts/timing-gates.sh` exactly the way CONTENTION-1's gates are (ignored + run serialized by the script); no existing ceiling or threshold changes; no contract fixture changes.
- Standing block = from `server/`: `cargo test --workspace`, `cargo test -p atlas-graph-types`, `bash ../scripts/timing-gates.sh`, run DETACHED by the controller; sum only `^test result:` lines. The `canon-ids` ON gate is `cd graph-types && cargo test --features canon-ids`.
- Never build `--release`; never touch port 8080; port 8000 is free. Untracked files never staged: `client.Tests/FrontierMatrix*.cs`, `publish-apphome/`, `tests/ux/frontier-matrix.spec.ts`. No `git stash`.
- Commit trailers on every commit: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W`. Git Bash; `export PATH="$PATH:$HOME/.cargo/bin"`.

## Prerequisite (environment, owner action)

`rusqlite`'s `bundled` feature compiles SQLite's C amalgamation through the `cc` crate. The pinned toolchain is `x86_64-pc-windows-gnu` and **no C compiler is installed** (no `gcc`, `clang` or `cl.exe` on PATH; rustup's self-contained dir ships only `gcc-ld`). Task 1 cannot build until one exists. The one-line fix on this machine (scoop is installed):

```powershell
scoop install gcc        # winlibs GCC 15.2 (main bucket); puts gcc.exe on PATH
```

Task 1's first step verifies `gcc --version` and stops with `BLOCKED` if it is absent. DB-4's zstd (also bundled C) needs the same compiler, so this is a one-time cost.

## Judgment calls (owner can reverse)

1. **`edge_index` is written from Rust, not by `INSERT … SELECT` per family** (deviation from spec §6.1 step 2): edge ids are content hashes SQLite cannot compute, and `Graph::row_edges()` (Task 2) is now the single row→edge lowering `build_indexes` itself uses, so there is still exactly one path. The law index≡rows keeps living where it lives today (`build_indexes` + the compile-time admission).
2. **EDGE-ID-1 (edge id = hash of `(rel, subject, object)` Debug text) and ROOT-1 (whole-graph dump under `canon-ids` ON) are left as DB-2a shipped them.** This batch runs with the feature OFF, must reproduce today's `EdgeId` strings byte-for-byte for the gate to pass, and computes its logical hashes per section from `row canonical bytes` as §3.4 says. Switching edge identity to per-row bytes is an ON-state decision for DB-4's plan (where every id moves once, §3.6); this plan adds `edge_index.row_family/row_id` so that switch is a column rewrite, not a schema change. Recorded in the plan so DB-4 cannot miss it.
3. **Scope of the schema in this batch:** §5.1 common tables (`meta`, `node`, `justification`, `ground`, `edge_index`), every row table of §5.3–5.6 with its `_locus`/`_step` sub-tables, and `reading_spine` (kjv, concord). NOT in this batch (DB-4, "sidecars folded"): the node projections (`place`, `era`, `polity_era`, `verse`, `concord_unit`), `event_date`, `heading_index`, `red_letter_span`, and the folded sidecar tables. They serve endpoints, not the five port methods this batch proves. `LOGICAL_TABLE_ORDER` lists only the tables written; DB-4 extends it and re-blesses every hash (§3.6).
4. **`meta.graph_version`** carries the in-memory `GraphVersion` hex so `SqliteSnapshot::version()` equals `MemSnapshot::version()` in this batch; DB-4 replaces it with the manifest root.
5. **No zstd in this batch:** sections are written as plain `.sqlite`; `manifest.toml`'s `blob` is the SHA-256 (full 64 hex) of the `.sqlite` file as written and `bytes` its size. DB-4 adds compression and `CommittedZstdSource`.
6. **One `rusqlite::Connection` behind a `Mutex`** in `SqliteSnapshot` (the gate is single-threaded); spec §2.5's connection-per-worker lands with the server switch-over in DB-4.
7. **`node.payload` holds `Node::encode()`** (the whole node's canonical bytes, `{"id","payload","provenance"}`), which is what §5.1 calls "the node's canonical bytes"; `label` is the hoisted display string (`label`/`canonical`/`title`/`citation`/`work` by kind, NULL for TextUnit) and is informational only.

## File Structure

| Path | Responsibility |
|---|---|
| `graph-types/src/graph.rs` (MODIFY) | `pub struct RowEdge`, `pub enum EdgeRel`, `pub fn row_edges(&self) -> Vec<RowEdge>` — the ONE row→edge lowering; `build_indexes` consumes it |
| `graph-types/src/canon/rows.rs` (MODIFY, extend-only) | `RowFamily::relation(self) -> EdgeRel` (RELMAP-1: the total family→relation map) |
| `server/atlas-graph/src/event_world.rs` (MODIFY) | `add_justified_by` consumes `row_edges()` instead of mirroring four loops |
| `server/atlas-graph/src/sections.rs` (MODIFY) | `justified_by_source_family` derived from `RowFamily::relation` |
| `server/atlas-graph/src/sqlite/mod.rs` (NEW) | constants (`SCHEMA_VERSION = 14`, `APPLICATION_ID = 0x424C4741`, `HASH_WIDTH`), `SqliteError`, `open_read_only`, `hash_bytes`/`hash_from_bytes` (ContentHash ↔ BLOB) |
| `server/atlas-graph/src/sqlite/ddl.rs` (NEW) | DDL per section (`create_tables(conn, section)`, `create_indexes(conn, section)`), `LOGICAL_TABLE_ORDER` |
| `server/atlas-graph/src/sqlite/columns.rs` (NEW) | LOCUS / RANGE / AUTHORED column encode+decode: `Locus<C>`, `LocusRange<C>`, `TextLocus`, `TokenSpan`, `Justification`+`Ground` |
| `server/atlas-graph/src/sqlite/rows/{mod,core,kjv,concord,kretzmann}.rs` (NEW) | per family: `insert_<family>(tx, ord, &row)` and `read_<family>(conn) -> Vec<Row>` in PK order; `mod.rs` dispatches by `RowFamily` |
| `server/atlas-graph/src/sqlite/partition.rs` (NEW) | `SectionPartition` (nodes, rows per family with their global ords, edge entries with `(row_family,row_id)`, spines) from `&Graph` |
| `server/atlas-graph/src/sqlite/logical.rs` (NEW) | `logical_dump_of_partition`, `logical_dump_of_db`, `logical_hash`, `root_of` |
| `server/atlas-graph/src/sqlite/manifest.rs` (NEW) | `Manifest`, `ManifestSection`, TOML write/read, `manifest_lines` |
| `server/atlas-graph/src/sqlite/writer.rs` (NEW) | `write_sections(&Graph, version, out_dir) -> Manifest` |
| `server/atlas-graph/src/sqlite/snapshot.rs` (NEW) | `SqliteSnapshot: GraphQuery + GraphSnapshot` |
| `server/atlas-graph/src/lib.rs` (MODIFY) | `pub mod sqlite;` |
| `server/atlas-graph/src/bins/compile_graph.rs` (MODIFY) | `--sections-out` (default sibling `../cache/sections-build`), writer + SQLite admission after the existing admission |
| `server/atlas-graph/Cargo.toml` (MODIFY) | `rusqlite`, `toml` |
| `server/atlas-graph/tests/sqlite_laws.rs` (NEW) | synthetic-graph laws: schema, column round-trips, writer→snapshot admission, logical-hash agreement, determinism |
| `server/atlas-graph/tests/sqlite_real_data.rs` (NEW) | the real-graph gate (ignored; run by `scripts/timing-gates.sh` as gate 9) |
| `scripts/timing-gates.sh` (MODIFY) | gate 9 |
| `.gitignore` (MODIFY) | `data/cache/` |
| `server/BENCHMARKS.md` (MODIFY) | compile wall time before/after; gate 9 time |

---
### Task 1: The dependency, the constants, and the hash-blob seam

**Files:**
- Modify: `server/atlas-graph/Cargo.toml` (`[dependencies]`)
- Modify: `server/atlas-graph/src/lib.rs` (add `pub mod sqlite;` after `pub mod service;`)
- Modify: `.gitignore` (add `data/cache/` after the `data/raw/*` block)
- Create: `server/atlas-graph/src/sqlite/mod.rs`
- Test: `server/atlas-graph/tests/sqlite_laws.rs` (created here, extended by later tasks)

**Interfaces:**
- Produces:
  - `pub const SCHEMA_VERSION: u32 = 14;` `pub const APPLICATION_ID: u32 = 0x424C4741;` `pub const HASH_WIDTH: usize = std::mem::size_of::<atlas_graph_types::id::ContentHash>();` (8 OFF, 16 ON — one expression, no cfg)
  - `pub struct SqliteError(pub String);` with `From<rusqlite::Error>`, `From<atlas_graph_types::canon::CanonError>`, `Display`, `std::error::Error`
  - `pub fn hash_bytes(h: &ContentHash) -> Vec<u8>` — OFF: `h.0.to_be_bytes().to_vec()` (8 bytes); ON: `h.0.to_vec()` (16). Written as ONE cfg split, the only one in the module.
  - `pub fn hash_from_bytes(b: &[u8]) -> Result<ContentHash, SqliteError>` — inverse; refuses any other length.
  - `pub fn open_read_only(path: &Path) -> Result<rusqlite::Connection, SqliteError>` — `OpenFlags::SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX`, then `PRAGMA query_only = ON`.
  - `pub fn stamp_pragmas(conn: &Connection) -> Result<(), SqliteError>` — `PRAGMA user_version = 14; PRAGMA application_id = 0x424C4741; PRAGMA page_size = 4096; PRAGMA encoding = 'UTF-8'; PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF` (run BEFORE any table exists — `page_size` and `encoding` only take effect on an empty file).

- [ ] **Step 1: Verify the C compiler exists** — Run: `gcc --version`. If it prints nothing / "command not found", STOP and report `BLOCKED: no C compiler; owner runs scoop install gcc`. Do not try to work around it.

- [ ] **Step 2: Add the dependencies**

```toml
# server/atlas-graph/Cargo.toml, [dependencies] — append after `bincode = "1"`:
# DB-2b: the SQLite section writer + SqliteSnapshot (spec §8 row 3: "rusqlite
# (bundled) in atlas-graph only"). `bundled` compiles the amalgamation via cc
# — a C compiler on PATH is a build prerequisite (plan: Prerequisite).
rusqlite = { version = "0.32", features = ["bundled"] }
# DB-2b: manifest.toml (spec §2.2) — serde derive on Manifest/ManifestSection.
toml = "0.8"
```

- [ ] **Step 3: Write the failing test**

```rust
// server/atlas-graph/tests/sqlite_laws.rs
//! DB-2b laws on synthetic data: the seams the section writer and the
//! SqliteSnapshot are built from, each proven in isolation before the
//! real-data gate (`sqlite_real_data.rs`) composes them.
use atlas_graph::sqlite::{hash_bytes, hash_from_bytes, open_read_only, stamp_pragmas, APPLICATION_ID, HASH_WIDTH, SCHEMA_VERSION};
use atlas_graph_types::id::ContentHash;

#[test]
fn hash_blob_round_trips_at_the_current_width() {
    let h = ContentHash(0x0123_4567_89ab_cdef);
    let b = hash_bytes(&h);
    assert_eq!(b.len(), HASH_WIDTH);
    assert_eq!(b, 0x0123_4567_89ab_cdefu64.to_be_bytes().to_vec());
    assert_eq!(hash_from_bytes(&b).unwrap(), h);
    assert!(hash_from_bytes(&b[..HASH_WIDTH - 1]).is_err(), "a short blob is refused, never padded");
}

#[test]
fn pragmas_are_stamped_and_read_back_from_a_read_only_open() {
    let dir = std::env::temp_dir().join(format!("db2b-laws-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("pragmas.sqlite");
    let _ = std::fs::remove_file(&path);
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        stamp_pragmas(&conn).unwrap();
        conn.execute_batch("CREATE TABLE t (k TEXT PRIMARY KEY) WITHOUT ROWID; INSERT INTO t VALUES ('x');").unwrap();
    }
    let ro = open_read_only(&path).unwrap();
    let uv: u32 = ro.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    let ai: u32 = ro.query_row("PRAGMA application_id", [], |r| r.get(0)).unwrap();
    let ps: u32 = ro.query_row("PRAGMA page_size", [], |r| r.get(0)).unwrap();
    let qo: u32 = ro.query_row("PRAGMA query_only", [], |r| r.get(0)).unwrap();
    assert_eq!((uv, ai, ps, qo), (SCHEMA_VERSION, APPLICATION_ID, 4096, 1));
    assert!(ro.execute("INSERT INTO t VALUES ('y')", []).is_err(), "read-only means read-only");
}
```

- [ ] **Step 4: Run to verify it fails** — Run from `server/`: `cargo test -p atlas-graph --test sqlite_laws`. Expected: compile error, `atlas_graph::sqlite` does not exist (after the `rusqlite` build — the first build of the amalgamation takes a few minutes; report its wall time).

- [ ] **Step 5: Implement `sqlite/mod.rs`**

```rust
//! DB-2b: the SQLite section artifact (spec §2, §5, §6) — writer,
//! logical dump, manifest and the `SqliteSnapshot` read port. Everything
//! SQLite in the workspace lives under this module; `graph-types` never
//! sees it (zero-dep covenant, spec §1.2).
pub mod columns;   // Task 3
pub mod ddl;       // Task 3
pub mod logical;   // Task 6
pub mod manifest;  // Task 5
pub mod partition; // Task 5
pub mod rows;      // Task 4
pub mod snapshot;  // Task 7
pub mod writer;    // Task 5
// (Task 1 declares only what it creates; later tasks add their `pub mod` line.)

use std::path::Path;
use atlas_graph_types::id::ContentHash;
use rusqlite::{Connection, OpenFlags};

/// Spec §5.0: every section's `PRAGMA user_version`.
pub const SCHEMA_VERSION: u32 = 14;
/// Spec §5.0: 'BLGA'.
pub const APPLICATION_ID: u32 = 0x424C_4741;
/// Bytes per hash column (`node.pid`, `edge_index.edge_id`): the current
/// `ContentHash` width — 8 while `canon-ids` is off, 16 once it is on.
pub const HASH_WIDTH: usize = std::mem::size_of::<ContentHash>();

#[derive(Debug)]
pub struct SqliteError(pub String);
impl std::fmt::Display for SqliteError { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.0) } }
impl std::error::Error for SqliteError {}
impl From<rusqlite::Error> for SqliteError { fn from(e: rusqlite::Error) -> Self { SqliteError(format!("sqlite: {e}")) } }
impl From<atlas_graph_types::canon::CanonError> for SqliteError { fn from(e: atlas_graph_types::canon::CanonError) -> Self { SqliteError(format!("canon: {e}")) } }
impl From<std::io::Error> for SqliteError { fn from(e: std::io::Error) -> Self { SqliteError(format!("io: {e}")) } }

// The hash width is graph-types' feature, which this crate cannot `cfg`
// on. `ContentHash::hex()` exists in both states and is width-honest
// (16 hex chars OFF, 32 ON), so the BLOB is the hex decoded to bytes —
// one path, no cfg. OFF: `hex()` is `{:016x}` of the u64, so the 8 bytes
// ARE the big-endian u64 (what the test asserts); ON: the 16 bytes.
pub fn hash_bytes(h: &ContentHash) -> Vec<u8> {
    let hex = h.hex();                    // 16 or 32 lowercase hex chars
    (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("ContentHash::hex is hex")).collect()
}
pub fn hash_from_bytes(b: &[u8]) -> Result<ContentHash, SqliteError> {
    if b.len() != HASH_WIDTH { return Err(SqliteError(format!("hash blob is {} bytes, expected {HASH_WIDTH}", b.len()))); }
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    ContentHash::from_hex(&hex).ok_or_else(|| SqliteError(format!("hash blob {hex} is not a ContentHash")))
}
pub fn open_read_only(path: &Path) -> Result<Connection, SqliteError> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    conn.execute_batch("PRAGMA query_only = ON;")?;
    Ok(conn)
}
pub fn stamp_pragmas(conn: &Connection) -> Result<(), SqliteError> {
    conn.execute_batch(&format!(
        "PRAGMA page_size = 4096; PRAGMA encoding = 'UTF-8'; PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF; \
         PRAGMA user_version = {SCHEMA_VERSION}; PRAGMA application_id = {APPLICATION_ID};"
    ))?;
    Ok(())
}
```

  The `pub mod` lines for modules later tasks create (`columns`, `ddl`, `logical`, `manifest`, `partition`, `rows`, `snapshot`, `writer`) are written as `// pub mod x; // Task N` comments in this task and uncommented by the task that creates each file.

- [ ] **Step 6: `.gitignore` and `lib.rs`** — append to `.gitignore` after the `data/raw` block: `# DB-2b: section build outputs (spec §2.3 unpack cache; committed sections arrive at DB-4)` / `data/cache/`. Add `pub mod sqlite;` to `server/atlas-graph/src/lib.rs`.

- [ ] **Step 7: Run to verify it passes** — Run: `cargo test -p atlas-graph --test sqlite_laws`. Expected: 2 passed. Then `cd ../graph-types && cargo test --test zero_deps`. Expected: 6 passed (graph-types untouched).

- [ ] **Step 8: Commit**

```bash
git add server/atlas-graph/Cargo.toml server/Cargo.lock server/atlas-graph/src/lib.rs server/atlas-graph/src/sqlite/mod.rs server/atlas-graph/tests/sqlite_laws.rs .gitignore
git commit -m "feat(graph): DB-2b -- rusqlite (bundled) in atlas-graph only; sqlite module constants, hash-blob seam, read-only open"
```

---

### Task 2: `Graph::row_edges()` — one row→edge lowering, named by row

**Files:**
- Modify: `graph-types/src/graph.rs` (`build_indexes`, lines 121–330 today: the `pairs`/`sym_pairs` loops)
- Modify: `graph-types/src/canon/rows.rs` (append `RowFamily::relation`)
- Modify: `server/atlas-graph/src/event_world.rs:891-983` (`add_justified_by`)
- Modify: `server/atlas-graph/src/sections.rs:205-215` (`justified_by_source_family`)
- Test: `graph-types/src/graph.rs` `#[cfg(test)] mod row_edge_laws` (new), `server/atlas-graph/tests/sections_real_data.rs` (unchanged — it must stay green: it proves the four justified-by families still mint identical ids)

**Interfaces:**
- Produces (in `graph-types/src/graph.rs`, all `pub`):

```rust
/// Which relation a row family lowers into (directed or symmetric).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeRel { Directed(RelationId), Symmetric(SymRelationId) }

/// One index entry BEFORE it is placed: the row that produced it, the
/// relation, the two ends, the meta. `row_ord` is the row's position in
/// its family's Vec (spec §5.0 `ord`); a set-valued row (a Contains row
/// with N loci, a Succession chain with N events) yields N entries with
/// the same `row_ord`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowEdge {
    pub family: crate::canon::RowFamily,
    pub row_ord: usize,
    pub rel: EdgeRel,
    pub subject: Position,
    pub object: Position,
    pub meta: crate::explore::EdgeMeta,
}

impl Graph {
    /// EVERY edge the row tables declare, in the exact order `build_indexes`
    /// has always visited them — the historical loop order, which is NOT
    /// `RowFamily::ALL` order: contains_bible, contains_concord, attests,
    /// succession, canon_succession, dated_by, comments_on, spoken_by,
    /// spoken_at, located_at, named_after, mentions, cross_refs, quotes,
    /// confesses, fulfills, typology, then symmetric: catechism,
    /// temporal_adjacency, analogue). Order is load-bearing: `BiIndex`
    /// Vec order = cursor order = pinned scene bytes.
    pub fn row_edges(&self) -> Vec<RowEdge>;
    /// The pure per-family edge id, so callers never re-spell `entry_id`
    /// vs `entry_id_symmetric`: Directed → `entry_id(rel, s, o)`,
    /// Symmetric → `entry_id_symmetric(rel, s, o)`.
    pub fn edge_id_of(e: &RowEdge) -> EdgeId;
}
```

  and in `canon/rows.rs`: `impl RowFamily { pub fn relation(self) -> crate::graph::EdgeRel }` — the TOTAL map (RELMAP-1): ContainsBible/ContainsConcord → Contains; Attests → Attests; Succession, CanonSuccession → Succession; DatedBy → DatedBy; LocatedAt → LocatedAt; Fulfills → Fulfillment; Typology → Typology; NamedAfter → NamedAfter; Catechism → Symmetric(CatechismLink); Mentions → Mentions; CorrespondsBible → Symmetric(Corresponds); TemporalAdjacency → Symmetric(TemporalAdjacency); Analogue → Symmetric(Analogue); CrossRefs → Cites; SpokenBy → SpokenBy; SpokenAt → SpokenAt; Quotes → Quotes; Confesses → Confesses; CommentsOn → CommentsOn. (`JustifiedBy`, `DerivedFrom`, `Parallel` have no family — `JustifiedBy` is synthesised from grounds; the other two have zero producers.)

- [ ] **Step 1: Write the failing laws** (in `graph.rs`, a new `#[cfg(test)] mod row_edge_laws`):

```rust
#[cfg(test)]
mod row_edge_laws {
    use super::*;
    use crate::canon::RowFamily;
    use crate::edge::{Analogue, Attests, Justification, LocatedAt, Succession};
    use crate::id::{EventId, NarrativeId, PlaceId};
    use crate::text::{BibleTag, Locus, LocusRange, VerseRef};

    fn g() -> Graph {
        // Two events, one place, one narrative chain of two, one analogue,
        // one attests over a range — enough for every EdgeRel arm to fire.
        let mut g = Graph::default();
        let e1 = EventId::new("e1"); let e2 = EventId::new("e2"); let p = PlaceId::new("p");
        g.located_at.push(LocatedAt { event: e1.clone(), place: p.clone(), provenance: "prov".into(), justification: Justification::default() });
        g.located_at.push(LocatedAt { event: e2.clone(), place: p.clone(), provenance: "prov".into(), justification: Justification::default() });
        g.succession.push(Succession::new(NarrativeId::new("n"), vec![e1.clone(), e2.clone()], "prov".into(), Justification::default()).unwrap());
        g.analogue.push(Analogue { a: e1.clone(), b: e2.clone(), provenance: "prov".into() });
        let from = Locus::<BibleTag> { unit: VerseRef { book: 1, chapter: 1, verse: 1 }, span: None };
        let to = Locus::<BibleTag> { unit: VerseRef { book: 1, chapter: 1, verse: 3 }, span: None };
        g.attests.push(Attests { event: e1, attestation: LocusRange::new(from, to).unwrap(), provenance: "prov".into(), justification: Justification::default() });
        g
    }

    #[test]
    fn every_row_edge_names_its_row_and_its_family_relation() {
        let g = g();
        let edges = g.row_edges();
        assert_eq!(edges.len(), 2 + 1 + 1 + 1, "2 located_at + 1 succession step + 1 analogue + 1 attests");
        for e in &edges { assert_eq!(e.rel, e.family.relation(), "{:?}", e.family); }
        let located: Vec<_> = edges.iter().filter(|e| e.family == RowFamily::LocatedAt).map(|e| e.row_ord).collect();
        assert_eq!(located, vec![0, 1]);
        let attests_subject = edges.iter().find(|e| e.family == RowFamily::Attests).unwrap();
        assert_eq!(crate::canon::ids::position_str(&attests_subject.object), "n:TextUnit:bible/1.1.1", "the range's FIRST verse is the endpoint");
    }

    #[test]
    fn build_indexes_is_exactly_the_row_edges_placed() {
        let mut g = g();
        g.build_indexes();
        let mut from_rows: BTreeMap<(EdgeRel, Position), Vec<EdgeId>> = BTreeMap::new();
        for e in g.row_edges() {
            let id = Graph::edge_id_of(&e);
            from_rows.entry((e.rel, e.subject.clone())).or_default().push(id.clone());
            match e.rel {
                EdgeRel::Directed(_) => {}
                EdgeRel::Symmetric(_) => from_rows.entry((e.rel, e.object.clone())).or_default().push(id),
            }
        }
        for (rel, ix) in &g.indexes {
            for (subject, entries) in &ix.fwd {
                let ids: Vec<EdgeId> = entries.iter().map(|(id, _, _)| id.clone()).collect();
                assert_eq!(ids, from_rows[&(EdgeRel::Directed(*rel), subject.clone())], "fwd order at {subject:?}");
            }
        }
        for (rel, ix) in &g.symmetric_indexes {
            for (subject, entries) in &ix.fwd {
                let ids: Vec<EdgeId> = entries.iter().map(|(id, _, _)| id.clone()).collect();
                assert_eq!(ids, from_rows[&(EdgeRel::Symmetric(*rel), subject.clone())], "sym order at {subject:?}");
            }
        }
    }

    #[test]
    fn relation_map_is_total_over_all_21_families() {
        for f in RowFamily::ALL { let _ = f.relation(); }   // compiles only if the match is exhaustive; runs to prove no panic
    }
}
```

  (`Graph::default()` exists today because `assert_answers_match`'s own laws build graphs by hand; if a constructor is required instead, use the one `store.rs`'s `laws` module uses and say so in the report.)

- [ ] **Step 2: Run to verify RED** — Run from `graph-types/`: `cargo test row_edge_laws`. Expected: compile errors (`RowEdge`, `EdgeRel`, `row_edges`, `relation` undefined).

- [ ] **Step 3: Implement** — In `graph.rs`: add `EdgeRel`, `RowEdge`; move the body of every `for row in &self.<family>` loop from `build_indexes` into `row_edges` unchanged except that each `pairs.entry(R::X).or_default().push((s, o, m))` becomes `out.push(RowEdge { family: RowFamily::X, row_ord: i, rel: EdgeRel::Directed(R::X), subject: s, object: o, meta: m })` (loops become `for (i, row) in self.<family>.iter().enumerate()`), and symmetric ones likewise with `EdgeRel::Symmetric(S::X)`. Keep the loop ORDER exactly as it is today. Then `build_indexes` starts with:

```rust
let mut pairs: BTreeMap<RelationId, Vec<(Position, Position, M)>> = BTreeMap::new();
let mut sym_pairs: BTreeMap<S, Vec<(Position, Position, M)>> = BTreeMap::new();
for e in self.row_edges() {
    match e.rel {
        EdgeRel::Directed(r) => pairs.entry(r).or_default().push((e.subject, e.object, e.meta)),
        EdgeRel::Symmetric(s) => sym_pairs.entry(s).or_default().push((e.subject, e.object, e.meta)),
    }
}
```

  and continues unchanged from `let n_threads = …`. `edge_id_of`: `match e.rel { Directed(r) => entry_id(r, &e.subject, &e.object), Symmetric(s) => entry_id_symmetric(s, &e.subject, &e.object) }`. Add `RowFamily::relation` in `rows.rs` with the exhaustive match listed above (no wildcard). Keep every explanatory comment from the old loops with its loop.

- [ ] **Step 4: Rewire `add_justified_by`** — replace the four hand-mirrored loops with one pass over `graph.row_edges()`: for each `RowEdge` whose family is `DatedBy | Fulfills | Typology | NamedAfter` (the only families whose rows carry grounds that are fanned out today — keep exactly these four; do NOT widen), fetch the row's `justification.grounds` by `(family, row_ord)` (`&graph.dated_by[e.row_ord]` etc.), skip when empty, `edge_id = Graph::edge_id_of(&e)`, and `push_grounds` as today. `bible_locus_node_id` and the four private mirrors are deleted (the test `sections_real_data.rs` test (c) re-mints the same ids through public `entry_id` and must stay green — that is the proof the rewire changed nothing). In `sections.rs`, `justified_by_source_family` becomes: parse the relation prefix as today, then `RowFamily::ALL.iter().copied().find(|f| f.relation() == EdgeRel::Directed(rel) && matches!(f, DatedBy|Fulfills|Typology|NamedAfter))` — no hand-written arms.

- [ ] **Step 5: Run to verify GREEN** — from `graph-types/`: `cargo test` (OFF) and `cargo test --features canon-ids` (ON); from `server/`: `cargo test -p atlas-graph --test sections_real_data --test canon_real_data --test version_root_regression --test determinism`. Expected: all pass; `version_root_regression` still `59c6f7dc236f6f23`.

- [ ] **Step 6: Commit**

```bash
git add graph-types/src/graph.rs graph-types/src/canon/rows.rs server/atlas-graph/src/event_world.rs server/atlas-graph/src/sections.rs
git commit -m "refactor(graph-types, graph): DB-2b -- Graph::row_edges() is the one row->edge lowering; build_indexes and add_justified_by consume it; RowFamily::relation is total (RELMAP-1)"
```

---
### Task 3: DDL per section and the LOCUS / RANGE / AUTHORED column codec

**Files:**
- Create: `server/atlas-graph/src/sqlite/ddl.rs`, `server/atlas-graph/src/sqlite/columns.rs`
- Modify: `server/atlas-graph/src/sqlite/mod.rs` (uncomment `pub mod ddl; pub mod columns;`)
- Test: `server/atlas-graph/tests/sqlite_laws.rs` (append)

**Interfaces:**
- Consumes: `Section` (`atlas_graph::sections`), `RowFamily`, `stamp_pragmas`.
- Produces (`ddl.rs`):
  - `pub const COMMON_DDL: &str` — spec §5.1 verbatim: `meta`, `node` (+ `node_by_kind`, `node_by_pid` created by `create_indexes`, not here), `justification`, `ground`, `edge_index` (+ `edge_by_id` in `create_indexes`).
  - `pub fn row_tables_of(section: Section) -> &'static [RowFamily]` — Core: ContainsBible, Attests, Succession, DatedBy, LocatedAt, Fulfills, Typology, NamedAfter, Catechism, Mentions, CorrespondsBible, TemporalAdjacency, Analogue; Kjv: ContainsBible, CanonSuccession, CrossRefs, SpokenBy, SpokenAt; Concord: ContainsConcord, Quotes, Confesses; Kretzmann: CommentsOn; Lexicon: `&[]`.
  - `pub fn family_ddl(f: RowFamily) -> &'static str` — the `CREATE TABLE` (+ sub-table) text of spec §5.3–5.6 for that family, verbatim (`contains_bible` + `contains_bible_locus`; `succession` + `succession_step`; the 14-column RANGE families exactly as the spec spells them). `pub fn family_index_ddl(f: RowFamily) -> &'static str` — its `CREATE UNIQUE INDEX <family>_ord` and any secondary index the spec lists.
  - `pub const SPINE_DDL: &str` — `reading_spine` + `spine_by_node` (kjv and concord only).
  - `pub fn create_tables(conn: &Connection, section: Section) -> Result<(), SqliteError>` — `stamp_pragmas`, then `COMMON_DDL`, then every `family_ddl` of `row_tables_of(section)`, then `SPINE_DDL` when `section` is Kjv or Concord.
  - `pub fn create_indexes(conn: &Connection, section: Section) -> Result<(), SqliteError>` — `node_by_kind`, `node_by_pid`, `edge_by_id`, every `family_index_ddl`, `spine_by_node` — called AFTER inserts (spec §6.1).
  - `pub fn logical_table_order(section: Section) -> Vec<&'static str>` — `["node"]`, then each family's table name in `row_tables_of` order, then `"reading_spine"` for Kjv/Concord. (This IS `LOGICAL_TABLE_ORDER` for the batch; `justification`/`ground`/`edge_index`/`meta` are derived or informational and are NOT in the dump — the rows' canonical bytes already contain their justification.)
- Produces (`columns.rs`) — every function is `pub`:

```rust
/// The seven LOCUS columns, in spec order, under a prefix: `{p}_corpus,
/// {p}_a, {p}_b, {p}_c, {p}_layer, {p}_start, {p}_end`.
pub fn locus_columns(prefix: &str) -> String;            // "att_from_corpus, att_from_a, …" for INSERT column lists
pub fn locus_placeholders() -> &'static str;             // "?, ?, ?, ?, ?, ?, ?"
/// Values to bind for a bible locus: ("bible", book, chapter, verse, layer?, start?, end?).
pub fn bible_locus_values(l: &Locus<BibleTag>) -> [rusqlite::types::Value; 7];
pub fn concord_locus_values(l: &Locus<ConcordTag>) -> [rusqlite::types::Value; 7];
pub fn text_locus_values(l: &TextLocus) -> [rusqlite::types::Value; 7];   // corpus from TextRef
pub fn opt_text_locus_values(l: &Option<TextLocus>) -> [rusqlite::types::Value; 7]; // all Null when None
/// Reading side: from a row, starting at column index `i`.
pub fn read_bible_locus(row: &rusqlite::Row, i: usize) -> Result<Locus<BibleTag>, SqliteError>;   // refuses corpus != "bible"
pub fn read_concord_locus(row: &rusqlite::Row, i: usize) -> Result<Locus<ConcordTag>, SqliteError>;
pub fn read_text_locus(row: &rusqlite::Row, i: usize) -> Result<TextLocus, SqliteError>;
pub fn read_opt_text_locus(row: &rusqlite::Row, i: usize) -> Result<Option<TextLocus>, SqliteError>; // None iff corpus is NULL
pub fn read_bible_range(row: &rusqlite::Row, i: usize) -> Result<LocusRange<BibleTag>, SqliteError>; // 14 columns: from at i, to at i+7; LocusRange::new (fallible)
/// TokenSpan: (layer, start, end) all Some or all None; TokenSpan::new is fallible (start <= end).
/// AUTHORED: `provenance TEXT NOT NULL, justification_id INTEGER`.
pub struct JustificationWriter { next_id: i64 }
impl JustificationWriter {
    pub fn new() -> Self;   // ids start at 1 per section
    /// Writes `justification` + `ground` rows; returns the id, or None when
    /// `j.text.is_none() && j.grounds.is_empty()` (spec: NULL when the family has no justification —
    /// AND, ruled here, when an authored family's row carries an empty one: an empty Justification
    /// round-trips as Justification::default()).
    pub fn write(&mut self, tx: &rusqlite::Transaction, j: &Justification) -> Result<Option<i64>, SqliteError>;
}
pub fn read_justification(conn: &Connection, id: Option<i64>) -> Result<Justification, SqliteError>;
// ground.kind: 0 Scripture | 1 Anchor | 2 Source; Scripture uses the 14 scr_ columns; Anchor/Source use anchor_id/source_id (the erased raw string).
```

- [ ] **Step 1: Write the failing tests** (append to `sqlite_laws.rs`):

```rust
use atlas_graph::sections::Section;
use atlas_graph::sqlite::ddl::{create_indexes, create_tables, logical_table_order, row_tables_of};
use atlas_graph::sqlite::columns::{bible_locus_values, locus_columns, read_bible_locus, read_bible_range, read_justification, JustificationWriter};
use atlas_graph_types::canon::{Canon, RowFamily};
use atlas_graph_types::edge::{Ground, Justification};
use atlas_graph_types::id::{AnchorId, SourceId};
use atlas_graph_types::text::{BibleTag, Locus, LocusRange, TokenSpan, TranslationId, VerseRef};
use std::collections::BTreeSet;

#[test]
fn every_section_schema_creates_in_memory_and_lists_its_tables() {
    for s in Section::MANIFEST_ORDER {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        create_tables(&conn, s).unwrap();
        create_indexes(&conn, s).unwrap();
        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name").unwrap();
        let tables: BTreeSet<String> = stmt.query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect();
        for common in ["meta", "node", "justification", "ground", "edge_index"] { assert!(tables.contains(common), "{s:?} lacks {common}"); }
        for f in row_tables_of(s) { assert!(tables.contains(f.name()), "{s:?} lacks {}", f.name()); }
        assert_eq!(tables.contains("reading_spine"), matches!(s, Section::Kjv | Section::Concord));
        let uv: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(uv, 14);
        assert_eq!(logical_table_order(s)[0], "node");
    }
    // Every family has exactly one home except ContainsBible (core + kjv).
    let mut homes: std::collections::BTreeMap<RowFamily, usize> = Default::default();
    for s in Section::MANIFEST_ORDER { for f in row_tables_of(s) { *homes.entry(*f).or_default() += 1; } }
    for f in RowFamily::ALL { assert_eq!(homes.get(&f).copied().unwrap_or(0), if f == RowFamily::ContainsBible { 2 } else { 1 }, "{f:?}"); }
}

#[test]
fn a_bible_locus_with_a_span_round_trips_through_seven_columns() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(&format!("CREATE TABLE t (id INTEGER PRIMARY KEY, {});", locus_columns("p").replace(",", " ,").replace("p_corpus", "p_corpus TEXT").replace("p_a ", "p_a INTEGER ").replace("p_b ", "p_b INTEGER ").replace("p_c ", "p_c INTEGER ").replace("p_layer", "p_layer TEXT").replace("p_start", "p_start INTEGER").replace("p_end", "p_end INTEGER"))).unwrap();
    let l = Locus::<BibleTag> { unit: VerseRef { book: 43, chapter: 3, verse: 16 }, span: Some(TokenSpan::new(TranslationId("kjv".into()), 2, 5).unwrap()) };
    let v = bible_locus_values(&l);
    conn.execute("INSERT INTO t (p_corpus, p_a, p_b, p_c, p_layer, p_start, p_end) VALUES (?, ?, ?, ?, ?, ?, ?)", rusqlite::params_from_iter(v.iter())).unwrap();
    let back: Locus<BibleTag> = conn.query_row("SELECT p_corpus, p_a, p_b, p_c, p_layer, p_start, p_end FROM t", [], |r| Ok(read_bible_locus(r, 0).unwrap())).unwrap();
    assert_eq!(back.encode(), l.encode(), "canon bytes equal ⇒ the struct is the same struct");
    let whole = Locus::<BibleTag> { unit: VerseRef { book: 1, chapter: 1, verse: 1 }, span: None };
    conn.execute("DELETE FROM t", []).unwrap();
    conn.execute("INSERT INTO t (p_corpus, p_a, p_b, p_c, p_layer, p_start, p_end) VALUES (?, ?, ?, ?, ?, ?, ?)", rusqlite::params_from_iter(bible_locus_values(&whole).iter())).unwrap();
    let back: Locus<BibleTag> = conn.query_row("SELECT p_corpus, p_a, p_b, p_c, p_layer, p_start, p_end FROM t", [], |r| Ok(read_bible_locus(r, 0).unwrap())).unwrap();
    assert_eq!(back.encode(), whole.encode());
}

#[test]
fn a_justification_with_three_ground_kinds_round_trips_and_an_empty_one_is_null() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    create_tables(&conn, Section::Core).unwrap();
    let mut grounds = BTreeSet::new();
    let from = Locus::<BibleTag> { unit: VerseRef { book: 2, chapter: 20, verse: 1 }, span: None };
    let to = Locus::<BibleTag> { unit: VerseRef { book: 2, chapter: 20, verse: 17 }, span: None };
    grounds.insert(Ground::Scripture(LocusRange::new(from, to).unwrap()));
    grounds.insert(Ground::Anchor(AnchorId::new("exodus")));
    grounds.insert(Ground::Source(SourceId::new("ussher")));
    let j = Justification { text: Some("because".into()), grounds };
    let mut conn = conn;
    let (id, none_id) = {
        let tx = conn.transaction().unwrap();
        let mut w = JustificationWriter::new();
        let id = w.write(&tx, &j).unwrap();
        let none_id = w.write(&tx, &Justification::default()).unwrap();
        tx.commit().unwrap();
        (id, none_id)
    };
    assert_eq!(id, Some(1));
    assert_eq!(none_id, None, "an empty justification writes no row and binds NULL");
    let back = read_justification(&conn, id).unwrap();
    assert_eq!(back.encode(), j.encode());
    assert_eq!(read_justification(&conn, None).unwrap().encode(), Justification::default().encode());
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM ground", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 3);
}
```

- [ ] **Step 2: Run to verify RED** — `cargo test -p atlas-graph --test sqlite_laws`. Expected: compile errors (`ddl`, `columns` missing).

- [ ] **Step 3: Implement `ddl.rs`** — paste the spec's DDL per table as `&'static str` constants (one per family, named `DDL_<FAMILY>` / `IDX_<FAMILY>`); `COMMON_DDL` = §5.1 minus the three `CREATE … INDEX` lines, which go in `create_indexes`. `create_tables` runs `stamp_pragmas` FIRST (page_size/encoding only bind on an empty file), then `execute_batch` over the concatenation. Assert in a `#[cfg(test)]` unit test that no DDL string contains `FOREIGN KEY`, `CHECK (` or `TRIGGER` (spec §1.3/§5.0).

- [ ] **Step 4: Implement `columns.rs`** — values: `Value::Text("bible")`, `Value::Integer(book as i64)`, …, span → `Text(layer.0)`, `Integer(start)`, `Integer(end)` else three `Null`. Readers: `row.get::<_, String>(i)?` for corpus, `i64` → `u8`/`u16` with `try_from` (refuse out-of-range with the column name in the error), span: if `layer` is NULL all three must be NULL (refuse a half-span), else `TokenSpan::new(TranslationId(layer), start, end)` mapping `SpanError` into `SqliteError`. `read_bible_range` = two loci + `LocusRange::new`. `JustificationWriter::write`: if `j.text.is_none() && j.grounds.is_empty()` return `Ok(None)`; else `INSERT INTO justification (id, text) VALUES (?, ?)` with `next_id`, then for `(ord, g)` in `grounds.iter().enumerate()` insert one `ground` row (kind 0/1/2; Scripture binds 14 scr_ columns via `bible_locus_values` of `from` and `to`; Anchor binds `anchor_id = a.0` (the erased raw), Source `source_id`). `read_justification(None)` → `Justification::default()`; `Some(id)` → text + grounds `ORDER BY ord` into a `BTreeSet` (assert the BTreeSet's iteration order equals the stored `ord` order — it must, the writer wrote in set order; refuse otherwise so a drift in `Ground: Ord` is caught).

- [ ] **Step 5: Run to verify GREEN** — `cargo test -p atlas-graph --test sqlite_laws`. Expected: 5 passed, no warnings.

- [ ] **Step 6: Commit**

```bash
git add server/atlas-graph/src/sqlite/ddl.rs server/atlas-graph/src/sqlite/columns.rs server/atlas-graph/src/sqlite/mod.rs server/atlas-graph/tests/sqlite_laws.rs
git commit -m "feat(graph): DB-2b -- per-section DDL (spec 5.1, 5.3-5.6) and the LOCUS/RANGE/AUTHORED column codec"
```

---

### Task 4: Row writers and readers for all 21 families

**Files:**
- Create: `server/atlas-graph/src/sqlite/rows/mod.rs`, `rows/core.rs`, `rows/kjv.rs`, `rows/concord.rs`, `rows/kretzmann.rs`
- Modify: `server/atlas-graph/src/sqlite/mod.rs` (uncomment `pub mod rows;`)
- Test: `server/atlas-graph/tests/sqlite_laws.rs` (append)

**Interfaces:**
- Consumes: Task 3's DDL/columns; `Canon` for every row (DB-2a) — the tests compare `encode()` bytes, never `Debug`.
- Produces (`rows/mod.rs`):

```rust
/// One row of one family, borrowed from the Graph (the writer's input shape).
pub enum RowRef<'a> {
    ContainsBible(&'a Contains<BibleTag>), ContainsConcord(&'a Contains<ConcordTag>), Attests(&'a Attests),
    Succession(&'a Succession), CanonSuccession(&'a CanonSuccession), DatedBy(&'a DatedBy), LocatedAt(&'a LocatedAt),
    Fulfills(&'a Fulfills), Typology(&'a Typology), NamedAfter(&'a NamedAfter), Catechism(&'a CatechismLink),
    Mentions(&'a Mentions), CorrespondsBible(&'a Corresponds<BibleTag>), TemporalAdjacency(&'a TemporalAdjacency),
    Analogue(&'a Analogue), CrossRefs(&'a CrossRef), SpokenBy(&'a SpokenBy), SpokenAt(&'a SpokenAt),
    Quotes(&'a Quotes), Confesses(&'a Confesses), CommentsOn(&'a CommentsOn),
}
impl<'a> RowRef<'a> { pub fn family(&self) -> RowFamily; pub fn to_value(&self) -> Value; /* delegates to Canon::to_value */ }

/// Owned rows read back (the reader's output shape) — same 21 arms, owned.
pub enum RowOwned { /* … */ }
impl RowOwned { pub fn family(&self) -> RowFamily; pub fn to_value(&self) -> Value; }

/// Write one row at global ord `ord` with primary key `id = ord` (the id IS the ord in this batch —
/// one INTEGER PRIMARY KEY, one UNIQUE(ord); they coincide by construction and the reader asserts it).
pub fn insert_row(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &RowRef) -> Result<(), SqliteError>;
/// Read every row of `family` from this section in `ORDER BY id`, with its ord.
pub fn read_rows(conn: &Connection, family: RowFamily) -> Result<Vec<(i64, RowOwned)>, SqliteError>;
```

  Per-section files hold the `fn insert_<family>` / `fn read_<family>` pairs for the families `row_tables_of(section)` names (ContainsBible lives in `core.rs` and is reused by kjv). Each insert binds exactly the spec's columns in the spec's order; each read `SELECT`s the same column list and reconstructs the struct through the fallible constructors (`LocusRange::new`, `TokenSpan::new`, `Succession::new`, `Year::new` inside `DatePlacement`'s `Duration` — `Duration` has no ctor; `PlacementBasis` 0/1; `DatePlacement` kind 0..3 with the columns §5.3 lists; `Namesake` kind 0/1/2; `MentionedEntity` kind 0..3; `ContainerContent` = `child_container_id` non-NULL ⇒ `Container`, else `Loci` from `<family>_locus ORDER BY ord` into a `LocusSet`).

- [ ] **Step 1: Write the failing test** (append to `sqlite_laws.rs`) — a specimen per family, written then read, bytes compared:

```rust
use atlas_graph::sqlite::rows::{insert_row, read_rows, RowRef};

/// One hand-built row per family plus the nodes and spines the rows
/// refer to. The 21 row constructions are COPIED VERBATIM from the golden
/// test `graph-types/tests/canon_row_vectors.rs` (DB-2a Task 2 + final
/// fix item 14: one pinned specimen per family, every optional field
/// populated — a span on a locus, `to_last` on the cross-ref, `note` on
/// the typology, a `Container` child on one contains row and a `Loci`
/// set on a second, text + three ground kinds on every authored
/// justification), so their canonical bytes are ALREADY pinned there.
/// Add: a second `contains_bible` row whose `content` is `Loci` of two
/// verses (the goldens carry only one shape per family) and a second
/// `contains_concord` row likewise; and these nodes/spines:
fn specimen_graph() -> atlas_graph_types::graph::Graph {
    use atlas_graph_types::graph::{Graph, ReadingSpine};
    use atlas_graph_types::id::{AnyNodeId, NodeKind};
    use atlas_graph_types::node::{Node, NodePayload};
    let mut g = Graph::default();
    // 21 rows: paste from canon_row_vectors.rs here (one push per family), then the two extra contains rows.
    /* rows */
    // Nodes every specimen row can reach from an edge endpoint (ids must
    // match the rows' erased ids exactly): two bible verses, one concord
    // paragraph, one event, one place, one anchor, one era, one source,
    // one catechism item, one commentary item, one bible chapter
    // container, one curated container ("passage-…"), one concord container.
    let node = |kind: NodeKind, raw: &str, payload: NodePayload| Node { id: AnyNodeId { kind, raw: raw.to_string() }, payload, provenance: "prov".to_string() };
    for n in [
        node(NodeKind::TextUnit, "bible/1.1.1", NodePayload::TextUnit { corpus: "bible", renderings: [(atlas_graph_types::text::TranslationId("kjv".into()), "In the beginning".to_string())].into_iter().collect() }),
        node(NodeKind::TextUnit, "bible/1.1.2", NodePayload::TextUnit { corpus: "bible", renderings: [(atlas_graph_types::text::TranslationId("kjv".into()), "And the earth".to_string())].into_iter().collect() }),
        node(NodeKind::TextUnit, "concord/1.1.1", NodePayload::TextUnit { corpus: "concord", renderings: [(atlas_graph_types::text::TranslationId("en".into()), "We believe".to_string())].into_iter().collect() }),
        node(NodeKind::Container, "bible-chapter-GEN-1", NodePayload::Container { title: "Genesis 1".into() }),
        node(NodeKind::Container, "passage-creation", NodePayload::Container { title: "Creation".into() }),
        node(NodeKind::Container, "concord-ac-i", NodePayload::Container { title: "Article I".into() }),
    ] { g.nodes.insert(n.id.clone(), n); }
    // Event/Place/Anchor/Era/Source/CatechismItem/CommentaryItem/Person/PeopleGroup/Polity/Narrative nodes: one each, with
    // the SAME raw ids the pasted rows use (read them off the pasted rows; e.g. the LocatedAt specimen's `event`/`place`).
    /* nodes for the row endpoints */
    g.reading.insert("bible", ReadingSpine { order: vec![AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() }, AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.2".into() }] });
    g.reading.insert("concord", ReadingSpine { order: vec![AnyNodeId { kind: NodeKind::TextUnit, raw: "concord/1.1.1".into() }] });
    g
}

#[test]
fn every_family_round_trips_through_its_columns_with_identical_canon_bytes() {
    let g = specimen_graph();
    for s in [Section::Core, Section::Kjv, Section::Concord, Section::Kretzmann] {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        create_tables(&conn, s).unwrap();
        let rows: Vec<RowRef> = atlas_graph::sqlite::partition::rows_of_section(&g, s); // Task 5 adds this; here call the per-family Vec directly:
        // e.g. for Core: g.attests.iter().map(RowRef::Attests) … chained across row_tables_of(s)
        let tx = conn.transaction().unwrap();
        let mut jw = JustificationWriter::new();
        for (i, r) in rows.iter().enumerate() { insert_row(&tx, &mut jw, i as i64, r).unwrap(); }
        tx.commit().unwrap();
        create_indexes(&conn, s).unwrap();
        for f in row_tables_of(s) {
            let written: Vec<Vec<u8>> = rows.iter().filter(|r| r.family() == *f).map(|r| encode_row_in_family(*f, r.to_value())).collect();
            let read: Vec<Vec<u8>> = read_rows(&conn, *f).unwrap().into_iter().map(|(_, r)| encode_row_in_family(*f, r.to_value())).collect();
            assert!(!written.is_empty(), "{s:?}/{f:?}: the specimen graph must inhabit every family");
            assert_eq!(read, written, "{s:?}/{f:?}");
        }
    }
}
```

  (Replace the `partition::rows_of_section` line with the explicit per-family chain for this task; Task 5 swaps it for the real call.)

- [ ] **Step 2: Run to verify RED** — `cargo test -p atlas-graph --test sqlite_laws every_family`. Expected: compile error (`rows` missing).

- [ ] **Step 3: Implement** the five files. Rules: one `const COLS_<FAMILY>: &str` per family reused by INSERT and SELECT (so the two can never disagree); `insert_row` matches on `RowRef` and dispatches; `read_rows` matches on `RowFamily`. Sub-tables: `contains_*_locus` (`contains_id = id`, `ord` = set order, seven columns a,b,c,layer,start,end_ — note the spec's `end_`), `succession_step` (`succession_id`, `ord`, `event_id`). Provenance is the raw `String`; ids are the erased raw strings (`x.0` / `erase().raw`), never `any_node_id_str` — these columns are typed by the family (spec §5.0 "node ids are TEXT in their existing string form").

- [ ] **Step 4: Run to verify GREEN** — `cargo test -p atlas-graph --test sqlite_laws`. Expected: 6 passed, no warnings.

- [ ] **Step 5: Commit**

```bash
git add server/atlas-graph/src/sqlite/rows server/atlas-graph/src/sqlite/mod.rs server/atlas-graph/tests/sqlite_laws.rs
git commit -m "feat(graph): DB-2b -- row writers and readers for all 21 families, proven by canon-byte round-trip"
```

---
### Task 5: The partition, the manifest, and the writer

**Files:**
- Create: `server/atlas-graph/src/sqlite/partition.rs`, `server/atlas-graph/src/sqlite/manifest.rs`, `server/atlas-graph/src/sqlite/writer.rs`
- Modify: `server/atlas-graph/src/sqlite/mod.rs` (uncomment the three `pub mod`)
- Test: `server/atlas-graph/tests/sqlite_laws.rs` (append)

**Interfaces:**
- Consumes: `Graph::row_edges()`, `Graph::edge_id_of` (Task 2); `section_of_node`, `section_of_contains_bible`, `section_of_family`, `section_of_justified_by`, `justified_by_source_family` (DB-2a); Tasks 3–4; `position_str`, `any_node_id_str` (canon::ids); `RelationId::ALL` / `SymRelationId::ALL` ordinals (index in `ALL`).
- Produces (`partition.rs`):

```rust
pub struct EdgeEntryOut {
    pub subject: Position, pub rel: i64 /* RelationId ordinal, or 128 + SymRelationId ordinal */, pub dir: i64 /* 0 fwd | 1 inv | 2 sym */,
    pub ord: i64 /* index within the in-memory (subject, rel, dir) Vec */, pub object: Position, pub edge_id: EdgeId,
    pub meta: EdgeMeta, pub row_family: RowFamily, pub row_id: i64 /* the row's global ord */,
}
pub struct SectionPartition<'a> {
    pub section: Section,
    pub nodes: Vec<&'a Node>,                                  // in AnyNodeId order (BTreeMap order)
    pub rows: Vec<(RowFamily, i64 /* global ord */, RowRef<'a>)>, // family by row_tables_of(section) order, then ord ascending
    pub edges: Vec<EdgeEntryOut>,                              // in (subject, rel, dir, ord) order
    pub spine: Option<(&'static str, &'a [AnyNodeId])>,        // ("bible", …) for Kjv, ("concord", …) for Concord
}
/// Global ord = the row's index in its family Vec (`row_ord` of Task 2) — the same number for the
/// core and kjv halves of contains_bible, so a row's ord is unique within its family across sections.
pub fn rows_of_section<'a>(g: &'a Graph, s: Section) -> Vec<(RowFamily, i64, RowRef<'a>)>;
pub fn partition(g: &'a Graph) -> Result<Vec<SectionPartition<'a>>, SqliteError>;   // one per Section::MANIFEST_ORDER minus Lexicon
/// The edge-id → (family, ord, container raw) map every JustifiedBy entry resolves through.
pub fn edge_row_map(g: &Graph) -> BTreeMap<EdgeId, (RowFamily, i64, Option<String>)>;
```

  Edge placement rule: for every `(rel, BiIndex)` in `g.indexes`, every `(subject, entries)` in `fwd` and `inv`, every `(i, (edge_id, object, meta))`: if `rel == JustifiedBy`, the entry's row is `edge_row_map[&subject_edge_id]` and its section is `section_of_justified_by(family, container_raw)`; otherwise `edge_row_map[edge_id]` gives `(family, ord, raw)` and the section is `section_of_contains_bible`-or-`section_of_family` (through `section_of_justified_by(family, raw)`, which encodes exactly that rule). Symmetric indexes: `dir = 2`, both `fwd` sides. `ord = i`. If an edge id is missing from the map, `partition` returns `Err` naming it (never a silent Core).
- Produces (`manifest.rs`, serde derive):

```rust
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ManifestSection { pub name: String, pub required: bool, pub logical: String, pub blob: String, pub bytes: u64, pub schema_version: u32 }
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Manifest { pub schema: u32, pub compiler: String, pub built: String, pub root: String, #[serde(rename = "section")] pub sections: Vec<ManifestSection> }
pub const MANIFEST_SCHEMA: u32 = 1;
pub fn manifest_lines(sections: &[ManifestSection]) -> Vec<u8>;   // "name|logical|schema_version|required\n" each, manifest order
pub fn root_of(sections: &[ManifestSection]) -> String;           // hex of sha256_prefixed_128(DOMAIN_PREFIX, manifest_lines)
pub fn write_manifest(m: &Manifest, path: &Path) -> Result<(), SqliteError>;   // toml::to_string_pretty
pub fn read_manifest(path: &Path) -> Result<Manifest, SqliteError>;            // and verifies root_of(sections) == root, else Err
```

- Produces (`writer.rs`):

```rust
pub struct WrittenSection { pub section: Section, pub path: PathBuf, pub logical: String, pub blob: String, pub bytes: u64, pub node_count: usize, pub row_count: usize, pub edge_count: usize, pub elapsed: std::time::Duration }
/// Writes `<out_dir>/<name>.<logical>.sqlite` for each partition (a temp name first, renamed after the
/// logical hash is known), then `<out_dir>/manifest.toml`; deletes stale `<name>.*.sqlite` files first.
pub fn write_sections(g: &Graph, graph_version_hex: &str, compiler: &str, out_dir: &Path) -> Result<(Manifest, Vec<WrittenSection>), SqliteError>;
```

  Per section (spec §6.1 step 2): open `Connection::open(tmp)`, `create_tables`, one transaction: `node` rows (`id = any_node_id_str`, `kind = NodeKind ordinal` (index in the enum's declaration order — write `node_kind_ordinal(k)` as an exhaustive match in `partition.rs`), `pid = hash_bytes(&n.pid().hash)`, `label`, `provenance`, `payload = n.encode()`), then rows via `insert_row` (with `JustificationWriter`), then `edge_index` (`subject/object = position_str`, `edge_id = hash_bytes(&hash parsed from the EdgeId string after ':' via ContentHash::from_hex)`, `meta_kind/meta_narrative/meta_votes`, `row_family = family.ordinal()`, `row_id`), then `reading_spine` when present; commit; `create_indexes`; `meta` rows (`section_name`, `schema_version`, `logical_hash`, `compiler`, `canon_version` = `CANON_VERSION`, `built` = RFC-3339 now, `hash_width`, `graph_version` = `graph_version_hex`); `VACUUM`; close; `logical` = Task 6's `logical_hash(&logical_dump_of_partition(&p))` — computed BEFORE the meta rows so it can be stamped; `blob` = hex of `sha256` (full 32 bytes) over the file bytes after close; rename to the final name.

- [ ] **Step 1: Write the failing tests** (append):

```rust
use atlas_graph::sqlite::manifest::{read_manifest, root_of, Manifest, ManifestSection};
use atlas_graph::sqlite::partition::{edge_row_map, partition};
use atlas_graph::sqlite::writer::write_sections;

#[test]
fn every_index_entry_of_the_specimen_lands_in_exactly_one_section_and_names_its_row() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let parts = partition(&g).unwrap();
    let total: usize = parts.iter().map(|p| p.edges.len()).sum();
    let in_memory: usize = g.indexes.values().map(|ix| ix.fwd.values().map(Vec::len).sum::<usize>() + ix.inv.values().map(Vec::len).sum::<usize>()).sum::<usize>()
        + g.symmetric_indexes.values().map(|ix| ix.fwd.values().map(Vec::len).sum::<usize>()).sum::<usize>();
    assert_eq!(total, in_memory, "no entry lost, none duplicated");
    let map = edge_row_map(&g);
    for p in &parts { for e in &p.edges {
        let (fam, ord, _) = &map[&e.edge_id];
        if e.rel != atlas_graph_types::edge::RelationId::ALL.iter().position(|r| *r == atlas_graph_types::edge::RelationId::JustifiedBy).unwrap() as i64 {
            assert_eq!((e.row_family, e.row_id), (*fam, *ord));
        }
        assert_eq!(atlas_graph::sections::section_of_justified_by(e.row_family, map[&e.edge_id].2.as_deref()), p.section);
    } }
    assert!(parts.iter().any(|p| p.section == Section::Core && p.rows.iter().any(|(f, _, _)| *f == RowFamily::ContainsBible)), "curated container row in core");
    assert!(parts.iter().any(|p| p.section == Section::Kjv && p.rows.iter().any(|(f, _, _)| *f == RowFamily::ContainsBible)), "chapter container row in kjv");
}

#[test]
fn the_manifest_round_trips_and_its_root_is_over_the_section_lines_only() {
    let s = |name: &str, req: bool, logical: &str| ManifestSection { name: name.into(), required: req, logical: logical.into(), blob: "00".repeat(32), bytes: 1, schema_version: 14 };
    let sections = vec![s("core", true, &"a".repeat(32)), s("kjv", true, &"b".repeat(32)), s("concord", false, &"c".repeat(32))];
    let root = root_of(&sections);
    assert_eq!(root.len(), 32);
    let m = Manifest { schema: 1, compiler: "test".into(), built: "2026-09-17T00:00:00Z".into(), root: root.clone(), sections: sections.clone() };
    let dir = std::env::temp_dir().join(format!("db2b-manifest-{}", std::process::id())); std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("manifest.toml");
    atlas_graph::sqlite::manifest::write_manifest(&m, &path).unwrap();
    assert_eq!(read_manifest(&path).unwrap(), m);
    let mut later = m.clone(); later.built = "2030-01-01T00:00:00Z".into(); later.sections[0].bytes = 999;
    assert_eq!(root_of(&later.sections), root, "timestamps and byte sizes are outside the root (spec §2.2)");
    let mut tampered = m.clone(); tampered.sections[1].logical = "d".repeat(32);
    atlas_graph::sqlite::manifest::write_manifest(&tampered, &path).unwrap();
    assert!(read_manifest(&path).is_err(), "a manifest whose root does not recompute is refused (spec §11)");
}

#[test]
fn the_writer_produces_four_files_named_by_logical_hash_and_a_manifest_in_order() {
    let mut g = specimen_graph(); g.build_indexes(); atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-writer-{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir);
    let (m, written) = write_sections(&g, "0000000000000000", "test", &dir).unwrap();
    assert_eq!(m.sections.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["core", "kjv", "concord", "kretzmann"]);
    for (w, ms) in written.iter().zip(&m.sections) {
        assert_eq!(w.path.file_name().unwrap().to_str().unwrap(), format!("{}.{}.sqlite", ms.name, ms.logical));
        assert_eq!(ms.blob.len(), 64); assert_eq!(ms.bytes, std::fs::metadata(&w.path).unwrap().len());
        assert_eq!(ms.required, matches!(w.section, Section::Core | Section::Kjv));
    }
    assert_eq!(read_manifest(&dir.join("manifest.toml")).unwrap(), m);
    let (m2, _) = write_sections(&g, "0000000000000000", "test", &dir).unwrap();
    assert_eq!(m2.root, m.root, "a rewrite of identical content has an identical root");
    assert_eq!(m2.sections.iter().map(|s| &s.logical).collect::<Vec<_>>(), m.sections.iter().map(|s| &s.logical).collect::<Vec<_>>());
}
```

- [ ] **Step 2: Run to verify RED** — `cargo test -p atlas-graph --test sqlite_laws`. Expected: compile errors.

- [ ] **Step 3: Implement** the three files as specified. `partition` iterates `Section::MANIFEST_ORDER` skipping `Lexicon` (no tables yet; its absence from the manifest is honest — the manifest lists shipped sections). The `rel` ordinal: `RelationId::ALL.iter().position(..)` cached in a small array at first use; symmetric `128 + position`.

  This task ALSO creates `logical.rs` with the two partition-side functions the writer needs — `logical_dump_of_partition(&SectionPartition) -> Vec<u8>` and `logical_hash(&[u8]) -> String` — exactly as Task 6's interface block specifies them (Task 6 adds only `logical_dump_of_db`). Uncomment `pub mod logical;` here.

- [ ] **Step 4: Run to verify GREEN** — `cargo test -p atlas-graph --test sqlite_laws`. Expected: 9 passed, no warnings.

- [ ] **Step 5: Commit**

```bash
git add server/atlas-graph/src/sqlite/partition.rs server/atlas-graph/src/sqlite/manifest.rs server/atlas-graph/src/sqlite/writer.rs server/atlas-graph/src/sqlite/logical.rs server/atlas-graph/src/sqlite/mod.rs server/atlas-graph/tests/sqlite_laws.rs
git commit -m "feat(graph): DB-2b -- section partition (spec 2.1), manifest + root (spec 2.2), the section writer (spec 6.1) with the logical hash from the in-memory partition"
```

---

### Task 6: The logical dump from the SQLite file, and its agreement with the partition

**Files:**
- Modify: `server/atlas-graph/src/sqlite/logical.rs` (add `logical_dump_of_db`)
- Test: `server/atlas-graph/tests/sqlite_laws.rs` (append)

**Interfaces:**
- Consumes: `read_rows` (Task 4), `logical_table_order` (Task 3), `Node::decode`, `encode_row_in_family`, `canon::{obj, serialize, str_value, Value}`.
- Produces:

```rust
/// Spec §3.4: for each table in `logical_table_order(section)`, for each row in primary-key order,
/// `<table>\t<canonical row JSON>\n`. From the in-memory partition (Task 5)…
pub fn logical_dump_of_partition(p: &SectionPartition) -> Vec<u8>;
/// …and recomputed from an open section file by streaming its tables (`node ORDER BY id`,
/// `<family> ORDER BY id` → `read_rows`, `reading_spine ORDER BY ord`). The two MUST agree (the gate).
pub fn logical_dump_of_db(conn: &Connection, section: Section) -> Result<Vec<u8>, SqliteError>;
pub fn logical_hash(dump: &[u8]) -> String;   // hex of sha256_prefixed_128(DOMAIN_PREFIX, dump)
```

  Line bodies: `node` → `n.encode()` (partition) / the `payload` BLOB verbatim, after `Node::decode(payload)` succeeds AND `any_node_id_str(&decoded.id) == id` column (db); row table → `encode_row_in_family(family, row.to_value())` both sides; `reading_spine` → `serialize(&obj(vec![("corpus", str_value(corpus)), ("node_id", str_value(&any_node_id_str(id))), ("ord", Value::Int(ord))]))` both sides (`obj` sorts keys). Node order: `ORDER BY id` in SQLite is byte order on TEXT = `AnyNodeId`'s `Ord`? NOT necessarily — `AnyNodeId` derives `Ord` on `(kind, raw)`, while `id` TEXT sorts by `"Kind:raw"` bytes. Rule: the partition side sorts nodes by `any_node_id_str` (byte order) too, so both sides use the same key — state this in the doc comment.

- [ ] **Step 1: Write the failing tests**:

```rust
use atlas_graph::sqlite::logical::{logical_dump_of_db, logical_dump_of_partition, logical_hash};

#[test]
fn the_logical_dump_recomputed_from_each_written_file_equals_the_partitions_dump() {
    let mut g = specimen_graph(); g.build_indexes(); atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-logical-{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir);
    let (m, written) = write_sections(&g, "0000000000000000", "test", &dir).unwrap();
    let parts = partition(&g).unwrap();
    for (p, w) in parts.iter().zip(&written) {
        let from_mem = logical_dump_of_partition(p);
        let conn = open_read_only(&w.path).unwrap();
        let from_db = logical_dump_of_db(&conn, p.section).unwrap();
        assert_eq!(String::from_utf8_lossy(&from_db), String::from_utf8_lossy(&from_mem), "{:?}", p.section);
        assert_eq!(logical_hash(&from_db), m.sections.iter().find(|s| s.name == p.section.name()).unwrap().logical);
        let stamped: String = conn.query_row("SELECT value FROM meta WHERE key = 'logical_hash'", [], |r| r.get(0)).unwrap();
        assert_eq!(stamped, logical_hash(&from_mem));
        assert!(from_mem.starts_with(b"node\t{\"id\":\""), "first line is a node line");
    }
}

#[test]
fn a_changed_row_changes_the_logical_hash_and_a_changed_timestamp_does_not() {
    let mut g = specimen_graph(); g.build_indexes(); atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-logical2-{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir);
    let (m1, _) = write_sections(&g, "0000000000000000", "test", &dir).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let (m2, _) = write_sections(&g, "0000000000000000", "test", &dir).unwrap();
    assert_eq!(m1.root, m2.root);
    g.located_at[0].provenance = "another-source".into();
    let (m3, _) = write_sections(&g, "0000000000000000", "test", &dir).unwrap();
    assert_ne!(m3.sections[0].logical, m1.sections[0].logical, "core moved");
    assert_eq!(m3.sections[1].logical, m1.sections[1].logical, "kjv did not");
    assert_ne!(m3.root, m1.root);
}
```

- [ ] **Step 2: RED** — `cargo test -p atlas-graph --test sqlite_laws logical`. Expected: `logical_dump_of_db` undefined.

- [ ] **Step 3: Implement `logical_dump_of_db`** — for each table in `logical_table_order(section)`: `node` → `SELECT id, payload FROM node ORDER BY id`, decode-check, push `node\t` + payload + `\n`; family → `read_rows` then encode; `reading_spine` → `SELECT ord, node_id FROM reading_spine ORDER BY ord` with the corpus from `meta.section_name` (kjv → "bible", concord → "concord").

- [ ] **Step 4: GREEN** — `cargo test -p atlas-graph --test sqlite_laws`. Expected: 11 passed, no warnings.

- [ ] **Step 5: Commit**

```bash
git add server/atlas-graph/src/sqlite/logical.rs server/atlas-graph/tests/sqlite_laws.rs
git commit -m "feat(graph): DB-2b -- the logical dump recomputed from the SQLite tables agrees with the in-memory partition (spec 3.4)"
```

---
### Task 7: `SqliteSnapshot` — the read port over the attached sections

**Files:**
- Create: `server/atlas-graph/src/sqlite/snapshot.rs`
- Modify: `server/atlas-graph/src/sqlite/mod.rs` (uncomment `pub mod snapshot;`)
- Test: `server/atlas-graph/tests/sqlite_laws.rs` (append)

**Interfaces:**
- Consumes: `read_manifest`, `open_read_only`, `hash_bytes`/`hash_from_bytes`, `parse_position`/`position_str`, `parse_any_node_id`/`any_node_id_str`, `Node::decode`, `RelationId::ALL`/`SymRelationId::ALL`, `EdgeKind`, `EdgeMeta`, `EdgeEntry`, `EdgePage`, `EdgeSummary`, `GraphVersion`, `ContentHash::from_hex`.
- Produces:

```rust
pub struct SqliteSnapshot { conn: std::sync::Mutex<rusqlite::Connection>, version: GraphVersion, present: Vec<Section> /* manifest order, attached */ }
impl SqliteSnapshot {
    /// Spec §2.5 steps 1–4: read+verify the manifest; open `core` as main; ATTACH every other
    /// present section under its name in manifest order; `PRAGMA query_only = ON`; build the TEMP
    /// views `all_edge_index` and `all_node` over the attached sections. A `required` section
    /// whose file is missing is an error; an optional one is recorded absent and skipped.
    pub fn open(manifest_path: &Path) -> Result<SqliteSnapshot, SqliteError>;
    pub fn present(&self) -> &[Section];
}
impl GraphQuery for SqliteSnapshot { /* the five methods */ }
impl GraphSnapshot for SqliteSnapshot { fn version(&self) -> GraphVersion { self.version } }
```

  Method contracts (each must equal the in-memory answer exactly — `assert_answers_match` is the judge):
  - `node(id)`: `SELECT payload FROM all_node WHERE id = ?1` with `any_node_id_str(id)`; `Node::decode` → `Some(node)`; no row → `None`.
  - `derive(pid)`: `SELECT payload FROM all_node WHERE pid = ?1` with `hash_bytes(&pid.hash)`; decode; return `Some(node.canonical_bytes())` only if `node.pid() == *pid` (the kind must match too — `PositionKind` is not stored, so re-derive and compare); else `None`.
  - `edge_summary(p)`: `SELECT rel, dir, COUNT(*) FROM all_edge_index WHERE subject = ?1 GROUP BY rel, dir`; map `(rel, dir)` → `EdgeKind::Directed(RelationId::ALL[rel], Forward|Inverse)` or `EdgeKind::Symmetric(SymRelationId::ALL[rel - 128])` (dir 2); counts > 0 only (GROUP BY never yields 0).
  - `edges(p, q)`: `SELECT object, edge_id, rel, meta_kind, meta_narrative, meta_votes FROM all_edge_index WHERE subject = ?1 AND rel = ?2 AND dir = ?3 ORDER BY ord LIMIT ?4 OFFSET ?5` with `limit = q.limit`, `offset = q.cursor.unwrap_or(0)`; `EdgeId` = `format!("{:?}:{}", relation, hash_from_bytes(edge_id)?.hex())` where `relation` is the `RelationId`/`SymRelationId` Debug name (this reproduces `entry_id`'s string exactly — Task 5's writer stored the hash parsed from that same string); `next` = `Some(offset + entries.len())` iff `offset + entries.len() < total` where `total` = `SELECT COUNT(*) … WHERE subject AND rel AND dir` (one extra count query — matches `explore.rs:132-142` exactly, including `limit = 0` ⇒ `next = Some(offset)` when any entries exist). A position with no rows → empty page, `next: None`.
  - `reading_window(corpus, start, n)`: `SELECT node_id FROM <kjv|concord>.reading_spine WHERE ord >= ?1 ORDER BY ord LIMIT ?2` (`bible` → `kjv`, `concord` → `concord`; an unknown corpus or an absent section → empty Vec); `parse_any_node_id` each.
  - `version()`: `meta.graph_version` of `core`, `ContentHash::from_hex` → `GraphVersion`.

- [ ] **Step 1: Write the failing tests**:

```rust
use atlas_graph::sqlite::snapshot::SqliteSnapshot;
use atlas_graph_types::store::{assert_answers_match, GraphQuery, GraphSnapshot};

#[test]
fn the_sqlite_snapshot_answers_every_port_question_exactly_as_the_specimen_graph() {
    let mut g = specimen_graph(); g.build_indexes(); atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-snap-{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, "0123456789abcdef", "test", &dir).unwrap();
    let snap = SqliteSnapshot::open(&dir.join("manifest.toml")).unwrap();
    assert_eq!(snap.present(), &[Section::Core, Section::Kjv, Section::Concord, Section::Kretzmann]);
    assert_answers_match(&snap, &g);
    assert_eq!(snap.version().0.hex(), "0123456789abcdef");
}

#[test]
fn an_absent_optional_section_is_recorded_and_its_kinds_are_simply_uninhabited() {
    let mut g = specimen_graph(); g.build_indexes(); atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-absent-{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir);
    let (m, written) = write_sections(&g, "0123456789abcdef", "test", &dir).unwrap();
    std::fs::remove_file(&written.iter().find(|w| w.section == Section::Kretzmann).unwrap().path).unwrap();
    let snap = SqliteSnapshot::open(&dir.join("manifest.toml")).unwrap();
    assert_eq!(snap.present(), &[Section::Core, Section::Kjv, Section::Concord]);
    let item = g.comments_on[0].item.erase();
    assert!(snap.node(&item).is_none(), "the CommentaryItem node lives only in kretzmann");
    let verse_pos = atlas_graph_types::edge::at(&g.reading["bible"].order[0]);
    let kinds = snap.edge_summary(&verse_pos);
    assert!(!kinds.keys().any(|k| matches!(k, atlas_graph_types::edge::EdgeKind::Directed(atlas_graph_types::edge::RelationId::CommentsOn, _))));
    std::fs::remove_file(&written.iter().find(|w| w.section == Section::Kjv).unwrap().path).unwrap();
    let err = SqliteSnapshot::open(&dir.join("manifest.toml")).unwrap_err();
    assert!(err.0.contains("kjv") && err.0.contains(&m.sections[1].logical), "a missing REQUIRED section is refused by name and hash (spec §11)");
}

#[test]
fn paging_semantics_match_explore_rs_at_every_cursor_and_limit() {
    let mut g = specimen_graph(); g.build_indexes(); atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-paging-{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, "0123456789abcdef", "test", &dir).unwrap();
    let snap = SqliteSnapshot::open(&dir.join("manifest.toml")).unwrap();
    // The container with a Loci set of two verses has 2 Contains entries: walk every (cursor, limit) in 0..=3.
    let container = atlas_graph_types::edge::at(&g.contains_bible.iter().find(|c| matches!(c.content, atlas_graph_types::edge::ContainerContent::Loci(_))).unwrap().container.erase());
    let kind = atlas_graph_types::edge::EdgeKind::Directed(atlas_graph_types::edge::RelationId::Contains, atlas_graph_types::edge::Direction::Forward);
    for cursor in [None, Some(0), Some(1), Some(2), Some(3)] { for limit in 0..=3 {
        let q = atlas_graph_types::explore::EdgeQuery { kind, cursor, limit };
        assert_eq!(snap.edges(&container, &q), g.edges(&container, &q), "cursor {cursor:?} limit {limit}");
    } }
}
```

- [ ] **Step 2: RED** — `cargo test -p atlas-graph --test sqlite_laws snapshot`. Expected: `snapshot` module missing.

- [ ] **Step 3: Implement** — `open`: `read_manifest` (root verified inside); resolve each section's path as `<manifest dir>/<name>.<logical>.sqlite`; core missing → `Err("required section core (<logical>) missing at <path>")`; open core via `open_read_only`; for each remaining present section `ATTACH DATABASE ?1 AS <name>` (path bound; `?mode=ro` is implied by the connection's read-only flag — verify with `PRAGMA <name>.query_only`); optional missing → skip and record; required missing → Err; build `CREATE TEMP VIEW all_edge_index AS SELECT 0 AS sec, * FROM main.edge_index UNION ALL SELECT <rank>, * FROM <name>.edge_index …` and `all_node` likewise (only attached sections). Prepared-statement caching: use `conn.prepare_cached` for the five hot statements. `version`: read once at open.

- [ ] **Step 4: GREEN** — `cargo test -p atlas-graph --test sqlite_laws`. Expected: 14 passed, no warnings.

- [ ] **Step 5: Commit**

```bash
git add server/atlas-graph/src/sqlite/snapshot.rs server/atlas-graph/src/sqlite/mod.rs server/atlas-graph/tests/sqlite_laws.rs
git commit -m "feat(graph): DB-2b -- SqliteSnapshot: GraphQuery + GraphSnapshot over the attached sections (spec 2.5, 5.2), admitted on the specimen graph"
```

---

### Task 8: The real-data gate — the compile binary writes sections and admits the SQLite backend

**Files:**
- Modify: `server/atlas-graph/src/bins/compile_graph.rs` (args at lines 46–67; after the "ADMISSION passed for the encoded bytes" line ~261)
- Create: `server/atlas-graph/tests/sqlite_real_data.rs`
- Modify: `scripts/timing-gates.sh` (add gate 9), `server/BENCHMARKS.md` (append a DB-2b section)

**Interfaces:**
- Consumes: everything above; `committed_graph()` pattern from `server/atlas-graph/tests/canon_real_data.rs` (loads `data/compiled/graph.bin` once via `OnceLock`, `build_indexes` + `add_justified_by`); `MemStore::publish` → `version_hex` (as `compile_graph.rs:303-305` does).
- Produces: `--sections-out <dir>` on `atlas-graph-compile` (default: `<out_path parent>/../cache/sections-build`, i.e. `data/cache/sections-build` for the documented `--out data/compiled/graph.bin`); gate 9.

- [ ] **Step 1: Write the failing real-data test**:

```rust
// server/atlas-graph/tests/sqlite_real_data.rs
//! DB-2b's gate (spec §6.2) over the COMMITTED graph: write the four
//! sections, open them through SqliteSnapshot, and prove (a) every port
//! answer over the full position inventory equals the in-memory graph's,
//! (b) each section's logical hash recomputed from its tables equals the
//! one computed from the partition and stamped in the manifest, (c) two
//! writes are identical in every logical hash and in the root. Wall-clock
//! gate: `#[ignore]`d and run serialized by scripts/timing-gates.sh (gate 9),
//! the CONTENTION-1 mechanism — never in the parallel workspace run.
use std::sync::OnceLock;
use std::time::Instant;
use atlas_graph::sqlite::{logical::{logical_dump_of_db, logical_hash}, manifest::read_manifest, open_read_only, snapshot::SqliteSnapshot, writer::write_sections};
use atlas_graph_types::store::{assert_answers_match, GraphSnapshot};

fn committed_graph() -> &'static atlas_graph_types::graph::Graph { /* copy canon_real_data.rs's helper verbatim */ static G: OnceLock<atlas_graph_types::graph::Graph> = OnceLock::new(); G.get_or_init(|| { let d = atlas_graph::artifact::read_file(std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/compiled/graph.bin"))).expect("graph.bin"); let (mut g, ..) = atlas_graph::artifact::to_service_parts(d).expect("parts"); g.build_indexes(); atlas_graph::event_world::add_justified_by(&mut g); g }) }

/// Ceiling: set from the FIRST measured run × 2, rounded up to the next 30 s, and written into
/// scripts/timing-gates.sh beside the number measured (the report states both). Never loosened afterward.
const CEILING_SECS: u64 = 0; // replaced by the measured ceiling in Step 4

#[test]
#[ignore = "wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)"]
fn the_full_real_graph_is_admitted_over_the_sqlite_backend_and_the_logical_hashes_agree() {
    let g = committed_graph();
    let dir = std::env::temp_dir().join("db2b-real-sections"); let _ = std::fs::remove_dir_all(&dir);
    let t0 = Instant::now();
    let (m1, written) = write_sections(g, "deadbeefdeadbeef", "test", &dir).expect("write");
    let write_secs = t0.elapsed().as_secs_f64();
    for w in &written { println!("DB-2b SECTION {:?}: {} nodes, {} rows, {} edges, {} bytes, logical {}, in {:?}", w.section, w.node_count, w.row_count, w.edge_count, w.bytes, w.logical, w.elapsed); }
    let t1 = Instant::now();
    for w in &written {
        let conn = open_read_only(&w.path).unwrap();
        assert_eq!(logical_hash(&logical_dump_of_db(&conn, w.section).unwrap()), w.logical, "{:?}: the file's tables do not re-derive the partition's hash", w.section);
    }
    let dump_secs = t1.elapsed().as_secs_f64();
    let snap = SqliteSnapshot::open(&dir.join("manifest.toml")).expect("open");
    assert_eq!(snap.version().0.hex(), "deadbeefdeadbeef");
    let t2 = Instant::now();
    assert_answers_match(&snap, g);
    let admit_secs = t2.elapsed().as_secs_f64();
    let dir2 = std::env::temp_dir().join("db2b-real-sections-2"); let _ = std::fs::remove_dir_all(&dir2);
    let (m2, _) = write_sections(g, "deadbeefdeadbeef", "test", &dir2).expect("write 2");
    assert_eq!(m1.root, m2.root, "determinism: two writes, one root");
    assert_eq!(read_manifest(&dir.join("manifest.toml")).unwrap().root, m1.root);
    let total = t0.elapsed().as_secs_f64();
    println!("DB-2b GATE: write {write_secs:.1}s, dump-recompute {dump_secs:.1}s, assert_answers_match {admit_secs:.1}s, total {total:.1}s (ceiling {CEILING_SECS}s)");
    assert!(total <= CEILING_SECS as f64, "DB-2b gate {total:.1}s exceeds ceiling {CEILING_SECS}s");
}
```

- [ ] **Step 2: Add the writer to the compile binary** — the writer needs the version hex, which `compile_graph.rs:303-305` derives by `MemStore::publish(graph_a_indexed)` (consuming that graph) AFTER the exports are built. So the DB-2b block goes at the END of `main`, after the `kretzmann-chronology.json` write: `graph_b` (the model) is still alive there — the admission calls only borrowed it — and `version_hex` exists. Pass `&graph_b` and `&version_hex`:

```rust
println!("atlas-graph-compile: DB-2b -- writing SQLite sections to {} ...", sections_out.display());
let t = Instant::now();
let (manifest, written) = atlas_graph::sqlite::writer::write_sections(&graph_b, &version_hex, &format!("atlas-graph-compile {} (rustc {})", env!("CARGO_PKG_VERSION"), "1.97.1"), &sections_out).map_err(|e| anyhow::anyhow!("{e}"))?;
for w in &written { println!("atlas-graph-compile:   {:<10} {:>7} nodes {:>8} rows {:>8} edges {:>11} bytes  logical {}  ({:?})", w.section.name(), w.node_count, w.row_count, w.edge_count, w.bytes, w.logical, w.elapsed); }
println!("atlas-graph-compile: DB-2b -- sections written in {:?}; manifest root {}", t.elapsed(), manifest.root);
println!("atlas-graph-compile: DB-2b ADMISSION -- SqliteSnapshot vs the model graph ...");
let t = Instant::now();
let snap = atlas_graph::sqlite::snapshot::SqliteSnapshot::open(&sections_out.join("manifest.toml")).map_err(|e| anyhow::anyhow!("{e}"))?;
for w in &written {
    let conn = atlas_graph::sqlite::open_read_only(&w.path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let recomputed = atlas_graph::sqlite::logical::logical_hash(&atlas_graph::sqlite::logical::logical_dump_of_db(&conn, w.section).map_err(|e| anyhow::anyhow!("{e}"))?);
    anyhow::ensure!(recomputed == w.logical, "DB-2b: {} logical hash from tables {} != from partition {}", w.section.name(), recomputed, w.logical);
}
atlas_graph_types::store::assert_answers_match(&snap, &graph_b);
println!("atlas-graph-compile: DB-2b ADMISSION passed (assert_answers_match over SqliteSnapshot + per-section logical hashes) in {:?}", t.elapsed());
```

  `--sections-out` parsing beside `--out`; default as stated above; `create_dir_all`. The existing `graph.bin` write, red-letter spans and exports are untouched and still happen first (so a DB-2b failure never leaves `graph.bin` unwritten).

- [ ] **Step 3: Run the compile binary once** (from `server/`, debug build): `cargo run -p atlas-graph --bin atlas-graph-compile -- --data-dir ../data/compiled --out ../data/compiled/graph.bin` — Expected: every existing line as before, then the DB-2b lines, `ADMISSION passed`. Record: total compile wall time BEFORE this task (run once at the Task 7 commit) and AFTER; each section's byte size; the DB-2b admission time. **Then `git checkout -- data/compiled data/exports`** — this run rewrites `graph.bin`, `red-letter-spans.json` and the exports byte-identically (determinism law), but restore them anyway so the commit carries no data churn; assert `git status --short data/` is empty and say so in the report.

- [ ] **Step 4: Measure and pin the ceiling** — from the repo root: `cd server && cargo test -p atlas-graph --test sqlite_real_data -- --ignored --nocapture` — record `total`. Set `CEILING_SECS` = total × 2 rounded up to the next multiple of 30. Add gate 9 to `scripts/timing-gates.sh` following the exact shape of gates 1–8 (name `atlas-graph::sqlite_real_data::the_full_real_graph_is_admitted_over_the_sqlite_backend_and_the_logical_hashes_agree`, the same `cargo test … -- --ignored --exact` invocation the script uses, and update the `N/N passed` tally and any `8` literal to `9`). Run `bash scripts/timing-gates.sh`. Expected: `TIMING GATES: 9/9 passed`.

- [ ] **Step 5: Run the fast gates** — `cargo test -p atlas-graph --test sqlite_laws --test sections_real_data --test canon_real_data`; `cd ../graph-types && cargo test && cargo test --features canon-ids`. Expected: all green, no warnings. The controller runs the detached standing block after the commit.

- [ ] **Step 6: BENCHMARKS.md** — append `## DB-2b (2026-09-17)`: compile wall time before/after (§12), per-section sizes (uncompressed, against the 104,857,600-byte ceiling — compression arrives at DB-4), gate 9's measured time and ceiling, `assert_answers_match` over SQLite vs over `MemSnapshot` (gate 2's ~26–40 s).

- [ ] **Step 7: Commit**

```bash
git add server/atlas-graph/src/bins/compile_graph.rs server/atlas-graph/tests/sqlite_real_data.rs scripts/timing-gates.sh server/BENCHMARKS.md
git commit -m "feat(graph): DB-2b -- compile binary writes the sections and admits SqliteSnapshot against the model graph; gate 9 (spec 6.2)"
```

---

## Self-review (done while writing)

**Spec coverage.** §2.1 placement → Task 5 (`partition`, via DB-2a's `sections.rs`; every entry's section asserted). §2.2 manifest fields and root → Task 5 (`manifest.rs`; `built`/`bytes` outside the root, tested). §2.3 file naming `<name>.<logical>.sqlite` → Task 5 (`.zst` and the cache dir arrive with DB-4's `CommittedZstdSource`, §2.4 — deferred, judgment call 5). §2.5 open sequence steps 1–4 → Task 7 (step 3's mmap sizing and per-thread connections deferred to DB-4 with the server switch-over; `SQLITE_MAX_ATTACHED` note stays a spec note). §3.4 identity hierarchy: section logical hash and root → Tasks 5–6 (`table\tcanonical row JSON\n`, PK order); node pid and edge id unchanged in this batch (judgment call 2, EDGE-ID-1 handed to DB-4). §5.0 conventions → Task 3 (`WITHOUT ROWID`, `ord`, no FK/CHECK/trigger asserted, LOCUS/RANGE/AUTHORED, pragmas, hash width 8 with `meta.hash_width`). §5.1 → Task 3 DDL + Task 5 writer (`node`, `justification`, `ground`, `edge_index` with `row_family`/`row_id` — PROV-1's gap closed; `edge_by_id` index). §5.2 union views and cursor order → Task 7 (`ORDER BY ord`, global ord — spec's own escape hatch, proven by the gate). §5.3–5.6 row tables → Tasks 3–4 (projections/sidecars/`event_date`/`heading_index`/`red_letter_span` → DB-4, judgment call 3). §6.1 writer steps 1–4 → Task 5 + Task 8 (step 2's `INSERT…SELECT` → judgment call 1; step 3's zstd → DB-4; step 4's export stamping unchanged because the version root does not move). §6.2 gate → Task 8 (both required checks; the widened-method and companion checks are DB-3's). §8 row 3 deliverables: canon (DB-2a), flag OFF (unchanged), `sections.rs` (DB-2a), writer beside `graph.bin` (Task 8), `SqliteSnapshot: GraphQuery + GraphSnapshot` (Task 7), rusqlite in atlas-graph only (Task 1 + zero_deps). §11 → Task 7 (required missing refused by name+hash; optional absent recorded; manifest root mismatch refused in `read_manifest`). §12 → Task 8 (compile wall time before/after; section sizes vs ceiling).

**DB-2a plan inputs.** RELMAP-1 → Task 2 (`RowFamily::relation`, total). EDGE-ID-1, ROOT-1 → judgment call 2 (explicitly handed to DB-4's plan with the `row_family/row_id` columns making the switch a column rewrite). VALUE-INT-1 → every comparison in this plan is on bytes (`encode()` / `encode_row_in_family`), never on `Value` trees. R17 → partition routes ContainsBible by row. Catechism two-corpus fact → irrelevant to placement (rows are Core; subjects may be in Kjv/Concord — the union view answers across sections). XREF-GAP-1 → the orphan verse's cross-ref row is written like any other; its subject position has no node in any section, and `node()` returns `None` on both sides.

**Placeholder scan.** Task 4's specimen: the 21 rows are copied from a named file (`canon_row_vectors.rs`), the nodes/spines are written out; the two `/* … */` markers there are paste points with their source named, not TBDs. Task 8's `CEILING_SECS = 0` is replaced in Step 4 by the measured number — the step says how. No "add error handling", no "similar to Task N".

**Type consistency.** `RowRef`/`RowOwned` (Task 4) are consumed by Task 5's `rows_of_section` and Task 6's `read_rows`; `SectionPartition.rows: Vec<(RowFamily, i64, RowRef)>` is the same tuple in Tasks 5 and 6; `EdgeEntryOut` field names match the `edge_index` columns; `write_sections(g, graph_version_hex, compiler, out_dir) -> (Manifest, Vec<WrittenSection>)` is called identically in Tasks 5–8; `logical_hash(&[u8]) -> String` (hex) everywhere; `SqliteSnapshot::open(&Path)` with the manifest path in Tasks 7–8; `section_of_justified_by(RowFamily, Option<&str>)` is DB-2a's R3 signature.

**Judgment calls the reviewer should hold me to:** 1–7 above; plus the Task 4 ruling that an empty `Justification` binds NULL (so imported and authored families share one column shape and `Justification::default()` round-trips).

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-17-db2b-sqlite-writer.md`. Two execution options:

1. **Subagent-Driven (recommended)** — a fresh subagent per task, task review between tasks, one final whole-branch review.
2. **Inline Execution** — executing-plans in this session, batch execution with checkpoints.

Prerequisite before Task 1 either way: a C compiler on PATH (`scoop install gcc`).
