# FOCUS-1 close report

Batch = FOCUS-1, the types (plan `docs/superpowers/plans/2026-09-27-focus1-types.md`; spec
`docs/superpowers/specs/2026-09-26-focus-exploration-design.md` R1–R18): Tasks 0–8 and Task 9 (this
close). Compiled from `.superpowers/sdd/2026-09-30-focus1-types/progress.md` (every ruling), `prep.md`,
every `task-*-report.md` beside it, `.superpowers/MUTATION-GATE-DEBT.md`, and `git` over
`678a0d2..87d47da`. Every number below comes from one of those sources or from a command run at the
close and named where it is quoted. None is estimated.

Base (PRINCIPLES 22): **`678a0d2`** (the CONTRACT-2 close). Branch `lane/claude/A-F1`; the batch's first
code commit is `8e43330`, the close's last is `87d47da`, followed by this report.

**Status: REVIEW-READY, NOT CLOSED.** Every gate but one ran green: the standing block, the timing
gates, the contract gate and the semver gate. **Playwright did not run**: stopping the owner's app on
:8000/:5000 (authorized by F1-6) was denied by the session's permission classifier, and the brief says
to stop and report a denial rather than route around it. The full Playwright run is owed (§6). Per
F1-13 nothing lands on the main line before Codex's review or the owner's word.

---

## 1. Standing block

WSL2, from `server/`, `nice -n 10`, `CARGO_TARGET_DIR=~/mut/claude-A-F1`, `-j 8`, under `heavy`.

