# LEX-1 — The Lexicon Section Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The fifth section, `lexicon`: 13,548 `LexiconEntry` nodes (Strong's 1890 + STEPBible TBESG glosses + MACULA domains), 431,280 `Occurs` rows (every aligned Greek/Hebrew token, one row per word locus), the 452,689-row `token` inventory, the `lexicon_entry`/`lexicon_domain` projections — written by the compile, read by nothing new (the port and the loader are unchanged), with the four existing sections' logical hashes byte-identical (the acceptance test, spec §8 row 7).

**Architecture:** One new row family in graph-types (`Occurs`, spec §7.3), one new ETL reader (`atlas_etl::lexicon`: the Strong's-keyed JSON entries and the CoNLL-U morphology, upstream book codes resolved through the brain-fuel `books.json` map CORP-1a already built), one new adapter (`lexicon_adapter::normalize`: nodes + rows in canonical reading order, tokens whose Strong's number has no entry counted and skipped so every authored edge resolves), and the section plumbing DB-2b..DB-5 built for exactly this: `row_tables_of(Lexicon) = [Occurs]`, `extra_tables_of(Lexicon) = [lexicon_entry, lexicon_domain, token]`, `Section::SHIPPED` grows to five. The root moves (a fifth manifest line) — one re-record; the other four sections' hashes are pinned before the batch and asserted after.

**Tech Stack:** Rust 1.97.1; graph-types 0.2.1 → **0.3.0** (MINOR: a closed enum grows); no new dependencies.

**Spec:** `docs/superpowers/specs/2026-09-14-relational-artifact-design.md` §5.7 (the section), §7 (the model: 7.1 sources/licences, 7.2 the node kind, 7.3 words are loci, 7.4 deferred), §8 row 7 (acceptance), §2.1 (placement: `LexiconEntry` → Lexicon). Owner's standing "no apocrypha for now" (`morph/lxx`, the `lemma-*.json` LXX-only entries are not read).

## Global Constraints

