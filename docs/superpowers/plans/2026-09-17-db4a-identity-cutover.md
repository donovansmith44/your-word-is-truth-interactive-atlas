# DB-4a: The Identity Cutover — ORDER-1, `canon-ids` ON, Every Id and the Root Move Once

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the atlas its final identities — SHA-256-128 node pids, edge ids over canonical edge bytes, and a version root equal to the manifest root over per-section logical dumps — with the chronology order made total first (ORDER-1), and re-bless every pinned literal exactly once with its reason written down.

**Architecture:** DB-4 (spec §8 row 5) is three shippable sub-batches. **DB-4a (this plan)** is the identity move: it changes no file format the server reads and no wire shape, only the VALUES of ids and the root, plus the tie-break of the chronology sort. **DB-4b** completes the section writer (node projections, `event_date`, `heading_index`, `red_letter_span`, folded sidecars, zstd, `CommittedZstdSource`, `bibex verify`). **DB-4c** is the read cutover (the server opens sections, connection per worker, `graph.bin` no longer read, AQC through the one path, §9 gates and bumps). Each lands green on its own; DB-4a is first because both later halves need ids and root to have stopped moving. Mechanically: the section placement rules and the per-section logical dump move from `atlas-graph` into zero-dep `graph-types` so `MemStore::publish` can compute the manifest root; `entry_id` hashes canonical edge bytes instead of `Debug` text; the atlas workspace turns the `canon-ids` feature on through its path dependency (graph-types' default stays off for the sibling repo); every literal the survey found is re-recorded through its own recorder.

**Tech Stack:** Rust 1.97.1 pinned; `graph-types` zero-dep (`canon`, `sha256`, now `sections`); `rusqlite` 0.32 bundled in `atlas-graph`; recorders: `atlas-graph-compile`, `ATLAS_BLESS_PACT=1` (HTTP then CLI), `contract-runner --bless`, `export_aqc_examples`; `scripts/contract-gate.sh`.

**Spec:** `docs/superpowers/specs/2026-09-14-relational-artifact-design.md` — §2.2 (manifest, root), §3.1–3.6 (identity), §5.0 (hash width, `meta.hash_width`), §6.2 (gate), §8 row 5, §9 (bumps). Inputs decided here: DB-2a final review item 4 (EDGE-ID-1, ROOT-1; `.superpowers/sdd/2026-08-17-bible-atlas-m1/db2a-final-review-report.md:33,76-77`), the owner's ORDER-1 finding (memory), the DB-4a identity survey (recorded under "DB-4a PLAN" in `.superpowers/sdd/2026-08-17-bible-atlas-m1/progress.md`).

## Global Constraints

- **`graph-types` stays zero-dependency**; its `canon-ids` feature stays declared as `canon-ids = []`, default OFF. **The atlas workspace turns it on** through `atlas-graph-types = { path = "../../graph-types", features = ["canon-ids"] }` in `server/atlas-graph/Cargo.toml`, `server/atlas-server/Cargo.toml`, `server/atlas-cli/Cargo.toml`. The OFF code paths stay compiled and tested (`cd graph-types && cargo test`) for the sibling repo that still depends on the u64 hash; deleting them is DB-5's call. graph-types' version stays `0.2.0` (DB-3's MINOR was "the one coordinated MINOR"; turning a feature on in a consumer is not a graph-types change).
- **What moves, exactly once (spec §3.6):** every node pid (in-memory and `node.pid`), every edge id string (`<Rel>:<32 hex>`), the version root (32 hex), `data/exports/*.json`'s `atlas_version_root`, `contracts/pacts/{http,cli}.json`, `contracts/atlas-graph-contract/fixtures/edges-hazor-1-site-of.json`, the 19+3 `contracts/atlas-query-contract/fixtures/*.json` carrying `version`/`edge`, `server/atlas-cli/tests/cli.rs:578`'s transcript, `server/atlas-graph/tests/version_root_regression.rs:371`. **What must NOT move:** `data/compiled/graph.bin` (pids are not stored; `determinism.rs`), `FORMAT_VERSION = 13`, the 25 scene hashes in `scene_byte_identity.rs` EXCEPT by ORDER-1 (Task 1 says which and why), every HTTP body shape, the CLI's plain-text shapes other than the id width and its column, every `.feature` file's text (spec §9: "if one must, the migration failed").
- **Contract bumps are derived by the gate, declared by us:** `contracts/atlas-graph-contract` `0.3.0 → 0.4.0`, `contracts/atlas-query-contract` `0.1.0 → 0.2.0` (both MAJOR-class fixture re-blesses, read as MAJOR under the pre-launch 0.x rule, `scripts/contract-semver-gate.sh`). One CHANGELOG entry each, in the same commit as the fixtures. `scripts/contract-gate.sh` must PASS at the end.
- **Recorders, in this order, from `server/`:** `cargo run -p atlas-graph --bin atlas-graph-compile -- --data-dir ../data/compiled --out ../data/compiled/graph.bin` (exports + sections) → `ATLAS_BLESS_PACT=1 cargo test -p atlas-server --test contract_pact` (fails on purpose after writing) → `ATLAS_BLESS_PACT=1 cargo test -p atlas-cli --test contract_pact_cli` → `"$RUNNER" run --replay contracts/pacts --exports data/exports --bless contracts/atlas-graph-contract` (repo root; `RUNNER` = `cd contracts/runner && cabal list-bin contract-runner`) → `cargo run -p atlas-server --bin export_aqc_examples` → hand-edit `cli.rs:578` → VERSION/CHANGELOG → `bash scripts/contract-gate.sh`. Every bless is followed by the same command WITHOUT the bless switch.
- **Load ceiling 4 s (gate 1) and gate 9's 570 s stay.** SHA-256 is hand-written Rust and runs once per node and twice per index entry at load; Task 3 measures before committing the flip and names the mitigation if the ceiling is threatened.
- **map-generator** (owner: out of scope) keeps compiling: graph-types' default is OFF. Its vendored export pin parses `atlas_version_root` as a u64 and will refuse the 32-hex root when it next re-vendors — stated in the report, not fixed here.
- Standing block = from `server/`: `cargo test --workspace`, `cargo test -p atlas-graph-types`, `cd ../graph-types && cargo test --features canon-ids`, `bash ../scripts/timing-gates.sh`, run DETACHED with `RUST_TEST_THREADS=4` (the CLI and graph_api suites OOM at full parallelism); sum only `^test result:` lines.
- Never build `--release`; never touch port 8080; port 8000 is free. Untracked files never staged: `client.Tests/FrontierMatrix*.cs`, `publish-apphome/`, `tests/ux/frontier-matrix.spec.ts`. No `git stash`. Commit trailers: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W`. Git Bash; `export PATH="$HOME/scoop/apps/gcc/current/bin:$PATH:$HOME/.cargo/bin:$HOME/.local/bin"`.

## Judgment calls (owner can reverse)

1. **EDGE-ID-1: the edge id is keyed on the EDGE, not the row.** Spec §3.4's `family|row canonical bytes` would give one id to every index entry a multi-locus `Contains` row produces (N loci, N entries, one id) and to both directions of a symmetric row; edge POSITIONS (`Position::Edge`, the justified-by subjects) need one id per edge. Ruling: `entry_id` ON hashes `DOMAIN_PREFIX ‖ {"object":<position_str>,"rel":"<RelationId Debug name>","subject":<position_str>}` (canonical JSON via `canon::obj`/`serialize`, keys in byte order) — decodable, `Debug`-free (spec §3.1 defect 4 for edges), and a symmetric id sorts its two ends first as today. The string spelling stays `<Rel>:<hex>`. §3.4's edge row is amended in this plan; the spec file itself is edited in Task 5 with a one-line pointer.
2. **ROOT-1: the root is the manifest root, computed in graph-types.** `Section`, the placement rules, `row_tables_of`, `has_spine`, `logical_table_order`, the per-section logical dump and the manifest lines move into `graph-types::sections` (zero-dep, pure functions). `version_of` ON = `sha256_prefixed_128(DOMAIN_PREFIX, manifest_lines)` over the four shipped sections in `MANIFEST_ORDER` (Lexicon excluded until LEX-1), each line `name|logical|14|required`. The whole-graph `logical_dump` and its `spine\tbible\tid,id` line format retire; DB-2b's `table\t<canonical row JSON>\n` per-section format is THE dump. `SqliteSnapshot::version()` becomes the manifest's `root`; `meta.graph_version` is dropped (it existed only to make DB-2b's two versions agree).
3. **ORDER-1 is a data-carried total order, not a new sort key.** Execution finding (Task 1): the curated same-year sequence inside `chronology.order` is SEMANTIC -- a narrative's own leg order (David's flight: `df_ramah` -> `df_nob`), pinned by `narrative_real_data.rs`'s E5 laws -- so an `id` tie-break would reorder history and was reverted. The total order already exists as `resolved[id].seq` (`SeqKey` = position in `order`, persisted in the artifact and, from DB-4b, in `event_date.seq`), so SQL reproduces the served prior/following with `ORDER BY seq`. Task 1 therefore: states the scene path's key explicitly (`(from_year, id)`, unchanged by construction), pins the seq law, and moves nothing -- no scene hash, no root.
4. **The atlas flips the feature; graph-types' default does not.** One line per consumer crate. The graph-types ON gate stays `cd graph-types && cargo test --features canon-ids`; the OFF gate stays `cargo test` there. The atlas workspace never builds OFF again.
5. **Re-record once, at the end (Task 4).** Between Task 1 and Task 4, `contract_pact`, `contract_pact_cli`, `exports_laws`' root-equality law and `version_root_regression` are red on purpose; each task's own verification names the tests it runs and excludes those four. The batch ships green.
6. **`write_sections` loses its `graph_version_hex` parameter.** The version is the manifest root; the writer computes it. This also deletes the fourteen hard-coded 16-hex version strings in `sqlite_laws.rs`/`sqlite_real_data.rs` the survey flagged.
7. **`FORMAT_VERSION` stays 13** — `graph.bin` bytes do not change (pids and edge ids are computed at load). The artifact wall moves at DB-4c when `graph.bin` stops being read.

## File Structure

| Path | Responsibility |
|---|---|
| `server/atlas-core/src/data.rs:1556,1767` (MODIFY) | ORDER-1: `(from_year, id)` and `(from_year, order_key, id)` |
| `server/atlas-graph/src/event_world.rs:142-146` (MODIFY) | ORDER-1: `(from_year, order_key, id)` replaces the array-index tie-break |
| `server/atlas-graph/src/scene_source.rs:210` (MODIFY) | ORDER-1: `(from_year, id)` — same order as today, now stated |
| `server/atlas-server/tests/scene_byte_identity.rs` (MODIFY if moved) | the 25 hashes |
| `graph-types/src/sections.rs` (NEW) | `Section`, placement, `row_tables_of`, `has_spine`, `logical_table_order`, `SECTION_SCHEMA_VERSION`, `logical_dump_section`, `logical_hash`, `manifest_lines`, `root_of_lines`, `version_root` |
| `graph-types/src/store.rs` (MODIFY) | `version_of` ON = `sections::version_root`; `logical_dump` retired; laws updated |
| `graph-types/src/edge.rs:610-646` (MODIFY) | `entry_id`/`entry_id_symmetric` ON hash canonical edge bytes |
| `graph-types/src/canon/mod.rs` or `canon/ids.rs` (MODIFY) | `edge_canonical_bytes(rel_name, subject, object)` |
| `graph-types/src/lib.rs` (MODIFY) | `pub mod sections;` |
| `server/atlas-graph/src/sections.rs` (MODIFY) | becomes `pub use atlas_graph_types::sections::*;` plus `justified_by_source_family` if it does not move |
| `server/atlas-graph/src/sqlite/{ddl,logical,manifest,writer,snapshot,mod}.rs` (MODIFY) | consume graph-types' sections; writer computes the root; `meta.graph_version` dropped; snapshot version = manifest root |
| `server/atlas-graph/src/bins/compile_graph.rs` (MODIFY) | `write_sections(&graph_b, &compiler, &sections_out)` |
| `server/atlas-graph/Cargo.toml`, `server/atlas-server/Cargo.toml`, `server/atlas-cli/Cargo.toml` (MODIFY) | `features = ["canon-ids"]` |
| `server/atlas-graph/tests/{sqlite_laws,sqlite_real_data,version_root_regression,canon_real_data,sections_real_data}.rs` (MODIFY) | width pins, root pin, moved module paths |
| `server/atlas-cli/tests/cli.rs:578` (MODIFY) | transcript |
| `data/exports/*.json`, `contracts/pacts/*.json`, `contracts/atlas-graph-contract/{fixtures,VERSION,CHANGELOG.md}`, `contracts/atlas-query-contract/{fixtures,features,VERSION,CHANGELOG.md}` (RE-RECORDED) | the pins |
| `docs/superpowers/specs/2026-09-14-relational-artifact-design.md` §3.4 (MODIFY, one row) | edge-id amendment pointer |
| `server/BENCHMARKS.md` (MODIFY) | gate 1 and gate 9 under ON |

---

### Task 1: ORDER-1 — the chronology sort gets a total key

**Files:**
- Modify: `server/atlas-core/src/data.rs:1556` and `:1767`, `server/atlas-graph/src/event_world.rs:142-146`, `server/atlas-graph/src/scene_source.rs:210`
- Modify (only if the measurement says so): `server/atlas-server/tests/scene_byte_identity.rs:162-192`
- Modify: `server/atlas-graph/tests/version_root_regression.rs:371` (re-pin, with the ORDER-1 paragraph)
- Test: `server/atlas-graph/tests/chronology_laws.rs`, `timeline_equivalence.rs`, `narrative_real_data.rs` (unchanged, must stay green), `server/atlas-core` unit tests, new law below

**Interfaces:**
- Produces: the ORDER-1 law, stated in `event_world.rs` beside `timeline_order`: *the chronology's total order is `(from_year, order_key, id)`; no two events compare equal; a SQL `ORDER BY from_year, order_key, event_id` reproduces it.*

- [ ] **Step 1: Write the failing law** — append to `server/atlas-graph/tests/chronology_laws.rs` (read its header for the real-graph helper it already uses; reuse it):

```rust
/// ORDER-1 (owner finding, 2026-09-15): the chronology order is TOTAL --
/// `(from_year, order_key, id)` -- so no served order depends on the
/// position an event happened to have in a curated file, and a SQL
/// `ORDER BY` can reproduce it (DB-4c). Asserted on the real graph and on
/// a synthetic tie group.
#[test]
fn the_chronology_order_is_from_year_then_order_key_then_id() {
    let s = real_service(); // the file's existing GraphService helper; name it as the file does
    let order = &s.chronology.chrono.order;
    let mut keys: Vec<(i32, u32, &str)> = Vec::with_capacity(order.len());
    for id in order {
        let r = s.chronology.chrono.resolved.get(id).expect("every ordered event resolves");
        keys.push((r.date.from.year.get(), r.seq.0, id.as_str()));
    }
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "chronology.order must equal its own (from_year, seq, id) sort");
    assert!(keys.windows(2).all(|w| w[0] < w[1]), "no two events compare equal");
}
```

  (Read `event_world::Chronology`/`ResolvedPlacement` for the exact field names: the survey names `chrono.order: Vec<String>` and `chrono.resolved`; `ResolvedPlacement { date: ResolvedDate { from: TimePoint { year: Year, .. }, .. }, seq: SeqKey(u32), .. }`. If `seq` is not the `order_key`, use whichever field `timeline_order` sorts by and say so in the test's doc comment.)

- [ ] **Step 2: RED** — `cargo test -p atlas-graph --test chronology_laws the_chronology_order`. Expected: FAIL (ties in 137 groups are in array order today).

- [ ] **Step 3: Implement** —
  - `data.rs:1556`: `self.events.sort_by(|a, b| (a.when.from_year, &a.id).cmp(&(b.when.from_year, &b.id)));`
  - `data.rs:1767`: the closure returns `(e.when.from_year, e.order_key, id.clone())` (or `id.as_str()` if lifetimes allow).
  - `event_world.rs:142-146`: `keyed.sort_by_key(|(_, e)| (e.when.from_year, e.order_key, e.id.clone()))` — the `idx` tie-break is deleted; update the doc comment (`:129-141`) to state the ORDER-1 law and that it supersedes "original array order".
  - `scene_source.rs:210`: `events.sort_by(|a, b| (a.when.from_year, &a.id).cmp(&(b.when.from_year, &b.id)));` with the comment: *ORDER-1: the same order as before by construction (the input was id-ordered and the sort stable), now stated as a total key.*

- [ ] **Step 4: Measure what moved** — `cargo test -p atlas-server --test scene_byte_identity`. If green: the 25 hashes did not move (state it in the commit). If red: the failure lists each window's new hash; re-pin each in `scene_byte_identity.rs:162-192` and write ONE paragraph above the table naming ORDER-1 and the number of windows that moved. Then `cargo test -p atlas-graph --test version_root_regression`: it fails with the new root on the left; set `EXPECTED_VERSION_HEX` to it and append the file's customary "MOVED AGAIN" paragraph: *ORDER-1 — `temporal_adjacency` rows follow `(from_year, order_key, id)`; the root hashes them.* Run `cargo test -p atlas-graph --test chronology_laws --test timeline_equivalence --test narrative_real_data --test determinism` and `cargo test -p atlas-core`. Expected: green.

- [ ] **Step 5: Commit**

```bash
git add server/atlas-core/src/data.rs server/atlas-graph/src/event_world.rs server/atlas-graph/src/scene_source.rs server/atlas-graph/tests/chronology_laws.rs server/atlas-graph/tests/version_root_regression.rs server/atlas-server/tests/scene_byte_identity.rs
git commit -m "fix(core, graph): ORDER-1 -- the chronology order is total: (from_year, order_key, id); root re-pinned; <N or 0> scene windows moved"
```

---

### Task 2: Identity in graph-types — sections, the per-section dump, the manifest root, canonical edge bytes

**Files:**
- Create: `graph-types/src/sections.rs`
- Modify: `graph-types/src/lib.rs` (`pub mod sections;`), `graph-types/src/store.rs` (`version_of` ON, retire `logical_dump`, laws), `graph-types/src/edge.rs:610-646` (`entry_id`, `entry_id_symmetric` ON bodies), `graph-types/src/canon/ids.rs` (`edge_canonical_bytes`)
- Modify (atlas-graph consumers): `server/atlas-graph/src/sections.rs` (re-export), `server/atlas-graph/src/sqlite/ddl.rs` (`row_tables_of`/`has_spine`/`logical_table_order` re-exported from graph-types), `server/atlas-graph/src/sqlite/logical.rs` (`logical_dump_of_partition` deleted; `logical_hash` re-exported), `server/atlas-graph/src/sqlite/manifest.rs` (`root_of` delegates), `server/atlas-graph/src/sqlite/writer.rs` (root from graph-types; no `graph_version` meta), `server/atlas-graph/src/sqlite/snapshot.rs` (`version` = manifest root), `server/atlas-graph/src/sqlite/mod.rs` (`SCHEMA_VERSION` = `sections::SECTION_SCHEMA_VERSION`), `server/atlas-graph/src/bins/compile_graph.rs` (`write_sections` call), `server/atlas-graph/tests/{sqlite_laws,sqlite_real_data,sections_real_data,canon_real_data}.rs`
- Test: `graph-types/src/sections.rs` `mod laws`, `graph-types/src/store.rs` `mod laws`, `graph-types/tests/canon_vectors.rs` (an edge-bytes golden), `server/atlas-graph/tests/sqlite_laws.rs`

**Interfaces:**
- Produces (`graph-types/src/sections.rs`, everything `pub`):

```rust
pub const SECTION_SCHEMA_VERSION: u32 = 14;               // spec §5.0; the sqlite module re-exports it
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Section { Core, Kjv, Concord, Kretzmann, Lexicon }
impl Section {
    pub const MANIFEST_ORDER: [Section; 5];
    pub const SHIPPED: [Section; 4];                        // MANIFEST_ORDER minus Lexicon, until LEX-1
    pub fn name(self) -> &'static str; pub fn required(self) -> bool;
}
pub fn section_of_node(node: &Node) -> Section;             // moved verbatim from atlas-graph
pub fn section_of_family(f: RowFamily) -> Section;          // panics on ContainsBible (R17), verbatim
pub fn section_of_contains_bible(row: &Contains<BibleTag>) -> Section;
pub fn section_of_justified_by(source_family: RowFamily, source_container_raw: Option<&str>) -> Section;
pub fn justified_by_source_family(source_edge_id: &EdgeId) -> Option<RowFamily>;
pub fn row_tables_of(section: Section) -> &'static [RowFamily];
pub fn has_spine(section: Section) -> bool;
pub fn logical_table_order(section: Section) -> Vec<&'static str>;
/// Spec §3.4, DB-2b's binding format: `node\t<Node::encode>\n` for the section's nodes in
/// `any_node_id_str` BYTE order; `<family>\t<encode_row_in_family>\n` per row table in
/// `row_tables_of` order, rows in ord order (ContainsBible filtered by section_of_contains_bible);
/// `reading_spine\t{"corpus":..,"node_id":..,"ord":N}\n` when has_spine.
pub fn logical_dump_section(g: &Graph, section: Section) -> Vec<u8>;
pub fn logical_hash(dump: &[u8]) -> ContentHash;            // sha256_prefixed_128(DOMAIN_PREFIX, dump)
/// `name|logical_hex|schema_version|required\n` per shipped section, MANIFEST_ORDER.
pub fn manifest_lines(entries: &[(&str, &str, u32, bool)]) -> Vec<u8>;
pub fn root_of_lines(lines: &[u8]) -> ContentHash;
/// THE version root: manifest lines over `Section::SHIPPED`, each logical = logical_hash(logical_dump_section).
pub fn version_root(g: &Graph) -> ContentHash;
```

  `ContentHash::hex()` renders `logical_hex`; both cfg states compile (OFF: 16 hex in the lines, ON: 32 — the lines are what they are in each state; the atlas only ever runs ON after Task 3).

- Produces (`canon/ids.rs`): `pub fn edge_canonical_bytes(rel_name: &str, subject: &Position, object: &Position) -> Vec<u8>` = `serialize(&obj(vec![("object", str_value(&position_str(object))), ("rel", str_value(rel_name)), ("subject", str_value(&position_str(subject)))]))`.
- Changes (`edge.rs`): inside `entry_id` and `entry_id_symmetric`, the local `E`'s `canonical_bytes` becomes cfg-split: OFF the existing `format!("{:?}|{:?}|{:?}")`; ON `crate::canon::ids::edge_canonical_bytes(&format!("{:?}", self.0), self.1, self.2)`. The id string stays `format!("{:?}:{}", rel, pid.hash.hex())`.
- Changes (`store.rs`): `#[cfg(feature = "canon-ids")] fn version_of(g) -> GraphVersion { GraphVersion(crate::sections::version_root(g)) }`; `pub fn logical_dump` and its tests (`logical_dump_carries_nodes_then_rows_then_spines`, the ON `version_root_covers_rows_not_only_nodes` keeps its assertion but through `version_root`) are replaced — the dump law moves to `sections::laws`.
- Consumers: `server/atlas-graph/src/sections.rs` → `pub use atlas_graph_types::sections::*;` (its doc header kept, pointing at the move); `sqlite/ddl.rs` deletes its copies of `row_tables_of`/`has_spine`/`logical_table_order` and `pub use atlas_graph_types::sections::{row_tables_of, has_spine, logical_table_order};`; `sqlite/logical.rs` keeps `logical_dump_of_db` and `spine_line_body` (the DB side), deletes `logical_dump_of_partition`, and `pub fn logical_hash(dump) -> String { atlas_graph_types::sections::logical_hash(dump).hex() }`; `sqlite/manifest.rs::root_of` → `atlas_graph_types::sections::root_of_lines(&manifest_lines_bytes).hex()` where the bytes come from `sections::manifest_lines`; `writer.rs::write_sections(g, compiler, out_dir)` computes `logical = logical_hash(&logical_dump_section(g, p.section)).hex()`, writes no `graph_version` meta; `snapshot.rs::open` sets `version = GraphVersion(ContentHash::from_hex(&manifest.root)?)`.

- [ ] **Step 1: Write the failing laws** — `graph-types/src/sections.rs` `mod laws` (the module is new, so write the file with the laws at the bottom and the implementation above; RED is the first compile of the consumers):

```rust
#[cfg(test)]
mod laws {
    use super::*;
    use crate::edge::{Analogue, Justification, LocatedAt};
    use crate::id::{EventId, NodeKind, PlaceId};
    use crate::node::NodePayload;

    fn unit(raw: &str, corpus: &'static str) -> Node {
        let mut renderings = crate::text::LayerMap::new();
        renderings.insert(crate::text::TranslationId("kjv".into()), "x".into());
        Node { id: AnyNodeId { kind: NodeKind::TextUnit, raw: raw.into() }, payload: NodePayload::TextUnit { corpus, renderings }, provenance: "p".into() }
    }
    fn g() -> Graph {
        let mut g = Graph::default();
        for n in [unit("bible/1.1.2", "bible"), unit("bible/1.1.1", "bible"), unit("concord/1.1.1", "concord")] { g.nodes.insert(n.id.clone(), n); }
        g.reading.insert("bible", crate::graph::ReadingSpine { order: vec![AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() }, AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.2".into() }] });
        g.located_at.push(LocatedAt { event: EventId::new("e1"), place: PlaceId::new("p"), provenance: "p".into(), justification: Justification::default() });
        g.analogue.push(Analogue { a: EventId::new("e1"), b: EventId::new("e2"), provenance: "p".into() });
        g.build_indexes();
        g
    }

    #[test]
    fn the_section_dump_walks_node_then_families_then_spine_in_byte_order() {
        let g = g();
        let kjv = String::from_utf8(logical_dump_section(&g, Section::Kjv)).unwrap();
        let tags: Vec<&str> = kjv.lines().map(|l| l.split('\t').next().unwrap()).collect();
        assert_eq!(tags, ["node", "node", "reading_spine", "reading_spine"], "kjv: two bible nodes, no rows, the spine");
        assert!(kjv.starts_with("node\t{\"id\":\"TextUnit:bible/1.1.1\""), "byte order of any_node_id_str, not insertion order");
        assert!(kjv.contains("\nreading_spine\t{\"corpus\":\"bible\",\"node_id\":\"TextUnit:bible/1.1.1\",\"ord\":0}\n"));
        let core = String::from_utf8(logical_dump_section(&g, Section::Core)).unwrap();
        let tags: Vec<&str> = core.lines().map(|l| l.split('\t').next().unwrap()).collect();
        assert_eq!(tags, ["located_at", "analogue"], "core: no nodes here, the two rows in row_tables_of order");
        assert!(logical_dump_section(&g, Section::Kretzmann).is_empty());
        assert_eq!(logical_table_order(Section::Kjv).first().copied(), Some("node"));
    }

    #[test]
    fn the_root_is_the_manifest_root_over_the_shipped_sections_and_moves_with_a_row() {
        let g = g();
        let lines: Vec<(String, String, u32, bool)> = Section::SHIPPED.iter().map(|s| (s.name().to_string(), logical_hash(&logical_dump_section(&g, *s)).hex(), SECTION_SCHEMA_VERSION, s.required())).collect();
        let borrowed: Vec<(&str, &str, u32, bool)> = lines.iter().map(|(n, l, v, r)| (n.as_str(), l.as_str(), *v, *r)).collect();
        let text = String::from_utf8(manifest_lines(&borrowed)).unwrap();
        assert_eq!(text.lines().count(), 4);
        assert!(text.starts_with(&format!("core|{}|14|true\n", lines[0].1)));
        assert!(text.contains("|14|false\n"), "concord and kretzmann are optional");
        assert_eq!(version_root(&g), root_of_lines(text.as_bytes()));
        let mut g2 = g();
        g2.located_at[0].provenance = "q".into();
        assert_ne!(version_root(&g), version_root(&g2), "a row byte moves the root (spec 3.1 defect 1, closed)");
        let mut g3 = g();
        g3.build_indexes(); // derived state is not in the root
        assert_eq!(version_root(&g), version_root(&g3));
    }

    #[test]
    fn placement_moved_verbatim() {
        assert_eq!(section_of_family(RowFamily::CommentsOn), Section::Kretzmann);
        assert_eq!(section_of_justified_by(RowFamily::ContainsBible, Some("bible-chapter-GEN-1")), Section::Kjv);
        assert_eq!(section_of_justified_by(RowFamily::ContainsBible, Some("passage-creation")), Section::Core);
        assert_eq!(row_tables_of(Section::Lexicon).len(), 0);
        assert_eq!(Section::SHIPPED.len(), 4);
    }
}
```

  and in `graph-types/tests/canon_vectors.rs` (both states):

```rust
#[test]
fn edge_canonical_bytes_are_sorted_decodable_json_over_position_strings() {
    use atlas_graph_types::canon::ids::{edge_canonical_bytes, position_str};
    use atlas_graph_types::id::{EventId, PlaceId, Position};
    let s = Position::Node(EventId::new("e1").erase());
    let o = Position::Node(PlaceId::new("jordan").erase());
    let bytes = edge_canonical_bytes("LocatedAt", &s, &o);
    assert_eq!(String::from_utf8(bytes).unwrap(), r#"{"object":"n:Place:jordan","rel":"LocatedAt","subject":"n:Event:e1"}"#);
    assert_eq!(position_str(&s), "n:Event:e1");
}
```

  and an ON-only golden in `graph-types/src/store.rs` laws (replacing the retired `logical_dump_carries_nodes_then_rows_then_spines`):

```rust
    #[cfg(feature = "canon-ids")]
    #[test]
    fn edge_ids_hash_canonical_edge_bytes_not_debug_text() {
        use crate::edge::{entry_id, RelationId};
        let s = Position::Node(EventId::new("e1").erase());
        let o = Position::Node(PlaceId::new("jordan").erase());
        let id = entry_id(RelationId::LocatedAt, &s, &o);
        let expected = crate::sha256::sha256_prefixed_128(crate::canon::DOMAIN_PREFIX, &crate::canon::ids::edge_canonical_bytes("LocatedAt", &s, &o));
        assert_eq!(id.0, format!("LocatedAt:{}", ContentHash(expected).hex()));
        assert_eq!(id.0.len(), "LocatedAt:".len() + 32);
    }
```

- [ ] **Step 2: RED** — `cd graph-types && cargo test`. Expected: compile errors (`sections` missing, `edge_canonical_bytes` missing).

- [ ] **Step 3: Implement graph-types** — write `sections.rs` by MOVING `server/atlas-graph/src/sections.rs`'s functions (keep every doc comment; `Section` gains `SHIPPED`), `row_tables_of`/`has_spine`/`logical_table_order` from `sqlite/ddl.rs`, and `logical_dump_section` written as: collect `g.nodes.values().filter(|n| section_of_node(n) == section)`, sort by `any_node_id_str`, emit `node\t` lines; for `f in row_tables_of(section)`, walk that family's `Vec` (ContainsBible filtered by `section_of_contains_bible(row) == section`), emit `f.name()\t encode_row_in_family(f, row.to_value())`; if `has_spine`, corpus `"bible"` for Kjv / `"concord"` for Concord, emit `reading_spine\t` + `serialize(&obj(vec![("corpus", str_value(corpus)), ("node_id", str_value(&any_node_id_str(id))), ("ord", Value::Int(i))]))`. The family walk is a 21-arm `match` — destructure nothing; but ADD to `Graph`'s exhaustive destructure test in `store.rs` (the one FINAL-REVIEW-item-3 wrote) an assertion that every `RowFamily::ALL` appears in exactly one `row_tables_of(section)` for `Section::SHIPPED` except `ContainsBible` (two) — that is the closedness guard that replaces the retired `logical_dump`'s destructure. `edge_canonical_bytes` in `canon/ids.rs`. `edge.rs` cfg split as specified. `store.rs`: `version_of` ON delegates; delete `logical_dump` and update/replace its laws (`version_root_covers_rows_not_only_nodes` stays and passes; `equal_nodes_have_equal_pids…` stays).

- [ ] **Step 4: graph-types GREEN both states** — `cargo test` and `cargo test --features canon-ids` from `graph-types/`; `zero_deps` green.

- [ ] **Step 5: Re-home the atlas consumers** — as the Interfaces block states. `write_sections(g, compiler, out_dir)`; `insert_meta` drops `graph_version`; `SqliteSnapshot::open` derives `version` from `manifest.root` (`ContentHash::from_hex(&manifest.root).ok_or(...)`). `sqlite_laws.rs`: every `write_sections(&g, "<hex>", "test", &dir)` → `write_sections(&g, "test", &dir)`; the two `snap.version().0.hex() == "<hex>"` asserts become `assert_eq!(snap.version(), { let mut s = MemStore::default(); s.publish(g.clone_for_version()) … }` — simpler and stronger: `assert_eq!(snap.version().0, atlas_graph_types::sections::version_root(&g))` (the graph is still in scope). `sqlite_real_data.rs:75` likewise. `hash_blob_round_trips_at_the_current_width` builds its hash with `ContentHash::from_hex(&"0123456789abcdef".repeat(HASH_WIDTH / 8)).unwrap()` and asserts `hash_bytes(&h)` equals the hex decoded (no `u64` constructor). `sqlite_laws.rs:864`'s `"LocatedAt:0000000000000000"` → `format!("LocatedAt:{}", "0".repeat(HASH_WIDTH * 2))`. `compile_graph.rs`: the DB-2b block passes `&compiler` only; its printed `manifest.root` line stays. Add a law to `sqlite_laws.rs`:

```rust
#[test]
fn the_sqlite_snapshots_version_is_the_manifest_root_and_equals_the_in_memory_root() {
    let mut g = specimen_graph(); g.build_indexes(); atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db4a-root-{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir);
    let (m, _) = write_sections(&g, "test", &dir).unwrap();
    let snap = SqliteSnapshot::open(&dir.join("manifest.toml")).unwrap();
    assert_eq!(snap.version().0.hex(), m.root, "SqliteSnapshot::version IS the manifest root");
    assert_eq!(snap.version().0, atlas_graph_types::sections::version_root(&g), "and equals the in-memory root (one root, spec 3.4)");
    let mut store = atlas_graph_types::store::MemStore::default();
    let v = atlas_graph_types::store::GraphPublisher::publish(&mut store, g);
    assert_eq!(snap.version(), v, "and equals what MemStore stamps");
}
```

  (In the OFF state, before Task 3, `MemStore::publish`'s `version_of` is still the node-only digest, so the last assertion is `#[cfg]`-free but will only hold after Task 3 flips the workspace ON. Mark it `#[ignore = "DB-4a Task 3 turns canon-ids on; this law is un-ignored there"]` in THIS task and un-ignore it in Task 3 — the ONE temporary ignore, removed inside the batch.)

- [ ] **Step 6: GREEN** — from `server/`: `cargo test -p atlas-graph --test sqlite_laws --test sections_real_data --test canon_real_data --test determinism`, `cargo test -p atlas-graph --lib`, `cargo build --workspace --tests` (no warnings). NOT run here: `version_root_regression` (moves at Task 3), `contract_pact*`, `exports_laws` (judgment call 5).

- [ ] **Step 7: Commit**

```bash
git add graph-types/src/sections.rs graph-types/src/lib.rs graph-types/src/store.rs graph-types/src/edge.rs graph-types/src/canon/ids.rs graph-types/tests/canon_vectors.rs server/atlas-graph/src/sections.rs server/atlas-graph/src/sqlite server/atlas-graph/src/bins/compile_graph.rs server/atlas-graph/tests/sqlite_laws.rs server/atlas-graph/tests/sqlite_real_data.rs server/atlas-graph/tests/sections_real_data.rs server/atlas-graph/tests/canon_real_data.rs
git commit -m "feat(graph-types, graph): DB-4a -- sections and the per-section logical dump live in graph-types; the version root is the manifest root (ROOT-1); edge ids hash canonical edge bytes (EDGE-ID-1); SqliteSnapshot::version is the manifest root"
```

---

### Task 3: The flip — `canon-ids` on for the atlas workspace, measured

**Files:**
- Modify: `server/atlas-graph/Cargo.toml:22`, `server/atlas-server/Cargo.toml:40`, `server/atlas-cli/Cargo.toml:13` (`features = ["canon-ids"]`), `graph-types/Cargo.toml:15-31` (the header comment: the atlas workspace is ON through its path dependency; OFF is kept for the sibling; the two gates)
- Modify: `server/atlas-graph/tests/sqlite_laws.rs` (un-ignore Task 2's root law), `server/atlas-graph/tests/version_root_regression.rs:371` (re-pin, ON paragraph)
- Test: `cargo build --workspace --tests`; gate 1 and gate 9 measured

- [ ] **Step 1: Flip** — the three Cargo lines; un-ignore the root law; `cargo build --workspace --tests` from `server/`. Expected: compiles with no warnings (Task 2 removed every width pin; if a `ContentHash(<u64>)` constructor survives anywhere under `server/`, this build names it — replace it with `ContentHash::from_hex`).

- [ ] **Step 2: Measure gate 1 BEFORE committing** — `cargo test -p atlas-graph --test artifact_conformance -- --ignored --exact serialized_artifact_is_admitted_and_loads_under_the_committed_ceiling --nocapture --test-threads=1`. Record the load time (before DB-4a: 2.71 s standalone). SHA-256 now runs once per node (93k) and twice per index entry (~2.1M). If the load time is over 4 s: the mitigation is `[profile.dev.package.atlas-graph-types] opt-level = 2` in `server/Cargo.toml` beside the `libsqlite3-sys` override, with the comment that graph-types' laws still run under the debug profile from `graph-types/` itself; apply it, re-measure, and report both numbers. If still over 4 s, STOP and report — the ceiling is not loosened.

- [ ] **Step 3: Re-pin the root** — `cargo test -p atlas-graph --test version_root_regression`; set `EXPECTED_VERSION_HEX` to the 32-hex value on the left of the failure; append the paragraph: *DB-4a — `canon-ids` ON: SHA-256-128 over canonical bytes; the root is the manifest root over the four sections' logical dumps (spec §3.4). 32 hex from here on.* Run it again: green.

- [ ] **Step 4: Fast gates** — `cargo test -p atlas-graph --test sqlite_laws --test canon_real_data --test sections_real_data --test determinism --test artifact_conformance` (the non-ignored parts), `cargo test -p atlas-server --test scene_byte_identity --test api --test graph_api -- --test-threads=4`, `cargo test -p atlas-cli -- --test-threads=4` EXPECTING exactly one red: `edges_plain_output_is_byte_unchanged_by_the_json_addition` (the transcript, re-pinned in Task 4). Everything else green. Measure gate 9: `cargo test -p atlas-graph --test sqlite_real_data -- --ignored --nocapture --test-threads=1` (16-byte BLOBs now; record the numbers; ceiling 570 s stays).

- [ ] **Step 5: Commit**

```bash
git add server/atlas-graph/Cargo.toml server/atlas-server/Cargo.toml server/atlas-cli/Cargo.toml graph-types/Cargo.toml server/atlas-graph/tests/sqlite_laws.rs server/atlas-graph/tests/version_root_regression.rs server/Cargo.toml
git commit -m "feat(server): DB-4a -- canon-ids ON for the atlas workspace: SHA-256-128 pids, canonical edge ids, the manifest root; gate 1 <x> s of 4 s"
```

---

### Task 4: Re-record every pin, once

**Files:**
- Re-recorded: `data/compiled/graph.bin` (must be byte-identical — verify), `data/exports/{gazetteer,chronology,kretzmann-chronology}.json`, `contracts/pacts/http.json`, `contracts/pacts/cli.json`, `contracts/atlas-graph-contract/fixtures/edges-hazor-1-site-of.json` (+ any other fixture the runner re-blesses; expect only that one), `contracts/atlas-query-contract/fixtures/*.json` + `features/*.feature` + `fixtures/index.json` (expect: the 19 `version` values, the 22 `edge` values, and the two scene fixtures' body order from ORDER-1; NO scenario text change — if a `.feature` changes text, STOP: spec §9 says the migration failed)
- Modify: `server/atlas-cli/tests/cli.rs:578`, `contracts/atlas-graph-contract/{VERSION,CHANGELOG.md}`, `contracts/atlas-query-contract/{VERSION,CHANGELOG.md}`
- Test: `contract_pact`, `contract_pact_cli`, `exports_laws`, `cli.rs`, `aqc_corpus_generation`, `aqc_cucumber`, `scripts/contract-gate.sh`

- [ ] **Step 1: Compile** — `cargo run -p atlas-graph --bin atlas-graph-compile -- --data-dir ../data/compiled --out ../data/compiled/graph.bin` (~6 min). Then `git status --short data/`: EXPECTED exactly the three `data/exports/*.json` modified (root and, from ORDER-1, chronology row order) and NOTHING under `data/compiled/` (graph.bin byte-identical — if it moved, STOP: pids are not stored, so a moved graph.bin means ORDER-1 changed a stored order; report which). Read the new root off the compile's `atlas_version_root=` line.

- [ ] **Step 2: Pacts** — `ATLAS_BLESS_PACT=1 cargo test -p atlas-server --test contract_pact` (fails on purpose), `cargo test -p atlas-server --test contract_pact` (green), then `ATLAS_BLESS_PACT=1 cargo test -p atlas-cli --test contract_pact_cli`, `cargo test -p atlas-cli --test contract_pact_cli` (green). `git diff --stat contracts/pacts`: only `version` values and `edge` ids should differ; read the diff and confirm no other key moved.

- [ ] **Step 3: Graph contract fixtures** — from the repo root: `RUNNER="$(cd contracts/runner && cabal list-bin contract-runner)"; "$RUNNER" run --replay contracts/pacts --exports data/exports --bless contracts/atlas-graph-contract` then the same without `--bless` plus `--results /tmp/agc.txt`. `git diff contracts/atlas-graph-contract/fixtures`: only `edges-hazor-1-site-of.json`'s two `edge` values. `VERSION` → `0.4.0`; CHANGELOG entry (prepend):

```markdown
- **0.4.0** (Batch DB-4a) — **MINOR under the 0.x policy (MAJOR class: a blessed fixture moved).**
  Every edge id moved once: `canon-ids` is on for the atlas, so an id is
  SHA-256-128 over canonical edge bytes (`{object, rel, subject}` as position
  strings), 32 hex, no longer a `DefaultHasher` digest of debug text (spec
  §3.1, §3.4 as amended by DB-4a). `edges-hazor-1-site-of.json`'s two ids are
  the only fixture values that changed; no scenario text moved. The version
  root (32 hex now, the manifest root over the four sections) is asserted as a
  law between artifacts, never as a literal, so nothing else in this suite
  re-blessed.
```

- [ ] **Step 4: AQC fixtures** — `cargo run -p atlas-server --bin export_aqc_examples`. `git diff --stat contracts/atlas-query-contract`: fixtures and `index.json` only; `git diff contracts/atlas-query-contract/features` MUST be empty. `VERSION` → `0.2.0`; CHANGELOG line: `- 0.2.0 (DB-4a): re-recorded through the one assembly path — every fixture's "version" is the 32-hex manifest root and every "edge" a canonical edge id; scene-time/scene-scripture bodies follow ORDER-1's total chronology order; no scenario text changed (MAJOR class, MINOR bump under the pre-launch 0.x rule).` Run `cargo test -p atlas-server --test aqc_corpus_generation --test aqc_cucumber -- --test-threads=4`.

- [ ] **Step 5: The CLI transcript** — `cargo run -p atlas-cli --bin bibex -- --data-dir ../data/compiled edges Event:ab_ur --kind located-at`; paste the two output lines verbatim into `cli.rs:578` (the `{:<24}` column now contains a 42-char id, so the spacing changes too); add one comment line above: `// DB-4a: re-pinned once -- canon-ids ON (32-hex edge id); spec 3.6.` Run `cargo test -p atlas-cli -- --test-threads=4`: green.

- [ ] **Step 6: The gate** — `bash scripts/contract-gate.sh`. Expected: `CONTRACT GATE: PASSED`, leg 5 reporting both suites `ok (diff requires major, declared minor = major under the 0.x rule, …)`. Also `cargo test -p atlas-graph --test exports_laws` green.

- [ ] **Step 7: Commit (one commit: the pins and their bumps travel together)**

```bash
git add data/exports contracts/pacts contracts/atlas-graph-contract contracts/atlas-query-contract server/atlas-cli/tests/cli.rs
git commit -m "chore(contracts, exports, cli): DB-4a -- every pinned id and root re-recorded once (spec 3.6): atlas-graph-contract 0.4.0, atlas-query-contract 0.2.0, pacts, exports, the bibex transcript"
```

---

### Task 5: Spec amendment, measurements, the standing block

**Files:**
- Modify: `docs/superpowers/specs/2026-09-14-relational-artifact-design.md` (§3.4 identity table, the `edge id` row: `domain prefix + canonical edge bytes {"object","rel","subject"} (DB-4a judgment call 1; the per-row key collided across a Contains row's N loci)`), `server/BENCHMARKS.md` (append `## DB-4a`), `.superpowers/sdd/2026-08-17-bible-atlas-m1/progress.md`
- Test: the standing block, detached, `RUST_TEST_THREADS=4`

- [ ] **Step 1: BENCHMARKS** — append the table: gate 1 load (2.71 s → measured), gate 9 (377.7 s → measured; 16-byte BLOBs), section byte sizes under ON (from the compile's DB-2b lines), the root before/after (`dfcf6ee4c2a39965` → the 32-hex root), the count of re-recorded literals (26 roots, 29 edge ids, per the survey), and the ORDER-1 count of moved scene windows from Task 1.

- [ ] **Step 2: Standing block** — the DB-3 wrapper (`.superpowers/sdd/2026-08-17-bible-atlas-m1/logs-db3-standing/run.sh`) copied to `logs-db4a-standing/`, launched detached. Expected: workspace green; graph-types green OFF and ON; `TIMING GATES: 9/9 passed`.

- [ ] **Step 3: Commit; then finishing-a-development-branch (push per the batch convention)**

```bash
git add docs/superpowers/specs/2026-09-14-relational-artifact-design.md server/BENCHMARKS.md
git commit -m "docs: DB-4a -- spec 3.4 edge-id row amended; identity cutover measurements"
```

---

## Self-review (done while writing)

**Spec coverage.** §3.1 defects 1 (root blind to rows → manifest root over per-section dumps, Task 2), 2 (`DefaultHasher` → ON, Task 3), 3 (toolchain — already pinned), 4 (debug-print bytes → canonical node bytes ON, canonical edge bytes Task 2). §3.3 canon — DB-2a, consumed. §3.4 identity table: node pid ✓ (ON), edge id ✓ (amended, judgment call 1), section logical hash ✓ (`logical_dump_section`), version root ✓ (`version_root`). §3.5 `bibex verify` → DB-4b. §3.6 "what moves once": pids, edge ids, root, CLI transcript, AQC fixtures (Task 4); map-generator's pin → out of scope by owner, stated. §2.2 root line format `name|logical|schema_version|required` ✓. §5.0 `meta.hash_width` = "16" ✓ (follows `HASH_WIDTH`); `meta.graph_version` dropped (judgment call 2; it was DB-2b's own judgment call 4 placeholder). §6.2 gate: `assert_answers_match(&SqliteSnapshot, &graph)` still runs in the compile binary and gate 9; the root equality law added (Task 2 Step 5). §8 row 5 items not in this plan are named in Architecture as DB-4b/DB-4c. §9 bumps: AGC → 0.4.0 (spec said 0.3.0 at DB-4; DB-3 took 0.3.0 and its CHANGELOG pre-announced 0.4.0), AQC → 0.2.0 ✓; `graph-types` no bump (constraint 1). ORDER-1 (owner finding) → Task 1.

**Placeholder scan.** Task 1 Step 1 names the helper and field names to read and the rule for the `seq`/`order_key` question. Task 3 Step 2's mitigation is concrete (the override line) with the STOP rule. Task 4 lists expected diffs and STOP conditions. No "similar to Task N"; no TBD.

**Type consistency.** `logical_hash(&[u8]) -> ContentHash` in graph-types, rendered `.hex()` by the sqlite module's `logical_hash(..) -> String` shim (Task 2 names both). `write_sections(g, compiler, out_dir)` in Tasks 2, 3 (tests), 4 (compile binary). `version_root(&Graph) -> ContentHash` used by `version_of`, the sqlite law and the writer. `Section::SHIPPED` used by `version_root` and the laws. `manifest_lines(&[(&str, &str, u32, bool)])` matches `root_of_lines(&[u8])`.

**Judgment calls the reviewer should hold me to:** 1–7 above; plus Task 2's single temporary `#[ignore]`, removed in Task 3.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-17-db4a-identity-cutover.md`. Two execution options:

1. **Subagent-Driven (recommended)** — a fresh subagent per task, task review between tasks, one final whole-branch review.
2. **Inline Execution** — executing-plans in this session, batch execution with checkpoints.
