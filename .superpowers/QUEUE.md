# QUEUE: the one list of work

Protocol: `AGENTS.md`. Plan: `docs/superpowers/plans/2026-09-30-v1-roadmap.md`.

- **Statuses:** `proposed` · `ready` · `claimed:<agent>:<time>` · `blocked:owner` · `blocked:<item>` · `review:<commit range>` · `done`
- **Lanes:** A = Claude · B = maps (Codex) · C = analyses (Codex) · D = infrastructure (Codex) · F = FOCUS batches (whoever the item names)
- **Repos:** `atlas` = this repo · `mapgen` = `map-generator`. On the owner's machine they are siblings: `~/src/bible-atlas` and `~/src/map-generator`.

## STATUS (the controller rewrites this at every landing)
- **As of** 2026-09-30 morning. CONTRACT-2 is closing: Tasks 1–6, 7a, 8a, 9, 10a, the ETL landing and the one rebuild (schema 16, root `8d8dd1d0…`) are on `worktree-bible-atlas-m1`. Running: 7b+8b (anchors on text windows). Left: 10b, then 11 (the close).
- **Mutation:** owed from base `13111dd` (`.superpowers/MUTATION-GATE-DEBT.md`); the roadmap defers the run to Nov 5 – Dec 31.
- **Next for Claude:** A-C2 close → A-F1 (FOCUS-1), with A-NAMES alongside.
- **Next for Codex:** CX-M0 → CX-M1 and CX-M2 (maps) · CX-R1 · CX-R2 · CX-R3 · CX-I1 (to the B2 step).
- **Before "go":** O-CODEX (`codex login`).

## OWNER QUESTIONS (answer in one line each; agents append)
1. **O-PUSH:** resolved 2026-09-30: `gh auth login` + `gh auth setup-git` in WSL; a push and a lock take/release from WSL both succeeded.
2. **O-CODEX:** Codex CLI 0.159.2 is installed in WSL (`~/.local/bin/codex`, works from any shell). Run `codex login` once. It runs in `~/src` on this machine.
3. **O-PASTOR:** Pastor Hromowyck demo: Oct 21 or 22?
4. **O-NAMES:** Sign off the NAMES draft's types (`docs/superpowers/specs/2026-10-01-names-design.md`) and answer its §9 (5 one-line questions). It is drafted as ONE spec for the text and the maps' entity registry.
5. **O-ORDER:** Can FOCUS-6 move up to right after FOCUS-1? Can FOCUS-4+5 and FOCUS-7+8 each close as a single batch?
6. **O-ERRATA-SEED:** List the map errors you've already noticed (map, place or polity, what's wrong). They seed CX-M2.
7. **O-STYLE:** After CX-M1's gallery, which style (parchment / canaan / slate), and which eras get a finished map (all 10 atlas eras?)
8. **O-GPL:** Redraw the GPL `historical-basemaps` world borders as our own CC0 work (recommended), or accept GPL? Swap the OSM rivers (ODbL) for Natural Earth (recommended)? Your licensing rule (2026-09-29) disqualifies both as they stand.
9. **O-PDF:** `Saltwater (Notation and Tab).pdf` in map-generator: remove it from HEAD (recommended), and also purge it from history? Is the repo public?
10. **O-B2:** When you're ready, create a Backblaze B2 account with Object Lock and put the restic password in your password manager. CX-I1 is prepared up to that step. Its local repository goes on `C:` (120 GB free, 92% full) unless you name an external drive.
11. **O-EXPLORER:** Map-generator's `codebase-explorer` branch ("The Atlas Engine", 29 commits past `origin/master`, which include all 27 of `stage1-overnight-2026-09-07`): merge it, or park it?
12. **O-MG-LOCAL:** Your Windows copy of map-generator (`Documents/the-best-maps-ever`) has 2 commits on `master` that were never pushed: `91d089c` "A source travels with its terms…" and `089a4bf` "RCA: why 6e349a3 broke the map…". Push them as they are, hand them to CX-M0 for review, or drop them?
13. **O-CATECHISM:** Post the license request drafted at `.superpowers/sdd/queue-name-model/catechism-license-request.md` on github.com/brain-fuel/catechism. The fallback is our own mapping from the public-domain 1921 Triglot.
14. **O-CACHE:** FOCUS-0 owes one proof (R44): clear `data/cache/sections/*` and run `cargo test -p atlas-cli` green first time. OK to clear the cache (regenerable) in `~/src/bible-atlas`?

