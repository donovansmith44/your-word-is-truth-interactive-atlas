# CONTRACT-1 close report

Batch = CONTRACT-1a (server) + CONTRACT-1b (client) + the batch-close mutation
gate (Rust, sharded, and C#/Stryker). Compiled from
`.superpowers/sdd/2026-09-26-contract1a-server/progress.md`,
`.superpowers/sdd/2026-09-26-contract1b-client/progress.md`, the two mutation
reports, and 1b's final review / final fix report. HEAD at time of writing:
`711ba8e` (as directed). Every number below is sourced from one of those
files or from this brief; none is estimated.

---

## 1. Standing block

**Measured at the close, 2026-09-29 (HEAD `711ba8e` when the run began; `dd72809`, a docs-only commit, landed during it):**

| leg | result |
|---|---|
| `bash ../scripts/timing-gates.sh run` | **TIMING GATES: 10/10 passed** (serialized, one process each), exit 0 |
| `bash ../scripts/timing-gates.sh check` | **"timing-gates check: 10 gates reconcile with the tree"**, exit 0 |
| gate 8 (DB-4b, `atlas-graph::sqlite_real_data::the_full_real_graph_is_admitted…`) | **741.5 s** of a 1560 s ceiling (write 190.5 s incl. zstd-19, dump-recompute 8.0 s, assert_answers_match 365.9 s) — **under the 779.6 s baseline** by 38 s |
| gate 1 (M-B/M-C full-scale conformance: 31,102 text units, 343,558 cites edges, 1,711 events / 912 dated, 13 narratives, 21 anchors, 33,355 attests, 955 located-at, 912 dated-by) | assert_answers_match **11.6 s** of 60 |
| startup (DB-4c, `the_served_path_starts_under_the_ceiling`) | **839.6 ms** of 4 s (from_sections 135.2 ms, finish 4.3 ms, scene priming 700.0 ms); 21 sources, 1,711 events |
| gates 2–7 (perf smoke) | all under their gates by ≥ 2×: scene time full span 24.8 ms / 75; NT window 12.4 / 75; scripture chapter 27.0 / 50; xrefs 5.5 / 30; text window 2.1 / 30; chapter window 3.6 / 50 |
| counting procedure (`cargo test --workspace` + `graph-types --all-features`) | **PENDING — deferred by the owner's request.** The run was started and then stopped when the owner asked for the app ("start my app … i want it running now"); `cargo test --workspace` relinks `atlas-server.exe`, which a running app holds (R-D-7), so it re-runs the moment :8000 is free. Evidence that stands in for it meanwhile, all green and all later than 1a's 1262-count: the mutation gate's verification re-run (584 tested / 0 missed, every crate's suite green at `fe79a31`), `cargo test -p atlas-etl` and `-p atlas-graph` whole suites (2026-09-28 22:xx, 0 failures), client.Tests 472 + client.ContractTests 54 from a cold clone at `13111dd`. |
| contract gate / semver gate at `dd72809` | **stand at their `e36824b` verdicts** (PASSED; AQC 0.9.0 "declared minor = major under the 0.x rule"; AGC 0.13.0). Proof they carry: `git diff --stat e36824b dd72809 -- server/atlas-contract/src contracts/openapi.yaml contracts/atlas-query-contract/{aqc.schema.json,VERSION} contracts/pacts contracts/atlas-graph-contract` shows ONE file, `graph_wire.rs` +1 — a vector added inside its `#[cfg(test)] mod tests` by the mutation gate (`LexiconEntry:H430`). Every published document, fixture and pact is byte-identical to `e36824b`. The gates re-run for FOCUS-0 at `--base dd72809` once its Task 1 regenerates the documents. |

**1a's close, for comparison** (34fb6e5, 2026-09-28 05:44–06:33):
canonical 1262 = 1133 (workspace) + 129 (graph-types) across 83 sections; AQC
50 scenarios / 201 steps; TIMING GATES 10/10 + "10 gates reconcile" (gate 8
784.9 s of a 1560 s ceiling, at its 779.6 s baseline; gate 1 11.7 s of 60;
startup 1.12 s of 4); contract gate PASSED; semver gate exit 0 vs `cbe6a5a`.

