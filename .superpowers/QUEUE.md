# QUEUE: the one list of work

Protocol: `AGENTS.md`. Plan: `docs/superpowers/plans/2026-09-30-v1-roadmap.md`.

- **Statuses:** `proposed` · `ready` · `claimed:<agent>:<time>` · `blocked:owner` · `blocked:<item>` · `review:<commit range>` · `done`
- **Lanes:** A = Claude · B = maps (Codex) · C = analyses (Codex) · D = infrastructure (Codex) · F = FOCUS batches (whoever the item names)
- **Repos:** `atlas` = this repo · `mapgen` = `map-generator`. On the owner's machine they are siblings: `~/src/bible-atlas` and `~/src/map-generator`.

## STATUS (the controller rewrites this at every landing)
- **As of** 2026-09-30 (prep). CONTRACT-2 is in its close. The mutation run is owed from base `13111dd`.
- **Next for Claude:** A-C2 close, then A-F1 (FOCUS-1).
- **Next for Codex:** CX-M0, then CX-M1 and CX-M2 (the maps) · CX-R3 · CX-I1.

## OWNER QUESTIONS (answer in one line each; agents append)
1. **O-CODEX:** Install and log in to the Codex CLI in WSL. Will Codex run in `~/src` on this machine (recommended) or in the cloud?
2. **O-PW:** Has `cd ~/src/bible-atlas/tests/ux && sudo npx playwright install-deps chromium` been run?
3. **O-PASTOR:** Pastor Hromowyck demo: Oct 21 or 22?
4. **O-NAMES:** Design names and entities as ONE spec covering both the text and the maps' entity registry? (Recommended: yes.)
5. **O-ORDER:** Can FOCUS-6 move up to right after FOCUS-1? Can FOCUS-4+5 and FOCUS-7+8 each close as a single batch?
6. **O-ERRATA-SEED:** List the map errors you've already noticed (map, place or polity, what's wrong). They seed CX-M2.
7. **O-STYLE:** After CX-M1's gallery, which style (parchment / canaan / slate), and which eras get a finished map (all 10 atlas eras?)
8. **O-GPL:** Redraw the GPL `historical-basemaps` world borders as our own CC0 work (recommended), or accept GPL? Swap the OSM rivers for Natural Earth (recommended)?
9. **O-PDF:** `Saltwater (Notation and Tab).pdf` in map-generator: remove it from HEAD (recommended), and also purge it from history? Is the repo public?
10. **O-B2:** When you're ready, create a Backblaze B2 account with Object Lock and put the restic password in your password manager. CX-I1 is prepared up to that step.
11. **O-EXPLORER:** Map-generator's `codebase-explorer` branch ("The Atlas Engine", 29 commits, which include all of `stage1-overnight`): merge it, or park it?

---

## Lane A: Claude

### A-C2: CONTRACT-2 close
- **Status:** claimed:claude (in progress on 09-29)
- **Where it's tracked:** the ledger `.superpowers/sdd/2026-09-29-contract2-pushdown/`
- **Done when:** Task 11's gates are green, the push is done, the close report is written, and `MUTATION-GATE-DEBT.md` has CONTRACT-2 appended.

### A-F1: FOCUS-1
- **Status:** blocked:A-C2
- **Plan:** `docs/superpowers/plans/2026-09-27-focus1-types.md` (9 tasks), plus rulings R11–R14 in the FOCUS spec.
- **Note:** build no interaction that works only by hovering.
- **Done when:** the plan's Task 9 gates pass and it's reviewed by Codex.

### A-NAMES: the NAMES spec (one registry of names and entities, for text and maps)
- **Status:** ready (can be written alongside A-F1; no code)
- **Input:**
  - the roadmap's NAMES section
  - the name-model items and rulings in the controller's local queue, folded in here
  - map-generator's `map_canon::Registry` and `docs/superpowers/specs/2026-09-06-map-api-contract-design.md` §2
  - CX-R3's report
- **Deliverable:** `docs/superpowers/specs/2026-10-0x-names-design.md`, with every type shown (PRINCIPLES 12). OWNER QUESTIONS carries the sign-off.

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
- **Scope:** the PRINCIPLES 14b pass. Land what passes, holding the `land` lock.

---

## Lane B: maps (Codex, repo `mapgen` unless noted)

### CX-M0: map-generator hygiene
- **Status:** ready
- **Base:** `origin/master` (`6608db4`)
- **Files:** `crates/*/Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates/map-types/tests/toolchain_pin.rs`, the PDF
- **Steps:**
  1. Fast-forward to `origin/stage1-overnight-2026-09-07` (master is its ancestor; 27 commits).
  2. Cherry-pick `3d6a3c6` (TOOLCHAIN-1: pins rustc 1.97.1 in step with the atlas). Leave the rest of `codebase-explorer` for O-EXPLORER.
  3. Replace the `atlas-graph-types` path dependency (`../../../../scratch/bible-atlas-sketch/...`) in all seven crates that name it with `git = "https://github.com/donovansmith44/your-word-is-truth-interactive-atlas", rev = "<sha>"`.
     - Try the atlas's current HEAD first.
     - If `map-types` doesn't compile against it, **do not edit the types** (the map covenant: consult first). Pin the newest atlas rev that compiles, and write the breakage list into the item as input for A-MAPS-SPEC.
  4. `git rm "Saltwater (Notation and Tab).pdf"` (HEAD only; history is O-PDF).