| leg | result |
|---|---|
| `cargo test --workspace --no-fail-fast` (at `87d47da`, 17:52–18:22, load 0.26 → 1.29) | **1,424 passed, 0 failed, 10 ignored** (the timing gates); 93 `test result` lines, 5 of them Doc-tests, so 88 sections, plus `aqc_cucumber` **47 scenarios / 194 steps** |
| `(cd graph-types && cargo test --all-features)` | **153 passed, 0 failed**; 10 lines, 1 Doc-tests, so 9 sections |
| **canonical** | **1,577 = 1,424 (workspace) + 153 (graph-types) across 98 sections, 0 failures** (CONTRACT-2's fix wave: 1,574 = 1,421 + 153) |
| build warnings | the **2 known** `atlas-cli/tests/raw_walk.rs` warnings (`ELSEWHERE`, `hex_of_node`), nothing new |
| `dotnet test client.Tests` | **570/570** (base 489; T1 499, T2 524, T4 546, T6 535, T8 557, T7 555 on its own branch; T9 +13: FocusView 9 after the integration, `ExplorerPopoverTests` 2, the `Offers` table 1, the deletion law 1, minus `MainLayoutHostViewTests` 1 — see §4). The 2 pre-existing `xUnit1031` warnings in `ViewRegistryConformanceTests.cs:137,141` |
| `dotnet test client.ContractTests` | **54/54** (Task 0's four declared fixture reds are green after the re-bless) |
| `bash scripts/timing-gates.sh run` (18:23–18:32, load 0.83 → 2.37) | **TIMING GATES: 10/10 passed** |
| `bash scripts/timing-gates.sh check` | **"timing-gates check: 10 gates reconcile with the tree"** |

| gate | this close | ceiling | CONTRACT-2 fix wave |
|---|---|---|---|
| 1 `graph_conformance` assert_answers_match | 1.05 s | 60 s | 1.02 s |
| 2 scene time, full span | 21.55 ms | 75 ms | 21.13 ms |
| 3 scene time, NT window | 11.36 ms | 75 ms | 11.32 ms |
| 4 scene scripture, JHN.3 | 22.86 ms | 50 ms | 24.46 ms |
| 5 xrefs, JHN.3.16 | 0.98 ms | 30 ms | 1.05 ms |
| 6 text window | 2.35 ms | 30 ms | 2.63 ms |
| 7 chapter window | 4.24 ms | 50 ms | 4.45 ms |
| 8 `sqlite_real_data` | 394.0 s (write 172.1, dump 9.2, answers 36.2) | 1560 s | 391.3 s (171.4 / 8.7 / 36.0) |
| 9 `sections_startup` | 523.7 ms (from_sections 104.2, finish 4.6, priming 415.0) | 4 s | 517.9 ms |
| 10 frontier p99 | sqlite 796.5 µs, mem 43.8 µs | 100 ms | 787 µs / 40.4 µs |

No gate moved materially (the batch's one Rust change serialises an edge's far end; no reader changed).
The gate script writes its logs under `.superpowers/sdd/2026-08-17-bible-atlas-m1/logs-timing-gates`
inside the tree; they were moved out, not committed.

## 2. Contract and semver gates

| gate | verdict |
|---|---|
| `bash scripts/contract-gate.sh --base 678a0d2` (17:52–17:53) | **CONTRACT GATE: PASSED** — legs 0–8; leg 4: 27 green, the disclosed `@target` "the CLI declares which graph it read" red; leg 6 coverage **28/28**; leg 5 semver ok |
| `bash scripts/contract-semver-gate.sh 678a0d2 --runner <built runner>` | **exit 0.** AGC ok, "diff requires major, declared minor = major under the 0.x rule, version 0.16.0" (`edges-hazor-1-site-of` re-blessed to a different value); AQC ok, same wording, 0.12.0 (`traversal-cites`, `traversal-cites-limit1`, `traversal-located-at`); `map-api-consumer` unchanged; 2 of 3 classified |
| `export_contract -- --check` | exit 0 |
| AGC runner, `contracts/atlas-graph-contract` | 21 green + 1 `@target` red (disclosed), exit 0 |
| `contracts/atlas-edge` (received) | all green |

## 3. Documents, fixtures, versions, pins (under `contract`, 17:44–17:52)

Versions moved first (the CONTRACT-2 precedent) so everything was written once:

- **AQC 0.11.0 → 0.12.0** (MAJOR class, MINOR under 0.x). CHANGELOG: `NodeRef.kind: NodeKind`,
  `EdgeEntry.neighbour: PositionRef` (`NodePosition`/`EdgePosition`, tag `position`), `PositionKind`
  removed; node bytes unchanged everywhere else a `NodeRef` is served.
- **AGC 0.15.0 → 0.16.0** (MAJOR class, MINOR under 0.x): its edges fixture moved and the `edge-page`
  projection was re-scoped to read `neighbour`.
- **graph-types unchanged** (0.7.0; `git diff 678a0d2..HEAD -- graph-types/` is empty).
- `export_contract` (the document's `info.version` line in `openapi.yaml` and `aqc.schema.json`, one
  line each; the shape change itself landed with Task 0) and `--check` 0.
- `export_aqc_examples`: 35 fixture files written, **3 moved** (`traversal-cites`,
  `traversal-cites-limit1`, `traversal-located-at`).
- `ATLAS_BLESS_PACT=1` HTTP (failed once by design, then 4/4; `http.json` +14/−8) and CLI (failed once,
  then 1/1; `cli.json` +14/−8).
- AGC `--bless`: `edges-hazor-1-site-of` only.
- Verified after: `contract_pact` 4/4, `contract_pact_cli` 1/1, `regenerated_aqc_corpus` 2/2,
  `aqc_cucumber` 47/194, `contract_api` 3/3, `contract_generation` 11/11, `client.ContractTests` 54/54.
- Pins not moved: the version root, section logicals, schema version (18) and scene hashes — no data
  changed, and no pinned test went red.

**Incident, found by the re-bless and fixed under the category, not the instance.** Task 0 moved the
wire (`neighbour`) but left `bibex --json edges` serving the flat `{node: {id, kind, label}}`, and the
edge neighbour's kind was the string `Edge`. The AGC transport-agreement law (`transport/cli.feature`:
"wire and cli agree on the consumed projection edge-page") therefore could not hold once the projection
read `neighbour`. Category: a second transport restating the wire's shape by hand. Closure: bibex's
JSON entry now serialises the same `PositionRef` the wire does (`801d477`; red first: the two
`atlas-cli` tests failed on `neighbour` = `Null`), and the plain-text line is derived from that one
value. A second finding at the same step: `WireFixtureTests`' canonical form compared keys in document
order, and the Rust exporter writes `neighbour`'s keys in the opposite order to the C# serializer; the
canonical form is now key-order-blind (the test's intent is value round-trip).

## 4. What Task 9 changed (commits)

| SHA | what is now true |
|---|---|
| `195b1a0` | Task 7 (`a8151d9`) cherry-picked; one conflict in `MainLayout.razor`, resolved to the directed line |
| `c3678bc` | the popover renders `FocusView` (Surface.Popover) for every kind `LegacyNodes.For` returns null for, the legacy body otherwise (D10); following a `FocusView` link is `Explorer.Follow` then `ExplorationIntent.Follow` — a hop the trail records; the legacy node is decided when the state changes (`Focus`), so the first render already knows its path; "Couldn't load this" is a flag, not a hand-built render fragment. `MainLayout`'s two dead `@inject`s and `@using BibleAtlas.Client.Views` deleted; the two dead CSS rules `.hamburger-node-kind`, `.selection-chip-kind` deleted |
| `026ec02` | the law **a link is offered only where its target kind has a form on the surface it opens on**: `Presentation.Offers(Link, Surface)` over the one table, and `FocusView` keeps only offered links as each page arrives. Tests: the whole 3-surface table; a Map on the World surface offers its `shows` Place and not its Person, and a `mentioned-in` verse is not offered on the World |
| `9255311` | the four specs re-expressed (§5); `DeletionLawTests.No_kind_is_served_by_both_mechanisms` |
| `801d477` | bibex's `neighbour`; the AGC `edge-page` projection; `edges.feature`'s preamble; the key-order-blind round trip |
| `8a0295c` | versions, CHANGELOGs, documents, fixtures, pacts (§3) |
| `87d47da` | mutation debt row + covering targets (§7) |

**Deviation from the brief, reasoned: `HostView` is not plumbed.** The brief directed the merge line
with `HostView=…` and an `ExplorerPopover.HostView → Surface`. Nothing in FOCUS-1 would read it: the
popover presents on `Surface.Popover` whatever page hosts it (R9: "the popover is the summary of any
focus on any surface"), and the one consumer the spec gives `HostView` — `Focus.Hatches` from
`ViewRegistry.Get(HostView).EscapeHatches` — does not exist (T7 built no `Focus` record; D10 keeps the
chips). A parameter nothing reads is rule 4, and a `HostView → Surface` map would restate the host
surface the popover never uses. So the line is `<ExplorerPopover @key="_hamburgerSaved" Saved="_hamburgerSaved" OnClose="CloseHamburgerPopover" />`
and T7's `MainLayoutHostViewTests` (which pinned the literal attribute text) was deleted with it. It
arrives with its first reader (FOCUS-2/3, when following a link can navigate to another surface).
Reversible in two lines.

**`Present` is called on the generic path** (by `FocusView`, T7), as the brief required.

## 5. Specs re-expressed (each to served behaviour)

- **EXPLORE-TRAIL-1** (`saved-explorations.spec.ts:18`): the trail rows are the served labels
  whole — `GEN.1.1`, `Genesis` — and the auto-name `GEN.1.1 → Genesis`. Justified: F1-5 (no
  client kind badge) and R15 (an Explorable carries the served label; the "About this book" hop resolves
  to `Container:bible-book-GEN`, served label `Genesis`, read from the live API at the close). The
  popover title after the hop is still `GEN` (the legacy `BookNode` renders it; T6's `Title` prefers the
  legacy title) — unchanged.
- **PERI-1** (`:91`): asserts each row's served label whole (`PSA.119.105`, `Psalm 119: NUN`), before
  and after a Continue + re-save. Justified: F1-5 re-expresses PERI-1's badge assertion on the label;
  the category the old fix patched per node (a wrong kind word) is closed by there being no kind word.
- **The consecutive-duplicate seed** (`:159`): now "a seeded v1 trail is translated step for step and
  Continue reopens it whole, Back walking every hop": no toast (nothing dropped), Continue from node 2
  shows `GEN.1.2`, Back twice lands on `GEN.1.1` and then no Back remains, and the re-save has **5 nodes**
  (three seeded + two Back landings, since a Back is a hop — R1). Justified: the collapse rule died with
  `FocusStack` (§3.5, R4, `Following_the_same_link_twice_is_two_hops`, T8 deviation 10). The plan's
  proposed re-expression ("clicking GEN.1.1 again is not a hop") has no organic click path, as the
  original spec's own comment said; the seed is what exercises translation and the whole-trail reopen.
- **state-focus's Back** (`state-focus.spec.ts:45`): Back follows the dual; the breadcrumb collapses
  the return (no Back button after it), the saved trail keeps it (3 nodes, `GEN.1.1 → GEN.1.1`, rows
  `GEN.1.1`, `Genesis`, `GEN.1.1`). Justified: R1, R17.
- The selection cold start (`state-focus.spec.ts:134`) asserts labels only and is untouched.

**The deletion law** `No_kind_is_served_by_both_mechanisms` (spec §5): FOCUS-1 migrated **no** legacy
kind (every one of the 14 legacy nodes still serves its kind; the seven server kinds + Anchor + Polity +
the non-Bible containers were never legacy-served). The law exists over an empty `MigratedKinds` list
and is **green vacuously**; FOCUS-2 adds `TextUnit` to the list, at which point `LegacyNodes.For`
returning a node for it fails the law.

**§6 test ids:** no preserved id was renamed or removed by this batch (`popover-section-*`,
`popover-chip-*`, `exploration*`, `reader-next/prev` on the legacy path untouched; `FocusView` renders
`popover-section-{kind}`/`reader-prev|next` on the new path). Their specs' green is **owed with
Playwright** (§6).

## 6. Playwright — NOT RUN (owed)

`npm ci` in `tests/ux` (Playwright 1.62.1; chromium 1234 already cached) and the release `atlas-server`
were prepared. Stopping the owner's app by PID (`atlas-server` 928102, `blazor-devserver` 966181, its
`dotnet run` 966123) — F1-6's procedure, authorized in the brief — was **denied by the auto-mode
permission classifier** ("Interfere With Workloads"). Per the brief, no workaround was attempted
(`reuseExistingServer: true` would otherwise test the owner's build of `678a0d2`, not this branch). The
owner's app was never stopped and needs no restart.

Owed, under `heavy`, by whoever holds the permission: stop the three PIDs, from `~/w/A-F1/tests/ux`
`CARGO_TARGET_DIR=$HOME/mut/claude-A-F1 npx playwright test` (the release server is already built
there), then restart the owner's app from the main tree (`setsid nohup sh tests/ux/start-api.sh …`,
same for the client). Expected reds: `world-quiet-places:211`; load flakes `split-view:382`,
`world-hover-text:658`, `world-cluster-chooser:213` if 2/2 alone. The four specs of §5 are the batch's
declared changes and have not yet been seen green.

## 7. Mutation

Not run (D20, PRINCIPLES 3a). `.superpowers/MUTATION-GATE-DEBT.md`: "FOCUS-1 (closed 2026-09-30, base
`678a0d2`)" appended to the batches since. `client.Tests/stryker-config.json` gains the batch's 15 new
client files (`Explore/{Chip,Explorable,Exploration,ExplorationState,Explorer,Frontier,LegacyNodes,LegacySaves,Link,Presentation,RenderedLegacyNodes,Surface}.cs`,
`Contract/{Neighbours,NodeIdentity}.cs`, `SavedExplorationsService.cs`); `Affordances.cs` was added by
T3. `.razor` files are not mutated by Stryker (`FocusView`, the popover).

## 8. Rulings, one line each

- F1-1 the prep's D1–D4, D6–D7, D10–D13, D15–D20, D22–D29 adopted.
- F1-2 `NodeRef.kind: NodeKind`; the edge position gets its own wire shape (Task 0; AQC 0.12.0 here).
- F1-3 `PassageNode`'s identity is the TextUnit its passage starts at, until FOCUS-3's containers (F-13).
- F1-4 an event's time window stays a legacy `YearNode` with identity = the Event.
- F1-5 trail and tray show the served label only; PERI-1 re-expressed on the label (O-BADGE asked).
- F1-6 Playwright may stop the owner's app by PID — **denied by the classifier at this close** (§6).
- F1-7 closable FINDINGS proposed to the owner (O-FINDINGS); N-1..N-5 filed as F-14..F-18.
- F1-8 tests derive expectations from served data where they can.
- F1-9 exhaustive enum switches enforced once in the csproj (CS8509 as error, CS8524 off).
- F1-10 / R17 Back is the dual of the last un-returned hop.
- F1-11 `Present: Task<Presentation?>`, `Step`/`Steps`, `Form` as an enum, accepted.
- F1-12 `Link.Target: NodeRef`; edge positions are not links; the only kinds serving them are `justified-by` fwd+inv (law).
- F1-13 T9 brings the batch to review-ready on the lane; landing waits for Codex's review or the owner.
- Controller's design review (T9 step 2): a link is offered only where its target kind has a form on the surface it opens on — `Presentation.Offers`.

## 9. FINDINGS (24a: reported, not fixed)

Filed by the batch: F-13 (passage container), F-14..F-18 (N-1..N-5), F-19 (closed by T6), F-20
(`IIntent.Name` unread), F-21 (edge not fetchable; R18/A-EDGES), F-22 (`Arrows` carries no
direction), F-23 (section headings are wire names), F-24 (`SectionOrder` unread). New at this close:

- **F-25 — the timing-gate script writes into the working tree** (`scripts/timing-gates.sh` →
  `.superpowers/sdd/2026-08-17-bible-atlas-m1/logs-timing-gates`), so a gate run leaves the tree dirty.
  Closure: the log dir under the target dir or `$TMPDIR`.
- **F-26 — `start-api.sh` builds into `server/target`**, ignoring the worktree's `CARGO_TARGET_DIR`
  convention unless the caller exports it; a worktree Playwright run can trigger a cold release build.
  Closure: the script honours `CARGO_TARGET_DIR` explicitly and the AGENTS.md procedure names it.
- **F-27 — the popover's `Title` prefers the legacy title over the served label** (`GEN` vs `Genesis`
  for the same node in the title and the trail). Category: the strangler bridge's labels (D7); closes
  per kind as FOCUS-2…9 retire the legacy nodes.
- **F-28 — a `FocusView` link with no form on its surface is dropped from the page but still counted**
  in the section heading (`mentioned-in (1)` with no link on the World surface). Closure: a surface's
  frontier is served per surface, or the count reads the offered page.
- **F-29 — `Root` and `Saved` are two optional popover parameters** (T8 deviation 6), not one sum
  `PopoverOpening = At | Along`.

## 10. Owed

1. The full Playwright run (§6), including the four re-expressed specs.
2. Codex's review of `678a0d2..<this report's commit>` on `lane/claude/A-F1`, then the landing under `land`.
3. R18's types (A-EDGES) for the owner's sign-off before FOCUS-2.