---

## 2. Gates

| gate | verdict | last green at (commit) |
|---|---|---|
| Contract gate | PASSED (9 legs incl. leg 3 HTTP, leg 6 vocab reconciliation) | `e36824b` (1b Task 6); also PASSED at every 1a task close through `34fb6e5` |
| Semver gate — AQC | 0.8.0 → 0.9.0. Gate's own verdict, quoted: *"diff requires major, declared minor = major under the 0.x rule"* | `e36824b` |
| Semver gate — AGC | 0.12.0 → 0.13.0 ("its contract fixture follows the advertised version, re-blessed as 1a's `e0f44e9` did") | `e36824b` |
| `export_contract --check` | exit 0 | `e36824b` (1b Task 6); also `34fb6e5` (1a fix wave) |
| `cargo build --workspace --all-targets` | 0 warnings | `e36824b`; standardised as the standing block's build leg by R42 at `34fb6e5` (benches + golden.rs had silently stopped compiling under `-p`/`--lib`-only runs before that) |

Semver base for the whole batch is `cbe6a5a` (R34), passed positionally —
`contract-semver-gate.sh <ref> --runner <path>` (R43 corrects every brief
that says `--base`).

---

## 3. Mutation — once for the batch

### Rust (`task-13-report.md`, base `78f51ff`)

**1,376 mutants total: 551 caught, 784 unviable, 41 missed → all 41 killed on
re-run. 0 equivalents recorded, 0 `#[mutants::skip]`. Verification re-run: 584
tested, 0 missed, 0 timeout.**

Per crate:

| crate | tested | caught | missed | timeout | unviable |
|---|---|---|---|---|---|
| `atlas-cli` | 65 | 60 | 0 | 0 | 5 |
| `atlas-contract` | 392 | 142 | 6 | 0 | 244 |
| `atlas-core` | 485 | 53 | 10 | 0 | 422 |
| `atlas-etl` | 169 | 141 | 6 | 0 | 22 |
| `atlas-graph` | 128 | 89 | 16 | 0 | 23 |
| `atlas-server` | 1 | 0 | 1 | 0 | 0 |
| `graph-types` (union, both `canon-ids` states) | 136 | 66 | 2 | 0 | 68 |
| **TOTAL** | **1,376** | **551** | **41** | **0** | **784** |

Per file that carried a survivor, before vs. after the fix wave:

| file | before (tested/caught/missed/unviable) | after (tested/caught/missed/unviable) |
|---|---|---|
| `atlas-contract/src/aqc_export.rs` | 8/6/2/0 | 8/8/0/0 |
| `atlas-contract/src/bins/export_aqc_examples.rs` | 1/0/1/0 | 1/1/0/0 |
| `atlas-contract/src/bins/export_contract.rs` | 9/8/1/0 | 9/8/0/1 |
| `atlas-contract/src/graph_wire.rs` | 21/18/1/2 | 21/19/0/2 |
| `atlas-contract/src/load.rs` | 5/1/1/3 | 5/2/0/3 |
| `atlas-core/src/chronology.rs` | 12/6/4/2 | 12/10/0/2 |
| `atlas-core/src/data.rs` | 414/9/4/401 | 414/13/0/401 |
| `atlas-core/src/event_merge.rs` | 13/12/1/0 | 13/13/0/0 |
| `atlas-core/src/scene.rs` | 11/4/1/6 | 11/5/0/6 |
| `atlas-etl/src/bins/gen_sources.rs` | 1/0/1/0 | 1/1/0/0 |
| `atlas-etl/src/compile.rs` | 3/0/2/1 | 10/9/0/1 |
| `atlas-etl/src/main.rs` | 1/0/1/0 | 1/1/0/0 |
| `atlas-etl/src/validate.rs` | 17/15/2/0 | 17/17/0/0 |
| `atlas-graph/src/bins/compile_graph.rs` | 1/0/1/0 | 1/1/0/0 |
| `atlas-graph/src/description_adapter.rs` | 8/7/1/0 | 8/8/0/0 |
| `atlas-graph/src/heading.rs` | 15/11/2/2 | 20/18/0/2 |
| `atlas-graph/src/law_check.rs` | 5/4/1/0 | 5/5/0/0 |
| `atlas-graph/src/lib.rs` | 2/0/2/0 | 2/2/0/0 |
| `atlas-graph/src/pipeline.rs` | 4/3/1/0 | 4/4/0/0 |
| `atlas-graph/src/sqlite/serve.rs` | 11/1/8/2 | 11/9/0/2 |
| `atlas-server/src/main.rs` | 1/0/1/0 | 1/1/0/0 |
| `graph-types/src/present.rs` | 1/0/1/0 | 1/1/0/0 |
| `graph-types/src/wire_form.rs` | 8/0/1/7 | 8/1/0/7 |
| **TOTAL** | 626/... | **584/157/0/427** |

