# QUEUE: the one list of work

Protocol: `AGENTS.md`. Plan: `docs/superpowers/plans/2026-09-30-v1-roadmap.md`.

- **Statuses:** `proposed` · `ready` · `claimed:<agent>:<time>` · `blocked:owner` · `blocked:<item>` · `review:<commit range>` · `done`
- **Lanes:** A = Claude · B = maps (Codex) · C = analyses (Codex) · D = infrastructure (Codex) · F = FOCUS batches (whoever the item names)
- **Repos:** `atlas` = this repo · `mapgen` = `map-generator`. On the owner's machine they are siblings: `~/src/bible-atlas` and `~/src/map-generator`.

## STATUS (the controller rewrites this at every landing)
- **As of** 2026-09-30 afternoon. **CONTRACT-2 is CLOSED** at `678a0d2` (report: `docs/superpowers/reports/2026-09-30-contract-2-close.md`): 1,574 tests / 98 sections / 0 failures; timing gates 10/10 (gate 1 37 s → 1.0 s after an O(n²) paging fix); contract gate PASSED; semver ok; Playwright 453/2/4 (the known `world-quiet-places:211` + one load flake); AQC 0.11.0, AGC 0.15.0, graph-types 0.7.0, section schema 18, root `3e91f83b…`.
- **The base for every new item is `678a0d2`.**
- **Mutation:** owed (base `13111dd`; `.superpowers/MUTATION-GATE-DEBT.md`), deferred to after Nov 4 by the roadmap.
- **Next for Claude:** A-F1 (FOCUS-1), with A-NAMES waiting on O-NAMES.
- **Next for Codex:** CX-M0 (claimed) → CX-M1, CX-M2 · CX-R1 · CX-R2 · CX-R3 · CX-I1 (to the B2 step).
- **Before "go":** O-CODEX (`codex login`) if not done.

