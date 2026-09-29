# FOCUS-0 + RAW-INTEGRITY close report

Batch = FOCUS-0 graph side (plan Tasks 1–9 plus controller Task 10, the R44
race fix), RAW-INTEGRITY (Tasks 1–6), the whole-branch review, its fix wave
and group 9b. The two ran together from one base. Compiled from
`.superpowers/sdd/2026-09-27-focus0-graph/progress.md` (rulings F0-1…F0-26), every
`task-*-report.md` there, `final-review.md`, `fix-wave-report.md`,
`group-9b-report.md`; `.superpowers/sdd/2026-09-28-raw-integrity/progress.md`
(F-RI-1…F-RI-9) and its task reports; `.superpowers/MUTATION-GATE-DEBT.md`; the
two plans; and `git` over `dd72809..7b6b2c3`. HEAD at time of writing: `7b6b2c3`,
equal to `origin/worktree-bible-atlas-m1`. Every number below comes from one of
those sources. None is estimated.

Base (PRINCIPLES 22): **`dd72809`** for both batches, from HEAD at dispatch on
2026-09-29 at about 08:05. The first batch commit is `f4b098c` (08:08) and the last
is `7b6b2c3` (16:14).

---

## 1. Standing block

**Measured at Task 9, 2026-09-29, from `server/`, with the app down on a quiet machine, at `6dd48fd`:**

| leg | result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **1,294 passed, 0 failed, 10 ignored** (the timing gates). 85 `test result` lines, of which 5 are Doc-tests with 0 each, so 80 sections, plus `aqc_cucumber`, which has no line. Wall time 32 m 03 s |
| `(cd ../graph-types && cargo test --all-features)` | **149 passed**; 10 lines, 1 Doc-tests, so 9 sections |
| **canonical** | **1,443 = 1,294 (workspace) + 149 (graph-types) across 90 sections (80 + 1 + 9), 0 failures.** CONTRACT-1's close was 1,291 = 1,160 + 131 across 87 |
| AQC corpus (harness=false, not in the sum) | 47 scenarios / 195 steps, unchanged |
| `cargo build --workspace --all-targets` | exit 0, **0 warnings** |
| `bash ../scripts/timing-gates.sh run` | **TIMING GATES: 10/10 passed** (15:10–15:25) |
| `bash ../scripts/timing-gates.sh check` | **"timing-gates check: 10 gates reconcile with the tree"** |
| `--build-from-raw` smoke (F0-25) | `GET /api/contract` **200**, `{"min_version":"0.10.0","max_version":"0.10.0","manifest_schema":1,"section_schema_version":15}`; from-raw build 57.1 s; 31,102 KJV text units, 343,558 cites edges (1,241 negative-vote rows dropped), graph version `f3eab9b319696aeb098aaf47fffc050e`; 21 sources loaded |

Timing gates:

| gate | this run | ceiling | baseline |
|---|---|---|---|
| gate 1 (`graph_conformance`) | assert_answers_match **11.72 s** | 60 s | 11.6 s (CONTRACT-1) / 11.7 s (1a) |
| gate 8 (`sqlite_real_data`) | **797.6 s** total (write 213.8, dump 8.1, answers 376.4) | 1560 s | 741.5 s (CONTRACT-1) / 779.6 s (recorded) |
| gate 9 (`sections_startup`) | **826.3 ms** (from_sections 129.3, finish 4.1, priming 692.9) | 4 s | 839.6 ms |
| gates 2–7 | 23.75 / 11.61 / 25.46 / 2.32 / 2.85 / 3.48 ms | 75 / 75 / 50 / 30 / 30 / 50 ms | similar |
| gate 10 | sqlite p99 2.76 ms, mem p99 59.3 µs | 100 ms | none recorded |

**Gate 8 is the second reading above CONTRACT-1's 741.5 s.** It is +2.3 % over
779.6 s and +7.6 % over 741.5 s. The write step is +23 s (213.8 vs 190.5), which
tracks this batch's added rows (Authored, Shown, MapSuccession, Concord
CanonSuccession), and assert_answers_match is +10.5 s. The reading is inside the
gate's recorded spread (1064 → 845 → 823 → 780 → 741) at 51 % of the ceiling. It
was judged not material and is being watched. No ceiling was touched.

