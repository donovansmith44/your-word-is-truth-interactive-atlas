# CONTRACT-2 close report

Batch = CONTRACT-2, the contract pushdown (plan `docs/superpowers/plans/2026-09-27-contract2-pushdown.md`
and its amendment `2026-09-29-contract2-amendment.md`): Tasks 1–6, 7a/7b, 8a/8b, 9, 10a/10b, 12 (with
its R-C2-R9 addendum), the wave-5 ETL landing and its one rebuild, and Task 11 (this close). Compiled
from `.superpowers/sdd/2026-09-29-contract2-pushdown/progress.md` (every ruling), every `task-*-report.md`
and `landing-report.md` beside it, `task-11-report.md` (this close's own measurements),
`.superpowers/MUTATION-GATE-DEBT.md`, and `git` over `7b6b2c3..HEAD`. Every number below comes from one
of those sources or from a command run at the close and named where it is quoted. None is estimated.

Base (PRINCIPLES 22): **`7b6b2c3`** (the FOCUS-0 close). The first batch commit is `d38344b`
(2026-09-29 16:45); the close's last code commit is `059790f` (2026-09-30), followed by this report.

**Status: CLOSED by the fix wave (§6a, 2026-09-30 afternoon).** As of Task 11 it read: the standing block, the timing gates and the semver gate are green; the contract
gate is red on one AGC law that the batch's own card shape breaks (§2), Playwright has seven new failures
that are not the batch's declared changes (§2), and timing gate 1 regressed materially (§1). Each is named
with its cause below and routed in §7. No ceiling was raised; no failing spec was re-pinned to pass.

---

## 1. Standing block

Measured at the close in WSL2 (Ubuntu 22.04, 16 threads, 25 GB), from `server/`, `nice` 0, the owner's
app idle or down. `uptime` load is recorded with each timing.

| leg | result |
|---|---|
| `cargo test --workspace --no-fail-fast` (first run, at `afa7e13`+) | **1,415 passed, 1 failed, 10 ignored** (the timing gates); 93 `test result` lines, 5 of them Doc-tests, so 88 sections, plus `aqc_cucumber` (no line). Wall 29 m 46 s; load 2.03 → 1.56. The one failure, `year_labels`' whole-schema pin of `TimeRange`, was this close's own comment strip moving a description into `#[schema(description)]`; re-pinned in `8357140`, then 10/10 |
| `cargo test --workspace --no-fail-fast` (final clean run, at `059790f`) | **1,416 passed, 0 failed, 10 ignored**; 93 lines, 5 Doc-tests, so 88 sections, plus `aqc_cucumber`. Wall 28 m 53 s; load 1.58 → 2.15 |
| `(cd ../graph-types && cargo test --all-features)` | **152 passed, 0 failed**; 10 lines, 1 Doc-tests, so 9 sections |
| **canonical** | **1,568 = 1,416 (workspace) + 152 (graph-types) across 98 sections (88 + 1 + 9), 0 failures** (the final run). FOCUS-0's close was 1,443 = 1,294 + 149 across 90 |
| AQC corpus (harness=false, not in the sum) | **47 scenarios / 194 steps** (195 → 194: P8 removed the `advertises` line) |
| dotnet | `client.Tests` **476/476** · `client.ContractTests` **54/54** (47 scenarios + the non-scenario tests) |
| `cargo build --workspace --all-targets` | 0 errors; the **2 known Linux-only warnings** in `atlas-cli/tests/raw_walk.rs` (`ELSEWHERE`, `hex_of_node`), reported and not fixed |
| `bash scripts/timing-gates.sh run` | **TIMING GATES: 10/10 passed** (11:31–11:43, 11 m 10 s; load 1.37 → 2.55) |
| `bash scripts/timing-gates.sh check` | **"timing-gates check: 10 gates reconcile with the tree"** |

These are the first WSL baselines for the whole block:

| gate | WSL (this close) | ceiling | Windows baseline (FOCUS-0) | verdict |
|---|---|---|---|---|
| 1 `graph_conformance` assert_answers_match | **37.10 s** (re-run alone, load 1.39: 32.09 s) | 60 s | 11.72 s | **MATERIAL REGRESSION** (below) |
| 2 scene time, full span | 23.33 ms | 75 ms | 23.75 ms | same |
| 3 scene time, NT window | 11.91 ms | 75 ms | 11.61 ms | same |
| 4 scene scripture, JHN.3 | 24.29 ms | 50 ms | 25.46 ms | same |
| 5 xrefs, JHN.3.16 | 1.10 ms | 30 ms | 2.32 ms | faster |
| 6 text window, JHN.3.1 n=20 | 2.21 ms | 30 ms | 2.85 ms | now measures anchors too (R-C2-R8) |
| 7 chapter window, JHN.3 | 4.06 ms | 50 ms | 3.48 ms | now measures anchors too (R-C2-R8); not comparable |
| 8 `sqlite_real_data` | 497.2 s (write 178.0, dump 9.3, answers 124.1) | 1560 s | 797.6 s (213.8 / 8.1 / 376.4) | faster (WSL; T6 measured −39 %) |
| 9 `sections_startup` | 453.7 ms (from_sections 97.1, finish 4.3, priming 352.4) | 4 s | 826.3 ms | faster |
| 10 frontier latency | sqlite p99 569.5 µs, mem p99 61.5 µs | 100 ms | sqlite p99 2.76 ms, mem 59.3 µs | same / faster |

**Gate 1, investigated (PRINCIPLES 14a), not endured.** Measured on WSL at the batch base in a
throwaway worktree (data copied, 0 links, removed after): `7b6b2c3` **13.70 s**; `69a6648` (T6, the token
layer) **11.84 s**; `1fc79e3` (T7a, one `mentions` row per located occurrence) **27.20 s**; HEAD 32.1–37.1 s.
The step is T7a: 35,852 → 41,548 mention rows, concentrated on a few entities (God's `mentioned-in`
frontier serves 12,530 entries). Working hypothesis, not yet proved: the conformance sweep drains every
frontier at `limit 1` and at `limit count`, and a page of the in-memory store costs O(frontier), so the
largest frontiers go quadratic. Routed as a queue item (§7). The ceiling was not touched.

Handler timings for R-C2-R8 (1), debug `atlas-server` on the sections, 60 requests each, median:
`/api/text?ref=JHN.3.1&n=20` 4.29 → **3.61 ms**; JHN.3 chapter 6.91 → **5.60 ms** (T7b measured 5.81
before anchors, 8.29 after); BoC 7.9.1 n=20 3.19 → **3.18 ms** (load 2.84 before, 3.31 after).

---

## 2. Gates

| gate | verdict |
|---|---|
| `bash scripts/contract-gate.sh --base 7b6b2c3` | **FAILED** (exit 1) on leg 4 alone: one non-target red in `atlas-graph-contract`, `graph/identity.feature` "an Event node, as a consumer reads it" — *"1 value(s) of "kind" are declared nowhere in graph-types' kind_tags!/relations! manifests"*. The value is `"event"`: T5 (P7, D11) serves `NodeCard.event.kind: EventKind`, and the AGC law checks EVERY `kind` key in an answer against the graph vocabulary. It stayed hidden until the pact was re-recorded here (T3–T9 replayed the pre-batch pact). Leg 3: HTTP recorder 4/4, CLI 1/1. Leg 6: 28 committed expectations, 26 green, 1 `@target` red (disclosed), 1 red; coverage reconciles 28/28. Leg 5 (semver) ok |
| `bash scripts/contract-semver-gate.sh 7b6b2c3 --runner <built runner>` | **exit 0.** AGC ok, *"diff requires major, declared minor = major under the 0.x rule, version 0.15.0"*; AQC ok, same wording, 0.11.0; `map-api-consumer` unchanged; 2 of 3 versioned suites classified |
| `export_contract -- --check` | exit 0 after the version bump and after every later change |
| `contracts/atlas-edge` (received suite, D7) | replays **all green** against the re-blessed pact |

**Playwright** (`cd tests/ux && npx playwright test`, WSL, config-started `start-api.sh`/`start-client.sh`):
the first attempt could not launch a browser (the cached `chromium_headless_shell-1200` predates the
installed `@playwright/test` 1.62.1, which wants rev 1234; 397 launch failures); `npx playwright install
chromium` fixed it (no missing libraries under `ldd`). The run: **439 passed / 13 failed / 5 skipped of
457 in 9.9 min** (load 1.29 at start, 5.49 peak — the other agent — 1.49 at end). FOCUS-0 had 449/5/2 of
456 (T12 added PERSON-4). Re-running the 13 alone: 12 reproduce, `world-cluster-chooser.spec.ts:213`
passed (flake). Of the known list, `world-quiet-places:211` still fails; `split-view:293`,
`reader-xref-anchoring:201` and the `world-hover-text:658` flake passed.

Updated to what the batch deliberately serves (commit `059790f`, each 2/2 under `--repeat-each=2`):
- `reader-map.spec.ts:34` WORLD-8 — the legacy place page's events carry a labelled `when` (T3, P1); the spec read `when.from_year`, now the served `when.label`.
- `state-window.spec.ts:91` ST-2/R2 — the atlas pane's readout is labelled from the served window (R-C2-R4); the spec recorded the readout before the label arrived, now waits for it.

NOT updated — new failures outside the declared changes, each with its observed cause:
- `reader-persons.spec.ts:160` PERSONS-1 — a person's MENTIONED IN SCRIPTURE list shows GEN.24.29 twice: one `mentions` row per occurrence (D21) reaches the card as two frontier entries under one edge id, and the list renders each. A verse listed twice is a regression.
- `w2-passages.spec.ts:9`, `w3-passages.spec.ts:45`, `popover-sections.spec.ts:2029` — clicking `verse-line-N` now lands on a served mention anchor (the page snapshot for PSA.14.1 shows God's person card open, reached through the verse's "God" link), so the verse popover and its `verse-event-*` row never appear. Whether the old client linked the same word there, or the served text or anchors moved it under the click point, is not established.
- `world-place-history.spec.ts:35` NAME-1, `:117` and `:183` DATE-1 — hovering `marker-jerusalem` at −590..−580 opens **Baal-perazim**'s card (a quiet place 5.9 km away in that window); the three share the cause. Not traced to a commit.
- `world-slider.spec.ts:7` WORLD-5 — intermittently a typed range is replaced by the previous window's label (counterexamples −2426..−411, then −785..−2; neighbours pass): a late served label clobbering a typed one, most likely R-C2-R4's resync.
- `person-card.spec.ts:77` PERSON-4 — T12's spec, never run until now: it clicks a "Jesus" mention in LUK.3.23, but the served anchors do not locate that word (T7a searches a person's `displayTitle` "Jesus Christ" and `alsoCalled`, not `name`; the queued name model).