`compile.rs` and `heading.rs` grew tested-counts (3→10, 15→20) because two
fixes extracted named functions out of inline blocks no test could reach
before; the new functions' own mutants are scored in the "after" column too.

**160 h → 4.1 h.** Scoping (`additional_cargo_test_args` narrowed per crate
to covering targets): 419 s/mutant → 40 s/mutant (10.5×). Two fixtures were
building a whole `GraphService` per test: `graph_api` 238.7 s → 54.0 s,
`brainfuel_layers` 72.0 s → 54.9 s (3.2 min off every full
`cargo test -p atlas-contract`, not only the gate). Concurrency: N=4 shard
worktrees (not `--jobs`, which cannot resolve `graph-types` as a sibling path
dependency outside the copied workspace root) — cores support 8 (1.85×
contention), but memory is the ceiling on this box: at N=8 free RAM fell to
2.4 GB during `atlas-graph` and the harness's low-memory reaper killed the
run; N=6 fell to 0.8 GB; N=4 completed (R46).

### C# — Stryker (`task-8-stryker-report.md`, base `642ed58`)

**Combined: 100.00% — 146/146 tested mutants killed, 0 survived, 0
no-coverage, 0 timeout, 0 equivalents recorded.**

Two configs, because Stryker mutates one project per run, and the eleven
scoped files span two compiled projects:

| config | project | tested | killed | wall-clock |
|---|---|---|---|---|
| `stryker-config.json` | `BibleAtlas.Client.csproj` | 124 | 124 | 34.8 s |
| `stryker-config.contractgenerator.json` | `BibleAtlas.Client.ContractGenerator.csproj` | 22 | 22 | 20.5 s |

Per file, `stryker-config.json`:

| file | tested | killed |
|---|---|---|
| `AtlasClient.cs` | 60 | 60 |
| `Components/ContentsTreeModel.cs` | 39 | 39 |
| `Contract/NodeIds.cs` | 10 | 10 |
| `GraphExplorableClient.cs` | 5 | 5 |
| `Contract/EdgeKinds.cs` | 6 | 6 |
| `Contract/WireNames.cs` | 2 | 2 |
| `Explore/EdgeSectionRegistry.cs` | 1 | 1 |
| `RequiredResponses.cs` | 1 | 1 |
| `IExplorableClient.cs` | 0 | — (pure interface, vacuously 100%) |

Per file, `stryker-config.contractgenerator.json`:

| file | tested | killed |
|---|---|---|
| `ContractGeneration.cs` | 14 | 14 |
| `PascalCasePropertyNames.cs` | 8 | 8 |

`AtlasClient` had **zero unit tests before this task** — a pre-1b house rule
routed its coverage through Playwright, invisible to Stryker. 37 new tests
in `AtlasClientTests.cs` closed 60 `NoCoverage` mutants alone. **46 new tests
total; client.Tests 426 → 472; client.ContractTests 54 unchanged.**

---

## 4. Playwright

**Full suite at HEAD (`934040f`): 451 passed / 3 failed / 2 skipped (7.7
min), quiet machine.**

Adjudication (2026-09-28): a throwaway worktree at `34fb6e5` built **both**
the pre-1b client (:5000) and the pre-1b API (:8000), a matched 0.8.0/0.8.0
pair, so the same unchanged spec ran against a true baseline (no
`git stash`).