---

## 2. Gates

| gate | verdict | at |
|---|---|---|
| Contract gate `bash scripts/contract-gate.sh --base dd72809` | **PASSED**, exit 0. Leg 3 HTTP 4/4, CLI 1/1. 28 committed expectations: 27 green, 1 `@target` red (disclosed), 0 red. Coverage reconciles 28/28 | `6dd48fd` |
| Semver gate `contract-semver-gate.sh dd72809 --runner …` (positional base) | **exit 0.** AGC ok, *"diff requires major, declared minor = major under the 0.x rule, version 0.14.0"*. AQC ok, same wording, 0.10.0. `map-api-consumer` unchanged, nothing to classify. Runner rebuilt with `cabal build all` first | `6dd48fd` |
| `export_contract -- --check` | exit 0 (after the bump and again before committing) | `6dd48fd` |
| Semver leg between Task 1 and Task 9 | expected red by ruling (F0-10). Non-semver legs were run per wave; the whole gate ran once at Task 9 | none |

**Playwright** was run from `tests/ux`, because the brief's repo-root path finds
no config. The result was **449 passed / 5 failed / 2 skipped (6.4 min)**,
against 451/3/2 at CONTRACT-1. There are 456 tests, the same total as before.

- The 3 known failures are unchanged: `world-quiet-places.spec.ts:211`, `split-view.spec.ts:293`, `reader-xref-anchoring.spec.ts:201`.
- **`api-place-history.spec.ts:127` failed every time (2/2 on re-run).** Its whole-body pin for Jerusalem's `established` predated Task 7's served `event`. Task 9 stopped before pushing, as its brief required. **F0-26:** the controller re-pinned it in `7b6b2c3` (`event: 'sam2_jerusalem_captured'` / `'exl_jerusalem'`), and the rerun of both specs gave 20 passed / 1 skipped.
- **`world-hover-text.spec.ts:658` is a flake:** expected `JOS.22.1-4`, got `JOS.22.4` under the full suite. It passed 2/2 alone (`--repeat-each=2`) and was green in the controller's rerun. It is load-dependent and was **routed to FOCUS-1** (hover/popover work), not re-pinned.

---

## 3. What the batch delivered

41 commits in `dd72809..7b6b2c3`:

| segment | commits |
|---|---|
| RAW-INTEGRITY T1–T6 | 9 |
| FOCUS-0 T1–T8 and T10 | 10 |
| fix wave | 14 |
| group 9b | 3 |
| Task 9 | 2 |
| F0-26 | 1 |
| CONTRACT-1 close-report commits (`121470b`, `cd986ef`) that interleaved | 2 |

Commits made in companion worktrees reached the branch by cherry-pick, so they
carry new SHAs. Both are given below as `wip → branch`.

### FOCUS-0

