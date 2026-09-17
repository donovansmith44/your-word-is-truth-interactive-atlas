# DB-3: The Port Widened, the Vocabulary Extended, the Companions Retired — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the ONE coordinated `graph-types` change spec §8 row 4 names — five new `GraphQuery` methods, `NodeKind::LexiconEntry` + `NodePayload::LexiconEntry`, the `Occurs` relation — implement them on `Graph`, `MemSnapshot` and `SqliteSnapshot`, extend the conformance harness to them, and retire every load-time companion the port now answers, each behind an equivalence test over the real artifact.

**Architecture:** `graph-types` grows additively (a MINOR, 0.1.0 → 0.2.0): five port methods with default compositions where a composition exists, two small side tables on `Graph` (`edge_rows`, `spine_index`) built inside `build_indexes` so `row_provenance` and `position_of` are lookups, and the uninhabited lexicon vocabulary. `SqliteSnapshot` overrides the three methods its indexes answer directly (`node_by_kind`, `edge_by_id`, `spine_by_node`). `GraphService` loses nine companions; every caller (`handlers.rs`, `graph_handlers.rs`, `scene_source.rs`, `find.rs`, `verse.rs`) reads the port instead, and each retirement carries a real-data equivalence test whose oracle is the retired computation moved into the test. Nothing observable changes: wire order, JSON, the 25 scene hashes and the version root stay put.

**Tech Stack:** Rust 1.97.1 pinned (`x86_64-pc-windows-gnu`; map-generator pins the same rustc in lockstep — never move it here); `graph-types` zero-dep; `rusqlite` 0.32 bundled in `atlas-graph` (DB-2b); `contracts/atlas-graph-contract` semver gate (`scripts/contract-semver-gate.sh`).

**Spec:** `docs/superpowers/specs/2026-09-14-relational-artifact-design.md` — §4 (the port, widened), §7.2–7.3 (lexicon vocabulary, uninhabited here), §3.4/§3.6 (identity; nothing moves in this batch), §6.2 (gate), §8 row 4, §9 (version bumps). Surveys this plan argues from: the DB-3 retirement inventory and the map-generator coupling survey, both recorded in `.superpowers/sdd/2026-08-17-bible-atlas-m1/progress.md` under "DB-3 PLAN".

## Global Constraints