---

## Lane A: Claude

### A-C2: CONTRACT-2 close
- **Status:** claimed:claude:2026-09-29
- **Tracked in:** `.superpowers/sdd/2026-09-29-contract2-pushdown/progress.md` (every ruling).
- **Left:** 7b+8b (running), 10b (client reads served anchors; `PlaceMentions`/`ScriptureRefScan` retire for the Concord), 11 (strip the 290 comment lines CONTRACT-2 added; one regeneration of fixtures and pacts; AQC 0.10.0 → 0.11.0, AGC minor; standing block; gates; Playwright in WSL).
- **Done when:** Task 11's gates are green, the push is done, the close report is written, and `MUTATION-GATE-DEBT.md` has CONTRACT-2 appended.

### A-F1: FOCUS-1
- **Status:** blocked:A-C2
- **Plan:** `docs/superpowers/plans/2026-09-27-focus1-types.md` (9 tasks), plus rulings R11–R14 in the FOCUS spec.
- **Note:** build no interaction that works only by hovering.
- **Done when:** the plan's Task 9 gates pass and it's reviewed by Codex.

### A-NAMES: the NAMES spec (one registry of names and entities, for text and maps)
- **Status:** blocked:owner (O-NAMES). A types-only draft is at `docs/superpowers/specs/2026-10-01-names-design.md`, with 5 open questions in its §9.
- **Rulings to build on (owner, 2026-09-29):**
  - A **name** is its own node, separate from the **entities** it can denote, and one name can denote entities of different kinds ("Israel": Jacob / the Israelites / the kingdom / the land; "Egypt", "Assyria", "Babylon" city vs empire, "Edom"/Esau). Entities that share a name are linked to each other (eponym: person → people → polity → land).
  - Every **occurrence** (a span of words in one text; multi-word and hyphenated names are one name) resolves to exactly one meaning, or is listed unresolved. Never guessed.
  - The occurrence anchors to the **specific original-language word** it translates (its own book/chapter/verse/layer/position, carrying a Strong's number), never to the Strong's number alone: one lemma covers every sense (H3478).
  - **Each text keeps its own verse numbering**; moving between numberings goes through an explicit versification map.
  - **Each translation reaches the original words only through its own alignment table**, never by position. No alignment → unresolved and listed.
  - The brain-fuel/STEPBible Hebrew and Greek arrive **already renumbered to KJV verses** (JOE 2 has 32 verses, MAL 4 exists, PSA 3 has 8). Record that as provenance; keep the original numbering too.
  - **Titles are not names but are mapped and explorable** (a title layer resolves each occurrence to its entity). **Pronouns/coreference: out of scope.**
  - **Resolution is by rules, plus curation** for what the rules refuse. Theologically loaded titles (Son of man, Angel of the LORD, the Word) go on an owner review list before they're served.
- **Input:** `.superpowers/analysis/name-sources.md` (licenses and candidates: BibleForgeDB, TIPNR, TVTMS, BibleData); map-generator's `map_canon::Registry` and `docs/superpowers/specs/2026-09-06-map-api-contract-design.md` §2; CX-R3's report; the 6,187 unlocatable mentions from CONTRACT-2 Task 7a (its worklist).
- **Supersedes:** CONTRACT-2 Task 7a's search-name scan, `event_world::kjv_aliases_of`, NAME-1's dated renames (a name's time window becomes a property of the name).
- **Deliverable:** `docs/superpowers/specs/2026-10-0x-names-design.md`, every type shown (PRINCIPLES 12). OWNER QUESTIONS carries the sign-off.

### A-MAPS-SPEC: the MAPS migration spec
- **Status:** blocked:A-NAMES
- **Covers:**
  - map-generator's crates joining this workspace; `map-types` becoming additive `graph-types` extensions (the upstreamability law)
  - a new `maps` section in the compiled artifact (following LEX-1's precedent: the other sections' hashes stay unchanged)
  - the map API under the same generated OpenAPI document
  - the World view as the Geography renderer (FOCUS-6)
  - what's deleted: the atlas's polities, the Esri tiles, the border drawing in `map.js`, `PolityDelta`, the cross-repo exports, pins and CDC suite
  - how the errata register carries over

### A-DATA-SPEC: backups and the artifact store
- **Status:** ready (small)
- **Covers:**
  - restic to Backblaze B2 with Object Lock, plus a local drive
  - the artifact store at `sections/<sha256>` (the manifest's `blob` hashes)
  - `bibex fetch`
  - read-only keys for agents
  - compiled data leaving git after Nov 4
  - a restore rehearsal
- **Unblocks:** CX-I2.

### A-FPLANS: write each FOCUS-n plan from FOCUS-1's actual result
- **Status:** blocked:A-F1
- **Order:** FOCUS-6 first if O-ORDER allows, then 2, 3, 4+5, 7+8, 9.
- **Assignment:** each plan names its agent, its base commit and its files. Codex takes the ones that pair against Claude's (server-heavy beside client-heavy).

### A-F3, A-F6, A-F9: FOCUS-3, FOCUS-6 (with the map switch), FOCUS-9
- **Status:** blocked:A-FPLANS (FOCUS-6's map half is also blocked on the MAPS migration)

### A-REVIEW: review every Codex item in `review`
- **Status:** standing
- **Scope:** the PRINCIPLES 14b pass, including a grep of the diff for added comment lines. Land what passes, holding the `land` lock.

### A-THEO: replace Theographic (CC BY-SA)
- **Status:** proposed (ruled 2026-09-29: replace in a queued batch; credit it until then)
- **Scope:** Theographic supplies 450 of 552 events, part of places, and the Easton text bundle. Replace from BibleData (CC BY: persons, events, Ussher dates), STEPBible TIPNR (CC BY), Easton re-sourced from its 1897 public-domain text, and our own curation. Needs a spec (types, what each source supplies, the id mapping so no node id churns without a recorded reason).

### A-STRIP: remove every comment from application code
- **Status:** proposed (owner, 2026-09-30: "no comments in my app code stop doing that"; PRINCIPLES 9)
- **Scope:** everything already in `server/`, `client/` and `graph-types/` before CONTRACT-2 (the close strips CONTRACT-2's own 290 lines). Wire descriptions the OpenAPI document needs become `#[schema(description)]`. One crate per commit; `export_contract --check` shows what the document loses.

### A-BACKLOG: routed items waiting for their batch
- **Status:** proposed; each moves into the batch named, or is planned on its own.
- FOCUS-1 R36 (typed ids): `BookId(pub u8)` with a panicking `code()`, `TranslationId(String)`, `PlaceDateClaim.verses` as strings, `SectionReport` strings, `ConcordTitleOverride.document`.
- A place date read from two served sources (card + place-page verse refs) → T10b / FOCUS.
- The card details restate the legacy structs' fields until FOCUS retires the legacy routes.
- `atlas-cli/tests/raw_walk.rs`: the link-refusal tests are Windows-only (junctions); on Linux the rule that protects `data/raw` is untested. Add symlink twins.
- An on-disk real-atlas cache keyed by the raw root (the `ATLAS_COMPILED_DIR` resolver), so nextest stops recompiling the atlas per test process (17 min / 240 CPU-min vs `cargo test` 26 min / 43).
- "One AQC harness, not two" (owner, 2026-09-28 discussion).
- The owner's 2026-09-15 directives (`owner-queue-2026-09-15` memory): the one-scripture-selector law in follow mode, corpus × corpus split windows, Small Catechism/BoC explorability and containers, the tree table of contents, person cards. Re-check against the app; plan what's still open (the ToC and BoC nav are now FOCUS-3's R11–R13).

---

## Lane B: maps (Codex, repo `mapgen` unless noted)

### CX-M0: map-generator hygiene
- **Status:** claimed:codex:2026-09-30T12:33:21+00:00
- **Repo / base:** `mapgen`, `origin/master` = `6608db4`. (Your Windows copy's `master` also has 2 unpushed commits, O-MG-LOCAL; work from `6608db4`, not from them.)
- **Worktree:** `git -C ~/src/map-generator worktree add -b lane/codex/CX-M0 ~/w/mg-CX-M0 6608db4`
- **Files:** `crates/*/Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates/map-types/tests/toolchain_pin.rs`, `scripts/demo.sh`, the PDF.
- **Steps:**
  1. `git merge --ff-only origin/stage1-overnight-2026-09-07` (verified: `6608db4` is its ancestor; 27 commits).
  2. `git cherry-pick 3d6a3c6` (TOOLCHAIN-1, from `codebase-explorer`: pins rustc 1.97.1 like the atlas). Leave the rest of `codebase-explorer` for O-EXPLORER.
  3. All seven crates that depend on `atlas-graph-types` use `path = "../../../../scratch/bible-atlas-sketch/.claude/worktrees/bible-atlas-m1/graph-types"`, which only resolves on Windows (verified: `cargo build` in WSL fails, "No such file or directory"). Replace it with `git = "https://github.com/donovansmith44/your-word-is-truth-interactive-atlas", rev = "<sha>"` (one declaration in the workspace `Cargo.toml`, `workspace = true` in the crates, if the workspace allows it).
     - Try the atlas's current `worktree-bible-atlas-m1` first.
     - If `map-types` doesn't compile against it, **do not edit the types** (the map covenant: consult first). Pin the newest atlas rev that compiles, and write the breakage list into this item as input for A-MAPS-SPEC.
  4. `scripts/demo.sh` only works on Windows (`map-viewer.exe` via PowerShell). Make `start`/`stop` work on Linux too: the binary without `.exe`, started in the background with its PID written to `out/demo.pid`, stopped by that PID. `make demo` then serves on 8090 in WSL.
  5. `git rm "Saltwater (Notation and Tab).pdf"` (HEAD only; history is O-PDF).
- **Gates (in WSL, after `. ~/.bible-atlas-env`):** `CARGO_TARGET_DIR=~/mut/codex-CX-M0 nice -n 10 cargo test --workspace -j 4`; `make demo` then `curl -s localhost:8090/api/meta` answers; `make contract-gates` (GHC 9.6.7 + cabal are installed via ghcup); `make stop`.
- **Done when:** all of the above, pushed to `lane/codex/CX-M0`, reviewed. The controller then fast-forwards `master`.

- **Execution notes:** Queue read from origin/prep/v1-go until prep lands. Workspace Cargo.toml is explicitly authorized by step 3. Owner priority: audit 1446, 1406 and 1200 BC first; seed concerns are oversized Edom and Davidic-era content at 1406 BC. CX-R3/R2 await a concrete CONTRACT-2 close base.

### CX-M1: a map gallery for choosing a style
- **Status:** blocked:CX-M0
- **Base:** CX-M0's landed `master`. **Files:** none tracked; output in `out/gallery/` (untracked).
- **Steps:**
  1. `make demo` (8090).
  2. Render the canonical Bible map set with `scripts/make-maps.sh` once per style. The script picks `parchment` by name (`grep '"name":"parchment"'`); take the style name as an argument (default `parchment`) so `canaan` and `slate` render too, and write each style to `out/maps/<style>/`. That script change is this item's one tracked edit.
  3. Render one map per atlas era, at each era's midpoint year, from `~/src/bible-atlas/data/curated/eras.toml`, in `parchment`.
  4. Write `out/gallery/index.html`: one contact sheet, maps grouped by era, styles side by side, each image captioned with its query.
  5. Copy it to `/mnt/c/Users/donov/Documents/map-gallery/` so the owner can open it from Windows.
- **Done when:** the gallery exists, the script change is pushed to `lane/codex/CX-M1`, and O-STYLE is asked with the gallery's Windows path.

### CX-M2: the errata register
- **Status:** blocked:CX-M0
- **Repo / base:** `mapgen`, CX-M0's landed `master`. **Worktree:** `~/w/mg-CX-M2`. **Files:** `docs/errata/**` (new).
- **Inputs:** the renders from CX-M1 (`out/gallery/`); the atlas's facts at `~/src/bible-atlas/data/curated/{eras.toml,polities/,place-names-kjv.toml,place-history.toml}` and its gazetteer as exported to `~/src/map-generator/data/atlas-exports/`; the KJV at `~/src/bible-atlas/data/raw/kjv.json`.
- **Rule:** **find, don't fix.**
- **Steps:**
  1. Seed the register from O-ERRATA-SEED.
  2. For every era map from CX-M1 (and every canonical map), audit each of the following and record a finding for each problem:
     - polity extents against their reign windows (the atlas's `polities` and eras, and map-generator's standings)
     - anachronisms: anything shown outside its time
     - boundaries against the Scripture boundary texts (Num 34; Josh 13–19; 1 Ki 4:21, 24; 2 Ki 14:25; Ezek 47–48; Luke 3:1)
     - place positions against OpenBible geocoding (the atlas's gazetteer)
     - labels: KJV spelling, the right name for the era
     - water, coasts and rivers
     - gaps and overlaps between neighbours
     - entities Scripture names at that time that are missing from the map
- **Record format** (one file per era, plus `docs/errata/README.md` with the format and a summary table). Fields per entry:
  - `id`
  - `map / era / year`
  - `what is wrong`
  - `evidence`: verse refs, atlas fact ids, or source
  - `severity`: wrong / misrendered-uncertainty / cosmetic
  - `probable cause`: data or pipeline
  - `proposed fix`
  - `status`: open / ruled:<owner's words> / fixed:<sha> / wontfix
- **Done when:** every era has been audited and the summary table is complete. Owner rulings are asked in batches of about 10, most severe first.

### CX-M3: fix the ruled errata
- **Status:** blocked:CX-M2 (starts as soon as the first entries are ruled). Paused during the MAPS migration itself.
- **Each fix, test first:**
  1. A law or golden stop that fails on the error.
  2. The fix: curated data with its justification (Scripture grounds first), or a pipeline fix. Never a tuned constant.
  3. The census diff reviewed; the golden gate green. A re-bless happens only with owner approval, recorded in the entry.
- One errata id per commit. The register entry gets `fixed:<sha>`.

### CX-M4: copyright data replacements
- **Status:** blocked:owner (O-GPL)
- **Steps:**
  1. Replace the OSM rivers with Natural Earth's `ne_10m_rivers_lake_centerlines`, via a new adapter test.
  2. Inventory every use of `historical-basemaps`: which maps and eras depend on it.
  3. Propose a CC0 replacement, e.g. the atlas's 14 CC0 polities plus Natural Earth coasts, with the regions it doesn't cover listed.
- The replacement is implemented after the owner rules on the proposal.

### CX-MIG: the heavy lifting of the MAPS migration
- **Status:** blocked:A-MAPS-SPEC
- **Deliverable:** the tasks the MAPS plan assigns to Codex.

---

## Lane C: analyses (Codex, read-only on `atlas`; outputs go to `.superpowers/analysis/`)

### CX-R3: prove the KJV → original-word alignment join
- **Status:** ready
- **Base:** the atlas at `@CLOSE@` (CONTRACT-2's close; `kjv_token` is in the tracked artifact's Kjv section: 790,892 words).
- **Background:** `.superpowers/analysis/name-sources.md` already chose **BibleForgeDB** (public domain / CC0): it aligns 763,778 KJV words to specific Hebrew/Greek tokens. Its KJV edition differs from our `data/raw/kjv.json` in 452 verses; ~17.9k of its rows are flagged ERROR; its Greek needs STEPBible TAGNT's Textus Receptus.
- **Question:** how much of OUR KJV word layer can reach a specific original token through BibleForgeDB + STEPBible TAHOT/TAGNT, and what's left?
- **Steps:** a spike in `~/w/CX-R3` scratch space (nothing committed but the report): download BibleForgeDB and TAHOT/TAGNT into scratch; join our `kjv_token` rows (read with `sqlite3` or a small script from the unpacked Kjv section, `data/cache/sections/`) to BibleForge's words by an explicit per-verse diff, never by position; then BibleForge's original tokens to TAHOT/TAGNT tokens. Report: the percentage of our words aligned; the unaligned by kind (italic/supplied, ERROR rows, the 452 differing verses, Greek TR gaps); 20 worked examples including Gen 32:28 "Israel"; the licence text of every file used.
- **Done when:** `.superpowers/analysis/kjv-alignment-join.md` is pushed on `lane/codex/CX-R3` and reviewed. It feeds A-NAMES.

### CX-R1: the 65 overlapping-event cases
- **Status:** ready (they surfaced when CONTRACT-2 removed the 20-verse cap on attesting verses; landed at `68fb00f`)
- **Base:** `@CLOSE@`. **Source:** `server/atlas-graph/src/attestation_pending.rs`, whose `PENDING` inventory grew 824 → 889 pairs when the cap came off: 65 new collisions, most of them Robertson passion-week events contained in `theo-443`/`theo-448`. `git diff 68fb00f~1 68fb00f -- server/atlas-graph/src/attestation_pending.rs` lists exactly the new pairs.
- **Deliverable:** a curation sheet, one row per case: the events (id and title), their verses, the kind of overlap (containment / overlap), and a proposed resolution with its Scripture grounds, each answerable by the owner in one line.
- **Output:** `.superpowers/analysis/overlapping-events.md` on `lane/codex/CX-R1`. Read-only on everything else.

### CX-R2: Easton doctrinal review list (for the Pastor)
- **Status:** ready
- **Base:** `@CLOSE@`. **Source:** `data/raw/theographic/theographic-bible-metadata-master/json/easton.json` (Easton's Bible Dictionary, 1897, Presbyterian; it supplies the `description` of places, people and people groups). Which entities use which entry: `server/atlas-etl` (grep `easton`).
- **Deliverable:** flag every entry that touches doctrine: baptism, the Lord's Supper, election/predestination, conversion, the law and the gospel, the church and ministry, the end times, and the like. Quote the passage and say why it may conflict with Lutheran teaching. No judgment beyond flagging.
- **Output:** `.superpowers/analysis/easton-doctrinal-review.md` on `lane/codex/CX-R2`, sorted by how prominent the entity is in the app (its node's edge count from `bibex`/the artifact).
- **Note:** Easton is re-sourced from its public-domain original when A-THEO replaces Theographic; the review carries over (same text).

---

## Lane D: infrastructure (Codex)

### CX-I1: backups
- **Status:** ready up to the B2 step, then blocked:owner (O-B2)
- **Base:** `@CLOSE@`. **Files:** `scripts/backup/**` (new) and its tests, `scripts/backup/test/*.bats` (bats 1.14 is at `~/.local/bin/bats`).
- **Tools (installed):** `restic` 0.19.1, `jq` 1.8.2 in `~/.local/bin`; `bibex` is `cargo run --release -p atlas-cli --` from `server/`.
- **Deliverable:** `scripts/backup/backup.sh` taking restic snapshots of:
  - `data/raw`, tagged `raw:<root>` from `bibex --json verify --data-dir ../data/compiled` (`.raw.root`)
  - `data/curated`
  - `data/compiled`, tagged `compiled:<root>` from the same JSON (`.root`)
  - git bundles of both repos (`git bundle create --all`)
- **Destinations:** a local restic repository first, at `/mnt/c/Users/donov/Backups/bible-atlas-restic` (C: has ~120 GB free; O-B2 may name an external drive instead), then B2 once O-B2 is done. The password comes from `RESTIC_PASSWORD_FILE`, never from the repo.
- **Also:** `scripts/backup/RESTORE.md`, a restore procedure tried once against the local repository (restore to a scratch dir, `bibex verify` it, roots match).
- **Done when:** the bats tests pass, one real snapshot and one restore are recorded in the item, pushed to `lane/codex/CX-I1`, reviewed.

### CX-I2: `bibex fetch` and the artifact store
- **Status:** blocked:A-DATA-SPEC

### CX-PW: the 3 known Playwright failures and the `world-hover-text` flake
- **Status:** blocked:A-F1 (FOCUS-1 retargets the popover the flake lives in)
- **Failing specs:** `world-quiet-places.spec.ts:211`, `split-view.spec.ts:293`, `reader-xref-anchoring.spec.ts:201`, and `world-hover-text.spec.ts:658` (the flake)

---

## Lane F: FOCUS batches assigned to Codex
Created by A-FPLANS, e.g. `CX-F2`, `CX-F45`, `CX-F78`, each with its plan, base commit and files.

## Deferred (after Nov 4 unless the owner says otherwise)
- **The words-as-base inversion** (owner, 2026-09-29, deferred): verses composed from their words with no stored verse text (R-C2-W4), red letter as word spans (OPEN-3), Kretzmann's comments anchored on the exact words they quote (T8c; 81.4% of OT phrases align), Concord stored as words only (OPEN-7). The KJV and Concord token layers and `TextPoint { unit, word }` already landed in CONTRACT-2.
- **Kretzmann as generic "text anchored on text"** (FOCUS-7's direction): commentary is itself tokenized text whose units anchor to word spans, resolvable through original tokens to any aligned translation. A second translation needs an owner ruling under the KJV directive.
- **eBible's "Or, …" marginal alternates:** never ingested without an owner ruling (KJV directive).
- **Alias coverage:** 6,187 name occurrences CONTRACT-2 couldn't place (KJV spellings missing from search names; people named by title). Becomes A-NAMES' worklist.
- Map-generator Stages 2–4 (continued inside the atlas)
- The mutation run (owed from `13111dd`)
- Mobile
- Compiled data leaving git
- The production pipeline and the user-data policy
- Choosing code and content licenses (neither repo licenses its own code yet)