| task | what | SHA(s) on branch |
|---|---|---|
| T1 | `AuthoredBy` (code 20) and `Shows` (21) appended last. EdgeKind 46 → 50. F0-6 applied: one `DECLARED_DIRECTED_RELATIONS = 22` / `DECLARED_SYMMETRIC_RELATIONS = 6` in `graph-types/tests/common/mod.rs`, and the 50 is derived | `77bef67` |
| T2 | `Authored` row family in Core. `RowFamily::ALL` 26. Justified rows. One `DECLARED_ROW_FAMILIES` | `6ff48c6` |
| T3 | `authored: 32`: 14 authors across 32 books, e.g. Moses 5, Paul 13 (F0-18). `BookAuthorship` beside `BookMeta` (F0-17). Law `every_authored_edge_resolves`. `curated-books` provenance | `5a5ec04 → dde1b46` |
| T4 | One minter `corpus_root.rs` (`Container:bible` "The Holy Bible", `Container:concord` "The Book of Concord"). Contents finds roots by parentage, and the bytes were proven unchanged | `8b4bb9f` |
| T5 | Concord succession (9 document + 125 article steps). `CanonSuccession` in the Concord section. **Schema 14 → 15**, with `sqlite::SCHEMA_VERSION` derived and six stray literal 14s removed. F0-20, F0-21, F0-22. Two defects fixed red-first (§6). **The one artifact rebuild** | `7d2973f` |
| T6 | Small Catechism chief parts numbered I–VI through `data/curated/concord-titles.toml`, applied in `concord::read_all` after the Smalcald splice. Fails loudly on an override that names no article | `375a0d0 → 5b2163c` |
| T7 | `NodeId<K>` wire form: serialized as the raw string, with the schema `{Kind}Id` per tag (F0-19). `PlaceDateClaim.event: Option<EventId>`, with six claims filled | `a72f8d9 → 9273647`, `15f4790 → caea0fc` |
| T8 | `NodeKind::Map`, ordinal 15, one map per era. `Shown` / `MapSuccession` families (`RowFamily::ALL` 28). `MapPass` (F0-11). One `DECLARED_NODE_KINDS = 16`. F0-3 applied: `present::focusable`/`display` deleted, which closes R48. Real corpus: 10 maps / 9 steps / 3,188 shown | `92a7e6c` |
| T10 (R44) | `decompress_verified` race fixed. Per-call tmp `sqlite.{pid}.{seq}.tmp`, the pre-delete of `dst` dropped, and a loser accepts the winner's file by SHA-256. `tests/blob_concurrency.rs` was red on the old code 6 of 6 times | `6eaba76 → c511e69` |
| review | Whole-branch review of `dd72809..7d2973f`: **0 Critical / 12 Important / 22 Minor** | none (read-only) |
| fix wave | Groups 1–12 plus two follow-ups; see `fix-wave-report.md`'s table | `e3cff40 cb2c297 ab6baf9 33fbcc2 621495c ccf6a88 05d60cb c5b9031 5dad231 01be6fe df5cef0 af64ae8 d1b5bd5 574fcd0` |
| group 9b | `atlas-graph/tests/common` (34 files; 57 path-derivation sites in 33 files → 0; `real_atlas()` for 23 memoised compiles; `OptionalCorpora` for the 7 raw builds). `--build-from-raw` now goes through `sibling_dir`. `.gitattributes` `eol=lf` for `sources.json` | `4161e6a`, `4bec5fd`, `49cd6c7` |
| T9 | Pins; pact `http.json` +24/−9 and `cli.json` +4; 22 AQC fixtures; 4 AGC fixtures via `contract-runner --bless`; versions moved; `blob_concurrency` added to atlas-graph's mutation targets | `98b7fd4`, `6dd48fd` |
| F0-26 | Playwright re-pin | `7b6b2c3` |

### RAW-INTEGRITY: a merkle manifest over `data/raw`

| task | what | SHA(s) on branch |
|---|---|---|
| T1 | `graph-types/src/raw_manifest.rs` (96 lines, pure): per-file SHA-256 leaves, name-ordered directory nodes, one 128-bit root (`node_hash` → `sha256_prefixed_128` directly, F-RI-1 accepted wrapper) | `f4b098c` |
| T2 | `Sha256([u8;32])` / `RawHash([u8;16])` newtypes with typed `HexError` (F-RI-4); the walker skips links and lists them. Real tree: **27,408 files / 365 MB, root `ca285021ab2a62274ca84741bd50629c`**. Release warm 2.3–2.8 s, cold 42.4 s once, stat-only 0.015 s, so the gate hashes fully | `9ca8b27 → d980985`, `5e854a8 → 154ef15` |
| T3 | **`bibex verify`** gains a raw section: exit 6 naming the file, with MISSING / EXTRA / TRUNCATED (both sizes) / MISMATCH (both hashes) / `LinkAdded` / `LinkRemoved`. An absent manifest reads `raw: unrecorded`. **`bibex raw bless`** prints the root, the previous root, and added/removed/changed entries by name. It refuses zero files (exit 5) and an unreadable previous manifest (exit 6). `data/raw/MANIFEST.toml` committed. Verify warm 9.27–10.1 s, bless 2.14 s, cold bless 99.5 s once | `d6742a7 → c94f2b4`, `dd18292 → c40a89a`, `64e2374 → fd2b2da` |
| T4 | **`bibex raw check <path>`** answers 0 (as recorded), 3 (unrecorded) or 6 (names every differing path). `fetch-raw.ps1` guards on that exit code instead of `Test-Path`, supports `-WhatIf`, and ends with `bibex verify`. Fixed the `morph\nt` bug (§6) | `a814c7a → 6a25411` |
| T6 | `scripts/archive-raw.ps1` + Pester 3.4.0 tests. Real archive `…\Documents\bible-atlas-backups\bible-atlas-data-raw-2026-09-29-raw-ca285021ab2a62274ca84741bd50629c.7z`, 30.7 MiB. The owner's `…2026-09-28-graph-11c50986.7z` was verified byte-identical before and after | `9e0c276 → e898daa` |
| T5 | `Manifest.raw_root: Option<RawHash>`, recorded as provenance **outside** the root's preimage (D1). `RawProvenance { Unrecorded, Unchanged, Changed }`. The manifest diff is exactly one added line, with root and `built` unchanged and all five blobs reused. `manifest_raw_provenance` 11/11 | `8a55e7a` |