## OWNER QUESTIONS (answer in one line each; agents append)
1. **O-PUSH:** resolved 2026-09-30: `gh auth login` + `gh auth setup-git` in WSL; a push and a lock take/release from WSL both succeeded.
2. **O-CODEX:** Codex CLI 0.159.2 is installed in WSL (`~/.local/bin/codex`, works from any shell). Run `codex login` once. It runs in `~/src` on this machine.
3. **O-PASTOR:** Pastor Hromowyck demo: Oct 21 or 22?
4. **O-NAMES:** 4 of 5 answered 2026-09-30 (spec §10: one Name per language; events denotable; map-generator's ids pending licensing; many names per entity as a `Naming` type). Open: which titles go on the review list (all held until you choose), and your sign-off of the amended types.
5. **O-ORDER:** FOCUS-6 comes right after FOCUS-1 (owner, 2026-09-30). Open: can FOCUS-4+5 and FOCUS-7+8 each close as a single batch?
6. **O-ERRATA-SEED:** List the map errors you've already noticed (map, place or polity, what's wrong). They seed CX-M2.
7. **O-STYLE:** After CX-M1's gallery, which style (parchment / canaan / slate), and which eras get a finished map (all 10 atlas eras?)
8. **O-GPL:** Redraw the GPL `historical-basemaps` world borders as our own CC0 work (recommended), or accept GPL? Swap the OSM rivers (ODbL) for Natural Earth (recommended)? Your licensing rule (2026-09-29) disqualifies both as they stand.
9. **O-PDF:** `Saltwater (Notation and Tab).pdf` in map-generator: remove it from HEAD (recommended), and also purge it from history? Is the repo public?
10. **O-B2:** When you're ready, create a Backblaze B2 account with Object Lock and put the restic password in your password manager. CX-I1 is prepared up to that step. Its local repository goes on `C:` (120 GB free, 92% full) unless you name an external drive.
11. **O-EXPLORER:** Map-generator's `codebase-explorer` branch ("The Atlas Engine", 29 commits past `origin/master`, which include all 27 of `stage1-overnight-2026-09-07`): merge it, or park it?
12. **O-MG-LOCAL:** Your Windows copy of map-generator (`Documents/the-best-maps-ever`) has 2 commits on `master` that were never pushed: `91d089c` "A source travels with its terms…" and `089a4bf` "RCA: why 6e349a3 broke the map…". Push them as they are, hand them to CX-M0 for review, or drop them?
13. **O-CATECHISM:** Post the license request drafted at `.superpowers/sdd/queue-name-model/catechism-license-request.md` on github.com/brain-fuel/catechism. The fallback is our own mapping from the public-domain 1921 Triglot.
14. **O-GODLINK:** with persons linked in verse text (CONTRACT-2), "God" and "LORD" are links in nearly every verse (8,587 verses for God). Keep every occurrence linked, or link only the first in a window/chapter?
15. **O-BADGE:** FOCUS-1 drops the client-derived kind badge ("Passage"/"Verse") from the trail and selection tray and shows the served label only (rule 25). Want a served display kind on the card instead (a contract item)?
16. **O-FINDINGS:** FOCUS-1 touches the files of F-6, F-7 (popover level), F-9 and F-12 (rewritten files). Close them inside FOCUS-1 (yes/no per finding)?
17. **O-CHOOSER:** the map's place chooser (world-cluster-chooser:213) stays open after zooming dissolves the cluster it was opened on; it fails 2 of 3 on today's build too (not FOCUS-1's). Fix it (close the chooser when its cluster dissolves)?
18. **O-LAND-F1:** Codex has been stalled all day, so FOCUS-1 has no reviewer. Land it on my own review, or wait for Codex?
19. **O-F6 (five, from the FOCUS-6 plan `docs/superpowers/plans/2026-10-01-focus6-geography.md`; the plan builds the [default] if unanswered):** (a) add edge-end relations `from`/`source-of` and `to`/`target-of` (22 → 24), node cards not counting them? [yes] (b) with `PlaceCard` gone, does hovering a place still preview it, or is a click the only way in? [click only] (c) a place card's time-window content (period name/blurb, events in the window, narrative prev/next): a card that ignores the window, or one scoped to it? [ignores] (d) `/world` with nothing focused: today's free slider, or always open on a Map? [free slider] (e) crossing into the next era: follow `follows-in` when the slider is dragged past the bound, or only on an arrow click? [click]
20. **O-EDGE-TYPES:** the client types (R18) stand, signed off 2026-09-30 ("Good go"). O-F6 (a) is WITHDRAWN (owner, 2026-09-30, PRINCIPLES 27): no `from`/`to` relations; the graph models the domain, never a view.
21a. **O-F6R (FOCUS-6 re-plan `f0f3470`, plan OPEN 6–13; defaults build if unanswered):** 6 vocabulary gate skips string literals [skip; alt: move `event_merge.rs` tables to `data/curated/`] · 7 delete the unread `frontier.rs` FocusKind×Capability matrix [delete] · 8 rename `exploration-roundtrip.feature` (AQC major) [rename] · 9 "focus" in the gate word list [no] · 10 edge label wording [`{subject} · {kind label} · {object}`; ties to O-LABELS] · 11 widen `/api/node/{id}/edges` to edge ids vs new `/api/position/{id}/edges` [widen] · 12 `edge_summary` bundled in the element record vs its own read [bundled] · 13 `loci`/`note` on neighbour pages only [yes]
21b. **O-GOLDEN-R4:** two FOCUS-6 changes move recorded map-scene bytes: (1) Task 4 adds `node: {kind, id, label}` to each scene place — `tests/fixtures/golden-scene.json` +10 lines (Jericho, Shiloh node refs; no geometry) and 22 scene hashes; (2) R4b corrects place labels ("Hazor 1" → "Hazor") — 11 scene hashes move again (each scene ~75 bytes smaller; golden-scene.json unchanged), so `scene_byte_identity` stays red until re-pinned. AGENTS needs your approval to re-bless map fixtures. Approve both?
21c. **O-VERSE-LABEL:** a verse's label is its locator ("JHN.3.16"), a Concord paragraph's "BoC p.a.p". Should a verse read "John 3:16" wherever it is labelled (compiled)? [default: keep the locator until you rule]
21. **O-LABELS:** edge kinds now show display labels derived from their wire names ("attested-in" → "Attested in", "spouse-of" → "Spouse of"). Keep derived, or hand-write a curated label per kind (a data file)?
22. **O-CACHE:** FOCUS-0 owes one proof (R44): clear `data/cache/sections/*` and run `cargo test -p atlas-cli` green first time. OK to clear the cache (regenerable) in `~/src/bible-atlas`?

## FINDINGS (rules 24a/24b: open categories, each with its proposed CLOSURE; the owner decides; agents append)
- **F-1 (rule 26, server facts in code): the canon table.** `server/atlas-core/src/canon.rs` holds the 66 books (codes, OSIS ids, names) as a static table in code. Proposed closure: the canon becomes a curated file (`data/curated/books.toml` already exists — check what it holds) compiled into the artifact's Kjv section, and `BookId`/codes are read from it; a law forbids book literals in server code. Cost: `BookId(pub u8)` and the code-string wire form re-anchor on the data.
- **F-2 (rule 26): the attestation-pending inventory.** `server/atlas-graph/src/attestation_pending.rs` pins 889 event pairs as a `PENDING` const in code (re-swept by hand when the attests cap came off). Proposed closure: the inventory is a curated file with each pair's grounds, or is derived by the compiler as a law output, never a code literal.
- **F-4 (rule 26a, the tool boundary): the served crate links the ETL.** `server/atlas-contract/Cargo.toml` depends on `atlas-etl`, and `atlas-contract/src/load.rs:53` calls `atlas_etl::compile::compile` for the `--build-from-raw` path; `atlas-graph` holds both the compiler and the served readers in one crate. Proposed closure: `atlas-etl` and the compile binary are tools; `atlas-contract`/`atlas-server` depend only on the artifact readers; `--build-from-raw` becomes "compile to a scratch artifact with the tool, then serve it" (in `atlas-cli` or the compile binary), so the crate graph makes the boundary unwritable.
- **F-5 (fix wave, server): O(position) node paging** — `graph-types/src/graph.rs` `nodes_of_kind` and `atlas-graph/src/sqlite/snapshot.rs` `nodes_of_kind` (`OFFSET`) skip `start` entries per page; not material today. Closure: a per-kind id vector / keyset paging on `(kind, id)`.
- **F-6 (fix wave, client, rule 25): two fetch layers** — `client/GraphExplorableClient.cs:7-9` holds its own `HttpClient`; `Program.cs:25` registers a scoped one nobody injects. Closure: it composes over `AtlasClient`; the registration deleted; a source law "HttpClient appears only in AtlasClient.cs / RequiredResponses.cs / Program.cs".
- **F-7 (fix wave, client): 14 Explore-layer fetches outside the request series** (`ExplorationDescriptor.cs:40,67,119`, `PassageBlock.cs:29,57`, `PopoverSectionProviders.cs:205,246,387,629,1285`, `BookNode.cs:30`, `ChapterNode.cs:61`, `YearNode.cs:91`, `PersonNode.cs:29`). Closure: the popover owns one `RequestSeries`; `IExplorable.ExploreAsync/BodyAsync` receive a `Request`. Until then the stale-response closure is partial.
- **F-8 (fix wave, server tests, rule 26): literal expectations in real-data tests** (`graph_api.rs:433-446`, `lexicon_real_data.rs:67-83`, every other real-data pin in `graph_api.rs`). Closure: derive from the artifact, as the wave's three new tests do.
- **F-9 (fix wave, client + server): saved explorations resolve a date/era BY LABEL** (`ExplorationDescriptor.cs:79,122`). Closure: `DateClaim`/`Era` publish ids on the wire (server), the client stores ids, and a client law "no function takes a label and returns a node".
- **F-10 (fix wave, client): PlaceCard's measurement guard** (`PlaceCard.razor:305-319`) — a stale-result check by hand. Closure: the card's `RequestSeries` covers it.
- **F-11 (fix wave, client): the picker's "served change discards the draft" policy** (`ScripturePicker.razor`, `_synced`) is not named by the `Draft` type. Closure: `Draft.Follow(served)` if the owner wants it typed.
- **F-12 (rule 9): pre-existing comments** remain around every site the wave touched; the wave added none. Closure: A-STRIP.
- **F-13 (FOCUS-1 prep, server/ETL, the container law): no passage container exists.** The ETL mints chapter/book/bible and Concord doc/article containers only; titled passages are not nodes, so a passage cannot be an Explorable identity. FOCUS-1 uses the first verse's TextUnit (F1-3); closure: FOCUS-3's ETL mints Container nodes for titled passages.
- **F-14 (FOCUS-1 prep, client): `FocusKind`/`FrontierMatrix` mirror the graph vocabulary as strings.** Closure: the generated `NodeKind`/`EdgeKind` enums are the only vocabulary on the client.
- **F-15 (client): `IExplorable.Kind` is a string.** Closure: `NodeKind`.
- **F-16 (server wire): `PositionKind` is `NodeKind` + `Edge`, so every client mapping from a served position is partial.** Closure: FOCUS-1 Task 0 (F1-2) splits the edge position into its own wire shape.
- **F-17 (server wire): the corpus is absent from `NodeRef`/`NodeCard`,** so the client parses ids to learn it (`HomeSurfaces`). Closure: `corpus` on the wire (a CONTRACT item, FOCUS-2/3).
- **F-18 (client): `RevealPageSize` restates the server's page clamp.** Closure: the served page size is the one declaration.
- **F-19 (FOCUS-1, client tests): the atom set was hand-declared in 4 places in ConformanceTests** — CLOSED in FOCUS-1 T6 (derived by reflection).
- **F-20 (client): `IIntent.Name` is read by nothing outside tests** (rule 4). Closure: delete it across all atoms, or give it a reader.
- **F-21 (server wire): an edge position is not fetchable via `/api/node`**; only `justified-by` serves edge positions. Closure: serve them only where a Sequence presents them (the Event/narrative batch), never as frontier neighbours.
- **F-22 (client, T3's table): `Affordance.Arrows` carries no direction.** Closure: `Arrows(ArrowDirection)`.
- **F-23 (contract): frontier section headings are the edge kind's wire name; no served display label.** Closure: a `label` per edge kind in the vocabulary document (O-BADGE's category).
- **F-24 (client): `SectionOrder` is read by nothing on the new path** (rule 4). Closure: the view orders by it, or it goes.
- **F-25 (tooling): `timing-gates.sh` writes its logs into the tree.** Closure: logs under the target dir; the tests-never-write-the-repo law covers scripts.
- **F-27 (client): the popover title shows the legacy title (`GEN`) while the trail shows the served label (`Genesis`).** Closure: every surface reads the served label (legacy titles die per kind).
- **F-28 (client): a FocusView section heading counts links not offered on that surface.** Closure: the count comes from `Presentation.Offers`.
- **F-29 (client): the popover's `Root`/`Saved` are two optionals, not one sum.** Closure: `PopoverOpening = Root | Saved`.
- **F-30 (tests): `tsc` over tests/ux has 16 pre-existing errors,** so spec types aren't enforced. Closure: tsc clean and in the gate.
- **F-31 (client): node constructors disagree on local vs wire ids** (`PersonNode` vs the rest). Closure: one typed id on the client (FOCUS-1 R36 / A-BACKLOG).
- **F-32 (map-generator): DUPLICATE of CX-M0 step 3, which already fixes it; closed here.** The workspace does not build in WSL. Seven crates take `atlas-graph-types` by a path into the old Windows-era tree (`../../../../scratch/bible-atlas-sketch/.claude/worktrees/bible-atlas-m1/graph-types`), which no longer exists; `cargo` fails at manifest load. Closure: one dependency declaration (workspace `[workspace.dependencies]` or a git dep on bible-atlas at a pinned rev), so a machine move cannot break seven sites.
- **F-33 (backend, rule 27): the client's exploration vocabulary lives in the backend.** 56 files under `server/` and `graph-types/` name explore/explorable/frontier (`graph-types::explore::Frontier`, `FrontierEdge`, `FrontierRefusals`, `frontier_at`, …), from CONTRACT-2 and FOCUS-1. Closure: rename to the graph's own words (adjacency, neighbours) and a gate that fails on any such name in `server/` or `graph-types/`. First task of the A-EDGES rework.
- **F-34 (API, rule 27a): the wire has view-shaped endpoints** (node "cards" with an edge summary, per-kind routes). Audit every route against the generic read set and propose the migration; the owner rules on order (likely after Nov 4).
- **F-35 (rule 26): event-merge tables are curated data in code** (`event_merge.rs` string literals). Closure: move to `data/curated/` with provenance; the vocabulary gate can then scan literals.
- **F-36 (rule 27): year and time-range labels are formatted per request.** Closure: compile them as labels.
- **F-37 (rule 27f): no budget gate at 10× size.** Closure: a synthetic 10× graph and p95/size gates over the generic reads.
- **F-38 (rule 27): a place's default name and blurb are derived per request.** `resolve_display_name_and_canonical` (alias → curated → suffix-stripped name, `atlas-core/src/history.rs`) runs in `place_detail` and `places.rs`; `resolve_blurb` picks a per-period `place_history_blurb` row at read time. Closure: the compiler writes the default name and blurb into the artifact; FOCUS-6 re-planned Task 4 takes it.
- **F-39 (integrity): the version root does not cover derived tables** (`edge_index`, `label`): a change to label logic with unchanged rows can reuse a stale blob under the same root; only a schema bump forces a rewrite. Closure: the root covers derived tables, or the derivation code's version is an input to the root.
- **F-40 (rule 27f): compiled labels grow the artifact with every position** (R2: +25 MB, kjv 48→58, lexicon 42→52); at 10× the 100 MB blob ceiling breaks. Owner to weigh: labels for every position vs only for positions a reader can land on; dedup/compression.
- **F-41 (gates): a test referenced by name in a script can vanish silently** (`timing-gates.sh` still names `frontier_page_latency_…`, renamed in R1; a filter matching nothing passes). Closure: the gate fails when a named test matches zero tests. FOCUS-6 close.
- **F-42 (rule 27): reference strings are composed in served code** (`dot_ref`: `TextUnit.ref`, `next`, `encode_node_id`). Closure: compiled, with F-36.
- **F-43 (wire): `/api/elements` percent-decodes before splitting `ids` on `,`** (`ids=X%2CX` reads two ids). Low: the law `no_served_id_carries_the_separator…` keeps `,` out of ids. Closure: split, then decode each.
- **F-44 (rule 4): `HomeSurfaces.Of` has no production reader** (pre-existing).
- **F-45 (FOCUS): no live popover offers an edge step** until a node kind moves to FocusView (the migrated-kind list is empty); A-EDGES is proven by bUnit + a seeded-save Playwright spec.
- **F-46 (rule 4 / 24b): test hooks ship in production `map.js`** (`debugLiveInstanceIds`, `debugIsPointOnLand`, … ; FOCUS-6 R8 added `debugClickMap`, `debugRecordSink`, `debugSinkCalls`). Closure: test hooks live in a test-only module the production bundle does not load.
- **F-47 (D.R.Y./Haskell): `MapDetail` and `EraDetail` are the same shape** (`{window}`), named twice.
- **F-48 (naming): "reign" means two things** — one era's span on `/api/polities` `Polity.reign`, the polity's whole span on `PolityDetail.reign`.
- **F-49 (rule 24, R4b residue): served `name` fields still carry the disambiguation suffix** ("Bethel 1" in scene `name`, `place-hazor-1.json`, `gazetteer-consumed.json`). Scene places carry `name`, `display_name` and `node.label` side by side; `PlaceDetail.display_name` ≡ `node.label` (D.R.Y.). Closure with F-34.
- **F-50 (data, reader-facing): anchor citations contain developer curation notes** (HOTFIX-7, `ANCHOR_DEFERRALS`, `ret_babylon`), now served as `NodeRecord.description`. Closure: split curation notes from the reader citation in `data/`.
- **F-51 (labels): placeholder labels remain** — "text unit ({corpus})" (never hit in the real artifact), "Commentary" for headless commentary items, commentary headings joined with ".:".
- **F-52 (D.R.Y.): geography is compiled twice** (the label pass and `GraphService::assemble`); the pipeline does not hand it on.
- **F-39 note:** R4b confirms it — labels sit outside the logical hash; only a manual schema bump moves the root. Proposed closure: labels in `logical_dump_section`.
- **F-3 (rule 25, client derivation): `client/CanonRef.cs`** — see A-BACKLOG; closes when the legacy routes retire.

---

## Lane A: Claude

### A-C2: CONTRACT-2 close
- **Status:** done (2026-09-30, `678a0d2`)
- **Tracked in:** `.superpowers/sdd/2026-09-29-contract2-pushdown/progress.md` (every ruling).
- **Closed by** Task 11 plus a fix wave under rules 24/24a/24b/25/26 (every fix names its category, its failed abstraction, its side and the closure; see the close report §6a).

### A-F1: FOCUS-1
- **Status:** review:678a0d2..3baaeb6 (branch `lane/claude/A-F1`; close report `docs/superpowers/reports/2026-09-30-focus-1-close.md` on the branch). Gates: 1,577 Rust / 572 client / 54 contract tests green; contract gate passed; AQC 0.12.0; Playwright green except the known world-quiet-places:211 and the pre-existing world-cluster-chooser:213 (O-CHOOSER). **Codex: review this range (AGENTS.md 14b + 24a/24b), then Claude lands it.**
- **Plan:** `docs/superpowers/plans/2026-09-27-focus1-types.md` (9 tasks), plus rulings R11–R14 in the FOCUS spec.
- **Note:** build no interaction that works only by hovering.
- **Done when:** the plan's Task 9 gates pass and it's reviewed by Codex.

### A-EDGES: edges are explorable (R18)
- **Status:** REWORK (owner, 2026-09-30, PRINCIPLES 27). `lane/claude/F6-t1` and `F6-t2` (`86baad7`, `54079c4`, `7f1ef67`) never land: they put the frontier into the backend (edge card endpoint, `EdgeSource`/`EdgeTarget` relations, labels composed per request). Rebuilt on `3baaeb6`: the generic element read (node or edge, many ids per call), edge labels compiled into the artifact, the edge frontier derived on the client; first the closure of F-33.
- **Ruling:** spec §12 R18 (owner, 2026-09-30). Server: an edge card and an edge's frontier in the generated document (AQC minor); wire `Link.Target: PositionRef`; client: `ElementKind = Node | Edge`, `Presentation.Of(ElementKind, Surface)` rows for every edge kind, `IExplorer.Resolve(PositionRef)`/`Follow` total. Closes F-21 and retires F1-12's filter. Types for owner sign-off at FOCUS-1's close.
- **Done when:** `Follow` is total (a law over every served position), the edge card serves its justification, and a Playwright spec follows a verse → its attests edge → the event.

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
- **Order:** FOCUS-6 first (owner, 2026-09-30), then 2, 3, 4+5, 7+8, 9. A-EDGES (R18) runs before FOCUS-6's plan is final, or alongside it if the tables it adds are disjoint.
- **Assignment:** each plan names its agent, its base commit and its files. Codex takes the ones that pair against Claude's (server-heavy beside client-heavy).

### A-F3, A-F6, A-F9: FOCUS-3, FOCUS-6 (with the map switch), FOCUS-9
- **Status:** blocked:A-FPLANS (FOCUS-6's map half is also blocked on the MAPS migration)

### A-REVIEW: review every Codex item in `review`
- **Status:** standing
- **Scope:** the PRINCIPLES 14b pass (D.R.Y., the Haskell bar) plus the rule 24a category pass: for every fix, was the category named, the failed abstraction named, the side chosen, every site migrated? Plus 24b: is the category CLOSED (the raw thing private, the abstraction the one door, an enum or an enumerating law), with the guarantee named? Plus a grep of the diff for added comment lines. Land what passes, holding the `land` lock.
- **Findings, not fixes:** offenders found during any review or task (code outside the agreed abstractions, a same-category site left unmigrated) are written under **FINDINGS** below with the category and the sites; the owner decides how each is addressed. Nobody fixes one on the side.

### A-THEO: replace Theographic (CC BY-SA)
- **Status:** proposed (ruled 2026-09-29: replace in a queued batch; credit it until then)
- **Scope:** Theographic supplies 450 of 552 events, part of places, and the Easton text bundle. Replace from BibleData (CC BY: persons, events, Ussher dates), STEPBible TIPNR (CC BY), Easton re-sourced from its 1897 public-domain text, and our own curation. Needs a spec (types, what each source supplies, the id mapping so no node id churns without a recorded reason).

### A-STRIP: remove every comment from application code
- **Status:** proposed (owner, 2026-09-30: "no comments in my app code stop doing that"; PRINCIPLES 9). Size: ~5,300 comment lines in `server/`, `graph-types/`, `client/` (2,466 `///` in Rust). Blocker to rule on: utoipa takes a FIELD's published description only from a `///` doc line (`#[schema(description)]` is type-level), so either field descriptions leave the contract (type-level descriptions only) or `///` on wire fields is the one exception.
- **Scope:** everything already in `server/`, `client/` and `graph-types/` before CONTRACT-2 (the close strips CONTRACT-2's own 290 lines). Wire descriptions the OpenAPI document needs become `#[schema(description)]`. One crate per commit; `export_contract --check` shows what the document loses.

### A-TOOLS-SPLIT: the tools leave the system's repo
- **Status:** proposed (owner, 2026-09-30: "we'll have to consider ripping those tools out of our repo and giving them their own home so that the pieces we care about are as clearly segregated as necessary"). Decide after FOCUS; pairs with A-DATA-SPEC (the artifact store) and "compiled data leaving git".
- **Shape to evaluate:** the tools (`atlas-etl`, the compile binary, `scripts/fetch-raw`, `archive-raw`, `bibex raw bless`, the ETL laws) get their own repository, whose product is the artifact plus its manifest; `graph-types` (already a root-level, non-member crate) becomes the published interface both depend on; the system repo keeps the artifact readers, `atlas-contract`, `atlas-server`, the client, the contracts, and `bibex verify` (users care that the data is what it claims). Open questions for the design: where `data/curated` lives (tool input) and how the artifact reaches the system (the store, A-DATA-SPEC); what `--build-from-raw` becomes (F-4); whether the data laws that today run in `atlas-graph`'s compile half move with the tools. Effect: the system's test suite shrinks and speeds up; the crate graph enforces 26a by construction.
- **The Polylith alternative (controller, 2026-09-30):** one workspace, segregated by PROJECTS, not repos. Bricks = crates with one interface module (24b's one door); bases = the binaries (`atlas-server`, the compile binary, `bibex`); projects = each deployable's brick list. The `server` project simply does not contain the ETL bricks, so 26a holds by construction, enforced by a law over `cargo tree` per deployable; the `development` project holds everything, so the two-agent workflow, one `data/`, and cross-cutting batches stay simple. The tools can still leave later; projects first makes the seam real without a repo move.

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
- **Status:** ready
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
- **Base:** the atlas at `678a0d2` (CONTRACT-2's close; `kjv_token` is in the tracked artifact's Kjv section: 790,892 words).
- **Background:** `.superpowers/analysis/name-sources.md` already chose **BibleForgeDB** (public domain / CC0): it aligns 763,778 KJV words to specific Hebrew/Greek tokens. Its KJV edition differs from our `data/raw/kjv.json` in 452 verses; ~17.9k of its rows are flagged ERROR; its Greek needs STEPBible TAGNT's Textus Receptus.
- **Question:** how much of OUR KJV word layer can reach a specific original token through BibleForgeDB + STEPBible TAHOT/TAGNT, and what's left?
- **Steps:** a spike in `~/w/CX-R3` scratch space (nothing committed but the report): download BibleForgeDB and TAHOT/TAGNT into scratch; join our `kjv_token` rows (read with `sqlite3` or a small script from the unpacked Kjv section, `data/cache/sections/`) to BibleForge's words by an explicit per-verse diff, never by position; then BibleForge's original tokens to TAHOT/TAGNT tokens. Report: the percentage of our words aligned; the unaligned by kind (italic/supplied, ERROR rows, the 452 differing verses, Greek TR gaps); 20 worked examples including Gen 32:28 "Israel"; the licence text of every file used.
- **Done when:** `.superpowers/analysis/kjv-alignment-join.md` is pushed on `lane/codex/CX-R3` and reviewed. It feeds A-NAMES.

### CX-R1: the 65 overlapping-event cases
- **Status:** ready (they surfaced when CONTRACT-2 removed the 20-verse cap on attesting verses; landed at `68fb00f`)
- **Base:** `678a0d2`. **Source:** `server/atlas-graph/src/attestation_pending.rs`, whose `PENDING` inventory grew 824 → 889 pairs when the cap came off: 65 new collisions, most of them Robertson passion-week events contained in `theo-443`/`theo-448`. `git diff 68fb00f~1 68fb00f -- server/atlas-graph/src/attestation_pending.rs` lists exactly the new pairs.
- **Deliverable:** a curation sheet, one row per case: the events (id and title), their verses, the kind of overlap (containment / overlap), and a proposed resolution with its Scripture grounds, each answerable by the owner in one line.
- **Output:** `.superpowers/analysis/overlapping-events.md` on `lane/codex/CX-R1`. Read-only on everything else.

### CX-R2: Easton doctrinal review list (for the Pastor)
- **Status:** ready
- **Base:** `678a0d2`. **Source:** `data/raw/theographic/theographic-bible-metadata-master/json/easton.json` (Easton's Bible Dictionary, 1897, Presbyterian; it supplies the `description` of places, people and people groups). Which entities use which entry: `server/atlas-etl` (grep `easton`).
- **Deliverable:** flag every entry that touches doctrine: baptism, the Lord's Supper, election/predestination, conversion, the law and the gospel, the church and ministry, the end times, and the like. Quote the passage and say why it may conflict with Lutheran teaching. No judgment beyond flagging.
- **Output:** `.superpowers/analysis/easton-doctrinal-review.md` on `lane/codex/CX-R2`, sorted by how prominent the entity is in the app (its node's edge count from `bibex`/the artifact).
- **Note:** Easton is re-sourced from its public-domain original when A-THEO replaces Theographic; the review carries over (same text).

---

## Lane D: infrastructure (Codex)

### CX-I1: backups
- **Status:** ready up to the B2 step, then blocked:owner (O-B2)
- **Base:** `678a0d2`. **Files:** `scripts/backup/**` (new) and its tests, `scripts/backup/test/*.bats` (bats 1.14 is at `~/.local/bin/bats`).
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