- **First attempt was invalid, and said so:** the pre-1b client still carried
  the startup contract-mismatch check, and the current API advertised 0.9.0
  after Task 6, so the old client rendered the mismatch page and 0 markers —
  not a real comparison. Rerun with the matched 0.8.0 pair was the valid one.
- `world-quiet-places.spec.ts:211` — **3/3 FAIL at base**, identical error.
  Pre-existing, deterministic.
- `split-view.spec.ts:293` — **6/8 FAIL at base** vs 1/3 at HEAD — *more*
  flaky before 1b. Pre-existing flake.
- `reader-xref-anchoring.spec.ts:201` — 1/3 fail at base, ~same at HEAD.
  Pre-existing flake.

Conclusion: **no 1b regression in the UI suite.** Spec files,
`playwright.config.ts`, and `client/wwwroot/js/map.js` are byte-unchanged in
1b; `merged_ids` (the one new field in the scene JSON) appears 0 times in
`map.js`. All three failing specs go to the owner queue, not to this batch.

---

## 5. Counts

| item | count |
|---|---|
| AQC scenarios / steps (0.9.0) | 47 / 195 |
| `client.Tests` | 472 |
| `client.ContractTests` | 54 |
| `openapi.yaml` (generated) | 2,848 lines |
| `aqc.schema.json` (generated) | 2,142 lines |
| `Wire.g.cs` (generated, untracked build output) | 3,058 lines |
| `atlas-contract/src` (hand-written) | 3,812 lines |
| `client.ContractGenerator` (hand-written) | 95 lines |

---

## 6. Net hand-written LOC, per batch

Controller-measured, rename-aware where stated:

| batch | range | files | + | − | net |
|---|---|---|---|---|---|
| CONTRACT-1a | `78f51ff..34fb6e5` | 296 | 9,839 | 28,000 | **−18,161** |
| CONTRACT-1b (incl. fix waves) | `34fb6e5..934040f` + fix waves | 129 | 2,201 | 3,291 | **−1,090** |
| **CONTRACT-1 total** | `78f51ff..d2ed170`, rename-aware | 401 | 12,510 | 31,178 | **−18,668** |
| generated contract artifacts (separate) | same | 16 | 12,494 | 704 | +11,790 |

**The honest split** (why the batch reads as −18,668):

| language / kind | + | − | net |
|---|---|---|---|
| Rust comments | 3,341 | 23,741 | **−20,400** |
| Rust CODE | 5,239 | 3,408 | **+1,831** |
| C# comments | 105 | 1,591 | **−1,486** |
| C# CODE | 1,389 | 1,217 | **+172** |

The −18,668 net is the owner's comment purge (R21/R22/R28: server workspace
23,885 → 3,155 comment lines; the 1b final fix wave stripped 1,308 more test
comments across 17 files down to Arrange/Act/Assert only). Real code written
across the batch is **≈ +6,600 / −4,600**, not the headline number.

1b by area (part of the −1,090 net):

| area | net |
|---|---|
| `client.Tests` | −928 |
| `client` | −285 |
| `client.ContractTests` | −170 |
| `tests/ux` | −94 |
| `contracts` | −71 |
| `scripts` | +197 |
| `server` | +131 |
| `client.ContractGenerator` | +111 |

---

## 7. Every ruling, with its cost if wrong

Where the ledger recorded no cost, the cell reads "not stated" — none is
invented.

### R1–R51 (1a ledger, R44–R49 from its Task 13 section; R50–R51 from the 1b ledger)