### Lines by area (`git diff --numstat dd72809..7b6b2c3`)

186 files, +145,375 / −1,646 in total. Of the insertions, 137,990 are
`data/raw/MANIFEST.toml` (F-RI-6). Excluding `data/raw/MANIFEST.toml`,
`data/compiled`, `contracts`, `docs` and `.superpowers`, the diff is **139
files, +6,797 / −1,576**.

| area | files | + | − |
|---|---|---|---|
| `server/atlas-cli` | 12 | 2,620 | 46 |
| `server/atlas-graph` | 69 | 2,205 | 992 |
| `graph-types` | 19 | 869 | 252 |
| `scripts` | 2 | 327 | 0 |
| `server/atlas-contract` | 7 | 270 | 118 |
| `server/atlas-etl` | 14 | 261 | 71 |
| `data/fetch-raw.ps1` | 1 | 137 | 69 |
| `data/curated` | 4 | 65 | 0 |
| `server/atlas-core` | 2 | 20 | 1 |
| `server/atlas-server` | 1 | 3 | 5 |
| `contracts` (generated and blessed) | 36 | 171 | 50 |

---

## 4. Artifact and version moves

| item | before (`dd72809`) | after (`7b6b2c3`) |
|---|---|---|
| section schema | 14 | **15** (`contract.json`, `graph-vocabulary.json`, every section's `schema_version`) |
| version root (`EXPECTED_VERSION_HEX`) | `11c50986e095373f3e1bd6ac2218ed28` | **`ce9d653b597e6893a25a5264c60ff306`**, re-pinned once at Task 9. Recorded along the way and not pinned: `32ee…` (T6), `9201a71a…` (T3), `c5039368…` (T4) |
| compiled manifest root | `18c7546b435791ee9422be8f32fbb5fa` (built 2026-09-19T06:04:23Z) | **`31e118e8e6d47276ebeec3ed23d53217`** (built 2026-09-29T15:03:32Z). The first rebuild root, `085ae8f3…`, was never committed (F0-22) |
| section logicals | core `f9294873…`, kjv `abec1ca1…`, concord `3ef8508f…` | core `f0088b69…`, kjv `a97cc311…`, concord `1b6b2b4c…`. Kretzmann and lexicon logicals are unchanged; their blobs were re-stamped at schema 15 |
| `raw_root` | absent | **`ca285021ab2a62274ca84741bd50629c`** (27,408 files) |
| AQC | 0.9.0 | **0.10.0** (F0-15) |
| AGC | 0.13.0 | **0.14.0** |
| vocabulary | EdgeKind 46, directed relations 20, NodeKind 15, RowFamily 25 | 50, 22, 16, 28 |
| `data/raw/MANIFEST.toml` | none | 137,990 lines, 3,963,943 bytes |
| `contracts/` at Task 9 | none | 35 files, +101/−42 |

The five `.zst` blobs moved in git **once**, at Task 5 (F0-13/F0-16).

---

## 5. Every ruling

### F0-1…F0-26 (FOCUS-0 ledger)

| id | ruling | cost if wrong |
|---|---|---|
| F0-1 | Task 7 adds `event` to the domain struct `PlaceDateClaim` with `skip_serializing_if`, because no wire twin exists after R23. The CHANGELOG entry is additive | one field's home |
| F0-2 | R44's `decompress_verified` race is fixed in this batch as controller Task 10, red first | a fresh clone's first test run stays flaky |
| F0-3 | `present::focusable`/`display` deleted by Task 8 unless something calls them. Done: deleted | two dead functions |
| F0-4 | The parallel schedule is the batch's shape: at most two Rust implementers, with raw-integrity running beside them | none stated |
| F0-5 | Every `cargo test -p atlas-graph-types` runs as `(cd ../graph-types && cargo test --all-features)` | ten minutes lost to a refusal |
| F0-6 | (owner: "Two locations means we broke D.R.Y.") A declared count lives in exactly one test constant. Cross-crate pins become agreement laws | one more two-place bump |
| F0-7 | Task 6 regenerates the `contents` fixtures but moves no VERSION or CHANGELOG. All version moves wait for Task 9 | one intermediate commit the semver gate would refuse |
| F0-8 | Every concurrent implementer builds in its own `CARGO_TARGET_DIR` | one cold build each |
| F0-9 | One working tree has one editor of `graph-types`. Companions work in their own worktrees with data robocopied, never linked. The controller cherry-picks | one cold build and ~1 GB copied per companion |
| F0-10 | The semver leg is expected red from Task 1 to Task 9 and runs at Task 9 | intermediate commits the gate would refuse |
| F0-11 | `Map` is its own `Pass` after `IndexPass`. `GraphSceneSource::build` is the real constructor | one pass in the wrong slot |
| F0-12 | `PlaceDateClaim.event: Option<EventId>`, a typed id, never a `String` or `NodeRef` | one field type |
| F0-13 | Task 5's schema bump rebuilds the committed `data/compiled` artifact once, and Task 5 does the rebuild | a red real-data suite from T5 to T9 |
| F0-14 | Task 10 becomes a wave-4 companion. The pre-delete `remove_file(dst)` is the race window | one wave's serialization |
| F0-15 | Task 9 moves AQC 0.9.0 → 0.10.0 and AGC minor, once. The app must be down for Task 9 | one version number |
| F0-16 | The artifact is rebuilt once, at T5. Until then each task reports its exact red set, and the controller verifies it equals the task's own additions | a regression hidden until T5, bounded by the red-set review |
| F0-17 | `author_ids` is not a `BookMeta` field. It lives in `BookAuthorship` + `AtlasData.book_authorship` | one input struct relocated |
| F0-18 | 32 books, following the spec's 14 authors. The plan's "14 books" is corrected in the commit subject | none |
| F0-19 | `NodeId<K>` gains a wire form (raw id string, `type: string` schema named per tag), made in Task 7's worktree | one impl FOCUS-1 needed anyway |
| F0-20 | Corpus roots belong to their corpus sections. Exact-match arms go in the one split function | two red assertions at T5 |
| F0-21 | `Contains` derives `PartialEq, Eq` and tests compare values, not Debug strings | one derive |
| F0-22 | The place-claim event is persisted on the sidecar, red first. One more rebuild and one committed artifact move | two sidecar columns |
| F0-23 | `MapSuccession` is a third `Succession` row type (spec §7.6 says `CanonSuccession`). The wire is identical | one row family's name |
| F0-24 | Group 9's atlas-graph half (34 files) goes as its own dispatch before T9. The 7 raw builds are not merged. `main.rs` `sibling_dir` and `.gitattributes` are folded in | one mechanical commit to revert |
| F0-25 | The four `build_real_ctx` helpers are routed to CONTRACT-2's first atlas-graph dispatch. `--build-from-raw` gets a smoke run at T9 | none stated |
| F0-26 | `api-place-history.spec.ts:127` re-pinned to Task 7's served `event`. `world-hover-text.spec.ts:658` routed to FOCUS-1 as a flake | none stated |

Unnumbered rulings recorded in the ledger:

- **Task 7 data:** Jerusalem established → `sam2_jerusalem_captured`; destroyed → `exl_jerusalem`; Nineveh → `theo-87`; Samaria → `theo-176` / `2ki_fall_of_samaria`; Shiloh → `cq_shiloh` / none.
- **Controller practice:** cherry-picks land only on a clean main tree.

### F-RI-1…F-RI-9 (RAW-INTEGRITY ledger)

| id | ruling | cost if wrong |
|---|---|---|
| F-RI-1 | T5's "root is unchanged" test asserts root-with-`raw_root` = root-without, rather than a literal | a test that breaks on every legitimate root move |
| F-RI-2 | The archive defaults to `%USERPROFILE%\Documents\bible-atlas-backups\` and refuses to overwrite an existing root's archive | one default path |
| F-RI-3 | Runs beside FOCUS-0. T5 waits for FOCUS-0 wave 4 (shared `sqlite/manifest.rs`) | one merge conflict |
| F-RI-4 | `Sha256` / `RawHash` newtypes. Only the TOML boundary is text | two newtypes |
| F-RI-5 | The second hex codec in graph-types is routed to FOCUS-1 (one codec, one format pin) | two small codecs for one batch |
| F-RI-6 | The 3.8 MB `MANIFEST.toml` in git is accepted: per-file rows let verify name the file | 3.8 MB of history per refetch |
| F-RI-7 | Three restatements outside T2/T3's files are routed to T5 or FOCUS-1 | three restatements for one batch |
| F-RI-8 | No raw-only verify verb: one verifier; scripts parse `--json verify`'s `raw` member | a longer refusal message |
| F-RI-9 | A raw tree that moved since the build is reported by `verify` (`RawProvenance::Changed`), not failed | an operator who ignores a printed line |

Also recorded: a **CORRECTION to the record** in the RAW-INTEGRITY ledger (§6).

---

## 6. Incidents

- **The `data/raw` junction deletion (2026-09-28, before this batch).** A throwaway mutation-gate worktree junctioned nine `data/raw/*` datasets. `git worktree remove --force` followed the junctions and deleted through them: 408 MB → 34 MB, so 374 MB lost. The data is gitignored and there was no backup. At this batch's pre-flight, `data/raw` held 27,409 files / 349 MB. The owner had made his own copy, `…2026-09-28-graph-11c50986.7z` (59 MB, verified). RAW-INTEGRITY exists because of this incident. The tree is now recorded (root `ca285021…`), verifiable (`bibex verify`), checkable per subtree (`bibex raw check`) and archived off-worktree (30.7 MiB). The batch's companion worktrees were made with `git -c core.autocrlf=false worktree add`, with data robocopied and never linked, and a reparse-point scan returned 0 before every removal (F0-9).
- **The `morph\nt` bug.** RAW-INTEGRITY T4 found it. The committed `fetch-raw.ps1` held `'morph<LF>t'`, a literal line feed, so the path could never exist. Under `$ErrorActionPreference = Stop`, it killed the 2026-09-28 refetch twice at exactly the morph step, and the buffered output was lost. It was fixed in `6a25411`. **CORRECTION to the record:** the earlier inference, "detached processes killed at ~8 minutes", rested on those two deaths and does not stand. What is established: the harness reaps background shells under memory pressure, and a foreground call past its timeout moves to background and completes. The compounding defect was that existence guards would have accepted the half-copied `lexicon/` and empty `morph/` forever. `bibex raw check` now decides by hash.
- **Fable 429, fix wave.** The implementer hit the API rate limit (HTTP 429, Fable model) mid-group. It had committed and pushed `e3cff40` and `cb2c297`. Groups 3+5 were left uncommitted across 10 files, with the claim "194 lib tests and canon_real_data 4/4 green". **Recovery:** an Opus 5.5 continuation agent treated that claim as unverified. It re-read the diff hunk by hunk and ran a multiline Debug-string grep over 98 files, which caught one missed compare (`canon_vectors.rs::round_trip`). It re-ran graph-types 149/0 and atlas-graph 194 / 4 / 27 before committing `ab6baf9` and `33fbcc2`, then took groups 4 and 6–12.
- **Opus 500, Task 9.** Task 9 survived an API 500 mid-task and was resumed with its context. The ledger records nothing more about it. Task 9 then stopped before the push at the new Playwright failure (§2), and the push followed F0-26.

Defects that were in the code before this batch, surfaced by its tests:

- R44's race: 6/6 red on the old code. Under cold-cache `atlas-cli` it produced 38 signatures and a truncated served section (`kjv: database disk image is malformed`).
- Blob reuse ignored the schema version (T5).
- The unpack cache was keyed by logical hash only, so the writer would have renamed onto the file the running app holds. It is now keyed `<logical>.<schema>.sqlite` (T5).
- `sidecars.rs` never persisted Task 7's `event`; found by `extras_real_data` and fixed as F0-22.
- `law_check::indexes_derive_exactly_from_rows` never cloned the three new families (T4).
- A stale `reload_real_data` pin: 106,754 vs 106,742 (`574fcd0`).
- `the_etl_binaries`' first-run CRLF flake (`49cd6c7`).

---

## 7. Owed and routed

**Owed by this batch:**

- **Step 6b: R44's cold-cache "green first time" proof is NOT DONE.** The permission classifier refused deleting `data/cache/sections/*` ("Irreversible Local Destruction"), and the controller did not route around the refusal. The cache holds 14 files, 1.2 GB, 0 reparse points: 5 current `<logical>.15.sqlite` and 9 orphaned pre-schema-15 files. **The deletion is the owner's call.** After it: `cargo test -p atlas-cli -j 8` once. Standing evidence meanwhile, from T10: under cold cache the fixed code showed 0 R44 signatures and materialised all 5 sections, against 38 signatures on the old code.
- **Mutation gate deferred** by the owner's window (2026-09-29 07:27 → 2026-09-30 09:00). `MUTATION-GATE-DEBT.md`'s single row reads: last run CONTRACT-1; base for the next run `13111dd`; batches since *"FOCUS-0 (closed 2026-09-29, base `dd72809`), … (append as they land)"*. Command: `bash scripts/mutants-parallel.sh -n 4 --base 13111dd`, then Stryker. Task 9 added `--test blob_concurrency` to `atlas-graph/.cargo/mutants.toml` (R49).

**Routed to CONTRACT-2:**

- **F0-25:** the four `build_real_ctx` pipeline helpers (description, fulfillment, justified_by, peoples tests), as its first atlas-graph dispatch's opening step.
- **Review Important 10:** bare-`String` node ids served on `map.rs` `Era.id`, `wire::Polity.id` and the legacy `atlas_core::wire` shapes (spec §8; CONTRACT-2's D12 types them).
- **Task 9's concern:** the AGC `kretzmann-chapter-gen-1` fixture pins the version root, so it moves on every rebuild (CONTRACT-2 or FOCUS-1).
- **D16 and the words-as-base direction**, recorded in `.superpowers/sdd/2026-09-29-contract2-pushdown/progress.md`:
  - D16 ("Kretzmann citation scanning stays out") was withdrawn after the owner said "the fundamental units … cannot be verses, but words that compose into verses".
  - The Text-Fabric evaluation was declined.
  - **R-C2-W1:** the KJV English word layer is the base (790,892 words over 31,102 verses).
  - **R-C2-W2:** the wire addresses words.
  - **R-C2-W3:** Kretzmann anchors on word spans where they align (81.4 % OT).
  - **R-C2-W4:** the KJV is stored once, as tokens + gaps.
  - **D13 revised:** `kjv_token` in the Kjv section.
  - FOCUS-7 direction: Kretzmann stops being a special corpus.

**Routed to FOCUS-1** (types / R36 inventory unless noted):

- The `world-hover-text.spec.ts:658` flake (F0-26).
- F-RI-5, one hex codec with one format pin.
- F-RI-7's graph-types collapses: a `RawPath` newtype, the `"raw"`/`MANIFEST.toml` literals, `Display for HexError`.
- Review Important 11: `SectionReport.transport` / `logical_check` strings, and `ConcordTitleOverride.document`.
- Review Minor 9: the `place_history` columns in both `ddl.rs` and `sidecars.rs`.
- Review Minor 11: `compress_file`'s shared tmp and pre-delete.
- Review Minor 16: the unreachable root `MISMATCH` in `verify.rs`.
- Review Minor 20: the partial `section_of_family` / `section_of_justified_by`.
- `sections.rs:154`, which parses a relation by its Debug name.
- F0-23's possible generic `Succession<K>`.

**Carried and noted:**

- The F0-16 improvement for the next batch: a test-side `ATLAS_COMPILED_DIR` resolver.
- Gate 8's trend (§1).
- PRINCIPLES 10 non-AAA comments beyond the review's list (fix-wave concern 3).
- `archive-raw` does not surface 7z's stderr.
- A lone LICENSE drift is caught by verify but repaired by no block.
- Three tests pin toml 0.8.23's error text.
- `tests/cli.rs` is +8 s in debug.
- The 3 pre-existing Playwright failures stay in the owner queue.
- The orphaned pre-15 cache files are cleared with 6b.

---

## 8. Parallelism as measured

The plan's nine tasks plus Task 10 were scheduled into six waves instead of ten
(F0-4), then revised to five. Ceilings throughout (F0-4/R46): at most two FOCUS-0
Rust implementers at once, `-j` caps set by memory, and every companion in its own
worktree and target dir (F0-8/F0-9).

| wave (dispatch) | main tree (primary) | companions (own worktree / target) | beside it |
|---|---|---|---|
| 1 (~08:05) | FOCUS-0 **T1** (`server/target`) | none | RAW T1 (landed `f4b098c` first; T1's parent) |
| 2 (~08:40) | **T2** at `77bef67` | **T6** `C:\w\f0t6`, `C:/mut/t2` | RAW T2 walker `C:\w\ri2`, `C:/mut/t3` |
| 3 (~09:45) | **T8** at `154ef15`, `-j 6` | **T3** `C:\w\f0t3`, `C:/mut/t1`, `-j 4`; T6 still running | RAW T3 verifier `C:\w\ri2`. The ledger's count: "Four Rust builds + the owner's app" |
| 4 (~11:10) | **T4** at `fd2b2da`, `-j 8` | **T7** `C:\w\f0t7`, `C:/mut/t1`, `-j 6`; **T10** `C:\w\f0t10`, `C:/mut/t4`, `-j 4` | RAW T4 `C:\w\ri2`; RAW T6 `C:\w\ri6` (`C:/mut/t5`) |
| 4 tail | **T5** alone (waited for T7 so the rebuild carried its data) | none | none |
| after T5 | RAW **T5** | none | whole-branch review (read-only, `dd72809..7d2973f`) |
| fix wave | alone; interrupted by the 429, continued; app stopped by the controller from here on | none | none |
| 9b | alone, app down | none | none |
| T9 (close) | alone, app down, quiet machine | none | none |

Landing order within a wave: the primary's commit, then the companions'
cherry-picks (file-disjoint by construction), then a push. Wave 3's T3
cherry-pick conflicted in `canon_real_data.rs` / `sections_real_data.rs` and was
resolved by keeping both sides. Everything else applied clean. Combined builds
were checked after each wave; at `fd2b2da`: 0 warnings, lib 190, laws 25, and
graph_api 44/19 with the 19 exactly F0-16's set.

Retired worktrees: `f0t6`, `f0t3`, `f0t7`, `f0t10`, `ri2`, `ri6`. Each was clean,
its last commit was on main, and its reparse-point scan was 0; the `wip/`
branches were deleted. Four worktrees remain (master, this branch, field-guide,
overlay1-base). Disk: 311 GB free.

---

**Footnote:** how `data/raw` was brought from 34 MB back to the 27,409 files
present at pre-flight is not recorded in either ledger or any report read for this
document. Only its state at 08:05 and the owner's 2026-09-28 archive are on record.