- **`graph-types` stays zero-dependency** (`[dependencies]` and `[dev-dependencies]` empty; `graph-types/tests/zero_deps.rs`). Its version moves `0.1.0` → `0.2.0` in this batch and nowhere else (spec §9: "graph-types — one MINOR (DB-3)").
- **The `canon-ids` feature stays OFF and opt-in.** `ContentHash` widening already landed behind it in DB-2a; map-generator (a live path dependency on this worktree's `graph-types`, eight crates) hard-codes `ContentHash` as a `u64` in ~70 places, so flipping the default is a multi-day port there, not a MINOR. DB-3 does nothing further to `ContentHash`.
- **Nothing observable changes:** `/api/*` bodies and orders, the 25 scene hashes (`scene_byte_identity.rs`), every `contracts/` fixture EXCEPT `atlas-graph-contract/fixtures/graph-vocabulary.json` (which gains `LexiconEntry` and `Occurs` — the one intended contract move, Task 1), `version_root_regression.rs`'s `59c6f7dc236f6f23`, the startup root `dfcf6ee4c2a39965`, `data/compiled/graph.bin` byte-identical (`determinism.rs`; `FORMAT_VERSION` stays 13 — an added enum variant at the end of a bincode enum changes no existing bytes), `atlas-cli/tests/cli.rs` transcripts.
- **`Occurs` is appended LAST in the `directed { … }` block of `relations!`** — `sqlite/partition.rs` codes a directed relation as its index in `RelationId::ALL`; inserting anywhere else renumbers every `edge_index.rel`.
- **Row families do not change:** `RowFamily::ALL` stays 21; the `Occurs` ROW struct, its `Graph` field, its table and its provenance sweep land at LEX-1 (spec §7.3), not here. `graph-types/tests/canon_row_vectors.rs`'s "closed at 21" pin stays.
- **map-generator is never edited.** `crates/map-types/Cargo.toml` states "the atlas repo is a READ-ONLY path dependency"; Task 1 proves compatibility with `cargo check --workspace` in `C:\Users\donov\Documents\the-best-maps-ever` and nothing more. Its `atlas-edge` suite is pure HTTP against `:8080` over six endpoints whose JSON this batch does not touch.
- No `#[ignore]` except through `scripts/timing-gates.sh`; no existing ceiling or threshold changes; gate 9's 570 s ceiling stays (Task 3 re-measures and reports).
- Standing block = from `server/`: `cargo test --workspace`, `cargo test -p atlas-graph-types`, `bash ../scripts/timing-gates.sh`, run DETACHED; sum only `^test result:` lines. The ON gate is `cd graph-types && cargo test --features canon-ids`.
- Never build `--release`; never touch port 8080; port 8000 is free. Untracked files never staged: `client.Tests/FrontierMatrix*.cs`, `publish-apphome/`, `tests/ux/frontier-matrix.spec.ts`. No `git stash`.
- Commit trailers on every commit: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W`. Git Bash; `export PATH="$HOME/scoop/apps/gcc/current/bin:$PATH:$HOME/.cargo/bin"`.

## Judgment calls (owner can reverse)

1. **`compose_frontier` is NOT in this batch.** Spec §4 defaults it to `crate::frontier::compose(self, focus, caps)`, but `frontier.rs` has no `compose`, `Frontier` or `FrontierCaps` — the composition lives in the client (`client/Contracts/Frontier.cs`) and the server handlers. Inventing the `Frontier` value type here would be a design decision with no spec behind it; it needs its own brainstorm before DB-4's FQ-1 measurement. DB-3 ships the other five methods.
2. **Three methods are required, two have defaults.** `nodes_of_kind`, `row_provenance`, `position_of` have no composition over the existing five (enumeration, row identity and spine index are not derivable), so they are required methods; `nodes` and `edges_with_nodes` keep the spec's default compositions, and `SqliteSnapshot` does NOT override them (the harness would only compare the composition with itself).
3. **`row_provenance` of a `justified-by` edge id is `None` on every backend.** Those entries are synthesised from grounds (`event_world::add_justified_by`), not rows; the SQLite `edge_index` row carries the SOURCE row's `(row_family, row_id)` for placement only, and the override filters `rel != JustifiedBy` so both backends agree.
4. **`cross_refs_by_from` is NOT retired here.** Its value carries `target_display` (the original citation string), which lives only on the `cross_refs` row; the spec's replacement is "a `kjv.cross_refs` seek" — a row-table read the port does not expose and the server cannot make until it reads SQLite (DB-4). Retiring it through the port would drop a served field. It stays, with a pointer in its doc comment.
5. **`ProvenanceIndex` shrinks to `by_family`/`families` only.** `attests_for_event`, `event_mentions_for_event` and `analogue_for_pair` are per-edge facts `row_provenance` answers; `by_family` is a per-family DISTINCT aggregate (the family set the verse and xrefs handlers serve), which is a section-level scan, not an edge lookup — it stays until DB-4 can `SELECT DISTINCT provenance`.
6. **`era_ids`' chronological order moves into the handler.** `nodes_of_kind` is id-ordered by spec; `/api/eras` sorts by `(from_year, id)` itself from the payloads it already fetches — the wire order is unchanged and pinned by the equivalence test.
7. **`temporal_neighbors` retires into a `GraphService` method, not a port method:** membership ("is this event dated at all") comes from the kept `chronology.chrono.order` policy table; direction comes from the same order (the neighbour earlier in `chrono.order` is `prior`); adjacency comes from `edges(Symmetric(TemporalAdjacency))`. The equivalence test proves the composition equals the retired map over every dated event.
8. **`Graph` gains two derived side tables built in `build_indexes`:** `edge_rows: Vec<EdgeRow>` (sorted by hash; one extra `entry_id` mint per index entry, ~1M, measured in Task 2 against the 4 s load ceiling) and `spine_index: BTreeMap<&'static str, BTreeMap<AnyNodeId, usize>>`. Both are "built, never authored", the same class as `pid_index`.
9. **The contract bump is whatever `scripts/contract-semver-gate.sh` derives.** `graph-vocabulary.json` gains two list elements; under the suite's own table a re-blessed fixture with a different value is MAJOR-class, and under the CDC-1 0.x policy MAJOR-class = MINOR bump: `0.2.0` → `0.3.0` with a CHANGELOG entry. Spec §9 already schedules `0.3.0` at DB-4; Task 1 records that DB-4 will then move to `0.4.0`.

## File Structure

| Path | Responsibility |
|---|---|
| `graph-types/Cargo.toml` (MODIFY) | version `0.2.0` |
| `graph-types/src/id.rs` (MODIFY) | `NodeKind::LexiconEntry`, `LexiconEntryTag`, `pub type LexiconEntryId` |
| `graph-types/src/node.rs` (MODIFY) | `NodePayload::LexiconEntry`, `card` arm |
| `graph-types/src/canon/node.rs` (MODIFY) | `LEXICON_ENTRY_KEYS`, encode/decode arms |
| `graph-types/src/canon/ids.rs` (MODIFY) | `node_kind_str`/`parse_node_kind` arms |
| `graph-types/src/edge.rs` (MODIFY) | `Occurs => "occurs-in" / "words"` appended |
| `graph-types/src/explore.rs` (MODIFY) | `NodePage`, `EdgeEntryWithNode`, `EdgePageWithNodes` |
| `graph-types/src/store.rs` (MODIFY) | `RowRef`, the five methods on `GraphQuery` (+ `Graph`, `MemSnapshot`, `Lying`), `assert_answers_match` extended |
| `graph-types/src/graph.rs` (MODIFY) | `EdgeRow`, `Graph::edge_rows`, `Graph::spine_index`, `row_provenance_of`, built in `build_indexes` |
| `graph-types/tests/canon_vectors.rs` (MODIFY) | 15-kind array; a `LexiconEntry` golden |
| `server/atlas-graph/src/sections.rs` (MODIFY) | `section_of_node` → `Section::Lexicon` arm |
| `server/atlas-graph/src/sqlite/partition.rs` (MODIFY) | `node_kind_ordinal` arm, `ALL: [NodeKind; 15]`, Lexicon-node guard |
| `server/atlas-graph/src/sqlite/writer.rs` (MODIFY) | `node_label` arm |
| `server/atlas-graph/src/sqlite/snapshot.rs` (MODIFY) | overrides: `nodes_of_kind`, `row_provenance`, `position_of` |
| `server/atlas-graph/src/artifact.rs` (MODIFY) | `DtoNodeKind::LexiconEntry`, `DtoPayload::LexiconEntry` (appended) |
| `server/atlas-graph/src/service.rs` (MODIFY) | companions removed; `position_of`/`concord_position_of` become port shims; `persons_at_verse`, `temporal_neighbors_of`, `attests_provenance`, `event_mentions_provenance`, `analogue_provenance` |
| `server/atlas-graph/src/provenance.rs` (MODIFY) | trimmed to `by_family`/`families` |
| `server/atlas-graph/src/scene_source.rs` (MODIFY) | id lists from `nodes_of_kind` |
| `server/atlas-server/src/handlers.rs` (MODIFY) | eras/narratives/polities/verse/event/timeline call sites |
| `server/atlas-server/src/graph_wire.rs` (MODIFY) | `decode_node_id` `LexiconEntry` arm |
| `server/atlas-server/tests/contract_pact.rs` (MODIFY) | `node_kind_manifest!` gains `LexiconEntry` |
| `server/atlas-cli/src/commands/find.rs`, `verse.rs` (MODIFY) | port reads |
| `contracts/atlas-graph-contract/{fixtures/graph-vocabulary.json,VERSION,CHANGELOG.md}` (MODIFY) | the one contract move |
| `server/atlas-graph/tests/sqlite_laws.rs` (MODIFY) | the new methods on the specimen |
| `server/atlas-graph/tests/port_widening_real_data.rs` (NEW) | the equivalence tests for every retired companion (real artifact) |
| `server/BENCHMARKS.md` (MODIFY) | load-path delta from `edge_rows`; gate 9 re-measurement |

---

### Task 1: The vocabulary — `LexiconEntry`, `Occurs`, and every closed match that must learn them

**Files:**
- Modify: `graph-types/Cargo.toml` (`version = "0.2.0"`), `graph-types/src/id.rs:23-40` (enum) and `:56-71` (`kind_tags!`) and `:114-127` (aliases), `graph-types/src/node.rs:61-170` (`NodePayload`) and `:241-258` (`card`), `graph-types/src/canon/node.rs:40-70` (keys), `:95-205` (`payload_to_value`), `:207-328` (`payload_from_value`), `graph-types/src/canon/ids.rs:22-60`, `graph-types/src/edge.rs:72-115` (`relations!` invocation)
- Modify: `graph-types/tests/canon_vectors.rs:186-201` (hand array) and the golden corpus (~`:600-672`)
- Modify: `server/atlas-graph/src/sections.rs:98-122`, `server/atlas-graph/src/sqlite/partition.rs:59-95` and `partition()`'s section slots, `server/atlas-graph/src/sqlite/writer.rs:59`, `server/atlas-graph/src/artifact.rs` (`DtoNodeKind`, `DtoPayload`, `payload_to_dto`, `payload_from_dto`), `server/atlas-server/src/graph_wire.rs:~158`, `server/atlas-server/tests/contract_pact.rs:120-135`
- Modify: `contracts/atlas-graph-contract/fixtures/graph-vocabulary.json`, `contracts/atlas-graph-contract/VERSION`, `contracts/atlas-graph-contract/CHANGELOG.md`
- Test: `graph-types/tests/canon_vectors.rs`, `server/atlas-server/tests/contract_pact.rs`, `server/atlas-graph/tests/{determinism,version_root_regression,canon_real_data}.rs` (unchanged, must stay green)

**Interfaces:**
- Produces (`graph-types`):
  - `NodeKind::LexiconEntry` (15th variant, appended), `LexiconEntryTag`, `pub type LexiconEntryId = NodeId<LexiconEntryTag>;`
  - `NodePayload::LexiconEntry { strong: String, lang: String, lemma: String, translit: Option<String>, pos: Option<String>, glosses: Vec<String>, senses: Vec<String>, domains: Vec<String>, root: Option<String> }` (spec §7.2, appended last)
  - `RelationId::Occurs` with `forward_label() == "occurs-in"`, `inverse_label() == "words"` (18th directed relation, appended last)
  - canon: `LEXICON_ENTRY_KEYS = ["domains","glosses","lang","lemma","pos","root","senses","strong","translit"]` (alphabetical — the encoder emits sorted objects and `expect_exact_keys` enforces the set)

- [ ] **Step 1: Write the failing tests** — append to `graph-types/tests/canon_vectors.rs`:

```rust
#[test]
fn the_lexicon_entry_vocabulary_is_present_and_uninhabited_shaped() {
    use atlas_graph_types::canon::ids::*;
    use atlas_graph_types::canon::Canon;
    use atlas_graph_types::edge::RelationId;
    // The kind names itself and parses back.
    assert_eq!(node_kind_str(NodeKind::LexiconEntry), "LexiconEntry");
    assert_eq!(parse_node_kind("LexiconEntry", "$.kind").unwrap(), NodeKind::LexiconEntry);
    // The relation is appended LAST among directed relations (edge_index.rel codes are positional).
    assert_eq!(RelationId::ALL.last().copied(), Some(RelationId::Occurs));
    assert_eq!(RelationId::Occurs.forward_label(), "occurs-in");
    assert_eq!(RelationId::Occurs.inverse_label(), "words");
    assert_eq!(RelationId::ALL.len(), 18);
    // The payload round-trips through canon with every optional field in both states,
    // and its bytes are pinned (spec §7.2's field set, keys in byte order).
    let full = Node {
        id: nid(NodeKind::LexiconEntry, "G3056"),
        payload: NodePayload::LexiconEntry {
            strong: "G3056".into(),
            lang: "grc".into(),
            lemma: "λόγος".into(),
            translit: Some("logos".into()),
            pos: Some("noun".into()),
            glosses: vec!["word".into(), "speech".into()],
            senses: vec!["a word, uttered by a living voice".into()],
            domains: vec!["33.98".into(), "33.99".into()],
            root: Some("G3004".into()),
        },
        provenance: "stepbible-tbesg".into(),
    };
    let bytes = full.encode();
    assert_eq!(
        String::from_utf8(bytes.clone()).unwrap(),
        r#"{"id":"LexiconEntry:G3056","payload":{"LexiconEntry":{"domains":["33.98","33.99"],"glosses":["word","speech"],"lang":"grc","lemma":"λόγος","pos":"noun","root":"G3004","senses":["a word, uttered by a living voice"],"strong":"G3056","translit":"logos"}},"provenance":"stepbible-tbesg"}"#
    );
    round_trip(&full);
    let bare = Node {
        id: nid(NodeKind::LexiconEntry, "H0430"),
        payload: NodePayload::LexiconEntry {
            strong: "H0430".into(), lang: "hbo".into(), lemma: "אֱלֹהִים".into(),
            translit: None, pos: None, glosses: vec![], senses: vec![], domains: vec![], root: None,
        },
        provenance: "stepbible-tahot".into(),
    };
    round_trip(&bare);
    assert!(String::from_utf8(bare.encode()).unwrap().contains(r#""root":null,"senses":[],"strong":"H0430","translit":null"#));
}
```

  and change the hand array at `:186-201` to 15 entries (add `NodeKind::LexiconEntry` after `CommentaryItem`), and in the golden corpus (`~:600-672`) add a 15th node after `CommentaryItem` (the `full` node above, verbatim) and change `assert_eq!(seen.len(), 14, …)` to `15` with a `NodePayload::LexiconEntry { .. } => "LexiconEntry"` arm in the discriminant match.

- [ ] **Step 2: Run to verify RED** — `cd graph-types && cargo test --test canon_vectors`. Expected: compile errors (`LexiconEntry` unknown).

- [ ] **Step 3: Implement in `graph-types`**:
  - `id.rs`: add `LexiconEntry,` after `CommentaryItem,` in `NodeKind` (with the doc line `/// DB-3 (spec §7.2): a Strong's-keyed lexicon entry; uninhabited until LEX-1.`); add `LexiconEntryTag => LexiconEntry,` as the last `kind_tags!` row; add `pub type LexiconEntryId = NodeId<LexiconEntryTag>;` after `CommentaryItemId`.
  - `edge.rs` `relations!`: after `DerivedFrom => "derived-from" / "derives"` add `,` and
    ```rust
            /// DB-3 (spec §7.3): entry -> word locus (one token). Rows arrive at LEX-1;
            /// appended LAST because `edge_index.rel` is positional.
            Occurs      => "occurs-in" / "words"
    ```
  - `node.rs`: append the variant to `NodePayload` (exact fields above, with the spec's comments), and in `card` (`:241-258`) add `NodePayload::LexiconEntry { lemma, .. } => lemma.clone()` in whatever position the match's label extraction uses (read the arm for `CatechismItem` and mirror it).
  - `canon/node.rs`: add `const LEXICON_ENTRY_KEYS: &[&str] = &["domains","glosses","lang","lemma","pos","root","senses","strong","translit"];`; in `payload_to_value` add
    ```rust
        NodePayload::LexiconEntry { strong, lang, lemma, translit, pos, glosses, senses, domains, root } => variant(
            "LexiconEntry",
            obj(vec![
                ("domains", vec_str(domains)),
                ("glosses", vec_str(glosses)),
                ("lang", str_value(lang)),
                ("lemma", str_value(lemma)),
                ("pos", opt_str(pos)),
                ("root", opt_str(root)),
                ("senses", vec_str(senses)),
                ("strong", str_value(strong)),
                ("translit", opt_str(translit)),
            ]),
        ),
    ```
    and in `payload_from_value` add, before the `other =>` arm,
    ```rust
        "LexiconEntry" => {
            expect_exact_keys(m, &p, LEXICON_ENTRY_KEYS)?;
            Ok(NodePayload::LexiconEntry {
                strong: field_str(m, &p, "strong")?,
                lang: field_str(m, &p, "lang")?,
                lemma: field_str(m, &p, "lemma")?,
                translit: field_opt_str(m, &p, "translit")?,
                pos: field_opt_str(m, &p, "pos")?,
                glosses: field_vec_str(m, &p, "glosses")?,
                senses: field_vec_str(m, &p, "senses")?,
                domains: field_vec_str(m, &p, "domains")?,
                root: field_opt_str(m, &p, "root")?,
            })
        }
    ```
  - `canon/ids.rs`: `NodeKind::LexiconEntry => "LexiconEntry"` in `node_kind_str`; `"LexiconEntry" => Ok(NodeKind::LexiconEntry)` in `parse_node_kind`.
  - `Cargo.toml`: `version = "0.2.0"` with the comment `# DB-3: the one coordinated MINOR (spec §9) -- five port methods, LexiconEntry, Occurs. Additive only.`

- [ ] **Step 4: graph-types GREEN in both states** — `cargo test` and `cargo test --features canon-ids` from `graph-types/`. Expected: all pass; `every_node_kind_names_itself_with_its_debug_name` (15) and the golden corpus (15) green; `zero_deps` green.

- [ ] **Step 5: Teach every closed atlas-side match** (the workspace does not compile until all are done):
  - `sections.rs` `section_of_node`: add `NodePayload::LexiconEntry { .. } => Section::Lexicon,` before the Core group, with the comment `// DB-3: the lexicon section's own kind (spec §7); no inhabitant until LEX-1.`
  - `sqlite/partition.rs`: `NodeKind::LexiconEntry => 14,` in `node_kind_ordinal`; `ALL: [NodeKind; 15]` with `NodeKind::LexiconEntry` appended; in `partition()`, replace `let slot = |s| sections.iter().position(..).expect(..)` usage for NODES with a fallible lookup: a node whose section is `Lexicon` returns `Err(SqliteError(format!("node {} routes to the lexicon section, which has no tables before LEX-1", any_node_id_str(&n.id))))` — never a silent drop.
  - `sqlite/writer.rs` `node_label`: `NodePayload::LexiconEntry { lemma, .. } => Some(lemma),`.
  - `artifact.rs`: append `LexiconEntry` to `DtoNodeKind` and both `From` impls; append `LexiconEntry { strong: String, lang: String, lemma: String, translit: Option<String>, pos: Option<String>, glosses: Vec<String>, senses: Vec<String>, domains: Vec<String>, root: Option<String> }` to `DtoPayload` (LAST — bincode variant indices are positional; appending keeps every existing `graph.bin` byte) and the two conversion arms field-by-field. Add a comment on `FORMAT_VERSION` (stays 13): `// DB-3 appended DtoNodeKind::LexiconEntry / DtoPayload::LexiconEntry at the END of their enums: no existing variant index moved, so a graph.bin written before DB-3 decodes identically (determinism.rs + version_root_regression.rs are the proof); the wall stays at 13.`
  - `graph_wire.rs` `decode_node_id`: `"LexiconEntry" => Some(AnyNodeId { kind: NodeKind::LexiconEntry, raw: rest.to_string() }),` with the one-arm comment pattern the file uses.
  - `contract_pact.rs` `node_kind_manifest![…, CommentaryItem, LexiconEntry]`.
  - `contracts/atlas-graph-contract/fixtures/graph-vocabulary.json`: add `"LexiconEntry"` after `"CommentaryItem"` in `node_kinds`; add `{ "forward": "occurs-in", "inverse": "words", "name": "Occurs" }` after the `DerivedFrom` object in `relations`. Keep the file's 4-space indentation and key order exactly.

- [ ] **Step 6: Workspace GREEN** — from `server/`: `cargo test -p atlas-server --test contract_pact`, `cargo test -p atlas-graph --test determinism --test version_root_regression --test canon_real_data --test sqlite_laws`, `cargo test -p atlas-cli`. Expected: all pass; `version_root_regression` still `59c6f7dc236f6f23`; `cli.rs` `kinds_plain_lists_the_full_vocabulary` green (it is count-free; `bibex kinds` now also lists `occurs-in`/`words`).

- [ ] **Step 7: The contract bump, derived not asserted** — from the repo root: `bash scripts/contract-semver-gate.sh` (read its usage header first; it compares against `origin/worktree-bible-atlas-m1`, so run it AFTER committing Step 5's files locally — commit first with the message below, then run the gate, then amend nothing: make the VERSION/CHANGELOG change its own second commit). Expected verdict: the fixture value changed → MAJOR class → under the 0.x policy `0.2.0` → `0.3.0`. Write `contracts/atlas-graph-contract/VERSION` = `0.3.0` and prepend to the CHANGELOG list:

```markdown
- **0.3.0** (Batch DB-3) — **MINOR under the 0.x policy (MAJOR class: a blessed fixture moved).**
  `graph-vocabulary.json` gains one node kind (`LexiconEntry`) and one directed
  relation (`Occurs`: `occurs-in` / `words`), both uninhabited until LEX-1 (spec
  §7). `artifact_format_version` stays 13: the new variants are appended, so
  every pre-DB-3 `graph.bin` decodes byte-identically. DB-4's own move (root
  semantics + id width, spec §9) therefore lands as 0.4.0, not 0.3.0.
```

  Re-run the gate; expected: passes with the declared bump.

- [ ] **Step 8: map-generator compiles against the change (read-only proof)** — `cd "$HOME/Documents/the-best-maps-ever" && cargo check --workspace 2>&1 | tail -3`. Expected: `Finished` with no errors (it imports none of the touched types; this is the C1 additive-extension proof). Do NOT edit anything there; do NOT run its tests (they need its own server on 8090). Record the output line in the task report.

- [ ] **Step 9: Commit (two commits)**

```bash
git add graph-types/Cargo.toml graph-types/src/id.rs graph-types/src/node.rs graph-types/src/canon/node.rs graph-types/src/canon/ids.rs graph-types/src/edge.rs graph-types/tests/canon_vectors.rs server/atlas-graph/src/sections.rs server/atlas-graph/src/sqlite/partition.rs server/atlas-graph/src/sqlite/writer.rs server/atlas-graph/src/artifact.rs server/atlas-server/src/graph_wire.rs server/atlas-server/tests/contract_pact.rs contracts/atlas-graph-contract/fixtures/graph-vocabulary.json
git commit -m "feat(graph-types, graph, server): DB-3 -- NodeKind::LexiconEntry, NodePayload::LexiconEntry and the Occurs relation, uninhabited (spec 7.2-7.3); graph-types 0.2.0"
git add contracts/atlas-graph-contract/VERSION contracts/atlas-graph-contract/CHANGELOG.md
git commit -m "chore(contracts): atlas-graph-contract 0.3.0 -- vocabulary fixture gains LexiconEntry + Occurs (DB-3)"
```

---

### Task 2: The port, widened — five methods on `GraphQuery`, `Graph`, `MemSnapshot`; the harness extended

**Files:**
- Modify: `graph-types/src/explore.rs` (after `EdgePage`), `graph-types/src/store.rs:28-69` (trait + `Graph` impl), `:309-325` (`MemSnapshot`), `:428-476` (`assert_answers_match`), `:705-729` (`Lying`), `graph-types/src/graph.rs` (struct fields, `build_indexes`, new fns)
- Test: `graph-types/src/store.rs` `mod laws` (append), `graph-types/src/graph.rs` `mod row_edge_laws` (append)

**Interfaces:**
- Produces (`explore.rs`):

```rust
/// One page of node ids of one kind, in id (byte) order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodePage { pub ids: Vec<AnyNodeId>, pub next: Option<usize> }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeEntryWithNode { pub entry: EdgeEntry, pub node: Option<Node> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgePageWithNodes { pub kind: EdgeKind, pub entries: Vec<EdgeEntryWithNode>, pub next: Option<usize> }
```

  (`Node` must be `PartialEq` for this derive: it is not today — `NodePayload` carries `f64`. Rule: derive `Clone, Debug` only on the two `*WithNode*` types, and compare in tests via `node_eq`-style `format!("{:?}")`, exactly as `assert_answers_match` already compares nodes.)

- Produces (`store.rs`):

```rust
/// Which row produced an edge, and its provenance (spec §4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowRef { pub family: crate::canon::RowFamily, pub row_id: u64, pub provenance: crate::ingest::ProvenanceId }

pub trait GraphQuery {
    // the existing five, unchanged …
    /// All node ids of one kind, in id (byte) order, paged: `next = Some(cursor + ids.len())`
    /// iff more remain (EdgePage's own rule). Retires the six id lists.
    fn nodes_of_kind(&self, kind: NodeKind, cursor: Option<usize>, limit: usize) -> NodePage;
    /// Batch lookup; position i answers ids[i].
    fn nodes(&self, ids: &[AnyNodeId]) -> Vec<Option<Node>> { ids.iter().map(|i| self.node(i)).collect() }
    /// One page WITH each target node (edge positions have no node).
    fn edges_with_nodes(&self, p: &Position, q: &EdgeQuery) -> EdgePageWithNodes {
        let page = self.edges(p, q);
        let ids: Vec<AnyNodeId> = page.entries.iter().filter_map(|e| match &e.node { Position::Node(id) => Some(id.clone()), Position::Edge(_) => None }).collect();
        let mut looked = self.nodes(&ids).into_iter();
        let entries = page.entries.into_iter().map(|entry| {
            let node = match &entry.node { Position::Node(_) => looked.next().flatten(), Position::Edge(_) => None };
            EdgeEntryWithNode { entry, node }
        }).collect();
        EdgePageWithNodes { kind: page.kind, entries, next: page.next }
    }
    /// The row behind an edge id; `None` for a synthesised (justified-by) edge or an unknown id.
    fn row_provenance(&self, e: &EdgeId) -> Option<RowRef>;
    /// Index of a unit in a corpus's reading spine; `None` off-spine or unknown corpus.
    fn position_of(&self, corpus: &'static str, id: &AnyNodeId) -> Option<usize>;
}
```

- Produces (`graph.rs`):

```rust
/// One index entry's row, keyed by the edge id's hash: the in-memory `row_provenance`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeRow { pub hash: crate::id::ContentHash, pub family: crate::canon::RowFamily, pub row_ord: u32 }
impl Graph {
    // new fields (built, never authored):
    //   pub edge_rows: Vec<EdgeRow>,                                    // sorted by hash; first row wins on a duplicate id
    //   pub spine_index: BTreeMap<&'static str, BTreeMap<AnyNodeId, usize>>,
    /// The `provenance` field of one row, by family and ord (the 21-arm match, one place).
    pub fn row_provenance_of(&self, family: RowFamily, row_ord: usize) -> Option<&str>;
    /// Parse `"Rel:hex"` and look the hash up in `edge_rows`.
    pub fn edge_row(&self, e: &EdgeId) -> Option<EdgeRow>;
}
```

- [ ] **Step 1: Write the failing laws** — in `graph-types/src/store.rs` `mod laws`, append:

```rust
    #[test]
    fn nodes_of_kind_pages_in_id_order_with_edge_page_semantics() {
        let g = with_edges(graph_with(&[("bible/1.1.2", "b"), ("bible/1.1.1", "a"), ("bible/1.1.3", "c")]));
        let all = g.nodes_of_kind(NodeKind::TextUnit, None, 10);
        let raws: Vec<&str> = all.ids.iter().map(|i| i.raw.as_str()).collect();
        assert_eq!(raws, ["bible/1.1.1", "bible/1.1.2", "bible/1.1.3"], "byte order of raw within the kind");
        assert_eq!(all.next, None);
        let first = g.nodes_of_kind(NodeKind::TextUnit, None, 2);
        assert_eq!((first.ids.len(), first.next), (2, Some(2)));
        let rest = g.nodes_of_kind(NodeKind::TextUnit, Some(2), 2);
        assert_eq!((rest.ids.len(), rest.next), (1, None));
        assert_eq!(g.nodes_of_kind(NodeKind::TextUnit, Some(9), 2), NodePage { ids: vec![], next: None });
        assert_eq!(g.nodes_of_kind(NodeKind::Place, None, 5).ids.len(), 1, "with_edges adds one Place node? -- read with_edges; assert the real count");
        assert_eq!(g.nodes_of_kind(NodeKind::Polity, None, 5), NodePage { ids: vec![], next: None });
        // limit 0 with entries remaining: next = Some(cursor) (explore.rs's rule, mirrored)
        assert_eq!(g.nodes_of_kind(NodeKind::TextUnit, Some(1), 0).next, Some(1));
    }

    #[test]
    fn nodes_answers_positionally_and_edges_with_nodes_carries_the_targets() {
        let g = with_edges(graph_with(&[("bible/1.1.1", "a")]));
        let ids = vec![AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() }, AnyNodeId { kind: NodeKind::TextUnit, raw: "nope".into() }];
        let got = g.nodes(&ids);
        assert!(got[0].is_some() && got[1].is_none());
        let e1 = Position::Node(EventId::new("e1").erase());
        let kind = crate::edge::EdgeKind::Directed(crate::edge::RelationId::LocatedAt, crate::edge::Direction::Forward);
        let page = g.edges_with_nodes(&e1, &EdgeQuery { kind, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 1);
        let target = page.entries[0].node.as_ref().expect("the place node exists");
        assert_eq!(target.id, PlaceId::new("p1").erase(), "read with_edges for the real place id and pin it");
        assert_eq!(page.entries[0].entry, g.edges(&e1, &EdgeQuery { kind, cursor: None, limit: 10 }).entries[0]);
    }

    #[test]
    fn row_provenance_names_the_row_and_position_of_names_the_spine_slot() {
        let g = with_edges(graph_with(&[("bible/1.1.1", "a"), ("bible/1.1.2", "b")]));
        let e1 = Position::Node(EventId::new("e1").erase());
        let kind = crate::edge::EdgeKind::Directed(crate::edge::RelationId::LocatedAt, crate::edge::Direction::Forward);
        let entry = &g.edges(&e1, &EdgeQuery { kind, cursor: None, limit: 1 }).entries[0];
        let r = g.row_provenance(&entry.edge).expect("a located_at row produced this edge");
        assert_eq!((r.family, r.row_id, r.provenance.as_str()), (crate::canon::RowFamily::LocatedAt, 0, "p"));
        assert_eq!(g.row_provenance(&crate::edge::EdgeId("LocatedAt:0000000000000000".into())), None);
        assert_eq!(g.row_provenance(&crate::edge::EdgeId("garbage".into())), None);
        let id = AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.2".into() };
        assert_eq!(g.position_of("bible", &id), Some(1));
        assert_eq!(g.position_of("concord", &id), None);
        assert_eq!(g.position_of("bible", &EventId::new("e1").erase()), None);
        // MemSnapshot delegates every new method.
        let mut store = MemStore::default();
        let v = store.publish(with_edges(graph_with(&[("bible/1.1.1", "a"), ("bible/1.1.2", "b")])));
        let snap = store.open(v).unwrap();
        assert_eq!(snap.position_of("bible", &id), Some(1));
        assert_eq!(snap.row_provenance(&entry.edge).map(|r| r.family), Some(crate::canon::RowFamily::LocatedAt));
        assert_eq!(snap.nodes_of_kind(NodeKind::TextUnit, None, 9).ids.len(), 2);
    }

    #[test]
    fn the_harness_catches_a_snapshot_that_lies_about_the_new_methods() {
        struct LiesAboutRows(MemSnapshot);
        impl GraphQuery for LiesAboutRows {
            fn node(&self, id: &AnyNodeId) -> Option<Node> { self.0.node(id) }
            fn derive(&self, pid: &Pid) -> Option<Vec<u8>> { self.0.derive(pid) }
            fn edge_summary(&self, p: &Position) -> EdgeSummary { self.0.edge_summary(p) }
            fn edges(&self, p: &Position, q: &EdgeQuery) -> EdgePage { self.0.edges(p, q) }
            fn reading_window(&self, c: &'static str, s: usize, n: usize) -> Vec<AnyNodeId> { self.0.reading_window(c, s, n) }
            fn nodes_of_kind(&self, k: NodeKind, c: Option<usize>, l: usize) -> NodePage { self.0.nodes_of_kind(k, c, l) }
            fn row_provenance(&self, e: &crate::edge::EdgeId) -> Option<RowRef> { self.0.row_provenance(e).map(|mut r| { r.provenance.push('!'); r }) }
            fn position_of(&self, c: &'static str, id: &AnyNodeId) -> Option<usize> { self.0.position_of(c, id) }
        }
        let g = with_edges(graph_with(&[("bible/1.1.1", "a"), ("bible/1.1.2", "b")]));
        let mut store = MemStore::default();
        let v = store.publish(with_edges(graph_with(&[("bible/1.1.1", "a"), ("bible/1.1.2", "b")])));
        let snap = store.open(v).unwrap();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| assert_answers_match(&LiesAboutRows(snap), &g)));
        assert!(caught.is_err(), "a provenance lie must fail conformance");
    }
```

  (Two asserts above say "read with_edges … and pin it": the implementer reads `with_edges` (`store.rs:~500-540`), replaces those two assertions with the real count / real place id, and deletes the prose.)

- [ ] **Step 2: RED** — `cd graph-types && cargo test store::laws`. Expected: compile errors (`nodes_of_kind`, `NodePage`, `RowRef` undefined).

- [ ] **Step 3: Implement**
  - `explore.rs`: the three structs (derives as ruled above). `NodePage` derives `Clone, Debug, PartialEq, Eq`.
  - `graph.rs`: add `pub edge_rows: Vec<EdgeRow>` and `pub spine_index: BTreeMap<&'static str, BTreeMap<AnyNodeId, usize>>` to `Graph` (after `pid_index`, in the "built, never authored" block, doc-commented as DB-3's `row_provenance`/`position_of` lookups). In `build_indexes`, change the first loop to keep the edges: `let edges = self.row_edges();` then the existing `for e in edges.iter()` pushing `(e.subject.clone(), e.object.clone(), e.meta.clone())` into `pairs`/`sym_pairs` — this clones where it used to move; then AFTER the parallel section (so it does not disturb PERF-2b's measured chunking) build
    ```rust
        // DB-3: edge id -> row, the in-memory `row_provenance`. One more id
        // mint per index entry (the same `entry_id` the BiIndex just minted);
        // sorted so lookup is a binary search; a duplicate id (two rows with an
        // identical (rel, subject, object)) keeps the FIRST row, as
        // `sqlite::partition::edge_row_map` does.
        let mut edge_rows: Vec<EdgeRow> = edges.iter().map(|e| EdgeRow { hash: edge_hash(&Graph::edge_id_of(e)), family: e.family, row_ord: e.row_ord as u32 }).collect();
        edge_rows.sort_by_key(|r| (r.hash, r.row_ord));
        edge_rows.dedup_by_key(|r| r.hash);
        self.edge_rows = edge_rows;
        self.spine_index = self.reading.iter().map(|(corpus, spine)| (*corpus, spine.order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect())).collect();
    ```
    with `fn edge_hash(e: &EdgeId) -> ContentHash` = `ContentHash::from_hex(e.0.split_once(':').map(|(_, h)| h).unwrap_or(""))` … returning `Option` — write it as `pub fn edge_hash(e: &EdgeId) -> Option<ContentHash>` and `.filter_map` above. (`ContentHash` derives `Ord` in both feature states — check `id.rs:182-205`; if it does not, add `PartialOrd, Ord` to both cfg'd derives: this is additive.) Measure `build_indexes` before/after on the real graph (Task 7 records it; the 4 s load ceiling, `artifact_conformance.rs`, is the gate).
  - `graph.rs` `row_provenance_of(family, row_ord)`: the 21-arm match `RowFamily::Attests => self.attests.get(row_ord).map(|r| r.provenance.as_str())` … for every family (`Contains*` rows have `provenance` too). `edge_row(e)`: `edge_hash(e)` then `self.edge_rows.binary_search_by_key(&h, |r| r.hash).ok().map(|i| self.edge_rows[i])`.
  - `store.rs`: `RowRef`; the trait additions; `impl GraphQuery for Graph`: `nodes_of_kind` = `self.nodes.range(AnyNodeId { kind, raw: String::new() }..).take_while(|(id, _)| id.kind == kind).map(|(id, _)| id.clone()).skip(start).take(limit + 1)` then split off the extra to compute `next` exactly like `edges` (`next = Some(start + ids.len())` iff an extra row existed); `row_provenance` = `self.edge_row(e).and_then(|r| self.row_provenance_of(r.family, r.row_ord as usize).map(|p| RowRef { family: r.family, row_id: r.row_ord as u64, provenance: p.to_string() }))`; `position_of` = `self.spine_index.get(corpus).and_then(|m| m.get(id)).copied()`. `MemSnapshot`: delegate all three. `Lying` (the existing lying test): add the three delegations.
  - `logical_dump`'s exhaustive destructure (`store.rs:185-212`, `canon-ids` ON): add `edge_rows: _, spine_index: _,`. Any other exhaustive `Graph { … }` destructure (grep `pid_index: _` across `graph-types/`, `server/atlas-graph/`) gets the same two lines.
  - `assert_answers_match`: after the existing per-position loop, add
    ```rust
        // DB-3: the five widened methods, over the same inventory.
        for kind in ALL_NODE_KINDS { // a local const listing every NodeKind variant; the compiler cannot enumerate an enum, so this list is the one hand-written place -- keep it beside `node_kind_str` in spirit and assert its length against `canon::ids` in a graph-types test
            let mut cursor = None;
            let mut got: Vec<AnyNodeId> = Vec::new();
            loop {
                let page = candidate.nodes_of_kind(kind, cursor, 97);
                got.extend(page.ids);
                match page.next { Some(c) => cursor = Some(c), None => break }
            }
            let want: Vec<AnyNodeId> = model.nodes_of_kind(kind, None, usize::MAX).ids;
            assert_eq!(got, want, "conformance: nodes_of_kind({kind:?}) diverges");
        }
        let all_ids: Vec<AnyNodeId> = model.nodes.keys().cloned().collect();
        for chunk in all_ids.chunks(1000) {
            let a = candidate.nodes(chunk);
            let b = model.nodes(chunk);
            assert_eq!(a.len(), b.len());
            for (x, y) in a.iter().zip(&b) { assert!(node_eq(x, y), "conformance: nodes() diverges"); }
        }
        for p in position_inventory(model) {
            for (kind, _) in model.edge_summary(&p) {
                let q = EdgeQuery { kind, cursor: None, limit: 1 };
                let a = candidate.edges_with_nodes(&p, &q);
                let b = model.edges_with_nodes(&p, &q);
                assert_eq!(a.kind, b.kind); assert_eq!(a.next, b.next); assert_eq!(a.entries.len(), b.entries.len());
                for (x, y) in a.entries.iter().zip(&b.entries) {
                    assert_eq!(x.entry, y.entry, "conformance: edges_with_nodes entry diverges at {p:?}");
                    assert!(node_eq(&x.node, &y.node), "conformance: edges_with_nodes node diverges at {p:?}");
                    assert_eq!(candidate.row_provenance(&x.entry.edge), model.row_provenance(&y.entry.edge), "conformance: row_provenance({:?}) diverges", x.entry.edge);
                }
            }
            if let Position::Node(id) = &p {
                for corpus in model.reading.keys() {
                    assert_eq!(candidate.position_of(corpus, id), model.position_of(corpus, id), "conformance: position_of({corpus}, {id:?}) diverges");
                }
            }
        }
    ```
    `ALL_NODE_KINDS` goes in `graph-types/src/id.rs` as `impl NodeKind { pub const ALL: [NodeKind; 15] = [...] }` (the same hand list `canon_vectors.rs` keeps; the canon test asserts `NodeKind::ALL` equals its own array so there is ONE list to maintain) — the `edge_index.rel`-style rule: appended, never reordered.

- [ ] **Step 4: GREEN both states** — `cargo test` and `cargo test --features canon-ids` from `graph-types/`. Expected: all pass, including the existing conformance laws (the Graph-vs-MemSnapshot pair now also covers the five new methods) and `zero_deps`.

- [ ] **Step 5: The workspace still compiles and the real graph still admits** — from `server/`: `cargo test -p atlas-graph --test artifact_conformance -- --ignored --exact serialized_artifact_is_admitted_and_loads_under_the_committed_ceiling --nocapture --test-threads=1` (gate 1 — records the load time WITH `edge_rows`; expected well under 4 s; report the number against 2.2 s before), then `cargo test -p atlas-graph --test sqlite_laws`. Expected: `sqlite_laws` FAILS to compile — `SqliteSnapshot` lacks the three required methods. That is Task 3's RED; commit this task with `sqlite_laws` red? NO: add the three methods to `SqliteSnapshot` here as `todo!()`? NO (never ship a `todo!`). Rule: Task 2 adds to `sqlite/snapshot.rs` three honest minimal implementations that already satisfy the harness — see Task 3 for their final form; here write them as the SIMPLEST correct SQL (no caching): `nodes_of_kind` = `SELECT id FROM all_node WHERE kind = ?1 ORDER BY id LIMIT ?2 OFFSET ?3` with `limit + 1`; `row_provenance` = the two-step lookup Task 3 specifies; `position_of` = `SELECT ord FROM <section>.reading_spine WHERE node_id = ?1`. Then `cargo test -p atlas-graph --test sqlite_laws` is green (the specimen admission now exercises the new methods on SQLite) and Task 3 only adds laws + measures.

- [ ] **Step 6: Commit**

```bash
git add graph-types/src/explore.rs graph-types/src/store.rs graph-types/src/graph.rs graph-types/src/id.rs graph-types/tests/canon_vectors.rs server/atlas-graph/src/sqlite/snapshot.rs
git commit -m "feat(graph-types): DB-3 -- GraphQuery gains nodes_of_kind, nodes, edges_with_nodes, row_provenance, position_of (spec 4); Graph edge_rows + spine_index; harness covers them"
```

---

### Task 3: `SqliteSnapshot` — the three overrides proven on the specimen and the real graph

**Files:**
- Modify: `server/atlas-graph/src/sqlite/snapshot.rs`
- Test: `server/atlas-graph/tests/sqlite_laws.rs` (append), `server/atlas-graph/tests/sqlite_real_data.rs` (unchanged; re-measured)

**Interfaces:**
- Consumes: Task 2's trait; `all_node`/`all_edge_index` TEMP views (`sec` column = rank in `present`); `rel_of_code`, `directed_rel_code`; `read_rows` is NOT used (one column, one row).
- Produces: the overrides —
  - `nodes_of_kind`: `SELECT id FROM all_node WHERE kind = ?1 ORDER BY id LIMIT ?2 OFFSET ?3` (`?2 = limit + 1`), `parse_any_node_id` each; `next` by the extra-row rule. (`node_by_kind (kind, id)` makes each arm a prefix seek; the union merges four sorted runs.)
  - `row_provenance`: `SELECT sec, row_family, row_id FROM all_edge_index WHERE edge_id = ?1 AND rel != ?2 LIMIT 1` with `?2 = directed_rel_code(RelationId::JustifiedBy)`; then `SELECT provenance FROM <present[sec].name() or "main" for rank 0>.<RowFamily::from_ordinal(row_family).name()> WHERE id = ?1`; `RowRef { family, row_id: row_id as u64, provenance }`. Unknown id → `None`.
  - `position_of`: corpus → section as `reading_window` does; absent section or unknown corpus → `None`; `SELECT ord FROM <schema>.reading_spine WHERE node_id = ?1` (`spine_by_node`).

- [ ] **Step 1: Write the failing laws** (append to `sqlite_laws.rs`):

```rust
#[test]
fn the_sqlite_overrides_answer_the_widened_port_exactly_as_the_specimen_graph() {
    use atlas_graph_types::id::NodeKind;
    let mut g = specimen_graph(); g.build_indexes(); atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db3-snap-{}", std::process::id())); let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, "0123456789abcdef", "test", &dir).unwrap();
    let snap = SqliteSnapshot::open(&dir.join("manifest.toml")).unwrap();
    // nodes_of_kind pages across sections (Container lives in core, kjv AND concord) in one byte order.
    let containers = snap.nodes_of_kind(NodeKind::Container, None, 2);
    assert_eq!(containers.ids.iter().map(|i| i.raw.as_str()).collect::<Vec<_>>(), ["bible-book-GEN", "bible-chapter-GEN-1"]);
    assert_eq!(containers.next, Some(2));
    let rest = snap.nodes_of_kind(NodeKind::Container, Some(2), 10);
    assert_eq!(rest.ids.iter().map(|i| i.raw.as_str()).collect::<Vec<_>>(), ["concord-ac", "concord-ac-1", "passage-creation"]);
    assert_eq!(rest.next, None);
    assert_eq!(snap.nodes_of_kind(NodeKind::LexiconEntry, None, 5).ids.len(), 0, "uninhabited until LEX-1");
    // row_provenance: a directed row, a symmetric row, a justified-by edge (None), an unknown id (None).
    let located = g.edges(&atlas_graph_types::edge::at(&g.located_at[0].event.erase()), &atlas_graph_types::explore::EdgeQuery { kind: atlas_graph_types::edge::EdgeKind::Directed(atlas_graph_types::edge::RelationId::LocatedAt, atlas_graph_types::edge::Direction::Forward), cursor: None, limit: 1 }).entries[0].edge.clone();
    assert_eq!(snap.row_provenance(&located).unwrap().provenance, "curated/events");
    let analogue = g.edges(&atlas_graph_types::edge::at(&g.analogue[0].a.erase()), &atlas_graph_types::explore::EdgeQuery { kind: atlas_graph_types::edge::EdgeKind::Symmetric(atlas_graph_types::edge::SymRelationId::Analogue), cursor: None, limit: 1 }).entries[0].edge.clone();
    assert_eq!(snap.row_provenance(&analogue).map(|r| (r.family, r.provenance)), Some((RowFamily::Analogue, "curated/analogues".into())));
    let justified = g.edges(&atlas_graph_types::id::Position::Edge(located.clone()), &atlas_graph_types::explore::EdgeQuery { kind: atlas_graph_types::edge::EdgeKind::Directed(atlas_graph_types::edge::RelationId::JustifiedBy, atlas_graph_types::edge::Direction::Forward), cursor: None, limit: 1 }).entries[0].edge.clone();
    assert_eq!(snap.row_provenance(&justified), None, "a synthesised edge has no row (judgment call 3)");
    assert_eq!(snap.row_provenance(&atlas_graph_types::edge::EdgeId("LocatedAt:0000000000000000".into())), None);
    // position_of over both spines and off-spine.
    let v2 = &g.reading["bible"].order[1];
    assert_eq!(snap.position_of("bible", v2), Some(1));
    assert_eq!(snap.position_of("concord", v2), None);
    assert_eq!(snap.position_of("bible", &g.located_at[0].event.erase()), None);
    assert_eq!(snap.position_of("nope", v2), None);
    // And the whole harness, which now covers the five methods.
    assert_answers_match(&snap, &g);
}
```

- [ ] **Step 2: RED** — `cargo test -p atlas-graph --test sqlite_laws sqlite_overrides`. Expected: FAIL on whichever Task 2 placeholder query is not yet exact (or PASS outright — then this task's value is the pinned law + the measurement; say so).

- [ ] **Step 3: Implement the overrides exactly as the Interfaces block states** (replacing Task 2's minimal versions if they differ), with `prepare_cached` for all four statements. The row-table statement is built per `(section, family)` pair on demand and cached by `rusqlite`'s statement cache.

- [ ] **Step 4: GREEN** — `cargo test -p atlas-graph --test sqlite_laws`. Expected: all pass.

- [ ] **Step 5: Re-measure gate 9** — `cargo test -p atlas-graph --test sqlite_real_data -- --ignored --nocapture --test-threads=1`. Expected: passes under the 570 s ceiling; record the new `DB-2b GATE:` line (the harness now also drains `nodes_of_kind` per kind, `nodes` in 1000-chunks, `edges_with_nodes` + `row_provenance` on the first page of every kind at every position, and `position_of` per node). If it exceeds 570 s, do NOT loosen: reduce the harness's DB-3 additions to a deterministic 1-in-8 sample of positions (`position_inventory` order, every 8th) and say so in the report — the ceiling is the law, the sample rate is the knob.

- [ ] **Step 6: Commit**

```bash
git add server/atlas-graph/src/sqlite/snapshot.rs server/atlas-graph/tests/sqlite_laws.rs
git commit -m "feat(graph): DB-3 -- SqliteSnapshot overrides nodes_of_kind (node_by_kind), row_provenance (edge_by_id), position_of (spine_by_node); admitted on the specimen and the real graph"
```

---

### Task 4: Retire the six id lists

**Files:**
- Modify: `server/atlas-graph/src/service.rs` (fields `era_ids`, `polity_ids`, `narrative_ids`, `event_ids`, `place_ids`, `person_ids` and their `assemble` blocks `:489-513`; the unit test `person_ids_enumerates_every_person_node` `:854-874`), `server/atlas-server/src/handlers.rs:110-126` (`eras`), `:136-146` (`narratives`), `:262-285` (`polities`), `server/atlas-graph/src/scene_source.rs:184-192`, `server/atlas-cli/src/commands/find.rs:40-49`
- Create: `server/atlas-graph/tests/port_widening_real_data.rs`
- Test: the new file; `server/atlas-server/tests/{api,graph_api,contract_pact,scene_byte_identity}.rs`, `server/atlas-cli/tests/cli.rs` (unchanged, must stay green)

**Interfaces:**
- Produces (`service.rs`): `pub fn ids_of_kind(&self, kind: NodeKind) -> Vec<AnyNodeId>` — drains `self.snapshot.nodes_of_kind(kind, cursor, 4096)` to exhaustion (the one place the pattern is spelled; every retired-list caller uses it). Doc comment: id (byte) order; `era_ids`' chronological order is the eras handler's own concern now.

- [ ] **Step 1: Write the failing equivalence tests** — `port_widening_real_data.rs`:

```rust
//! DB-3 (spec §4, §6.2): every retired companion, proven equal to its port
//! composition over the COMMITTED graph before it is deleted. Each test's
//! ORACLE is the retired computation, moved here verbatim from
//! `service.rs`/`provenance.rs` at the commit that deleted it -- so the
//! equivalence is between the exact code that used to serve and the code
//! that serves now.
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::OnceLock;
use atlas_graph::service::GraphService;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnyNodeId, NodeKind};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;

fn committed_graph() -> &'static Graph {
    static CACHED: OnceLock<Graph> = OnceLock::new();
    CACHED.get_or_init(|| {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled/graph.bin");
        let dump = atlas_graph::artifact::read_file(&path).expect("data/compiled/graph.bin must exist");
        let (mut graph, ..) = atlas_graph::artifact::to_service_parts(dump).expect("to_service_parts");
        graph.build_indexes();
        atlas_graph::event_world::add_justified_by(&mut graph);
        graph
    })
}
fn service() -> &'static GraphService {
    static S: OnceLock<GraphService> = OnceLock::new();
    S.get_or_init(|| GraphService::from_artifact(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled/graph.bin")).expect("from_artifact"))
}

/// ORACLE: service.rs's five alphabetical lists, verbatim.
fn oracle_ids(g: &Graph, kind: NodeKind) -> Vec<AnyNodeId> {
    g.nodes.keys().filter(|id| id.kind == kind).cloned().collect()
}
/// ORACLE: service.rs's era_ids, verbatim (chronological by from_year, then raw).
fn oracle_era_ids(g: &Graph) -> Vec<AnyNodeId> {
    let mut era_nodes: Vec<(i32, AnyNodeId)> = g.nodes.iter().filter_map(|(id, n)| match &n.payload {
        NodePayload::Era { from_year, .. } if id.kind == NodeKind::Era => Some((*from_year, id.clone())), _ => None }).collect();
    era_nodes.sort_by_key(|(from_year, id)| (*from_year, id.raw.clone()));
    era_nodes.into_iter().map(|(_, id)| id).collect()
}

#[test]
fn the_five_alphabetical_id_lists_equal_nodes_of_kind() {
    let g = committed_graph();
    let s = service();
    for kind in [NodeKind::Polity, NodeKind::Narrative, NodeKind::Event, NodeKind::Place, NodeKind::Person] {
        assert_eq!(s.ids_of_kind(kind), oracle_ids(g, kind), "{kind:?}");
        assert!(!s.ids_of_kind(kind).is_empty(), "{kind:?} is inhabited in the shipped graph");
    }
    assert_eq!(s.ids_of_kind(NodeKind::Person).len(), g.nodes.keys().filter(|id| id.kind == NodeKind::Person).count());
}

#[test]
fn the_eras_wire_order_is_reproduced_by_sorting_the_payloads() {
    let g = committed_graph();
    let s = service();
    let snap = s.snapshot();
    // The handler's own new composition (handlers::eras, Step 3): nodes_of_kind, then sort by (from_year, raw).
    let mut got: Vec<(i32, AnyNodeId)> = s.ids_of_kind(NodeKind::Era).into_iter().filter_map(|id| match snap.node(&id)?.payload {
        NodePayload::Era { from_year, .. } => Some((from_year, id)), _ => None }).collect();
    got.sort_by_key(|(y, id)| (*y, id.raw.clone()));
    assert_eq!(got.into_iter().map(|(_, id)| id).collect::<Vec<_>>(), oracle_era_ids(g));
}
```

- [ ] **Step 2: RED** — `cargo test -p atlas-graph --test port_widening_real_data`. Expected: compile error (`ids_of_kind` missing).

- [ ] **Step 3: Implement**
  - `service.rs`: add `ids_of_kind`; delete the six fields, their doc comments and the `assemble` blocks that build them; delete `person_ids_enumerates_every_person_node` (its claim moves to the real-data test above — say so in the commit message).
  - `handlers::eras`: `let mut eras: Vec<(i32, Era)> = graph.ids_of_kind(NodeKind::Era).into_iter().filter_map(|id| { let node = snap.node(&id)?; match node.payload { NodePayload::Era { label, from_year, to_year } => Some((from_year, Era { id: id.raw.clone(), name: label, from_year, to_year })), _ => None } }).collect(); eras.sort_by(|a, b| (a.0, &a.1.id).cmp(&(b.0, &b.1.id)));` and return the `Era`s. Update the doc comment: the order is the handler's own sort now (judgment call 6).
  - `handlers::narratives`, `handlers::polities`: `graph.ids_of_kind(NodeKind::Narrative)` / `NodeKind::Polity` in place of the fields (the polities handler already re-sorts).
  - `scene_source.rs:184-192`: `gs.ids_of_kind(NodeKind::Event)` etc. — the doc comment there says the pre-sort insertion order is load-bearing: it is the SAME id order (BTreeMap key order = `nodes_of_kind` order), so the 25 scene hashes must not move; `scene_byte_identity.rs` is the proof, run in Step 4.
  - `find.rs:40-49`: `let kinds: [(&'static str, NodeKind); 6] = [("Place", NodeKind::Place), ("Event", NodeKind::Event), ("Narrative", NodeKind::Narrative), ("Era", NodeKind::Era), ("Polity", NodeKind::Polity), ("Person", NodeKind::Person)]; for (kind_name, kind) in kinds { for id in graph.ids_of_kind(kind) { … } }` — note `find` used `era_ids` in chronological order; its hits are then sorted/deduped downstream? READ `find.rs` after the loop: if the output order depends on the enumeration order, sort Era ids by `(from_year, raw)` here too so `cli.rs`'s find transcripts stay byte-identical (run them in Step 4; if they move, the old order was load-bearing and this is where it is restored).

- [ ] **Step 4: GREEN** — `cargo test -p atlas-graph --test port_widening_real_data`; `cargo test -p atlas-server --test api --test graph_api --test contract_pact --test scene_byte_identity`; `cargo test -p atlas-cli`. Expected: all pass; 25 scene hashes unchanged; cli transcripts unchanged.

- [ ] **Step 5: Commit**

```bash
git add server/atlas-graph/src/service.rs server/atlas-server/src/handlers.rs server/atlas-graph/src/scene_source.rs server/atlas-cli/src/commands/find.rs server/atlas-graph/tests/port_widening_real_data.rs
git commit -m "refactor(graph, server, cli): DB-3 -- the six id-list companions retired for nodes_of_kind; eras keeps its wire order by sorting payloads; equivalence over the real artifact"
```

---

### Task 5: Retire `bible_position`, `concord_position`, `persons_by_verse`

**Files:**
- Modify: `server/atlas-graph/src/service.rs` (fields `:56`, `:62`, `:166`; `assemble` `:463-474`, `:559-570`; `position_of` `:653`, `concord_position_of` `:660`; tests `:842-843`), `server/atlas-server/src/handlers.rs:569-575`, `server/atlas-cli/src/commands/verse.rs:114`
- Test: `server/atlas-graph/tests/port_widening_real_data.rs` (append); `server/atlas-graph/src/window.rs` tests, `server/atlas-server/tests/perf_smoke.rs` (call `position_of`; unchanged signature, must stay green)

**Interfaces:**
- Produces (`service.rs`):
  - `pub fn position_of(&self, book: u8, chapter: u16, verse: u16) -> Option<usize>` — SAME signature, now `self.snapshot.position_of(kjv_adapter::BIBLE_CORPUS, &kjv_adapter::verse_node_id(book, chapter, verse))`.
  - `pub fn concord_position_of(&self, part: u8, article: u16, paragraph: u16) -> Option<usize>` — likewise over `CONCORD_CORPUS` / `concord_adapter::text_unit_id`.
  - `pub fn persons_at_verse(&self, book: u8, chapter: u16, verse: u16) -> Vec<(String, String)>` — `(person raw id, label)` in mentions-row order: drain `self.snapshot.edges_with_nodes(&at(&verse_node_id(..)), &EdgeQuery { kind: Directed(Mentions, Forward), cursor, limit: 256 })`, keep entries whose `node` is `Some(Node { id: AnyNodeId { kind: Person, .. }, payload: NodePayload::Person { label, .. }, .. })`, map to `(id.raw, label)`.

- [ ] **Step 1: Write the failing equivalence tests** (append):

```rust
/// ORACLE: service.rs's bible_position / concord_position, verbatim.
fn oracle_positions(g: &Graph, corpus: &'static str) -> HashMap<AnyNodeId, usize> {
    g.reading.get(corpus).map(|spine| spine.order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect()).unwrap_or_default()
}
/// ORACLE: service.rs's persons_by_verse, verbatim (Person mentions only, row order, keyed by dotted ref).
fn oracle_persons_by_verse(g: &Graph) -> HashMap<String, Vec<(String, String)>> {
    let mut out: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for row in &g.mentions {
        let atlas_graph_types::edge::MentionedEntity::Person(person_id) = &row.entity else { continue };
        let Some(key) = atlas_graph::legacy::locus_dot_ref(&row.locus) else { continue };
        let Some(label) = g.nodes.get(&person_id.erase()).and_then(|n| match &n.payload { NodePayload::Person { label, .. } => Some(label.clone()), _ => None }) else { continue };
        out.entry(key).or_default().push((person_id.0.clone(), label));
    }
    out
}

#[test]
fn every_spine_slot_is_answered_by_position_of_and_nothing_else_is() {
    let g = committed_graph();
    let s = service();
    for (corpus, decode, ask) in [
        ("bible", atlas_graph::kjv_adapter::decode_text_unit as fn(&AnyNodeId) -> Option<(u8, u16, u16)>, |s: &GraphService, (a, b, c): (u8, u16, u16)| s.position_of(a, b, c)),
        ("concord", atlas_graph::concord_adapter::decode_text_unit as fn(&AnyNodeId) -> Option<(u8, u16, u16)>, |s: &GraphService, (a, b, c): (u8, u16, u16)| s.concord_position_of(a, b, c)),
    ] {
        let oracle = oracle_positions(g, corpus);
        assert!(!oracle.is_empty(), "{corpus} spine is inhabited");
        for (id, want) in &oracle {
            let key = decode(id).unwrap_or_else(|| panic!("{corpus} spine id {id:?} decodes"));
            assert_eq!(ask(s, key), Some(*want), "{corpus} {id:?}");
        }
    }
    assert_eq!(s.position_of(1, 1, 200), None, "GEN 1:200 is off the spine");
    assert_eq!(s.concord_position_of(99, 1, 1), None);
}

#[test]
fn persons_at_verse_equals_the_retired_persons_by_verse_over_every_verse() {
    let g = committed_graph();
    let s = service();
    let oracle = oracle_persons_by_verse(g);
    assert!(oracle.len() > 1000, "the shipped graph mentions persons in thousands of verses: {}", oracle.len());
    for id in &g.reading["bible"].order {
        let (b, c, v) = atlas_graph::kjv_adapter::decode_text_unit(id).unwrap();
        let key = atlas_graph::legacy::locus_dot_ref(&atlas_graph_types::text::TextLocus { at: atlas_graph_types::text::TextRef::Bible(atlas_graph_types::text::VerseRef { book: b, chapter: c, verse: v }), span: None }).unwrap();
        assert_eq!(s.persons_at_verse(b, c, v), oracle.get(&key).cloned().unwrap_or_default(), "{key}");
    }
}
```

- [ ] **Step 2: RED** — `cargo test -p atlas-graph --test port_widening_real_data`. Expected: `persons_at_verse` missing.

- [ ] **Step 3: Implement** — the three methods as specified; delete the three fields and their `assemble` blocks; the two `service.rs` unit tests at `:842-843` keep calling `position_of(b, c, v)` (unchanged signature). Callers: `handlers.rs:569-575` → `graph.persons_at_verse(vid.book.0, vid.chapter, vid.verse)` (read how `vid` is typed there: `VerseId` with `.book.0` the index, as `first_verse_of_target` uses); `verse.rs:114` → the same (the CLI has the parsed ref in scope; read the surrounding code for its name). Keep the `PersonRefOut`/`attached` mapping as is.

- [ ] **Step 4: GREEN** — `cargo test -p atlas-graph --test port_widening_real_data --lib`; `cargo test -p atlas-server --test api --test graph_api --test perf_smoke -- --skip nothing` (perf_smoke's `#[ignore]` gates stay ignored; only compile + its non-gated tests); `cargo test -p atlas-cli`. Expected: green.

- [ ] **Step 5: Commit**

```bash
git add server/atlas-graph/src/service.rs server/atlas-server/src/handlers.rs server/atlas-cli/src/commands/verse.rs server/atlas-graph/tests/port_widening_real_data.rs
git commit -m "refactor(graph, server, cli): DB-3 -- bible_position/concord_position retired for position_of, persons_by_verse for edges_with_nodes over mentions; equivalence over the real artifact"
```

---

### Task 6: Retire the PROV-1 per-edge lookups and `temporal_neighbors`; keep `by_family` and `cross_refs_by_from` with reasons

**Files:**
- Modify: `server/atlas-graph/src/provenance.rs:98-220` (struct + `build` + methods), `server/atlas-graph/src/service.rs` (field `:186`, `assemble` `:590-595`; doc on `cross_refs_by_from` `:142`), `server/atlas-server/src/handlers.rs:1418-1432` (timeline), `:1720-1728` (analogue), `:1762-1763` (witnesses/mentions provenance)
- Test: `server/atlas-graph/tests/port_widening_real_data.rs` (append); `server/atlas-graph/tests/provenance_registry_real_data.rs` (unchanged: `families()` and `by_family` stay; must stay green), `server/atlas-server/tests/graph_api.rs`

**Interfaces:**
- Produces (`service.rs`):
  - `pub fn attests_provenance(&self, event_raw: &str) -> Vec<String>` — distinct, sorted provenance of the `Attests` rows for the event: drain `edges(at(Event id), Directed(Attests, Forward))`, `row_provenance` each, collect into a `BTreeSet`.
  - `pub fn event_mentions_provenance(&self, event_raw: &str) -> Vec<String>` — the same over `Directed(Mentions, Inverse)` at the event position (a `Mentions` row whose entity is this event lowers with the event as OBJECT, so the inverse reading at the event lists exactly those rows).
  - `pub fn analogue_provenance(&self, a_raw: &str, b_raw: &str) -> Option<String>` — drain `edges(at(Event a), Symmetric(Analogue))`, find the entry whose `node == at(Event b)`, `row_provenance(edge).provenance`.
  - `pub fn temporal_neighbors_of(&self, event_raw: &str) -> Option<(Option<String>, Option<String>)>` — `None` unless `self.chronology.chrono.order` contains `event_raw` (the DOMAIN fact, judgment call 7); else `Some((prior, following))` where the neighbours come from `edges(at(Event), Symmetric(TemporalAdjacency))` and each neighbour is `prior` if its index in `chrono.order` is smaller than the event's, `following` otherwise (at most one of each; if two land on one side, keep the nearer index — and assert in the equivalence test that this never happens on the real graph).
- Produces (`provenance.rs`): `ProvenanceIndex { by_family }` only; `build` keeps the 22-family sweep; `by_family` and `families` unchanged; the three per-edge methods and their two maps DELETED. Module header gains: `DB-3: the per-edge lookups (attests/mentions/analogue) moved to GraphQuery::row_provenance (GraphService::{attests_provenance, event_mentions_provenance, analogue_provenance}); by_family stays until DB-4's SELECT DISTINCT (plan judgment call 5).`

- [ ] **Step 1: Write the failing equivalence tests** (append):

```rust
/// ORACLE: provenance.rs's three per-edge maps, verbatim.
fn oracle_provenance(g: &Graph) -> (BTreeMap<String, Vec<String>>, BTreeMap<String, Vec<String>>, BTreeMap<(String, String), String>) {
    use std::collections::BTreeSet;
    let mut attests: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in &g.attests { attests.entry(row.event.0.clone()).or_default().insert(row.provenance.clone()); }
    let mut mentions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for row in &g.mentions { if let atlas_graph_types::edge::MentionedEntity::Event(e) = &row.entity { mentions.entry(e.0.clone()).or_default().insert(row.provenance.clone()); } }
    let mut analogue: BTreeMap<(String, String), String> = BTreeMap::new();
    for row in &g.analogue { analogue.insert((row.a.0.clone(), row.b.0.clone()), row.provenance.clone()); analogue.insert((row.b.0.clone(), row.a.0.clone()), row.provenance.clone()); }
    (attests.into_iter().map(|(k, v)| (k, v.into_iter().collect())).collect(), mentions.into_iter().map(|(k, v)| (k, v.into_iter().collect())).collect(), analogue)
}
/// ORACLE: service.rs's temporal_neighbors, verbatim (domain seeded from chrono.order, direction from the rows).
fn oracle_temporal_neighbors(g: &Graph, order: &[String]) -> HashMap<String, (Option<String>, Option<String>)> {
    let mut m: HashMap<String, (Option<String>, Option<String>)> = order.iter().map(|id| (id.clone(), (None, None))).collect();
    for row in &g.temporal_adjacency {
        m.entry(row.earlier.0.clone()).or_insert((None, None)).1 = Some(row.later.0.clone());
        m.entry(row.later.0.clone()).or_insert((None, None)).0 = Some(row.earlier.0.clone());
    }
    m
}

#[test]
fn per_edge_provenance_through_row_provenance_equals_the_retired_index_for_every_event() {
    let g = committed_graph();
    let s = service();
    let (attests, mentions, analogue) = oracle_provenance(g);
    assert!(attests.len() > 500 && !analogue.is_empty());
    for id in s.ids_of_kind(NodeKind::Event) {
        assert_eq!(s.attests_provenance(&id.raw), attests.get(&id.raw).cloned().unwrap_or_default(), "attests {}", id.raw);
        assert_eq!(s.event_mentions_provenance(&id.raw), mentions.get(&id.raw).cloned().unwrap_or_default(), "mentions {}", id.raw);
    }
    for ((a, b), prov) in &analogue {
        assert_eq!(s.analogue_provenance(a, b).as_deref(), Some(prov.as_str()), "analogue {a} <-> {b}");
    }
    assert_eq!(s.analogue_provenance("nativity", "nativity"), None);
}

#[test]
fn temporal_neighbors_of_equals_the_retired_map_over_every_dated_event() {
    let g = committed_graph();
    let s = service();
    let oracle = oracle_temporal_neighbors(g, &s.chronology.chrono.order);
    assert!(oracle.len() > 800);
    let mut dated = 0usize;
    for id in s.ids_of_kind(NodeKind::Event) {
        let got = s.temporal_neighbors_of(&id.raw);
        assert_eq!(got, oracle.get(&id.raw).cloned(), "{}", id.raw);
        dated += usize::from(got.is_some());
    }
    assert_eq!(dated, oracle.len(), "the domain is exactly chrono.order");
}
```

- [ ] **Step 2: RED** — `cargo test -p atlas-graph --test port_widening_real_data`. Expected: the four methods missing.

- [ ] **Step 3: Implement** — the four `GraphService` methods; trim `ProvenanceIndex`; delete `temporal_neighbors` and its `assemble` block; handlers: `:1428` → `graph.temporal_neighbors_of(&id).map(|(prior, following)| …)` (same body); `:1724` → `graph.analogue_provenance(&e.id, &other.id).filter(|p| !p.trim().is_empty()).ok_or_else(..)` (same fail-loud text); `:1762-1763` → `graph.attests_provenance(&e.id)` / `graph.event_mentions_provenance(&e.id)`. Update the three handler comments ("straight off the load-time companion index" → "through `GraphQuery::row_provenance`, one lookup per walked edge"). On `cross_refs_by_from`'s field doc add the DB-3 paragraph: it stays because its `target_display` is a row field the port does not expose; DB-4 retires it with the `kjv.cross_refs` seek (spec §4).

- [ ] **Step 4: GREEN** — `cargo test -p atlas-graph --test port_widening_real_data --test provenance_registry_real_data`; `cargo test -p atlas-server --test api --test graph_api --test contract_pact`. Expected: green; `provenance_registry_real_data` untouched and green (`families()` still 22).

- [ ] **Step 5: Commit**

```bash
git add server/atlas-graph/src/provenance.rs server/atlas-graph/src/service.rs server/atlas-server/src/handlers.rs server/atlas-graph/tests/port_widening_real_data.rs
git commit -m "refactor(graph, server): DB-3 -- per-edge provenance and temporal neighbours answered through the port (row_provenance, edges); ProvenanceIndex keeps only by_family; equivalence over the real artifact"
```

---

### Task 7: Measurements, docs, the standing block

**Files:**
- Modify: `server/BENCHMARKS.md` (append `## DB-3`), `server/atlas-cli/CONTRACT.md` (if it enumerates node kinds or relations by count — grep `LexiconEntry|occurs-in|CommentaryItem` and mirror the KRETZ-1 wording), `.superpowers/sdd/2026-08-17-bible-atlas-m1/progress.md`
- Test: the standing block, detached

- [ ] **Step 1: Measure** — gate 1 (`artifact_conformance`, load time with `edge_rows` + `spine_index`; before: 2.245 s of 4 s) and gate 9 (Task 3's number) via `bash scripts/timing-gates.sh` in the standing block; `PeakWorkingSet64` is not required by §12 for DB-3 (it is OVERLAY-1's and DB-4's), but state the `edge_rows` size arithmetic: entries × 16 bytes.

- [ ] **Step 2: BENCHMARKS.md** — append:

```markdown
## DB-3 (date): the port widened, companions retired

| Measure | Before | After |
|---|---:|---:|
| Gate 1 artifact load (ceiling 4 s) | 2.245 s | <measured> |
| Gate 9 SQLite admission (ceiling 570 s) | 271.5 s | <measured> (harness now covers the five new methods) |
| `Graph::edge_rows` | -- | <N> entries x 16 B = <MB> |

Retired: era_ids, polity_ids, narrative_ids, event_ids, place_ids, person_ids, bible_position,
concord_position, persons_by_verse, temporal_neighbors, ProvenanceIndex::{attests_for_event,
event_mentions_for_event, analogue_for_pair}. Kept with reasons (plan judgment calls 4-5):
cross_refs_by_from (row field `target_display`), ProvenanceIndex::by_family (per-family DISTINCT).
Not shipped (judgment call 1): compose_frontier -- no `frontier::compose` exists to default to.
```

- [ ] **Step 3: Standing block, detached** — the DB-2b wrapper pattern (`.superpowers/sdd/2026-08-17-bible-atlas-m1/logs-db2b-standing/run.sh`, copied to `logs-db3-standing/`). Expected: workspace all green; graph-types green OFF and ON; `TIMING GATES: 9/9 passed`.

- [ ] **Step 4: Commit and hand off to finishing-a-development-branch**

```bash
git add server/BENCHMARKS.md server/atlas-cli/CONTRACT.md
git commit -m "docs: DB-3 -- load and gate-9 measurements; the retirement ledger"
```

---

## Self-review (done while writing)

**Spec coverage.** §4 methods: `nodes_of_kind`, `nodes`, `edges_with_nodes`, `row_provenance`, `position_of` → Task 2 (types, trait, `Graph`, `MemSnapshot`, harness), Task 3 (SQLite). `compose_frontier` → judgment call 1 (excluded; `frontier::compose` does not exist). §4 vocabulary (`NodeKind::LexiconEntry`, `NodePayload::LexiconEntry`, `Occurs`) → Task 1; §7.2 field set verbatim; §7.3's ROW struct → LEX-1 (Global Constraints). §3.4/§3.6 `ContentHash` → Global Constraints (already gated in DB-2a; stays OFF; map-generator survey). §4 retirements: six id lists → Task 4; `bible_position`, `concord_position`, `persons_by_verse` → Task 5; PROV-1 per-edge → Task 6; `temporal_neighbors` → Task 6 (judgment call 7); `cross_refs_by_from` → kept, judgment call 4. §6.2 "every widened method equals its default composition over the full position inventory" → Task 2's harness extension (run by gate 9 and the compile admission); "the six port methods against the companions they retire, over the real artifact" → `port_widening_real_data.rs` (Tasks 4–6). §8 row 4 "coordinated with map-generator" → Task 1 Step 8 (compile proof; the survey found no code coupling to the touched types). §9 "graph-types one MINOR" → Task 1 (`0.2.0`); contract bump derived by the gate → Task 1 Step 7.

**Placeholder scan.** Task 2's two "read with_edges and pin it" asserts and Task 4's "READ find.rs after the loop" are explicit instructions naming the file and the decision rule, not TBDs. Task 7's `<measured>` cells are filled by Step 1's numbers. No "similar to Task N".

**Type consistency.** `NodePage { ids, next }`, `RowRef { family, row_id: u64, provenance }`, `EdgeEntryWithNode { entry, node }`, `EdgePageWithNodes { kind, entries, next }` are spelled identically in Tasks 2, 3 and the harness. `ids_of_kind(kind) -> Vec<AnyNodeId>` (Task 4) is what Tasks 5–6's tests call. `position_of(book, chapter, verse)` keeps its signature (Task 5) so `window.rs`, `perf_smoke.rs`, `graph_handlers.rs` and `chapter_span` compile unchanged. `EdgeRow { hash, family, row_ord: u32 }` vs `RowRef.row_id: u64`: the conversion is in `Graph::row_provenance` (Task 2). The SQLite `sqlite::rows::RowRef` (DB-2b's borrowed-row enum) and the port's `store::RowRef` are different types in different modules; snapshot.rs imports the port's as `atlas_graph_types::store::RowRef` and never `use`s the rows one.

**Judgment calls the reviewer should hold me to:** 1–9 above.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-09-17-db3-port-widening.md`. Two execution options:

1. **Subagent-Driven (recommended)** — a fresh subagent per task, task review between tasks, one final whole-branch review.
2. **Inline Execution** — executing-plans in this session, batch execution with checkpoints.