---

## 3. What the batch delivered

Commits `7b6b2c3..HEAD` are listed in `task-11-report.md`. By area (`git diff --numstat 7b6b2c3..059790f`):
282 files, +29,422 / −3,432; excluding `contracts`, `data`, `docs` and `.superpowers`, **214 files,
+8,225 / −3,027**.

| task | what | SHA(s) on branch |
|---|---|---|
| T2 | `TextRef` (book as `BookId`, canon code on the wire), `TextPoint { unit, word? }`, `TextSpan`, `Anchor` in scalar chars; `Err(ForeignLayer)` | `d38344b` |
| T1 | `atlas_core::label::{Year, TimeRange}` (`Year::of → Result`), `DateClaim`, `vectors/year-labels.json` | `0a06f20` |
| YearInput | the client reads back every served label | `a6998bd` |
| T3 | labelled years and structured loci on every graph-path struct; `/api/eras` and `/api/polities` gain `window`/`reign` beside the integers (D7) | `a4373de` |
| T4 | tests never write the repo (R-C2-R2); `TextRef` tagged by corpus (R-C2-R1); `EdgeEntry` votes/narrative/loci/note (P6) | `c299b59`, `ed81a15`, `136e5bd` |
| T5 | kind details on the card (P7); `TextUnit.heading`; R-C2-R3 (2)–(5) | `9bbc1f2`, `55c8fb5` |
| T10a | generator closes discriminated unions; the client reads served labels; `YearText`/`years.ts` deleted; R-C2-R4 | `cfaf9f3`, `5b94ec4`, `876a6d0` |
| T9 | `/api/contract` declares schema versions only (P8) | `264aadc`, `1e024aa` |
| T6 | one real pipeline context (F0-25); `kjv_token` in the Kjv section (schema 16) | `38885fc`, `69a6648` |
| T7a | mention spans in ETL, one row per located occurrence | `1fc79e3`, `ab75fdf` |
| attests cap | every verse of an event's account attests it (33,355 → 43,067) | `68fb00f` |
| T8a | Concord citations as `cites` rows; `concord_token` | `5a9a268`, `6db5803` |
| landing | runs vectors path; MACULA domains dropped; `sources.json`; **the one rebuild** (root `8d8dd1d0…`); canon pins | `33310f2`, `ef4cd99`, `2744b02`, `4f13161`, `e10d48c`, `a5f09c7` |
| 7b+8b | `compiled_app()` finishes its data; `UnitHeading`; mention and citation anchors on text windows | `1b5881b`, `0ff2d60`, `e8782c0` |
| T12 (+R-C2-R9) | `spouse-of`; typed `Parentage`; God → Jesus added; Mary ever virgin, `brethren-of`; schema 17 → 18 | `40e42f8`, `434744d`, `3bd4a93`, `533dc9a` |
| T10b | the client renders served anchors, runs and citations; `PlaceMentions`, `Versification`, `AcctCoalesceTests` deleted | `7771f7f`, `545df5d`, `e53a13a`, `d93b01f`, `07a6063` |
| T11 | comment strip; one lookup per window; `--build-from-raw` finishes; versions; fixtures and pacts; pins; the `Contract` record; mutation bookkeeping; two Playwright specs | `acf8d5a` … `059790f` (`task-11-report.md`) |