- **Gates:**
  - `cargo test --workspace`
  - `make contract-gates` (if `cabal`/GHC are missing in WSL, record that and skip only that part)
- **Done when:** all of the above, pushed to `lane/codex/CX-M0` and reviewed. The controller then fast-forwards `master`.

### CX-M1: a map gallery for choosing a style
- **Status:** blocked:CX-M0
- **Files:** none tracked. The output goes to `out/gallery/`, which is untracked.
- **Steps:**
  1. Start the workbench (`make demo`, port 8090).
  2. Render the canonical Bible map set (`scripts/make-maps.sh`) in each of `templates/{parchment,canaan,slate}.ron`.
  3. Render one map per atlas era (`~/src/bible-atlas/data/curated/eras.toml`, at each era's midpoint year) in the default style.
  4. Write `out/gallery/index.html`: one contact sheet, maps grouped by era, with styles side by side, each image captioned with its query.
  5. Copy it to `/mnt/c/Users/<owner>/Documents/map-gallery/` so the owner can open it in Windows.
- **Done when:** the gallery exists and O-STYLE is asked with its path.

### CX-M2: the errata register
- **Status:** blocked:CX-M0
- **Files:** `docs/errata/**` (new)
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

### CX-R3: source for aligning the KJV's English words to the original words
- **Status:** ready
- **Question:** which public-domain KJV texts tag English words with Strong's numbers (e.g. eBible.org's KJV) could supply the per-translation alignment table NAMES needs?
- **For each candidate, report:**
  - license
  - versification differences against our KJV
  - token-level coverage against CONTRACT-2's KJV word layer (`kjv_token`): the percentage of words aligned, and the unaligned cases by kind
  - italics / supplied-word marking
  - how it would reach the specific original word (not just the lemma) through STEPBible TAHOT/TAGNT
- **Rules:** a spike in scratch space; nothing committed except the report.
- **Done when:** `.superpowers/analysis/kjv-alignment-sources.md` is written, feeding A-NAMES.

### CX-R1: the 65 overlapping-event cases
- **Status:** blocked:A-C2 (they surfaced when the 20-verse attests cap was removed)
- **Deliverable:** a curation sheet, one row per case: the events, their verses, the overlap, and a proposed resolution with its Scripture grounds, written so the owner can rule on each in one line.
- **Output:** `.superpowers/analysis/overlapping-events.md`

### CX-R2: Easton doctrinal review list (for the Pastor)
- **Status:** ready
- **Context:** Easton's Bible Dictionary (1897, Presbyterian) supplies the `description` of places, people and people groups.
- **Deliverable:** flag every entry that touches doctrine: baptism, the Lord's Supper, election/predestination, conversion, the law and the gospel, the church and ministry, the end times, and the like. Quote the passage and say why it may conflict with Lutheran teaching.
- **Rules:** make no judgment beyond flagging.
- **Output:** `.superpowers/analysis/easton-doctrinal-review.md`, sorted by how prominent the entity is in the app.

---

## Lane D: infrastructure (Codex)

### CX-I1: backups
- **Status:** ready up to the B2 step, then blocked:owner (O-B2)
- **Files:** `scripts/backup/**` (new), plus tests in the style of `scripts/archive-raw.tests.ps1`, or bats for bash
- **Deliverable:** restic backups of the following, each snapshot tagged with its hash:
  - `data/raw` (tag: `raw_root` from `bibex raw bless`)
  - `data/curated`
  - `data/compiled` (tag: the manifest `root`)
  - git bundles of both repos
- **Destinations:** a local drive first, then B2 when O-B2 is done.
- **Also:** a written restore procedure, tried once against the local repository.

### CX-I2: `bibex fetch` and the artifact store
- **Status:** blocked:A-DATA-SPEC

### CX-PW: the 3 known Playwright failures and the `world-hover-text` flake
- **Status:** blocked:A-F1 (FOCUS-1 retargets the popover the flake lives in)
- **Failing specs:** `world-quiet-places.spec.ts:211`, `split-view.spec.ts:293`, `reader-xref-anchoring.spec.ts:201`, and `world-hover-text.spec.ts:658` (the flake)

---

## Lane F: FOCUS batches assigned to Codex
Created by A-FPLANS, e.g. `CX-F2`, `CX-F45`, `CX-F78`, each with its plan, base commit and files.

## Deferred (after Nov 4 unless the owner says otherwise)
- The words-as-base inversion (R-C2-W4, the red-letter word spans, T8c Kretzmann on word spans, OPEN-7)
- FOCUS-7's direction for Kretzmann (text anchored on text)
- Map-generator Stages 2–4 (continued inside the atlas)
- The mutation run
- Mobile
- Compiled data leaving git
- The production pipeline and the user-data policy
- Choosing code and content licenses