| id | ruling | cost if wrong |
|---|---|---|
| R1 | Task 3 also retargets atlas-cli (`atlas-server` path dep → `atlas-contract`; `use atlas_server::` → `atlas_contract::`) | none; the workspace would not build otherwise |
| R2 | Task 5's `parse_edge_kind` deletion / `describe_node` retype carry into atlas-cli's call sites | a CLI pact diff, visible |
| R3 | Task 7 renames the five `*Out` shape names to D4 names in feature files, `aqc_cucumber.rs`, `AqcSteps.cs` | a feature rename diff the owner can revert; the alternative is what D4 forbids |
| R4 | `document::aqc_schema_json()` mechanically rewrites `#/components/schemas/` → `#/$defs/` | one line in document.rs |
| R5 | if cargo-mutants cannot examine `../graph-types/**`, run a second scoped invocation from `graph-types/` | two config files instead of one |
| R6 | `atlas-graph-types` 0.5.0 → 0.6.0 (new public fns + optional features = MINOR) | a lockfile line |
| R7 | AQC bumps MINOR with one CHANGELOG line if the semver gate classifies the new `aqc.schema.json` as a change | a version line |
| R8 | existing narrative comments in Cargo.toml/scripts stay unless a task rewrites their lines; purge not in this plan's scope | comments the owner would have stripped |
| R9 | Task 2 amends `zero_deps.rs` to the D9 law (deps exactly serde+utoipa optional; dev-deps serde_json; features canon-ids/serde/openapi) | one test file to revert, if the owner wanted the old law kept |
| R10 | reviewer finding on `DECLARED_NODE_KINDS: usize = 15` rejected — a named constant is the law's prescribed form | none |
| R11 | standing counting procedure's second command becomes `(cd ../graph-types && cargo test --all-features)` | one comment line and one script line |
| R12 | `NodeRef.kind` becomes closed `wire::PositionKind` (node kinds + "Edge"), not a bare String | one enum to rename/retype (principle-12 debt, unsigned by the owner); 1b Task 2's assertion changes to `PositionKind.Event` |
| R13 | Task 8 registers the seven detail routes in Proj.hs as top-level `Keep` projections | seven one-line registry entries |
| R14 | dropped constraints: `Contract.min/max_version` get `#[schema(pattern=…)]`; `ContentsRoot.kind/group`, `ContentsChild.kind`, `Scene.mode` stay strings (map migration territory) | the enums arrive one batch later than a consumer might like |
| R15 | `Year`-typed fields carry `#[schema(value_type = i32)]` so no primitive-named component publishes | eight attributes |
| R16 | `lib.rs::openapi()` (raw, un-named document) stops being public API | one visibility keyword |
| R17 | Task 9's per-mutant test command scoped to the crate that owns the file, not `--workspace` | one afternoon of mutation runs the owner can order now instead |
| R18 | `wire/map.rs` `rings: Vec<Vec<(f64,f64)>>` fields get `#[schema(value_type = Vec<Vec<[f64;2]>>)]` (NJsonSchema rejected the emitted `prefixItems`) | two attributes |
| R19 | register `ErrorBody`/`ErrorInner` as components so the published `$ref: '#/components/schemas/ErrorInner'` resolves | two components an outsider never reads |
| R20 | `document::openapi()` closes every object component as a derivation rule (nine open data structs get `additionalProperties:false`) | one loop in document.rs |
| R21 | dedicated Task 10 (comment purge) over `atlas-contract/src` + touched graph-types files; scope = this crate unless the owner widens it | not stated |
| R22 | public contract surface doc comments rewritten as consumer prose; a law test scans openapi.yaml for forbidden internal tokens | prose the owner rewrites; the law can be widened |
| R23 | (owner directive) Task 11 inventories all 43 wire structs; deletes every twin whose fields are a subset of the domain struct's | the owner asked for exactly this; one struct to restore if wrong |
| R24 | (owner's Haskell bar) closed vocabularies on the contract surface (`Scene.mode`, `ContentsRoot.kind/group`, `ContentsChild.kind`) become real enums in Task 11 | an enum that must grow a variant later — the point |
| R25 | `HashMap<String,String>` query parsing becomes typed `Query<T>` extractors with an AQC-taxonomy-preserving rejection mapping, in its own Task 12 | a task the owner can drop; the pacts catch any byte drift |
| R26 | Task 11 scope: pure twins die everywhere; flattening/enums apply only to surviving families; legacy detail projections left as-is (retire with FOCUS) | nothing; FOCUS deletes them anyway |
| R27 | two death-row touches that landed before R26 (EventPlace→PlaceRef merge; CrossRef/CatechismRef `attributed` de-dup) stay | one rename-table line in 1b |
| R28 | comment purge widened to the whole server workspace (Task 10a/10b split) | comments the owner wanted are one revert away per crate |
| R29 | the batch's single mutation gate is Task 13 (batch close), concurrency work first | one batch's survivors found a day later than per-task runs would have |
| R30 | three duplicate vocabulary mechanisms collapse to one (`vocabulary!` moves to graph-types) | one dependency edge |
| R31 | unknown `scope`/`dir`/`corpus` answer 400 (`bad_dir`, `bad_corpus`, new `bad_scope`) instead of silently falling through | a 400 where a lenient default was; pacts pin only valid inputs so no fixture moves |
| R32 | `atlas-core/.cargo/mutants.toml` `examine_globs` widens to `atlas-core/src/**/*.rs` in Task 13 | not stated |
| R33 | Task 13 (cargo-mutants + Stryker, concurrent) runs once, after 1b's last task | survivors found after 1b instead of after 1a |
| R34 | the batch's semver base is `cbe6a5a`, passed explicitly for every remaining task and the final review | none; the batch-end gate runs against the same base |
| R35 | five closed vocabularies published as prose become `vocabulary!` enums in Task 12 (`Landmark.kind/size`, `ProvenanceEntry.confidence`, `Event.kind`, `Heading.kind`) | five enums the data already respects |
| R36 | type-design inventory from the 10b-1 purge (TokenSpan pub fields, ResolvedDate no ctor, stringly closed sets, etc.), ledgered for Task 12 / next batch | not stated |
| R37 | type-design inventory from the 10b-2 purge (closed sets as strings, bare-int encodings, hidden units/bounds, positional tuples), ledgered for the next batch touching each crate | not stated |
| R38 | `/api/place`'s `bad_window` on an unreadable year (both-window and single-window cases) is a sanctioned, disclosed behaviour change alongside R31 | a 400 where a silent default was |
| R39 | `corpus=concord&scope=chapter` refuses with `bad_scope`, documented and pinned in both harnesses | an error code a client never sees |
| R40 | `ErrorCode` becomes a `vocabulary!` enum, `$ref`'d from `ErrorInner` | one enum |
| R41 | `text_window`'s duplicated window/next logic deferred to CONTRACT-2 (its own D5 proof) | ~35 duplicated lines live one batch longer |
| R42 | standing block's build leg becomes `cargo build --workspace --all-targets` | seconds per run |
| R43 | parked trivia (Point doc restated, double blank line, `ok().expect()`, glossary history tokens, `is_none()` guard, lost `format: double`, semver gate base is positional not `--base`), swept in 1b's first dispatch | not stated |
| R44 | `decompress_verified` (atlas-graph blob.rs) races with itself on a cold cache — real correctness bug, not a mutation survivor, out of CONTRACT-1 scope, sent to FOCUS-0 | a fresh clone's first test run is flaky |
| R45 | five tuple-typed signatures are 510 of 1,240 mutants (41%); named records deferred to FOCUS-1 (types) | the gate stays ~1.7× more expensive than it needs to be |
| R46 | N = 4 shards, not 8 — memory is the ceiling on this box, not cores | the gate is ~1.7× slower than a bigger box would allow |
| R47 | new house rule: never edit a shell script while an instance of it is running (byte-offset re-read) | a completed run's merge is lost |
| R48 | `present::focusable`/`present::display` (graph-types) have no caller; not deleted here, sent to FOCUS-0's deletion inventory | two functions live one batch longer than they should |
| R49 | three of the 41 "survivors" were configuration gaps (`--lib` not selecting `--bins`; an unnamed covering target), not test gaps | an inflated score |
| R50 | B17's cold-clone guarantee re-proved, not assumed, from a fresh clone under `core.autocrlf=true` (ContractTests 54/54, client.Tests 472/472) | a fresh clone fails to build |
| R51 | `git diff 34fb6e5..HEAD -- '*.cs'` touches ~20 hand-written files beyond the Stryker brief's 11; all but `PopoverSectionProviders.cs` are mechanical renames; that file's two real branches deferred to FOCUS-1 | two small branches stay unmutated one batch longer |

### B1–B20 (1b ledger)

| id | ruling | cost if wrong |
|---|---|---|
| B1 | Task 2's typed-kind test asserts `new[] { PositionKind.Event }`, not `NodeKind.Event` | one identifier |
| B2 | client code compares `NodeRef.Kind`/`NodeCard.Kind` against the enum member, never a `ToString()` round-trip | a handful of comparisons |
| B3 | Task 6 re-pins `EXPECTED_SCENARIO_COUNT` and `CorpusCountTests.cs` by the same delta together | two constants |
| B4 | `GeneratedUsageTests` treats a `$defs` type as read if hand-written code names it OR it's reachable via `$ref` from a read type | a looser law by one hop |
| B5 | Task 1 also sweeps 1a's R43 trivia as its own commit | none |
| B6 | Stryker installed in Task 1 (needed by Task 13) but not run in 1b (R33) | not stated |
| B7 | `Point` stays inlined as `IReadOnlyList<double>`, no named wrapper type | one alias later |
| B8 | `Reading(…, WindowDir dir, Corpus corpus)` takes the generated enums, not strings | two parameter types |
| B9 | one generic `WireName<T>` helper reads `JsonStringEnumMemberName` for every enum in a query string; `EdgeKinds.Label` delegates | one helper |
| B10 | the leftover-label check derives its set from `graph-vocabulary.json`, never a hand list | none |
| B11 | NSwag's snake-derived property names get a build-time `PascalCasePropertyNames` generator, `JsonPropertyName` keeps the wire | one generator project |
| B12 | `ContentsTreeModel` carries the generated enums (Root/Child records), not strings turned back from them | one model |
| B13 | `EdgeKinds.Label` (an alias of `WireName`) deleted; `Parse` reads `WireNames`' table | one alias |
| B14 | the plan's rename table's names corrected to the actual generated/domain names (Heading not HeadingEntry; EventWitness has no `ResolvedWitness`; PlaceDateClaim, History, PlaceRef-merged) — referenced in-text at Task 5, no preceding formal "Ruling:" line in the ledger | not stated |
| B15 | a property colliding with its record's own name is named for its schema (`Number` for integer, `All` for array), else generation fails loudly | three names |
| B16 | `Contract.Contract` (type vs namespace collision) left as-is; Task 6 deletes most call sites | an alias |
| B17 | the contract generator restores itself inside the client build (proved from a cold clone) | none |
| B18 | VERSION bumps MINOR per the brief unless the semver gate demands otherwise — the gate wins | one version string |
| B19 | the generated-type exclusion set is one public declaration in the generator, read by the test through the project reference | one declaration |
| B20 | the brief's `Every_excluded_type_is_unreferenced` test dropped — the compiler already refuses naming an excluded type | one test |

---

## 8. Carried forward

- **R44** (`decompress_verified` races with itself, cold-cache tmp-path collision) → **FOCUS-0** (now F0-2).
- **R45** (five tuple-typed signatures, 41% of the gate's mutants) → **FOCUS-1**.
- **R48** (`present::focusable`/`present::display`, no caller) → **FOCUS-0's deletion inventory** (F0-3).
- **R51** (`PopoverSectionProviders.cs`'s two real, unmutated branches) → **FOCUS-1**.
- **R36 / R37** (the Haskell-bar type-design inventories from the 10b-1/10b-2 comment purges) → later batches, the next one to touch each crate.
- **The three pre-existing Playwright failures** (`world-quiet-places.spec.ts:211`, `split-view.spec.ts:293`, `reader-xref-anchoring.spec.ts:201`) → owner queue.
- **`server/target` at 257.8 GB / 302,024 files** (a PRINCIPLES-14a smell, not pruned by the gate) → owner queue.
- **The 2026-09-28 data-loss incident and the raw-integrity plan**, five lines:
  1. A throwaway mutation-gate worktree junctioned nine `data/raw/*` dataset directories instead of copying them; `git worktree remove --force` followed the junctions and deleted through them, emptying `data/raw` from 408 MB to 34 MB.
  2. No copy existed anywhere else on disk; restoring depends on re-running `data/fetch-raw.ps1` against live sources that may have drifted, and `borders` is no longer fetched by that script at all.
  3. A second, related failure the same hour: the refetch itself died mid-copy and left `brain-fuel-bible/lexicon` half-populated and `morph/` empty — a state the script's own `if (-not (Test-Path …))` guards would have accepted as "already have it" forever.
  4. House rule adopted immediately (R47's family): a throwaway worktree never junctions or symlinks ignored data; links are removed with `cmd /c rmdir` before `git worktree remove`, or worktrees are created with `git -c core.autocrlf=false worktree add`.
  5. `docs/superpowers/plans/2026-09-28-raw-integrity.md` (read for this report; not part of CONTRACT-1) designs the fix: a merkle manifest over `data/raw` (per-file SHA-256 leaves, name-ordered directory nodes, one root), `bibex verify`/`bibex raw bless`, a fetch script that verifies what it downloaded instead of trusting any file that exists, and an off-worktree archive copy — not yet implemented.

---

## 9. Commits and pushes

| segment | range | commits |
|---|---|---|
| Pre-flight (found tree, committed before Task 1) | `bb9d884..78f51ff` | 4 |
| CONTRACT-1a (Tasks 1–12, final review, fix wave) | `78f51ff..34fb6e5` | 46 |
| CONTRACT-1b (Tasks 1–7, final review, fix wave) | `34fb6e5..934040f` | 22 |
| Task 13 mutation gate — Rust (sharding, defects fixed, gate run, docs) | `934040f..642ed58` | 15 |
| Task 8 (batch close) — Stryker | `642ed58..13111dd` | 2 |
| Post-close principles/gates docs | `13111dd..711ba8e` | 2 |
| **Total, `bb9d884^..711ba8e`** | | **89** |

First commit of the batch proper (excluding pre-flight): `fe44e4d`
("graph-types: kinds name themselves"). Last commit at time of writing:
`711ba8e` ("principles: a mutation run measures the present…").

Pushes to `origin/worktree-bible-atlas-m1`, all fast-forward, never forced:

1. `cbe6a5a..569786b` (26 commits) — "push as backup" (owner instruction, mid-Task 11).
2. `569786b..34fb6e5` — CONTRACT-1a's close push (fast-forward, per the ledger's explicit "PUSHED" line).
3. `…..fe79a31` — the mutation-gate report records "11 commits pushed (`37d5145…fe79a31`)", controller-verified; because a push is cumulative and fast-forward-only, this push necessarily also carried every 1b commit (`34fb6e5..934040f`) and the earlier Task-13 commits up to `37d5145` that had not yet reached origin, not only the 11 named. `d2ed170` lies inside this same pushed range (it is not independently documented as its own push point in either ledger — see footnote below).
4. A further push carried the Stryker commits (`0c47883`, `13111dd`) and the closing principles/gates commits through `711ba8e` (and, as of this writing, one commit further to `dd72809`) to origin — confirmed by direct inspection: `git rev-parse HEAD` and `git rev-parse origin/worktree-bible-atlas-m1` are currently identical (`dd72809`), so everything through `711ba8e` is on origin. No explicit "PUSHED" ledger line names this push's exact starting commit or count.

**Footnotes (disagreements/gaps found between sources, not resolved silently):**

- The ledgers explicitly document only two of the batch's pushes by exact
  range (items 1–2 above) plus one push's *tail* and commit-count (item 3,
  `37d5145…fe79a31`, 11 commits). Nothing in either progress.md states the
  push that carried 1b's own close (`34fb6e5..934040f`) or the final push
  that carried the Stryker/close-out commits to `711ba8e`. Their occurrence
  is inferred here only from origin now matching local HEAD and from the
  fast-forward-only, never-forced constraint stated in the ledgers — not
  from a controller-written "PUSHED" line for each.
- This brief's own push chain — "`569786b` backup → `34fb6e5` → `d2ed170` →
  … → `711ba8e`" — names `d2ed170` as a waypoint, but neither ledger records
  a push stopping at `d2ed170` specifically; the only controller-verified
  push evidence in that region names the range `37d5145…fe79a31` (which
  contains `d2ed170` as its second commit, not its endpoint).
- At the time this report was compiled, the actual repository HEAD (and
  `origin/worktree-bible-atlas-m1`) had already advanced one commit past the
  `711ba8e` this report was scoped to, to `dd72809` ("plans: RAW-INTEGRITY
  …"), which commits the raw-integrity plan file read for §8. This report's
  figures are anchored to `711ba8e` as directed; `dd72809` is noted here
  only because it is outside CONTRACT-1's own scope.