- **Acceptance (spec §8 row 7):** "lands with NO change to the loader, the port, or any other section's hash". `core`/`kjv`/`concord`/`kretzmann` logical hashes after = `a34bb9212f730dec5470d340a2244ff4` / `abec1ca14dacb605148520b8d59cb4ec` / `fd4aca72b284947585df39619d67261b` / `cfebcd669fe2737850598fbc8b57ff24` (the values in `data/compiled/manifest.toml` at `ae8d3da`); a test pins them. The port trait, `SqliteSnapshot::open`, `CommittedZstdSource`, `from_sections` do not change. The root moves (a fifth line): AQC 0.4.0 → 0.5.0, AGC 0.7.0 → 0.8.0 (the TextUnit focus fixtures' `edge_summary` gains `words`; the root moves).
- Data: `data/raw/brain-fuel-bible/{lexicon/{grc,hbo}, morph/{nt,ot}}` at the pinned commit `94d44842cb242e8aa840330748e03d2803f2a7c1` (= upstream HEAD, 2026-07-31), vendored by `data/fetch-raw.ps1`'s LEX-1 block; `morph/lxx` and `lexicon/*/lemma-*.json` are never read. Only `[GH]NNNN.json` entries: 5,122 + 8,426 = 13,548.
- Licences (spec §7.1): Strong's 1890 PD; STEPBible TAGNT/TAHOT/TBESG CC BY 4.0 ("Credit STEPBible and link to https://github.com/STEPBible"); MACULA (Clear Bible) CC BY 4.0. `LICENSES.md` per-source rows, `data/raw/README.md` section, `data/curated/sources.toml` entries (→ `sources.json` via `gen_sources`), and provenance ids `strongs-1890`, `stepbible-tbesg`, `stepbible-tagnt`, `stepbible-tahot`, `macula-domains` registered — `provenance_registry_real_data.rs` reconciles every provenance id the graph carries against `sources.json` and fails otherwise.
- KJV inerrancy directive: the KJV column is never edited; the lexicon's "Yahweh" normalisation is upstream's on its own PD text; the seven "Jehovah" verses are untouched (spec §7.2).
- Ceilings: gate 9 (960 s) and gate 2 (60 s) are measured after; if the fifth section pushes gate 9 past 960 s the ceiling is re-derived by the DB-2b rule (measured × 2, next 30 s) and disclosed — the other ceilings are not loosened.
- Never `--release` while 8080 runs; never touch 8080; `RUST_TEST_THREADS=4`; gcc on PATH; no `git stash`; untracked frontier files never staged (`git add` by explicit path); map-generator never edited; graph-types zero-dep.
- Commit trailers: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` / `Claude-Session: https://claude.ai/code/session_01GsqLmd5gxtMWYDSeVSn43W`.
- Recorders as in the DB-5 plan (pacts, AGC bless/results, `export_aqc_examples`, five AQC copies + client cases, contract gate on a committed tree).

## Judgment calls

1. **`Occurs` edges are verse ↔ entry; the token index lives in the row.** Positions are nodes or edges (spec §5.0); a word is a locus with a span, not a node (§7.3). The edge `entry --occurs-in--> verse` (inverse `words`) is minted per row; two tokens of one entry in one verse are two rows minting one edge id (the leper lesson: `rows_behind` lists both), and `row_provenance` → the row → `locus.span` gives the token. `EdgeMeta` is not widened (a frozen wire shape).
2. **Rows in reading order.** `Occurs` rows are emitted book/chapter/verse/token order, so the inverse frontier at a verse lists its tagged words in token order and the forward frontier at an entry is a concordance in canonical order — "by construction and nothing else" (§7.3).
3. **Unmatched and orphan tokens.** `Align=unmatched` tokens (4.70 % / 4.74 %) get a `token` row and no edge. A matched token whose Strong's number has no lexicon entry (an upstream gap, if any) also gets no edge; the adapter counts both and prints them; a law asserts every `Occurs` entry exists as a node.
4. **The `token` table is an extra table fed from the corpus.** It is not graph-derived and not a sidecar of `AtlasData`; `Extras::compute` gains a `tokens: &[TokenRow]` parameter (empty on every path but the compile's).
5. **graph-types 0.3.0.** Adding a variant to `RowFamily` (a closed enum with `ALL`) breaks any downstream exhaustive match → MINOR under 0.x; the sibling repo's `atlas-edge` suite does not touch row families (DB-3's coordination note), so no cross-repo step.

## File structure

- `graph-types/src/edge.rs` (`Occurs` row struct), `canon/rows.rs` (`RowFamily::Occurs`, name/relation/`impl Canon`), `graph.rs` (`occurs` field, `row_edges` lowering, `canon_real_data` destructure), `sections.rs` (`row_tables_of`/`extra_tables_of`/`section_of_family`/`SHIPPED`), `Cargo.toml` 0.3.0, laws.
- `server/atlas-etl/src/lexicon.rs` (CREATE: `LexiconCorpus`, `read_all`), `src/lib.rs`, `brainfuel.rs` (`book_code_map` made `pub`), `tests/lexicon_real_data.rs` (CREATE).
- `server/atlas-graph/src/lexicon_adapter.rs` (CREATE), `pipeline.rs` (`BuildCtx.lexicon`, the normalize call), `build.rs` (`..._and_lexicon` constructor), `bins/compile_graph.rs`, `sqlite/{ddl.rs, rows/lexicon.rs (CREATE), rows/mod.rs, partition.rs, reload.rs, extras.rs, serve.rs (nothing), writer.rs (nothing)}`, `provenance.rs` (sweep + `family::OCCURS`), tests.
- `data/curated/sources.toml`, `data/compiled/sources.json` (regenerated), `LICENSES.md`, `data/raw/README.md`, `data/fetch-raw.ps1` (already committed in DB-5's tree), contracts/client bumps, `server/atlas-cli/tests/cli.rs` (verify: 5 sections), BENCHMARKS, spec §7 "as shipped" notes, ledger, memory.

---

### Task 1: `Occurs` in graph-types (0.3.0)

**Files:** `graph-types/src/edge.rs`, `canon/rows.rs`, `graph.rs`, `sections.rs`, `Cargo.toml`; tests in `sections.rs`/`rows.rs` law modules, `graph-types/tests/*` if they pin `RowFamily::ALL.len()` (grep `21`), `server/atlas-graph/tests/canon_real_data.rs` (the exhaustive `Graph {..}` destructure: `occurs`), `provenance_registry_real_data.rs` (`("edge.rs", 19)` → 20).

**Interfaces (produces):**

```rust
// edge.rs
#[derive(Clone, Debug)]
pub struct Occurs { pub entry: LexiconEntryId, pub locus: TextLocus /* span = Some(one token) */, pub provenance: ProvenanceId }
// canon/rows.rs: RowFamily::Occurs appended LAST (ordinal 21); name() = "occurs"; relation() = Directed(RelationId::Occurs);
// impl Canon for Occurs — keys ["entry", "locus", "provenance"] (locus through the existing TextLocus codec)
// graph.rs: pub occurs: Vec<Occurs>  (imported group); row_edges: push_edge(out, RowFamily::Occurs, i, Directed(R::Occurs), (at(&row.entry.erase()), at(&text_node(&row.locus)), M::None)) — appended after `analogue`'s loop (index order is positional)
// sections.rs: row_tables_of(Lexicon) = &[RowFamily::Occurs]; section_of_family(Occurs) = Lexicon; extra_tables_of(Lexicon) = &["lexicon_entry", "lexicon_domain", "token"]; SHIPPED: [Core, Kjv, Concord, Kretzmann, Lexicon]; Section::required(Lexicon) = false
```

- [ ] **Step 1: Failing laws.** In `sections.rs` laws: `row_tables_of(Section::Lexicon) == [RowFamily::Occurs]`, `SHIPPED.len() == 5`, `extra_tables_of(Lexicon)` as above, the closedness law ("every family has one home") still passes; `the_root_is_the_manifest_root…` expects 5 lines. In `rows.rs` laws: an `Occurs` round-trips through `encode_row_in_family`/decode with a span locus. In `graph.rs`'s law module: a graph with one `Occurs` row lowers to one `Occurs` edge whose subject is `n:LexiconEntry:G3056` and object the verse's `TextUnit` node.
- [ ] **Step 2: Run** `cd graph-types && cargo test --features canon-ids` → FAIL.
- [ ] **Step 3: Implement** as the interface says. `RowFamily::ALL` becomes 22; `ordinal()` derives from `ALL`; `encode_row_in_family`/`decode_row_in_family` gain the arm; `RowFamily::relation()` gains `Occurs => Directed(R::Occurs)`. `Cargo.toml`: `version = "0.3.0" # LEX-1: RowFamily::Occurs (a closed enum grew: MINOR), Section::SHIPPED gains Lexicon.`
- [ ] **Step 4: Run** both feature states green. `cd server && cargo build --workspace --tests` will now FAIL on non-exhaustive matches in atlas-graph (`rows/mod.rs`, `partition.rs`, `ddl.rs`, `logical.rs`'s fallback is by name, `reload.rs`, `provenance.rs`) — that is Task 4's work; commit graph-types alone.
- [ ] **Step 5: Commit** — `feat(graph-types): LEX-1 -- the Occurs row family (entry -> one-token word locus), the lexicon section shipped (0.3.0)`.

---

### Task 2: The ETL reader — lexicon entries and CoNLL-U morphology

**Files:** Create `server/atlas-etl/src/lexicon.rs`; modify `src/lib.rs` (`pub mod lexicon;`), `src/brainfuel.rs` (`pub fn book_code_map`); create `server/atlas-etl/tests/lexicon_real_data.rs`.

**Interfaces (produces):**

```rust
pub struct LexEntry { pub strong: String, pub lang: String, pub lemma: String, pub translit: Option<String>, pub pos: Option<String>, pub glosses: Vec<String> /* en texts, source order */, pub senses: Vec<String> /* gloss_en per sense, id order */, pub domains: Vec<String> /* sorted, deduped */, pub root: Option<String> }
pub struct TokenRow { pub book: BookId, pub chapter: u16, pub verse: u16, pub layer: &'static str /* "greek_textus_receptus" | "hebrew_masoretic" */, pub ord: u16 /* the CoNLL-U token id, 1-based */, pub form: String, pub lemma: Option<String>, pub xpos: Option<String>, pub translit: Option<String>, pub strong: Option<String>, pub aligned: bool }
pub struct LexiconStats { pub entries_grc: usize, pub entries_hbo: usize, pub tokens_nt: usize, pub tokens_ot: usize, pub unmatched_nt: usize, pub unmatched_ot: usize, pub files: usize }
pub struct LexiconCorpus { pub entries: Vec<LexEntry> /* sorted by strong */, pub tokens: Vec<TokenRow> /* book, chapter, verse, ord order */, pub stats: LexiconStats }
pub fn read_all(root: &Path /* data/raw/brain-fuel-bible */) -> anyhow::Result<LexiconCorpus>
```

Empty strings in the JSON (`translit: ""`, `pos: ""`) are `None`; `glosses.en[].text`; `senses[].gloss_en`; `domains` sorted + deduped; `root` as given. CoNLL-U: lines starting `# ref = ` set the verse (`CODE.c.v`, CODE through `book_code_map(data/books.json)`), blank line ends a sentence, other lines are 10 tab-separated columns: `id, FORM, LEMMA, UPOS, XPOS, FEATS, HEAD, DEPREL, DEPS, MISC`; `_` = absent; MISC `k=v|k=v`: `Strong=`, `Translit=`, `Align=unmatched`. Layer by testament directory (`nt`/`ot`). Files read in sorted order (`<CODE>/NNN.conllu`), rows emitted in file order; tokens are sorted (book index, chapter, verse, ord) after reading so order does not depend on the directory listing.

- [ ] **Step 1: Failing real-data test** (`lexicon_real_data.rs`): `read_all` over the vendored dir → `stats == { entries_grc: 5122, entries_hbo: 8426, tokens_nt: 140610, tokens_ot: 312079, unmatched_nt: 6615, unmatched_ot: 14794, files: 1189 }` (spec §7.1's coverage table; the `files` count = 260 + 929); `entries` sorted by `strong` and unique; `G3056` is `lemma "λόγος"`, `translit Some("lógos")`, `pos Some("G:N-M")`, first gloss starts `"from G3004;"`, second gloss `"word"`, `domains` contains `"13.115"`; `H0430` lemma `"אֱלֹהִים"`; the first NT token is `1CO.1.1` ord 1? No — sorted: `MAT.1.1` ord 1 (book index 39); every aligned token's `strong` matches `^[GH][0-9]{4,5}$`; every token's `(book, chapter, verse)` is ≤ `canon::BOOKS`' chapter/verse counts (resolve through `atlas_core::canon`); no `lemma-*` entry (`strong` never `None`).
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** (serde structs for the JSON; a hand parser for CoNLL-U; `bail!` on a malformed line naming file:line). **Step 4:** green (note the wall time). **Step 5: Commit** — `feat(etl): LEX-1 -- the lexicon reader: 13,548 Strong's entries (TBESG glosses, MACULA domains) and 452,689 CoNLL-U tokens with their Strong's alignment, coverage pinned`.

---

### Task 3: The adapter — `LexiconEntry` nodes and `Occurs` rows in the build

**Files:** Create `server/atlas-graph/src/lexicon_adapter.rs`; modify `lib.rs`, `pipeline.rs` (`BuildCtx.lexicon: Option<&LexiconCorpus>`, `with_…_and_lexicon` constructor, `crate::lexicon_adapter::normalize(ctx)` after `red_letter_adapter::normalize`), `build.rs` (`build_graph_from_sources_…_and_red_letter_and_lexicon(…, lexicon: Option<&LexiconCorpus>)`; the older names delegate with `None`), `service.rs` (`from_sources_…_and_lexicon` if a fixture needs it — none does; skip), `bins/compile_graph.rs` (reads the corpus, passes it to both builds, prints the stats). Test: `server/atlas-graph/tests/lexicon_real_data.rs` (CREATE).

**Interfaces (produces):**

```rust
pub struct LexiconAdapterStats { pub entries: usize, pub occurs: usize, pub tokens_unmatched: usize, pub tokens_without_entry: usize, pub tokens_off_canon: usize }
pub fn normalize(ctx: &mut BuildCtx) -> LexiconAdapterStats   // no-op (all zeros) when ctx.lexicon is None
pub fn entry_node_id(strong: &str) -> AnyNodeId               // AnyNodeId { kind: LexiconEntry, raw: strong }
pub const PROVENANCE_ENTRY: &str = "stepbible-tbesg";           // the entry nodes (Strong's PD text + TBESG glosses + MACULA domains: the registry rows say what each id draws on)
pub const PROVENANCE_OCCURS_NT: &str = "stepbible-tagnt";
pub const PROVENANCE_OCCURS_OT: &str = "stepbible-tahot";
```

Nodes: `NodePayload::LexiconEntry { strong, lang, lemma, translit, pos, glosses, senses, domains, root }`, `provenance: PROVENANCE_ENTRY`. Rows: for each token in corpus order with `aligned && strong.is_some()` and an existing entry: `Occurs { entry: LexiconEntryId::new(strong), locus: TextLocus { at: TextRef::Bible(VerseRef { book: book.0, chapter, verse }), span: Some(TokenSpan::new(TranslationId(layer.into()), ord, ord)?) }, provenance: NT/OT }`, skipped (and counted) when the verse node is absent from `ctx.graph.nodes` (`kjv_adapter::verse_node_id(book, chapter, verse)`).

- [ ] **Step 1: Failing real-data test:** build the full graph from raw with the corpus (the `build_graph_…_and_lexicon` call as `exports_laws.rs` builds, plus `Some(&lexicon)`), assert `nodes_of_kind(LexiconEntry).len() == 13548`, `g.occurs.len() == 431280` (452,689 − 6,615 − 14,794 = 431,280, minus `tokens_without_entry`, which the test prints and asserts `== 0` — if upstream has such tokens the number lands here and the assertion becomes the printed count, disclosed), every `Occurs` row's entry and verse node exist, rows are in `(book, chapter, verse, ord)` order, and `law_check` (the compile's laws: `every_authored_edge_resolves`) passes over the graph. Also `atlas_graph::sections::section_of_node` = Lexicon for an entry node.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**; wire the pipeline; the compile binary: `let lexicon = atlas_etl::lexicon::read_all(&raw_dir.join("brain-fuel-bible"))?;` passed to both builds; prints `LEX-1: {} entries, {} occurs rows, {} unmatched tokens, {} without entry`. **Step 4:** green. **Step 5: Commit** — `feat(graph): LEX-1 -- lexicon_adapter: LexiconEntry nodes and Occurs rows (one per aligned token, reading order) in the build pipeline and the compile`.

---

### Task 4: The section — DDL, rows, partition, reload, extras, laws

**Files:** `server/atlas-graph/src/sqlite/{ddl.rs, rows/lexicon.rs (CREATE), rows/mod.rs, partition.rs, reload.rs, extras.rs}`, `provenance.rs`; tests `sqlite_laws.rs`, `extras_real_data.rs`/`serve_real_data.rs` (families 23), `provenance_registry_real_data.rs` (23 families, `("edge.rs", 20)`), `reload_real_data.rs` (5 present, node count + 13548), `sections_startup.rs` (unchanged: absent empty), `atlas-cli/tests/cli.rs` (`verify`: 5 lines, `count == 5`, the JSON `sections.len() == 5`).

**Interfaces (produces):** DDL per spec §5.7 verbatim (`occurs` as the family table with `COLS_OCCURS`, `lexicon_entry`, `lexicon_domain`, `token` as extra tables); `rows::lexicon::{insert_occurs, read_occurs}` (LOCUS(locus) 7 columns with `locus_layer/locus_start/locus_end NOT NULL`, `entry_id`, `provenance`); `RowRef::Occurs(&Occurs)`/`RowOwned::Occurs(Occurs)`; `partition::rows_of_section` arm; `reload::push_row` arm; `extras`: `pub static LEXICON_ENTRY: TableSpec { columns: ["node_id","strong","lang","lemma","translit","pos","root_strong"], pk: ["node_id"] }`, `LEXICON_DOMAIN { ["node_id","ord","code"], pk ["node_id","ord"] }`, `TOKEN { ["book","chapter","verse","layer","ord","form","lemma","xpos","translit","strong","aligned"], pk ["book","chapter","verse","layer","ord"] }`; `table_specs_of(Lexicon)`; `Extras::graph_derived` emits `lexicon_entry` + `lexicon_domain` from `LexiconEntry` payloads; `Extras::compute(…, tokens: &[atlas_etl::lexicon::TokenRow])` emits `token`; `provenance::family::OCCURS = "occurs"` + the sweep.

- [ ] **Step 1: Failing laws** (`sqlite_laws.rs`): the specimen gains two `LexiconEntry` nodes (`G3056`, `H0430`) and three `Occurs` rows (two on `bible/1.1.1` tokens 1 and 3 for `G3056`, one on `bible/1.1.2` for `H0430`); `every_family_round_trips…` covers `Occurs`; `every_extra_table_spec_matches_its_ddl…` covers the three new specs; the writer produces FIVE files (manifest order ends with `lexicon`, `required = false`); the snapshot answers `edges(n:LexiconEntry:G3056, occurs-in forward)` = two entries at one edge id? — no: two rows, one `(rel, subject, object)` → ONE edge entry, `rows_behind` = 2 (assert both); `edge_summary(verse)` has `words: 1`; `assert_answers_match` over the specimen; `an_absent_optional_section…` now also proves the lexicon absent → `LexiconEntry` nodes uninhabited and `edge_summary(verse)` lacks `words`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** all sites; `ddl.rs` `extra_ddl(Lexicon)`/`extra_index_ddl(Lexicon)` with `lexicon_by_strong`, `lexicon_by_lemma`, `domain_by_code`, `occurs_by_locus`, `occurs_ord`. **Step 4:** `cargo test -p atlas-graph --test sqlite_laws` green; then the real-data files compile again (the `RowFamily` matches) — `cargo build --workspace --tests`. **Step 5: Commit** — `feat(graph): LEX-1 -- the lexicon section written and read back: occurs rows, lexicon_entry/lexicon_domain projections, the token inventory; laws over the specimen`.

---

### Task 5: Attribution, the registry, the compile, and the invariance proof

**Files:** `data/curated/sources.toml` (+ categories/sources/provenances rows for STEPBible and MACULA and Strong's), `data/compiled/sources.json` (regenerated: `cargo run -p atlas-etl --bin gen_sources`), `LICENSES.md` (per-source rows: Strong's 1890 PD; STEPBible TAGNT/TAHOT/TBESG CC BY 4.0 with the credit line verbatim; MACULA CC BY 4.0), `data/raw/README.md` (`brain-fuel-bible/` gains the lexicon/morph paragraph: layout, counts, "no apocrypha"), `server/atlas-graph/tests/lexicon_section_real_data.rs` (CREATE), `data/compiled/{manifest.toml, sections/lexicon.<logical>.sqlite.zst}` (committed), `data/exports/*.json` (root).

- [ ] **Step 1: The invariance test** (`lexicon_section_real_data.rs`): `read_manifest(data/compiled/manifest.toml)`: five sections in `MANIFEST_ORDER`; the first four's `logical` equal the four pinned hashes above (the acceptance test); `lexicon.required == false`; `bytes < 104_857_600`; open through `from_sections`: `nodes_of_kind(LexiconEntry)` = 13,548; `edge_summary(JHN.3.16)` has `words` = the verse's aligned token count (assert `> 10`); the `words` page at JHN 3:16 lists entries in token order (assert the first is `G3779` "οὕτως"? — verify against `morph/nt/JOH/003.conllu` line for verse 16 and pin the first three Strong's numbers found there); `edges(G3056, occurs-in)` first entry is `MAT.1.?`… pin the first verse that carries `G3056` per the CoNLL-U (find it in the test by scanning the corpus; assert the page's first object equals it); `rows_behind` for a verse with a repeated word returns > 1 row.
- [ ] **Step 2:** registry + licences + README; `gen_sources`; `cargo test -p atlas-etl --test sources_validate` green.
- [ ] **Step 3: Compile** (`cargo run -p atlas-graph --bin atlas-graph-compile -- --data-dir ../data/compiled`): expect four blobs REUSED, `lexicon.<logical>.sqlite.zst` new, root moved; record write time and the blob's size. Run the invariance test → green. `bibex verify` → 5 lines OK.
- [ ] **Step 4: Commit** code + data (`git add data/compiled/manifest.toml data/compiled/sections data/compiled/sources.json data/exports data/curated/sources.toml LICENSES.md data/raw/README.md` + the test) — `feat(graph, data): LEX-1 -- the lexicon section committed (13,548 entries, 431,280 occurs rows, 452,689 tokens); the other four sections' hashes byte-identical (spec 8 row 7); STEPBible/MACULA/Strong's attributed`.

---

### Task 6: Pins, bumps, gates, record

- [ ] **Step 1: Recorders** (order as always). Expect: HTTP pact roots + the TextUnit bodies' `edge_summary` (`words`); CLI pact — any `verse`/`node` transcript on a verse now shows `words` (re-record, disclose); AGC `contract` fixture range; AQC 19 roots + the TextUnit focus fixtures (`focus-textunit.json`, `traversal-*` if they page a verse) gain `words` counts. AQC 0.5.0 in its copies (+ client cases: accept ("0.5.0","0.5.0"), ("0.4.9","0.6.0"); reject ("0.6.0","0.8.0"), ("0.0.1","0.4.9")), AGC 0.8.0; CHANGELOG lines name the `words` addition. `bash scripts/contract-gate.sh` → PASSED.
- [ ] **Step 2: Gates.** Standing block (`logs-lex1-standing/run.sh`): workspace green, graph-types OFF/ON, gates 10/10 — read gate 2 and gate 9's measurements; re-derive gate 9's ceiling only if exceeded (disclosed).
- [ ] **Step 3: Record.** BENCHMARKS `## LEX-1`: reader wall time, adapter counts, the lexicon blob's size, compile time, gate 2/9 deltas, `bibex verify` time, startup delta (gate 10). Spec §7 "as shipped" notes (row counts; the entry provenance id; `senses`/`domains` shapes). Ledger, memory (NEXT: the owner directives D1–D5, or L2b/thesaurus per §7.4 if the owner wants). Push.

---

## Self-review

**Spec coverage.** §7.1 sources/licences → Task 5 (rows, README, registry; `fetch-raw.ps1` block landed with DB-5); §7.2 the node kind → Task 3 (payload field by field; provenance registered); §7.3 words as loci, `Occurs` rows, the `token` inventory, frontier consequences → Tasks 1, 3, 4, 5 (the three assertions in the invariance test); §7.4 deferred (relations, LXX, thesaurus) → not read, said so in Global Constraints; §5.7 DDL → Task 4 verbatim; §2.1 placement → Task 1 (`section_of_node` already maps `LexiconEntry` → Lexicon; `section_of_family(Occurs)`); §8 row 7 acceptance → Task 5 Step 1 (the four pinned hashes; the loader and the port untouched: no file under `sqlite/{snapshot,source,serve}.rs` or `service.rs`'s `from_sections` changes).

**Placeholder scan.** Task 5 Step 1's "pin the first three Strong's numbers found there" is an instruction to derive the value from the vendored file inside the test (scan the CoNLL-U) — not a placeholder. No TBDs.

**Type consistency.** `atlas_etl::lexicon::{LexiconCorpus, TokenRow, LexEntry}` (Task 2) are what Task 3's `BuildCtx.lexicon` and Task 4's `Extras::compute(…, tokens)` consume; `Occurs { entry, locus, provenance }` (Task 1) is what Task 3 emits and Task 4 encodes; `RowFamily::Occurs` (Task 1) is the arm Task 4 adds everywhere.