---

## 4. Artifact and version moves

| item | before (`7b6b2c3`) | after |
|---|---|---|
| section schema | 15 | **18** (16 token tables, 17 spouse + parentage, 18 brethren) |
| compiled manifest root | `31e118e8e6d47276ebeec3ed23d53217` | **`b745ec41abf7f060966f66bfe45524fb`** (via `8d8dd1d0…`, `6ffc6df2…`) |
| section logicals | core `f0088b69…`, kjv `a97cc311…`, concord `1b6b2b4c…` | core **`c6a98eba…`**, kjv **`6f413a55…`**, concord **`bfe52b2b…`**; kretzmann `cfebcd66…` unchanged; lexicon `1dc3b9df…` (MACULA) |
| from-raw version root (`EXPECTED_VERSION_HEX`) | `ce9d653b597e6893a25a5264c60ff306` | **`3e91f83bbdf58315b12eb2353ddbd7d8`**, pinned once at T11 |
| `raw_root` | `ca285021…` | unchanged |
| AQC | 0.10.0 | **0.11.0** (D1: MAJOR class, MINOR bump under the 0.x rule) |
| AGC | 0.14.0 | **0.15.0** |
| graph-types | 0.6.0 | **0.7.0** |
| rows | mentions 35,852; attests 33,355; parent-of 1,776 | mentions **41,548** (35,488 located, 6,187 unlocatable); attests **43,067**; `kjv_token` **790,892**; `concord_token` **289,916**; Concord `cites` **1,246**; parent-of **1,769**; spouse-of 104; brethren-of **4**; spoken-at 6,381 → 6,402 |
| `contracts/` at T11 | — | 24 AQC fixtures, `http.json` +465/−37, `cli.json` +1/−1, 5 AGC fixtures; documents carry 0.11.0 |

---

## 5. Every ruling

| id | ruling |
|---|---|
| R-C2-P1 | Wire tasks regenerate DOC only; FIX, ROOT (one rebuild) and VER happen once, at T11; each task declares its expected-red set |
| D1 | AQC 0.10.0 → 0.11.0: "MAJOR class, MINOR bump under the 0.x rule" |
| D2 | The spec's range labels win (`1450 – 1400 BC`); `YearInput` accepts every served form |
| D3 | Core `TimeRange` publishes as `YearSpan` (document-only) |
| D4 | `Year`/`TimeRange` live in atlas-core, re-exported by the contract; `Era` loses `ToSchema` |
| D5 | `ContentsChild.locus` is the unit the entry opens at |
| D6 | `DateClaim` in `PlaceDetail`/`BookDetail.written` moves to T5 |
| D7 | `/api/eras`/`/api/polities` are additive (`window`/`reign` beside the integers `atlas-edge` consumes) |
| D8 | `attested-in` loci are the coalesced runs; `attests` loci the edge's own range |
| D9 | Only coalescing `AcctCoalesceTests` cases become vectors |
| D10 | Legacy route structs publish as `EventPage`/`PlacePage` |
| D11 | `kind: EventKind` (typed, bytes as the spec's string) |
| D12 | Typed `EraId`/`PolityId`/`NarrativeId`/`votes`; `DateClaim.event` accepted |
| D13 (revised) | `kjv_token` in the Kjv section; gate 8 measured at T6 and T11 |
| D14 | Subsumed by R-C2-P1 |
| D15 | One accessor over `mentions` + token offsets; one char-offset rule |
| D16 | Kretzmann citation scanning out (withdrawn, then restored by the deferral; `KretzmannCitationScan` stays client-side) |
| D17 | One `include_str!` VERSION const; the AGC `contract` projection re-blesses per schema bump |
| D18 | Client build in the red set until 10a; Playwright only at T11 |
| D19 | No per-task mutation; at T11 the debt row (owner's roadmap defers the run) |
| D20 | Offsets are Unicode scalars; the lossless token law |
| D21 | One mention row per occurrence (edge ids do NOT move — corrected at T7a) |
| D22 | `existence_from/to: Option<Year>`; whole-body tests |
| HELD → LIFTED | Words as base (R-C2-W1 790,892 words, W2 the wire addresses words, W3 Kretzmann on word spans, W4 KJV stored once) — then **DEFERRED** by the owner: only the token layer and `TextPoint.word` kept |
| OPEN-1…9 | Stacked ETL lane; gates 6/7/9 judge composition; red letter untouched; `Year::of → Result` + `TextRef.book: BookId`; `ForeignLayer`; `loci` on comments-on only; Concord as words; layer hash; the spike's comparison — as ruled in the ledger |
| R-C2-R1 | `TextRef` internally tagged; `Year` deserializes through `Year::of`; year-zero 500 tested |
| R-C2-R2 | Tests never write the repository |
| R-C2-R3 | Attests cap fixed in the ETL stack; graph-defect panics; coalesce once per page; corpus tags once; `TimeRange` via `try_from`; the C# `TextRef` bag |
| R-C2-R4 | The first-visit blank slider readout is a regression, fixed; the one-fetch lag accepted |
| R-C2-R5 | Off-word occurrences count unlocatable; runs vectors via one path; `GENERATED_DOCUMENTS` merged; 6,187 misses queued |
| R-C2-R6 | `sources.json` committed with the landing; `f/ff/sq` citations cite the stated verse |
| MACULA | Semantic domains dropped before the rebuild (UBS ShareAlike) |
| Rule 9 | No comments in application code; T11 strips CONTRACT-2's |
| R-C2-R7 | `partner-of` → `spouse-of`; typed parentage with Scripture grounds (T12) |
| R-C2-R8 | T11's opening steps: batched window reads, `--build-from-raw` `finish()`, the 500 branch tested |
| R-C2-R9 | God → Eve `created` kept; Mary ever virgin; `brethren-of`; schema 18 |
| Queue | Every queued item moved to `.superpowers/QUEUE.md`; licensing rule (Theographic replacement, catechism license request) |

---

## 6. Incidents

- **The WSL move (2026-09-29 night).** The program moved from Windows to WSL2 mid-batch: a fresh clone at
  `a6998bd`, `data/raw` and `data/cache` copied, `bibex verify` OK, toolchains user-local
  (`~/.bible-atlas-env`), `.wslconfig` 26 GB. Task 6's uncommitted work travelled as a patch pair and
  applied cleanly on `7b6b2c3`. Measured effect: `cargo test --workspace` 25 m 59 s vs 32 m 03 s; gate 8
  −39 %, gate 9 −48 % (T6). The close found one leftover: Playwright's cached browsers predated the
  installed package (§2).
- **The controller's dispatch-overlap error.** T12 was dispatched into the main tree while 7b+8b's final
  `cargo test --workspace` was running, so that run (1,377/7) was over a moving tree and four doctest
  targets failed to compile on T12's half-written edits. The rule recorded: never dispatch into the main
  tree until the previous primary has REPORTED, not merely committed. The clean full-suite proof is this close's.
- **Masked seams found by the close's one regeneration.** (a) `1e024aa` declared the `Contract` record
  unread, but the ContractTests read it by name — fixed (`029cc4e`); (b) the AGC `kind` law vs `EventDetail.kind` (§2); (c) the
  Playwright regressions (§2) — all hidden while fixtures and pacts stayed stale by ruling (R-C2-P1).

---

## 6a. Fix wave (R-C2-R10 (1)–(6), 2026-09-30 afternoon)

Full report: `.superpowers/sdd/2026-09-29-contract2-pushdown/fixwave-report.md`. Nine commits
`4f90ad3` … `a149199` on top of `b0130e5` (with the owner's two principles commits between). Every
item was fixed as a category with a closure law (PRINCIPLES 24/24b), on the side separation of
concerns names; every number below is from a command run in the wave and named in that report.

| item | category / mechanism (proved) | fix, side | closure law | result |
|---|---|---|---|---|
| (1) AGC `kind` law | a law over a JSON key name, blind to the owning shape | runner: graph kinds by the shapes' own fields; twin law over every enum `aqc.schema.json` publishes (`--schema`) | the twin law enumerates the schema | leg 4 green; a narrowed scratch schema fails it naming `kind='event'` |
| (2) gate 1 | `raw_neighbors` CLONED the whole frontier per page; the sweep drains at limit 1 → O(n²); God 8,587 → 12,530 rows = ×2.13 | graph-types borrows the index | `Frontier` is the one door (3) | **37.10 s → 1.02 s** alone (load 0.00 → 0.63); micro-benchmark 7.83 s → 0.78 ms |
| (3) frontier by edge | pages/summaries/CLI enumerated ROWS; core section 9,108 duplicate id groups (Mentions), lexicon 92,360 (43,899 non-consecutive) | `explore::Frontier` (rows private, `page`/`edge_count`/`edges`); SQLite first rows + `COUNT(DISTINCT)`; writer places rows from the row tables; mention `loci` | a law over `RowFamily::ALL` on the artifact; tests derive from the artifact (P26) | God 12,530 → 8,587, Aaron 347 → 331, Hazor 12 → 11; pacts/fixtures re-recorded once; root, logicals, 25 scene hashes unmoved |
| (4) Jerusalem → Baal-perazim | hit-testing at TRUE positions while markers draw NUDGED (event log: the marker's own mouseover resolved to the quiet dot) — not a label lookup | map.js: the routed element wins, distance at rendered positions, coincidence judged in truth (two commits) | Playwright category spec (every lit marker of the window) + a source-scan law over `resolveHoverTarget` | NAME-1 ×2, DATE-1 ×2, HOVER-RESOLUTION-1 green |
| (5) slider race | (a) a served label written into an input being edited; (b) a late response over newer state, four hand-rolled idioms | `RequestSeries`/`Request.Fetch` (superseded answers dropped inside), `Draft`; 22 sites migrated, 4 idioms deleted | two source-scan laws (every component fetch through `.Fetch(`, every typed input on a draft) | WORLD-5 green; client.Tests 476 → 489/489; client +186/−109 |
| (6) specs | a spec pointing at a verse row whose centre is a mention link | `openVerse` (focus + Enter), 67 sites / 16 specs; PERSON-4 → LUK.3.22 | `spec-hygiene.spec.ts` refuses a row click | w2/w3/popover-sections/person-card green |

Gates at the wave's end, all under `heavy`: workspace **1,421 passed / 0 failed / 10 ignored** after one re-pin
(JHN.3.16's served `words` 26 rows → 21 entries), `aqc_cucumber` 47/194, graph-types 153 — **1,574 across 98
sections**; timing gates **10/10** (gate 1 **1.02 s**, gate 10 sqlite p99 787 µs); `contract-gate.sh --base
7b6b2c3` **PASSED** (coverage 28/28, semver ok); `client.Tests` 489/489, `client.ContractTests` 54/54; Playwright
**453 / 2 / 4 of 459** (6.4 min; the known `world-quiet-places:211` and `split-view:382`, a load flake
that passes alone) — the close's condition (the contract gate passes; Playwright's only failures are the known
reds and a flake) is met. Every gate was run and read; the batch is closed by this wave subject to the
owner's reading of the FINDINGS.

Open categories reported under FINDINGS (F1–F9) in the fix-wave report: two fetch layers holding
`HttpClient`; the Explore layer's 14 fetch sites outside the request series; saved explorations
resolving a date/era by label; literal expectations in older real-data tests; O(position) node paging.

## 7. Owed and routed

**Owed by this batch before it closes:**
- The AGC `kind` law vs `NodeCard.event.kind` — a ruling: narrow the law's scope (an AGC guarantee narrowed), or rename the served field, or publish `EventKind` in the graph vocabulary.
- The seven Playwright failures in §2 — PERSONS-1 (duplicate verse rows), the verse-line click landing on anchors (w2, w3, popover-sections:2029), NAME-1/DATE-1 (Baal-perazim at Jerusalem), WORLD-5 (typed readout race), PERSON-4 (Jesus not located).
- Gate 1's regression (13.7 s → 32–37 s on WSL, from `1fc79e3`) — confirm the mechanism and fix it.

**Carried:**
- **Mutation** (D19, the roadmap defers the run): `MUTATION-GATE-DEBT.md`'s row now reads "FOCUS-0 …, CONTRACT-2 (closed 2026-09-30, base `7b6b2c3`)"; base for the next run stays `13111dd`. New covering targets (`092ba7b`): atlas-contract `locus_wire`, `year_labels`; atlas-graph `kjv_tokens`, `runs_vectors`, `mention_spans_vectors`, `mention_spans_real_data`, `citation_vectors`, `concord_citations_real_data`. Stryker's list already has 10b's four; `PassageBlock.cs`, `CanonRef.cs`, `KretzmannCitationScan.cs` are a controller call (10b).
- Wire field descriptions utoipa 6 cannot carry per field (`NodeCard`'s details, `TextUnit.heading`/`locus`, `ContentsChild.locus`) were dropped with the comments; struct-level descriptions carry the rest.
- `Parentage`'s `#[doc = …]` inside `vocabulary!` (T12) is the macro's way to a schema description, not a comment line; left.
- D7's integers on `/api/eras`/`/api/polities` until map-generator moves; the legacy route restatement until FOCUS retires them (T5).
- Queued (QUEUE.md): the name model (6,187 unlocatable mentions; "Jesus"), 65 newly visible attestation collisions, the words-as-base inversion, 11 copies of the verse-to-locus conversion, two untested SQLite error branches (T12), the whole-repo comment strip.
- R44's cold-cache proof (O-CACHE) still waits on the owner.
- The two known `raw_walk.rs` warnings under Linux.
