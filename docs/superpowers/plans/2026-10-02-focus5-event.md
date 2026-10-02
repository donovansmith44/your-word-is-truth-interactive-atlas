# FOCUS-5 (Event) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Executor: Claude or Codex** (queue item `A-F5`/`CX-F5`, whichever the controller names). This plan is written so it can be run with no context beyond it. The executor touches only the files a task lists. Anything else found on the way goes to FINDINGS in the queue, never into the diff (rules 24a, AGENTS.md).

**Goal:** Move Event off the legacy popover onto FocusView and the generic presentation. Delete what spec §5 lists for it: the six `Event*Section`s (`EventProvenanceSection`, `EventDateAndPlacesSection`, `EventChronologySection`, `EventWitnessesSection`, `EventMentionsSection`, `EventAnaloguesSection`), `EventNode`, `/api/event/{id}` and `/api/narrative/event/{id}`. `ArrowNav` goes too: the arrows read the event's `follows-in`/`precedes-in`. Delete everything only they used: `INarrativeAware`, `NarrativeArrow`, `WitnessUnitsResolver`, `RefsList`, `EventAccounts`/`EventAccount`, `MapFocusHatch`, `YearNode` with `YearFrontierSection`, the two `AtlasClient` reads, and the popover's narrative-focus sync.

Fold in what the move closes, all on the server:
- the server works out an event's accounts on every neighbour page (`EventAccounts` in `graph.rs`);
- the server scans the whole chronology to find an event's prior and following event (`temporal_neighbors_of`);
- the client parses references and formats account references (`EventAccount`);
- F-54 (`MapFocusHatch`), F-57's EventNode/YearNode alias, F-63's event-accounts whole read, and the F-55 EVT-META-TOP-1 site.

**Architecture:** Rule 27 splits the work by who knows the inputs.
- **The compiler** writes what the data alone decides:
  - the timeline's order as succession (`follows-in`/`precedes-in`), not as a direction-less pair;
  - each account of an event as a passage Container, linked to its event (OPEN 1);
  - each narrative step's label naming its narrative.
- **The server** reads. It loses two view-shaped routes and the per-request account and timeline derivations. It gains no route and no read. Under OPEN 4(a), one map wire field carries the graph's own reference.
- **The client** derives the interaction:
  - the Event rows of `Presentation.Of` (a `Card` on the popover; under OPEN 4(a), a `Geography` on the World);
  - the arrows, one per served successor;
  - every list paged 20 at a time through the one root-aware door, and every follow through `Explore`.
- One relation is appended under OPEN 1(a) and one symmetric relation leaves under OPEN 2(a). Both are domain facts (an account narrates an event; the timeline is a succession), never a client construct.

**Tech Stack:** Rust (axum, utoipa, `atlas-etl`, `atlas-graph`, `atlas-contract`, `graph-types`); .NET 10 Blazor WebAssembly (`BibleAtlas.Client`, `BibleAtlas.Client.Exploring`); xUnit + bUnit; Playwright; Stryker.NET; cargo-mutants via `scripts/mutants-parallel.sh`.

**Spec:** `docs/superpowers/specs/2026-09-26-focus-exploration-design.md`:
- §3.3 (`Card`, `Geography`), §3.4 (`Affordances`), §5 (FOCUS-5 row), §6 (test ids: `event-*`, `popover-chip-map`), §8 (event chronology adjacency served as `follows-in`), §9;
- §12 R15–R20 (R19 the Explore monad, R20 `IPresenter`).

**Principles:** 4, 9, 12, 14b, 15–18, 21–23, 24–24b, 25, 26, 26a, 27–27g. **Queue:** FOCUS-5. Closes F-54, F-63 (event accounts), F-65 (`EventAccount`'s `CanonRef` sites), F-66 (`EventAccounts`, `MapFocusHatch` leave the core), F-57 (Event/Year alias), and F-55's EVT-META-TOP-1 site. Touches F-2/CX-R1 (not changed), F-53, F-56.

**Base:** verified against trunk `b3d7cfa` (FOCUS-1 + FOCUS-6 landed). **FOCUS-5 starts on the head FOCUS-2 lands at.** Several tasks also wait for named tasks of FOCUS-3 and FOCUS-4 (see "Overlap"). Task 0 writes the base into the ledger (`.superpowers/sdd/2026-10-0x-focus5/progress.md`) as this batch's `--base` (PRINCIPLES 22), and every gate takes it.

## OPEN: for the owner, before the task named starts (each blocks only that task)

Each is one line to answer. The plan builds the recommendation if unanswered.

1. **Parallel accounts (Task 4).** The graph links an event to every verse of every account (the Sermon on the Mount: 144 links), and the server regroups them into accounts on each request. Choose one:
   - (a) The compiler makes each account a passage (Matthew 5:1–7:29, Luke 6:17–49) linked to its event by a new link, "narrated in". The event then lists its 2 accounts, and a verse still lists its events directly.
   - (b) The event links only to its account passages. A verse then reaches its events through its passage, one step further, which goes back on your FOCUS-2 answer 3.
   - (c) No accounts: the event lists its 144 verses, 20 at a time.

   **Recommend (a).**
2. **Prior and next event in time (Task 3).** The timeline order is stored with no direction and worked out on every request. (a) Store it as "follows in"/"precedes in", the way a story's steps are stored. The event's arrows then show the next event in time and the next in each story it belongs to, and the direction-less "temporal adjacency" link goes (a contract major). (b) Keep it as it is: the arrows show only story steps, and the timeline neighbours are an unordered list. **Recommend (a).**
3. **Several arrows (Task 1).** An event in two stories can have three "next" arrows. (a) Each arrow shows the next event's title, and its ⋮ step names the story. (b) Each arrow shows the whole link label ("Exodus from Egypt · Follows in The Exodus · Crossing the Red Sea"). **Recommend (a).**
4. **Events on the map (Task 1).** Choose one:
   - (a) An event gets a map view. "Show on the map" opens the World at the event's years with its stories highlighted, and the slider's arrows step to the prior or next event. This needs one small map data addition.
   - (b) The same without the story highlight.
   - (c) No map view: the map button and the highlight go for events, and the event's places stay one click away.

   **Recommend (a).**
5. **What an event's card says (Task 1).** Show When, the KJV superscription (where there is one) and Source. The curator fields (harmony and section bookkeeping, the reference note) are not shown. **Recommend approve.**
6. **The verse list on an event (Task 1, only under 1a).** Should "Attested in" (every verse) open collapsed and last, as FOCUS-4 does for "Mentioned in"? **Recommend yes.**
7. **Account text (Tasks 2, 4).** An account is listed by its reference and opens its passage; its verse text is not shown inline on the event any more. The hover peeks under the arrows go too. **Recommend yes.**
8. **One label rule for every passage (Task 4).** Label every passage by its compiled run reference ("MRK.14.54, 66-72"), including FOCUS-3's cited spans: "GEN.29.32-30.24" instead of the source file's "GEN.29.32-GEN.30.24". **Recommend yes.**

**Rulings applied by analogy, not re-asked:**
- **FOCUS-2 answer 2** ("an event's parallel accounts stay"): binding. It is why OPEN 1 offers no "drop the accounts" option except (c).
- **FOCUS-2 answer 3 / FOCUS-4 ruling 3** (one group per edge kind, in served order, no client split): there is no CHRONOLOGY, PARALLEL ACCOUNTS or SIMILAR ACCOUNTS heading and no "PRIOR EVENT/FOLLOWING EVENT" role text. Each group is the served kind's display label with its count.
- **FOCUS-2 answer 2's principle** (a client-derived view one step away is dropped): the story-thread rows, which the client derived by comparing each narrative's legs with the timeline's, go. Every served successor is an arrow.
- **FOCUS-2 answer 7** (reach every served site of the category in the batch): every per-request event derivation in served code goes here: the accounts, the timeline scan, the mention-reference parse in `/api/event`.
- **FOCUS-2 answer 8** (keep the locator): account passages are labelled by locator runs ("MAT.5.1-7.29").
- **FOCUS-2 answer 9** (accept the AQC major): Tasks 3, 4 and 5 each declare one.
- **O-F6 (b)** (click only, no hover preview): the arrows' dwell peek dies.
- **O-F6 (c)** (a card ignores the time window): a place on an event shows its own served label, not a name resolved at the event's date.
- **O-F6 (e)** (crossing is an explicit click): under OPEN 4(a), the slider steps to the prior/next event only by its arrow.
- **Owner paging rulings:** every list shows 20 a page, More appends to 40 then slides, Less goes back 20 (`Affordances.PageSize`, `PageWindow.ShownEntries`). The page cap is the server's alone, and the first-page cursor is the contract default.

## Global Constraints

- `docs/PRINCIPLES.md` binds, especially:
  - rule 4: zero dead code (every member left without a production reader goes, verified by `grep` and recorded in the ledger);
  - rule 9: no comments in application code. No snippet below has one; a comment on a deleted line goes with it, and no other comment is touched;
  - rule 12: every signature below is for sign-off;
  - rules 24, 24a, 24b: every fix names its category and closes it; an offender found on the side goes to FINDINGS;
  - rule 25: the client composes over the contract. It parses no reference, composes no id, formats no year, and decides no kind from a string;
  - rules 26, 26a: domain facts live in `data/`; source parsing stays in the tools;
  - **rule 27**: data-only derivations are compiled, the server reads, interaction derivations are the client's, and the graph models the domain, never a view. No client exploration word (explore, explorable, frontier, card, popover, presentation) appears in `server/` or `graph-types/`; the vocabulary gate (`backend_vocabulary.rs`) reads every identifier, test names included.
- Tests:
  - whole-body assertions;
  - one behaviour per test, named as a sentence;
  - `// Arrange` `// Act` `// Assert` only;
  - no magic numbers;
  - newspaper order.
  - Real-data expectations are read from the artifact, never written as literals (F-8).
- **Total matches.** A closed sum is matched exhaustively. `Emphasis` gains an arm through its `Match<T>` (no `switch`; `ElementKindLawTests`). `Affordances.Of` and `Presentation.Of` stay total over the generated enums.
- **The one walk door.** Every follow goes `OnFollow(Link)` → `ExplorerPopover.FollowAsync` → `Explore.Follow`. There is no new caller of `IExplorer.Resolve` (`internal`, R20).
- **The one paging door.** Every list opens through `Paging.Window(PresentationRequest, EdgeKind)` (F-70). `RootConsistencyLawTests` stays green with one door.
- **Relations:** under OPEN 1(a), `Narrates` is appended **last** in `relations!` (`DECLARED_DIRECTED_RELATIONS` 22 → 23). Under OPEN 2(a), `TemporalAdjacency` leaves the symmetric list (`DECLARED_SYMMETRIC_RELATIONS` 7 → 6). Nothing else changes either list.
- Build no interaction that works only by hovering.
- Commit per task on `lane/<agent>/F5-t<n>`; integrate on `lane/<agent>/F5-int`. Landing on `worktree-bible-atlas-m1` is by cherry-pick under `land`, after the other agent's review. Never force.
- Every task runs in its own worktree (`git -c core.autocrlf=false worktree add -b lane/<agent>/F5-t<n> ~/w/F5-t<n> <base>`), with `CARGO_TARGET_DIR=~/mut/<agent>-F5-t<n>` and `nice -n 10 cargo -j 4`. Data is copied, never linked (AGENTS.md).

## Critical sections (PRINCIPLES 21): one holder at a time

| Section | Held by | Why |
|---|---|---|
| `contract`: rebuilding `data/compiled` | Task 3, then Task 4 | one artifact; the version root moves |
| `contract`: `export_contract`, `export_aqc_examples`, the AQC/AGC features, `client.ContractGenerator` | Task 1 Part A (OPEN 4a only), then Tasks 3, 4, 5, strictly in that order | one generated document |
| `contract`: re-blessing pacts and fixtures | right after each rebuild or regen | moves the version root |
| `heavy`: `cargo test --workspace`, full Playwright | Tasks 1A, 3, 4, 5, 7 | memory |
| `heavy` with "mutation" in the message | Task 7 only, inside the owner's window (`.superpowers/MUTATION-GATE-DEBT.md`) | once per batch (3a) |
| appending to `relations!` | Task 4 only (OPEN 1a), last position | positional |

## What an event's popover shows, before and after (acceptance)

Opened from a verse's `attests` link (FOCUS-2: `popover-link-attests-{Event id}`), from a pericope heading (`pericope-heading-{id}`), or from a place's `site-of` link. `popover-title` stays the served label. The body is FocusView (`popover-*` ids). Examples are read from the artifact in every spec (F-8).

| Legacy section (test ids today) | After FOCUS-5 (generic presentation) |
|---|---|
| `EventProvenanceSection`: `event-provenance-button`/`-panel`, confidence badge | `popover-section-card`: `popover-card-title` = label. Fields in this order, each only when served: `When` = `event.when.label` (`popover-field-When`); `Superscription` = `event.kjv_superscription` (OPEN 5); `Provenance` = the record's provenance (`popover-field-Provenance`), as for places. |
| `EventDateAndPlacesSection`: `event-time-value` (click → `YearNode` year chronology), `event-place-{id}` (→ `/world?…&place=` via `MapFocusHatch`) | Time: the `When` field, plain text. Under OPEN 4(a/b), `popover-chip-map` ("Show on the map") opens the World with the slider bounded to the event's window. Places: `popover-section-located-at`, `Located at (n)`, entries `popover-link-located-at-Place:{id}`, labelled by the place's served label (O-F6c). Following one opens the place on FocusView; its own `popover-chip-map` focuses it on the World. The year view is gone (`YearNode` dies). |
| `EventChronologySection`: `CHRONOLOGY` heading, `event-chrono-{prior,following}-event-global`, role lines, account `RefsList` under each arrow, `event-story-thread-*` rows, dwell peek | Arrows: one per served `precedes-in` entry (`popover-prev`, `‹ {label}`) and one per `follows-in` entry (`popover-next`, `{label} ›`). Each carries `data-entry="{neighbour id}"` and is followed by its ⋮ edge step (`popover-entry-edge-follows-in-{edge id}`), whose title is the compiled edge label naming its story (OPEN 3a). Under OPEN 2(a), the timeline's neighbour is one of the arrows. No heading, role text, refs or peek. |
| `EventWitnessesSection`: `PARALLEL ACCOUNTS` (`event-section-heading`), `popover-section-event-witness(es)`, `event-witness-{SPAN}` with inline verse text and expand | OPEN 1(a): `popover-section-narrated-in`, `Narrated in (n)`, one entry per account, `popover-link-narrated-in-{passage id}`, labelled by the passage's compiled run reference (`MAT.5.1-7.29`, `MRK.14.54, 66-72`). Following one opens the passage (`Card` + its `contains` verses; a verse opens with its text, FOCUS-2). There is no inline text (OPEN 7). One account or many renders the same way. |
| `EventMentionsSection`: `MENTIONED IN`, `event-mentioned-in-{vref}`, `event-mentions-provenance-*` | `popover-section-mentioned-in`, `Mentioned in (n)`, entries `popover-link-mentioned-in-text-unit:{ref}`. Each mention's source is its ⋮ edge. Under FOCUS-4's OPEN 6(b) the list is collapsed and last. |
| `EventAnaloguesSection`: `SIMILAR ACCOUNTS`, `event-analogues-{id}`, `event-analogues-provenance-*` | `popover-section-analogous-to`, `Analogous to (n)`, entries `popover-link-analogous-to-Event:{id}`. Each analogue's provenance is its ⋮ edge record. |
| `EventNode` chip `popover-chip-map` (when dated and placed) | OPEN 4(a/b): `popover-chip-map` on every event. For an undated event the World opens where it stands (`Frame.Current`). OPEN 4(c): gone. |
| — | Every other group the event serves appears too, in served order: `attested-in` (every verse; OPEN 6: collapsed and last), `participants`, `dated-by`, `dates`, and `shown-on` as an up-crumb (`popover-up-shown-on-Map:era-{id}`). |

Under OPEN 4(a), on the World: the slider is bounded to `event.when` with `Frame.Bounded(when, first precedes-in, first follows-in)`. The narratives the event belongs to are highlighted (`data-narrative-focus` on their arrows, as today). Leaving the event for a kind with no World presentation clears the highlight, and the slider stays where it is.

## Re-anchor table: what the event popover touches at `b3d7cfa`, and where each goes

| Today (`b3d7cfa`) | Role | FOCUS-5 |
|---|---|---|
| `client/Legacy/EventNode.cs` (56 lines; `INarrativeAware`, `EventNode`) | identity, map chip, `DetailAsync` over `/api/event`, `NarrativePositionsAsync` over `/api/narrative/event` | **deleted** (Task 2) |
| `client/Legacy/PopoverSectionProviders.cs:672–1099`: the six sections, `RenderArrowNav`, `NarrativeArrow`, `WitnessUnitsResolver` | event popover body | → FocusView groups (Task 1); **deleted** (Task 2) |
| `PopoverSectionProviders.cs:143–` `YearFrontierSection`; `client/Legacy/YearNode.cs` (60 lines) | the year view, built only by `EventDateAndPlacesSection` (`:736`) | **deleted** (Task 2; rule 4, pulled forward from FOCUS-9) |
| `client/Components/ArrowNav.razor` (428 lines), `client/Components/RefsList.razor` (22) | arrows, refs, peek | arrows → `Affordance.Arrows` (Task 1); **deleted** (Task 2) |
| `client/Exploring/EventAccounts.cs` (`EventAccount`, `EventAccounts`: `Paging.Whole` over `attested-in`, `CanonRef` formatting) | accounts on the client | accounts compiled (Task 4); **deleted** (Task 2) |
| `client/Exploring/MapFocusHatch.cs` | world query from a local place id (F-54) | **deleted** (Task 2) |
| `client/Legacy/PassageBlock.cs` `AccountSourceUnit`, `AccountBlock`, `VerseTextResolver.ResolveSpansAsync`/`ResolveGroupsAsync` | account blocks, peek verses | **deleted** (Task 2); `PassageSourceUnit`, `ResolveAsync`, `PassageList` stay (FOCUS-7, MAPS) |
| `client/Components/ExplorerPopover.razor:446–471` `SyncNarrativeFocusAsync`, `FocusMap`, `NarrativeFocus` | map highlight from `/api/narrative/event` | → `Emphasis.Narrative` on the World (OPEN 4a) or dropped (4b/c); **deleted** (Task 2) |
| `client/Components/PericopeHeading.razor:21` | opens `new EventNode(...)` | `PopoverOpening.Explore(new NodePosition(heading's served event))` (Task 2) |
| `client/Legacy/LegacyNodes.cs:26` `NodeKind.Event => new EventNode(...)` | bridge | → `null` (Task 2) |
| `client/Legacy/PopoverSections.cs:57–60, 63, 64` | six registrations | removed (Task 2) |
| `client/Exploring/PopoverChromeRegistry.cs:23` `["Event"]` | chip ids | removed (Task 2) |
| `client/AtlasClient.cs:139–143` `NarrativeEventPositions`, `Event` | legacy reads | **deleted** (Task 2) |
| `client/Exploring/Affordances.cs` `Arrows(...) : Affordance(ArrowsShown)` | one arrow per direction | every successor (Task 1); `ArrowsShown` deleted |
| `client/Exploring/Geography.cs` `Emphasis` | None, Site, Territory | + `Narrative` (Task 1, OPEN 4a) |
| `client/Exploring/Link.cs` `Entry(Link Neighbour, Link Edge)` | a served neighbour | + `Narrative` (Task 1, OPEN 4a) |
| `server/atlas-contract/src/events.rs` (189 lines), `wire/events.rs` (71) | `/api/event`, `/api/narrative/event` | **deleted** (Task 5) |
| `server/atlas-contract/src/graph.rs:313–392` `EventAccounts`, `AccountOf`, the two `Attests` arms of `node_edges` (with two `panic!`s) | accounts per request | compiled (Task 4); **deleted** |
| `server/atlas-contract/src/graph.rs:90, 209` `event_detail` via `event_from_node` (drains `located-at`) | the event record | reads the payload and the compiled chronology only (Task 5) |
| `server/atlas-graph/src/service.rs:479` `temporal_neighbors_of` (scans `chrono.order`), `analogue_provenance`, `attests_provenance`, `event_mentions_provenance` | per-request timeline and provenance | **deleted** (Task 5) |
| `server/atlas-graph/src/event_world.rs:475` `populate_temporal_adjacency`; `graph-types` `TemporalAdjacency` row → `S::TemporalAdjacency` | the timeline, direction-less | rows lower into `Succession` (Task 3, OPEN 2a) |
| `server/atlas-graph/src/event_world.rs:369–386` per-verse `Attests` from `accounts_of` | verse-level attestation | kept; plus one passage and one `Narration` row per account (Task 4, OPEN 1a) |
| `server/atlas-core/src/narrative.rs` (`NarrativeAdjacentEvent`, `adjacent_event`, `TimelinePosition`, `global_timeline_position`) | the route's types | **deleted** if no reader remains (Task 5; `cargo build` decides; its one test reader goes with it) |

**Call sites counted at `b3d7cfa`:**
- `EventNode`:
  - constructed at `LegacyNodes.cs:26`, `ArrowNav.razor:152`, `PericopeHeading.razor:21`, `YearNode.cs:43`, and `PopoverSectionProviders.cs` `:78` (`ChapterCardSection`, FOCUS-3 T8), `:642` (`VerseEventMembershipSection`, FOCUS-2 T4), `:903` (analogues) and `:1559` (`PersonEventsSection`, FOCUS-4 T4);
  - type-tested in the six sections;
  - `INarrativeAware` at `ExplorerPopover.razor:448`;
  - tests `LegacyViews.cs:21`, `LegacyNodesTests.cs:47, 88, 106, 110`.
- The six sections:
  - registered at `PopoverSections.cs:57–60, 63, 64`;
  - named by `PopoverSectionRegistryTests.cs:89–136`, `PushViaConformanceTests.cs:25–29` and `FrontierMatrixConformanceTests.cs:82–84, 118–120` (that file dies in FOCUS-2 T5).
- `ArrowNav`: `RenderArrowNav` (`PopoverSectionProviders.cs`), `ArrowNavTests.cs`.
- `RefsList`: `ArrowNav`, `EventMentionsSection`, `EventAnaloguesSection`, `ComponentParameterLawTests.cs`.
- `EventAccounts`/`EventAccount`: `EventWitnessesSection`, `VerseParallelsSection` (FOCUS-2 T4), `ArrowNav`, `PassageBlock.AccountSourceUnit`; tests `EventAccountsTests.cs`, `ArrowNavTests.cs`, `PassageBlockTests.cs`.
- `MapFocusHatch`: `PopoverSectionProviders.cs:744`; `MapFocusHatchTests.cs` (3).
- `YearNode`: `PopoverSectionProviders.cs:148, 736`; `YearNodeEventTimeTests.cs`.
- `AtlasClient.Event`/`.NarrativeEventPositions`: `EventNode.cs:50, 55`; `AtlasClientTests.cs:319–337`.
- Server:
  - `/api/event`: `tests/api.rs` (12), `graph_api.rs` (10), `contract_api.rs` (1), `aqc_export.rs:42`, `features/event-page.feature`, `atlas-edge/events.feature`, `pacts/http.json`, `CHANGELOG.md`.
  - `/api/narrative/event`: `api.rs` (6), `graph_api.rs` (6), `contract_api.rs` (2), `atlas-edge/narratives.feature`, `atlas-graph-contract/graph/detail-routes.feature`, `pacts/http.json` (2).
  - `benches/queries.rs` (3) and `server/BENCHMARKS.md` (1).
  - `GraphService` provenance and timeline methods: `tests/port_widening_real_data.rs` (6).
- Tests: `tests/ux/lib/api.ts:29` `narrativeEventPositions` (23 callers), `:32` `event` (63 callers; listed in Task 6).

**Playwright specs that cover the event popover (`tests/ux/`), re-expressed in Task 6:**
- `event-timeline.spec.ts`: every test L71–L847; L899 is verse-only and FOCUS-2's.
- `event-timeplace.spec.ts`: the whole file (L51, L92 EVT-META-TOP-1, L105, L115, L137, L174).
- `popover-sections.spec.ts`: L276 HATCH-DELIVERABLE-1, L371 READER-1, L1673 EVENT-1, L1817, L1940/L1977/L1999/L2038 ACCT-COALESCE-1, L2085, L2131, L2210, L2234.
- `accounts-and-mentions.spec.ts`: L69, L118, L193.
- `provenance.spec.ts`: L78, L102, L353 (wire only).
- `reader-headings.spec.ts`: L19, L79.
- `world-pin.spec.ts`: L96, L121, L154 (still present, though `CONTRACT.md:4965` says removed).
- `world-narrative-focus.spec.ts`: L20.
- `w1-passages` L11, L49; `w2` L10, L60, L82; `w3` L21, L46; `w4` L13, L35, L61; `w5` L13, L58.
- `frontier-matrix.spec.ts`: deleted by FOCUS-2 T5.
- **Stay green unchanged** (they assert only title, heading text or chrome): `reader-headings` L48, L109, L123, L160, L174, L191, L229; `w1` L66, L102; `w2` L96, L122; `w3` L77, L115; `w5` L81; `saved-explorations` L87; `explore-edges` L12; `world-hover-text` L193; `reader-recursion` L158.
- `tests/ux/CONTRACT.md`: PERI-1 (L68), UX-1 (L441, L461), EVT-3 (L532–873), the inventory's event lines (L1495, L1715–1832, L1918–1935), EVENT-1 (L3012–4065), CHRONO-1, PEEK-1, PEEK-TRUNC-1, TITLE-WRAP-1, CHRONO-MERGE-1, EV-1, ONE-RULE, AFFORDANCE-1, PIN-1/TRAVERSAL (L4944–4965), ATTEST-1 (L6498–6571): rewritten in Task 6.

## Overlap with FOCUS-2 (before this), FOCUS-3 and FOCUS-4 (beside it)

FOCUS-5 runs after FOCUS-2 and beside FOCUS-3 (Claude) and FOCUS-4 (Codex). Rule: **FOCUS-2 goes first on every shared file. On a file FOCUS-3 or FOCUS-4 also edits, FOCUS-5's task starts after the named task has landed on its integration branch, and whoever lands second rebases.** Row files (registries, law fixtures, CSS, specs) need only a rebase: each batch deletes its own rows.

| File | FOCUS-2 | FOCUS-3 | FOCUS-4 | FOCUS-5 | Order |
|---|---|---|---|---|---|
| `client/Exploring/{Presentation,Presenter}.cs`; `client.Tests/Explore/{GraphPresenterTests,PresentationTests,ServedGraph}.cs` | T3 (`Text`) | T5 (`Sequence`, book fields) | T3 (person fields, `Field.At`) | T1 (event fields, World row) | F2 T3 → F3 T5 → F4 T3 → F5 T1 |
| `client/Views/FocusView.razor`, `client/Views/ReadingArrows.razor` (new in F3), `client.Tests/Views/{FocusViewTests,ReadingArrowsTests}.cs` | T3 | T5 (extracts arrows) | T3 (`EntryText`, `Disclosure`) | T1 (every successor) | after F3 T5 and F4 T3 |
| `client/Exploring/Affordances.cs`, `AffordancesTests.cs` | — | maybe | T3 (`SectionList` members) | T1 (`Arrows` clamp; `AttestedIn` under OPEN 6), T3 (−`TemporalAdjacency`), T4 (+`Narrates`/`NarratedIn`) | after F4 T3 |
| `client/Exploring/{Geography,Link,Explorable}.cs`, `client/MapInterop.cs`, `client/wwwroot/js/map.js`, `client/Pages/World.razor` | — | T6 (`World.razor:76` picker) | — | T1 | after F3 T6 |
| `client/Components/ExplorerPopover.razor` | T4 (`XrefEntryPoint`) | listed | T3 (`OnShowYear`) | T2 (narrative sync deleted) | after F2 T4, F4 T3 |
| `client/Legacy/{PopoverSectionProviders,PopoverSections,LegacyNodes}.cs` | T4 (verse sections; `:642`, `:859`) | T8 (`ChapterCardSection` `:78`) | T4 (person sections; `:1559`) | T2 (event sections, `YearFrontierSection`) | row file; if F3 T8 / F4 T4 have not landed, F5 T2 rewrites their one `new EventNode` line to `PopoverOpening.Explore(new NodePosition(…))` and the later batch deletes it with its section |
| `client/Components/PericopeHeading.razor`, `client/Components/MiniReaderExpand.razor`, `client/Pages/Reader.razor` | T4 (openings) | T6 (headings from `TextUnit.heading`) | — | T2 (`PericopeHeading.Open` only) | after F3 T6 |
| `client/Legacy/PassageBlock.cs`, `client/Components/PassageList.razor`, `client/Exploring/CanonRef.cs` | T4 | T8 (`PassageList.ExploreNodeOf`, `CanonRef` passage sites) | — | T2 (account members) | after F3 T8 |
| `client/AtlasClient.cs`, `client.Tests/AtlasClientTests.cs` | T4 (`Verse`) | T8 (`Books`, `Chapter`, `Xrefs`, `Catechism`) | — | T2 (`Event`, `NarrativeEventPositions`) | row |
| `client/Exploring/PopoverChromeRegistry.cs`, `client.Tests/PopoverChromeConformanceTests.cs` | (FINDING) | rows | T4 (Person) | T2 (Event) | row |
| `client.Tests/Explore/{DeletionLawTests,LegacyNodesTests,LegacyViews,IdentityTests,PushViaConformanceTests,LegacyPresentationsTests}.cs`, `client.Tests/PopoverSectionRegistryTests.cs` | rows | rows | rows | T2 (Event rows) | row |
| `client.Tests/{ReferenceParsingLawTests,WholeReadLawTests}.cs` (F2 T5 creates) | create | T8 | T4 | T2 | ratchets shrink; F5 also relabels the entries its own deletions leave (below) |
| `client/wwwroot/css/app.css`, `client/wwwroot/js/reader.js` | rows | rows | rows | T2 | row |
| `server/atlas-graph/src/labels.rs` | T2 (`object_label`) | T3 (citations), T4 (passages) | T2 (`relation_label`) | T3 (narrative named), T4 (passage label rule) | F2 T2 → F4 T2 → F3 T3/T4 → F5 T3 → F5 T4 |
| `server/atlas-graph/src/references.rs` (new in F2) | T1A | — | — | T4 (`runs_reference`) | after F2 T1 |
| `graph-types/src/edge.rs` | — | T4 (`passage_container_id`) | — | T3 (relations, row family rename), T4 (`Narrates`, `Narration`, passages over runs) | after F3 T4 |
| `graph-types/src/{graph,sections}.rs`, `graph-types/src/canon/*`, `graph-types/tests/common/mod.rs` | T1A (`graph.rs`) | T4 | T1 (`node.rs`, canon) | T3, T4 | after F3 T4 and F4 T1 |
| `server/atlas-graph/src/event_world.rs` | — | — | T1 (`add_justified_by`, `grounds_of`) | T3, T4 | after F4 T1 (a new family gets its `grounds_of` arm) |
| `server/atlas-graph/src/{law_check,provenance}.rs`, `server/atlas-graph/src/sqlite/{ddl,partition,writer}.rs` | T1A (sqlite) | T4 (`law_check`) | T1 (`law_check`) | T3, T4 | after F3 T4, F4 T1 |
| `server/atlas-graph/src/service.rs` | T1A | — | T1 | T5 | after F4 T1 |
| `server/atlas-contract/src/graph.rs` | T1A/B (`unit_text`, references) | T2 (`node_text`) | T1B (person arm) | T4 (attests arms), T5 (`event_detail`) | after F3 T2, F4 T1B |
| `server/atlas-contract/src/events.rs` | T1A (`encode_node_id` threading) | — | — | T5 (deleted) | after F2 T1 |
| `server/atlas-contract/src/{lib,aqc_export}.rs`, `wire/mod.rs`, `tests/{api,graph_api,contract_api,contract_coverage,contract_pact}.rs`, `benches/queries.rs`, `server/BENCHMARKS.md` | T6 | T9 | T1 | T5 | `contract` lock order |
| `server/atlas-core/src/wire.rs` (`SceneArrow`), `server/atlas-core/src/scene.rs` | — | — | — | T1A (OPEN 4a) | alone |
| `contracts/*`, `data/compiled`, pacts | many | many | many | T1A, T3, T4, T5 | `contract` lock |
| `tests/ux/*.spec.ts`, `tests/ux/lib/api.ts`, `tests/ux/CONTRACT.md` | T7 | T10 | T5 | T6 | re-expressions on top of the earlier batches' |

**Ratchet relabels FOCUS-5 owes (found in this plan's grounding):**
- FOCUS-3 lists `MiniReaderExpand` and `VerseTextResolver` as retiring in FOCUS-5 (its OPEN 2 ratchet on `/api/text?ref=`), and `PassageList` as FOCUS-5's.
- They do not retire here. `PassageList` and `MiniReaderExpand` still serve `CatechismScripturesSection` (FOCUS-7) and `PolityDeltaScripturesSection` (MAPS), and `VerseTextResolver.ResolveAsync` serves `PolityDeltaScripturesSection`.
- Task 2 deletes only the event callers and relabels those entries `FOCUS-7`/`MAPS`. This is recorded in the close report and as a FINDING for FOCUS-3's table.

## Types (for sign-off, PRINCIPLES 12)

### The timeline is succession (Task 3, OPEN 2a)

`graph-types/src/edge.rs`:
```rust
pub struct ChronologicalSuccession {
    pub earlier: EventId,
    pub later: EventId,
    pub provenance: ProvenanceId,
}
```
- `TemporalAdjacency` (the row struct) is renamed `ChronologicalSuccession`, and its row family `RowFamily::TemporalAdjacency` becomes `RowFamily::ChronologicalSuccession`. The fields are unchanged.
- It lowers into `EdgeRel::Directed(RelationId::Succession)` from `earlier` to `later` with `EdgeMeta::None`, the third family beside `Succession` and `CanonSuccession` (the `MapSuccession` precedent).
- `SymRelationId::TemporalAdjacency` leaves `relations!`. The symmetric list is positional, so `edge_index.rel` codes after it move, and `SECTION_SCHEMA_VERSION` 22 → 23 (the DDL table is renamed `chronological_succession`).
- `DECLARED_SYMMETRIC_RELATIONS` 7 → 6. The published `EdgeKind` loses `temporal-adjacency`: AQC **major**, AGC **major**.
- The edge id is `(relation, subject, object)`. A timeline step and a narrative step between the same two events are therefore one edge with two rows, as two narratives sharing a step already are; `rows_of_edge` answers both. The entry's `narrative` and the edge's label read the first row (FINDING below).

`server/atlas-graph/src/labels.rs` (on FOCUS-4 T2's `relation_label`):
```rust
fn relation_label(record: &EdgeRecord, names: &ReaderNames) -> String;
```
- One new arm: a `Succession` record whose `meta` is `EdgeMeta::Narrative(n)` reads `"{kind display label} {names.narratives[n]}"`, which gives "Follows in The Exodus". `ReaderNames` gains `narratives: BTreeMap<NarrativeId, &'a str>`, filled from the Narrative nodes' compiled labels.
- A timeline step (`EdgeMeta::None`) reads the kind's display label, as a canon step does. Labels are data-only, so they are compiled (rule 27).

### Accounts are passages (Task 4, OPEN 1a, 8)

`graph-types/src/edge.rs`:
```rust
relations! { …, Narrates => "narrates" / "narrated-in" }

pub struct Narration {
    pub account: ContainerNodeId,
    pub event: EventId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

pub fn passage_container_id(runs: &[BibleLocusRange]) -> ContainerNodeId;
```
- `Narrates` is appended **last** among the directed relations (`DECLARED_DIRECTED_RELATIONS` 22 → 23, `DECLARED_EDGE_KINDS` 51 → 52 with OPEN 2a's removal). `Narration` is a Core-section row family. Its `justification.text` is the account's curated note (what `Attests` rows carry today); it is not shown (OPEN 5).
- `passage_container_id` generalizes FOCUS-3 T4's `passage_container_id(first, last)` to a set of runs. A cited span is the one-run case, and FOCUS-3's call sites pass `&[range]`. So a span that is both cited and an account is **one** passage (the container algebra: overlap is legal, identity is the verse set).
- The compiler (`event_world::populate_nodes_and_direct_rows`, where `accounts_of` already runs) does three things for each account:
  - it mints the passage once, with one `contains` row per verse in canon order;
  - it writes one `Narration` row;
  - it keeps the per-verse `Attests` rows, so a verse's `attests` group is unchanged (FOCUS-2 answer 3).
- The account's runs come from the account's verses through the existing `runs::coalesce`, at compile time.

`server/atlas-graph/src/references.rs` (on FOCUS-2 T1's file):
```rust
pub fn runs_reference(runs: &[BibleLocusRange]) -> String;
```
- This is the one passage-label function: `"MAT.5.1-7.29"`, `"MRK.14.54, 66-72"`, `"PSA.14.1-7"`. It is the format `EventAccount.Reference` computes on the client today, so it moves to the compiler and that rule-25 site closes.
- Every passage, the cited spans included, is labelled by it (OPEN 8). FOCUS-3 T4's `target_display` label is replaced and the cites row keeps `target_display` for provenance only.
- `labels::node_label`'s passage arm calls it.
- `grounds_of` (FOCUS-4 T1) gains the `Narration` arm (it records a `Justification`), with no wildcard (24b).

`server/atlas-contract/src/graph.rs`:
- `node_edges` loses its two `Attests` arms and `EventAccounts`/`AccountOf`/`account_verses` (with their two `panic!`s).
- An `attested-in` or `attests` entry now carries no `loci` and no `note`. The account is a node with its own runs, so those fields restated it (14b) and were a per-request derivation over data alone (rule 27).
- `EdgeEntry`'s published description is reworded (type-level `#[schema(description)]` only, per the A-STRIP ruling). AQC **major** (FOCUS-2 answer 9 by analogy).

### The event record reads no neighbours (Task 5)

`server/atlas-contract/src/graph.rs`:
```rust
fn event_detail(payload: &NodePayload, when: Option<TimeRange>) -> Option<wire::EventDetail>;
```
- `read_node_record`'s event arm reads the node payload and `graph.chronology.chrono.resolved` for `when`, and nothing else. `event_from_node` (which drains `located-at` per record, 27b) is no longer called from the record. It keeps its other readers (`scene_source.rs`, `sqlite/extras.rs`) until MAPS.

### Map references on the scene's arrows (Task 1 Part A, OPEN 4a only)

`server/atlas-core/src/wire.rs`:
```rust
pub struct SceneArrow {
    pub from_node: NodeRef,
    pub to_node: NodeRef,
}
```
- Added beside `from_event`/`to_event`, read from the compiled label table the way FOCUS-6 gave `ScenePlace`, `QuietPlace`, `Polity` and `Era` their `node`. The map compares a highlighted event by the graph's own reference and composes no id (rule 25, the F-31 category). AQC **minor**.
- `from_event`/`to_event` stay while `/api/scene` is view-shaped (F-34, MAPS).

### The client (Tasks 1, 2)

`client/Exploring/Affordances.cs`:
```csharp
public sealed record Arrows(ArrowDirection Direction) : Affordance(Affordances.PageSize);
```
- `ArrowsShown` is deleted. An arrows group shows every entry of its window (bounded by the page, 27e). A chapter has one successor, so the reader is unchanged.
- `Of` changes by OPEN:
  - Task 3 (OPEN 2a) deletes the `TemporalAdjacency` arm (the generated enum no longer has it).
  - Task 4 (OPEN 1a) adds `Narrates or NarratedIn => DefaultList`.
  - OPEN 6 (with FOCUS-4's `Disclosure`): `AttestedIn => Affordances.Mentions`, FOCUS-4's collapsed list. If FOCUS-4 renames that constant, use its name.

`client/Exploring/Link.cs`:
```csharp
public sealed record Entry(Link Neighbour, Link Edge, string? Narrative);
```
- `Narrative` is the served `EdgeEntry.narrative` (a `NarrativeId` string), set by `Explorable.Entries` and null elsewhere. It exists only under OPEN 4(a), for `Emphasis.Narrative`; under 4(b/c) it is not added (rule 4).

`client/Exploring/Geography.cs`:
```csharp
public sealed record Narrative(NodeRef Event, IReadOnlyList<string> Narratives) : Emphasis;

public abstract T Match<T>(Func<T> none, Func<NodeRef, double, double, T> site, Func<NodeRef, TimeRange, T> territory, Func<NodeRef, IReadOnlyList<string>, T> narrative);
```
- `Narrative` exists only under OPEN 4(a). Its equality is `PositionIdentity.Comparer` over `Event` plus `SequenceEqual` over `Narratives`, as `Site`/`Territory` do.

`client/Exploring/Presentation.cs` (OPEN 4a/b): one row changes. `Of(Node(Event), World)` is `Form.Geography` (it was `null`), so `HomeSurfaces.Of(Event)` becomes `World` and FocusView offers `popover-chip-map` on an event.

`client/Exploring/Presenter.cs` (`GraphPresenter`):
```csharp
private const string WhenField = "When";
private const string SuperscriptionField = "Superscription";
```
- `CardOf` adds, before `Provenance`: `Field(WhenField, element.Record?.Event?.When?.Label)` and `Field(SuperscriptionField, element.Record?.Event?.KjvSuperscription)` (OPEN 5). No year is formatted.
- `GeographyOf`'s `Event` arm (OPEN 4a/b) has two cases:
  - served `when`: `new Presentation.Geography(new Frame.Bounded(when, await Paging.FirstLink(element, EdgeKind.PrecedesIn), await Paging.FirstLink(element, EdgeKind.FollowsIn)), Emphasis)`;
  - no `when`: `new Presentation.Geography(new Frame.Current(), Emphasis)`.

  Under 4(a), `Emphasis` is `new Emphasis.Narrative(RefOf(record), narratives)`. The narratives are the distinct non-null `Entry.Narrative`s of the first `follows-in` and `precedes-in` pages, read through `element.Entries` (two reads, cached by `ServedPages` and shared with FocusView's windows). Under 4(b) it is `new Emphasis.None()`.

`client/MapInterop.cs`: `Emphasize`'s new arm sends `{ narratives, event = event.Id }`. `client/wwwroot/js/map.js` `setEmphasis` routes it to the arrows' `setFocus`, comparing `from_node.id`/`to_node.id`. The static `SetNarrativeFocus` is deleted.

`client/Pages/World.razor` `PresentTheFocus`: a current element with no World presentation applies `new Emphasis.None()` and keeps the frame, so the highlight clears when the event is left. Today it leaves the old emphasis in place; FOCUS-6's own places get the same clearing, which the ledger records.

`client/Views/FocusView.razor` (or `ReadingArrows.razor` once FOCUS-3 T5 has extracted the arrows):
- Every entry of an arrows group renders as `<button data-testid="{handle}-{prev|next}" data-entry="{neighbour id}">`, followed by its `EdgeStep`.
- `{handle}-prev`/`-next` stay the ids (FOCUS-3's `reader-*` specs are untouched); `data-entry` tells several arrows apart.
- Under OPEN 3(b) the label is `Label(entry.Edge)`; under 3(a) it stays `Label(entry.Neighbour)`.

`client/Components/PericopeHeading.razor`: `Open` is `OnExplore.InvokeAsync(new PopoverOpening.Explore(new NodePosition(Heading.Event)))`, over FOCUS-3 T6's served `UnitHeading.event`. The test id is unchanged.

## Deletion inventory (FOCUS-5 total, with OPEN defaults)

- **Client, files:**
  - `client/Legacy/EventNode.cs`, `client/Legacy/YearNode.cs`;
  - `client/Components/ArrowNav.razor`, `client/Components/RefsList.razor`;
  - `client/Exploring/EventAccounts.cs`, `client/Exploring/MapFocusHatch.cs`;
  - tests `client.Tests/ArrowNavTests.cs`, `EventAccountsTests.cs`, `MapFocusHatchTests.cs`, `YearNodeEventTimeTests.cs`.
- **Client, members:**
  - the six `Event*Section`s, `NarrativeArrow`, `WitnessUnitsResolver`, `YearFrontierSection` and their registry rows;
  - `PassageBlock`'s `AccountSourceUnit`, `AccountBlock`, `VerseTextResolver.ResolveSpansAsync`/`ResolveGroupsAsync`, and `PassageBlockData.FirstRangeEndVref` with its `PassageList` reader if no other block sets it (Task 2 verifies);
  - `CanonRef` members left with no reader (`SpanOf`, `Covers`, `VerseOf`, `LastVerseOf`: Task 2 greps each);
  - `AtlasClient.Event`, `.NarrativeEventPositions` and their two `AtlasClientTests`;
  - `ExplorerPopover.SyncNarrativeFocusAsync`, `FocusMap`, `NarrativeFocus`; `MapInterop.SetNarrativeFocus`;
  - `LegacyNodes.For`'s Event arm (→ `null`); `PopoverChromeRegistry["Event"]`;
  - `Affordances.ArrowsShown`; the `TemporalAdjacency` arm (Task 3);
  - the Event rows in `PopoverSectionRegistryTests`, `PushViaConformanceTests`, `LegacyNodesTests`, `LegacyViews`, `IdentityTests`, `LegacyPresentationsTests`, `PopoverChromeConformanceTests`;
  - `ComponentParameterLawTests`' `RefsList` case;
  - `reader.js` `getElementRect`, `isPointInsideEither` (ArrowNav their only caller);
  - the `app.css` rules only these used (`.popover-event-nav*`, `.popover-arrow-peek*`, `.popover-story-thread*`, `.event-timeline-heading`, the refs list).
- **Server (Task 5):**
  - `server/atlas-contract/src/events.rs`, `server/atlas-contract/src/wire/events.rs` (`NarrativeEventPositions`, `NarrativePosition`, `EventPage`, `EventAnalogue`) and their `lib.rs`/`wire/mod.rs` lines;
  - `server/atlas-core/src/narrative.rs` (if `cargo build` finds no other reader; `atlas-graph/tests/narrative_real_data.rs`'s `global_timeline_position` case goes with it);
  - `GraphService::{temporal_neighbors_of, analogue_provenance, attests_provenance, event_mentions_provenance}` and their `port_widening_real_data.rs` cases;
  - `aqc_export.rs`'s `event-page-ab-ur`; `contracts/atlas-query-contract/features/event-page.feature`;
  - the event lines of `atlas-edge/{events,narratives}.feature` and `atlas-graph-contract/graph/detail-routes.feature`;
  - the pact interactions; the `api.rs`/`graph_api.rs`/`contract_api.rs` cases; `benches/queries.rs`'s two benches and `BENCHMARKS.md`'s row;
  - `no_legacy_event_reads.rs`'s `adjacent_event` law (its subject is gone).
- **Server (Task 4):** `EventAccounts`, `AccountOf`, `account_verses` and the `Attests` arms in `graph.rs`.
- **Graph (Task 3):** `SymRelationId::TemporalAdjacency`; `populate_temporal_adjacency` becomes `populate_chronological_succession`.
- **Not deleted** (stated so no one "finishes" it):
  - `PassageList`, `PassageSourceUnit`, `MiniReaderExpand`, `VerseTextResolver.ResolveAsync` (FOCUS-7, MAPS);
  - `DwellTiming` (Reader);
  - `AtlasClient.NodeRecord` (`AuthorNode`, FOCUS-9);
  - `LegacySaves`' v1 `"Event"`/`"TimeAndPlace"` translations (FOCUS-9);
  - `event_from_node` (`scene_source.rs`, `extras.rs`: MAPS);
  - `GraphService.chronology` (scene and heading index: MAPS, FINDING);
  - `drain_edges` (`contents.rs`);
  - `scene::EventWitness`, `witnesses_for` (scene, event merge);
  - the per-verse `Attests` rows;
  - `attestation_pending.rs` (F-2, CX-R1).

---

### Task 0: Base and preconditions (no code)

**Files:** `.superpowers/sdd/2026-10-0x-focus5/progress.md` (the ledger; create).

- [ ] **Step 1:** Record the base (the head FOCUS-2 landed at), the OPEN answers (or "defaults"), and which FOCUS-3/FOCUS-4 tasks have landed.
- [ ] **Step 2:** Re-verify at the base, recording each result:
  - `grep -rn "EventNode\|INarrativeAware\|Event[A-Za-z]*Section\|ArrowNav\|EventAccount\|MapFocusHatch\|YearNode\|RefsList" client client.Tests --include=*.cs --include=*.razor` matches this plan's call-site list, minus FOCUS-2's `:642` and `:859` and, if landed, FOCUS-3's `:78` and FOCUS-4's `:1559`;
  - FOCUS-2's `LegacyTextUnits`, `ReferenceParsingLawTests`, `WholeReadLawTests` and `Presentation.Text` exist.

  If any differs, list the difference in the ledger and stop for the controller.

### Task 1: An event on FocusView: its card, every successor as an arrow, and (OPEN 4a) its map view (client; Part A backend under OPEN 4a)

**Backend change:** under OPEN 4(a) only, one wire field (`SceneArrow.from_node`/`to_node`). **Why there (rule 27, 25):** the map must compare the graph's own reference, and only the server holds it (compiled labels). No read is added. Everything else is presentation over served data: the client's under rule 27.

**Owner gate first:** OPEN 3, 4, 5, 6. **Starts after** FOCUS-2 T3, FOCUS-3 T5 (arrows extracted) and T6 (`World.razor`), and FOCUS-4 T3 (`Disclosure`, for OPEN 6).

**Files (Part A, OPEN 4a):**
- Modify:
  - `server/atlas-core/src/wire.rs` (`SceneArrow`);
  - `server/atlas-core/src/scene.rs` (fills both refs);
  - `server/atlas-contract/src/map.rs` (passes the snapshot, as FOCUS-6 did for places);
  - `contracts/*` regen, `contracts/atlas-query-contract/CHANGELOG.md` (AQC minor); pacts.
- Test: `server/atlas-contract/tests/graph_api.rs`, `scene_byte_identity.rs` (the recorded scene gains the two fields: **stop for the owner** if any other byte moves, O-GOLDEN rule).

**Files (Part B):**
- Modify:
  - `client/Exploring/{Affordances,Geography,Link,Explorable,Presentation,Presenter}.cs`;
  - `client/Views/FocusView.razor` or `client/Views/ReadingArrows.razor`;
  - `client/MapInterop.cs`, `client/wwwroot/js/map.js`, `client/Pages/World.razor`;
  - `client.Tests/stryker-config.exploring.json` (verify `Geography.cs`, `Presenter.cs` covered).
- Test:
  - `client.Tests/Explore/{AffordancesTests,GraphPresenterTests,PresentationTests,GeographyPresentationTests,ExplorableTests,ElementKindLawTests,ServedGraph}.cs`;
  - `client.Tests/Views/{FocusViewTests,ReadingArrowsTests}.cs`;
  - `client.Tests/Geography/EmphasisPayloadTests.cs` (create: `Emphasize` has no unit test today; `tests/ux/world-geography.spec.ts` is its only cover).

- [ ] **Step 1 (A, OPEN 4a): Failing test:** `graph_api.rs` `every_scene_arrow_names_its_two_events_by_their_graph_reference` (a scene window read against the artifact; each `from_node`/`to_node` equals `node_ref` of `from_event`/`to_event`).
- [ ] **Step 2 (A):** red → implement → regenerate (`contract`: `export_contract`, `export_aqc_examples`, `--check` clean, CHANGELOG minor, re-bless, `client.ContractGenerator`) → (lock `heavy`) `cargo test --workspace`, `bash scripts/contract-gate.sh --base <base>` → green. **Commit:** `scene: an arrow names its two events by the graph's own reference (rule 25, F-31 category)`.
- [ ] **Step 3 (B): Failing tests.**
  - `GraphPresenterTests`:
    - `An_event_on_the_popover_presents_when_it_happened_its_superscription_and_its_source` (whole `Card`);
    - `An_event_without_a_date_presents_only_its_source`;
    - (4a/b) `A_dated_event_on_the_world_is_bounded_by_its_window_between_its_prior_and_next_event`;
    - (4a/b) `An_undated_event_on_the_world_keeps_the_current_frame`;
    - (4a) `An_event_on_the_world_highlights_the_narratives_its_steps_belong_to` (whole `Geography`).
  - `PresentationTests`: the table test expects `Of(Event, World) = Geography` (4a/b); `HomeSurfaces.Of(Event) = World`.
  - `AffordancesTests`: `Arrows` opens at `PageSize`; OPEN 6 → `AttestedIn` is collapsed; `Affordances_are_total` stays green.
  - `ExplorableTests.An_entry_carries_the_narrative_its_edge_records` (4a).
  - `ElementKindLawTests`: `Emphasis` is still matched only through `Match`.
  - `FocusViewTests`/`ReadingArrowsTests`:
    - `Every_successor_is_an_arrow_with_its_edge_step` (an event with a timeline and two narrative successors; the whole ordered `data-entry` list and each step's title);
    - `A_chapter_still_has_one_next_arrow` (FOCUS-3's ids unchanged).
  - `EmphasisPayloadTests` (the payload `MapInterop.Emphasize` sends, built by a pure `Emphasis` → payload function extracted from `Emphasize`):
    - `An_events_emphasis_names_its_narratives_and_the_event` (4a: the whole payload);
    - `Every_emphasis_has_one_payload` (walks the four arms).
  - `tests/ux/world-geography.spec.ts` stays green; leaving an event for a kind with no map view clearing the emphasis is re-expressed in Task 6 (`world-narrative-focus`).
- [ ] **Step 4:** `dotnet test client.Tests --filter "GraphPresenter|Presentation|Affordances|Explorable|ElementKindLaw|FocusView|ReadingArrows|Geography"` → red or compile errors.
- [ ] **Step 5: Implement** the types exactly as above. `CardOf` stays one expression. Nothing formats a year, splits a label or composes an id.
- [ ] **Step 6:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green. Events still open on the legacy popover (Task 2 switches them), so record `npx playwright test tests/ux --grep "world|place|polity|edge|reader"` → the base's set. FOCUS-6's places now clear the emphasis on leaving; record any spec it touches.
- [ ] **Step 7: Commit:** `focus: an event presents its date and superscription, every successor is an arrow with its edge step, and (4a) its map view bounds the slider to its years and highlights its narratives (R16; §3.3/§3.4 rows)`.

### Task 2: Events open by position, and the legacy event path is gone (client)

**Backend change: no.**

**Owner gate first:** OPEN 7. **Starts after** Task 1, FOCUS-2 T4/T5, FOCUS-3 T6 (`PericopeHeading`) and T8 (`PassageList`/`PassageBlock`); FOCUS-4 T4 is optional (see Overlap).

**Files:**
- Modify:
  - `client/Components/{PericopeHeading,ExplorerPopover,PassageList}.razor`;
  - `client/Legacy/{LegacyNodes,PopoverSections,PopoverSectionProviders,PassageBlock}.cs`;
  - `client/Exploring/{PopoverChromeRegistry,CanonRef}.cs`;
  - `client/AtlasClient.cs`, `client/MapInterop.cs`;
  - `client/wwwroot/css/app.css`, `client/wwwroot/js/reader.js`;
  - `client.Tests/Explore/{DeletionLawTests,LegacyNodesTests,LegacyViews,IdentityTests,PushViaConformanceTests,LegacyPresentationsTests}.cs`;
  - `client.Tests/{PopoverSectionRegistryTests,PopoverChromeConformanceTests,AtlasClientTests,PassageBlockTests,ChapterTextTests}.cs`;
  - `client.Tests/Components/ComponentParameterLawTests.cs`;
  - `client.Tests/{ReferenceParsingLawTests,WholeReadLawTests}.cs` (remove this task's entries, relabel the survivors as above).
- Delete:
  - `client/Legacy/{EventNode,YearNode}.cs`, `client/Components/{ArrowNav,RefsList}.razor`, `client/Exploring/{EventAccounts,MapFocusHatch}.cs`;
  - `client.Tests/{ArrowNavTests,EventAccountsTests,MapFocusHatchTests,YearNodeEventTimeTests}.cs`.
- Test: `client.Tests/Explore/DeletionLawTests.cs`; `client.Tests/Components/PericopeHeadingTests.cs` (create if none).

- [ ] **Step 1: Failing tests.**
  - `DeletionLawTests`: add `NodeKind.Event` to `MigratedKinds`. Run → red: `EventNode`, six `AppliesTo(... == "Event")` strings.
  - `PericopeHeadingTests.A_heading_opens_its_served_event` (the whole `PopoverOpening.Explore`).
  - `IdentityTests`/`LegacyNodesTests`: the Event rows now expect `null`.
- [ ] **Step 2:** red.
- [ ] **Step 3: Implement.**
  - `LegacyNodes.For` answers `null` for `Event`, so every event, whether opened or followed (from a verse's `attests`, a place's `site-of`, a person's `participates-in`, an analogue, an arrow or a saved exploration), renders on FocusView.
  - `PericopeHeading` opens by position.
  - If FOCUS-3 T8 or FOCUS-4 T4 has not landed, rewrite their one `new EventNode(...)` line each to `new PopoverOpening.Explore(new NodePosition(...))` from the `NodeRef` they hold, and nothing else.
  - Delete everything in the inventory, then delete every member it leaves without a production reader (rule 4), recording each `grep` in the ledger.
  - `ExplorerPopover.LoadCurrent` no longer syncs a narrative focus; the World's emphasis is the one door (Task 1).
  - The ratchets: remove each entry this task's deletions retire; relabel `MiniReaderExpand`, `PassageList` and `VerseTextResolver` to `FOCUS-7`/`MAPS`.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green. Run every spec listed under "Playwright specs" → record the red set by name in the ledger (expected: all of them but the "stay green" list).
- [ ] **Step 5: Commit:** `event: an event opens and renders on FocusView from its served position; EventNode, its six sections, ArrowNav, RefsList, EventAccounts, MapFocusHatch and YearNode are gone (deletion law: Event; F-54, F-57, F-63, F-65 event sites)`.

### Task 3: The timeline is succession (backend: graph-types, compiler, labels; regen)

**Backend change: yes (compiler and vocabulary; served code loses nothing yet).** **Why there (rule 27):** which event comes before which in the timeline is a fact over the data alone (`chrono.order`), so the compiler writes it with its direction, once. The server stops guessing direction per request (Task 5 deletes the scan). Retiring the direction-less kind is the graph modelling the domain: the rows always recorded `earlier`/`later`.

**Owner gate first:** OPEN 2 (under 2(b) this task is skipped and the ledger says so). **Starts after** Task 2 (no client code names `TemporalAdjacency` but `Affordances`), FOCUS-3 T4 and FOCUS-4 T1/T2 (shared graph files, `labels.rs`).

**Files:**
- Modify:
  - `graph-types/src/{edge,graph,sections}.rs`, `graph-types/src/canon/*` (the row family), `graph-types/tests/common/mod.rs` (`DECLARED_SYMMETRIC_RELATIONS` 6);
  - `server/atlas-graph/src/{event_world,labels,provenance,law_check}.rs`;
  - `server/atlas-graph/src/sqlite/{ddl,partition,writer}.rs`;
  - `data/compiled` (rebuilt);
  - `contracts/*` regen, `contracts/atlas-query-contract/CHANGELOG.md` (AQC major), `contracts/atlas-graph-contract/VERSION` (major) and its pins;
  - pacts;
  - `client/Exploring/Affordances.cs` (the arm), `client.Tests/Explore/AffordancesTests.cs`.
- Test:
  - `event_world.rs` and `labels.rs` unit tests;
  - `server/atlas-graph/tests/sqlite_laws.rs`;
  - `server/atlas-contract/tests/graph_api.rs`;
  - `graph-types` relation-count laws.

- [ ] **Step 1: Failing tests.**
  - `event_world.rs`:
    - `the_timeline_is_one_succession_step_per_consecutive_pair_earlier_to_later` (the existing fixture `e5, e1, e2`, asserting the whole `follows-in` page of each);
    - `zero_or_one_dated_events_yield_no_timeline_step`.
  - `labels.rs`:
    - `a_narrative_step_is_labelled_by_its_narrative` (whole string);
    - `a_timeline_step_is_labelled_by_its_kind`.
  - `graph_api.rs`, read from the artifact:
    - `every_dated_events_next_in_time_is_a_follows_in_neighbour`: walk `chrono.order` through the service; each consecutive pair is a `follows-in` entry of the earlier and a `precedes-in` entry of the later;
    - `no_served_kind_is_temporal_adjacency`.
  - The relation-count law: 22 directed, 6 symmetric.
- [ ] **Step 2:** red.
- [ ] **Step 3: Implement** as in Types.
- [ ] **Step 4: Rebuild and regenerate (critical section `contract`).**
  - Rebuild `data/compiled` and re-bless. Record the `follows-in`/`precedes-in` count growth and the edges that merged with a narrative step.
  - `export_contract`, `export_aqc_examples`, `--check`; CHANGELOG major; AGC major; `client.ContractGenerator`.
  - If a golden map fixture or `scene_byte_identity` moves, **stop** for the owner.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace`, `(cd graph-types && cargo test --all-features)`, `bash scripts/contract-gate.sh --base <base>`, `bash scripts/contract-semver-gate.sh` (declared major), `dotnet build client && dotnet test client.Tests` → green. **Commit:** `chronology: the timeline is succession, earlier to later, and a narrative step is labelled by its narrative; the direction-less temporal-adjacency kind is gone (rule 27; AQC/AGC major)`.

### Task 4: Accounts are passages that narrate their event (backend: graph-types, compiler, labels, one served change; regen)

**Backend change: yes.**
- **Compiler:** mints account passages, `Narration` rows and passage labels. **Why there (rule 27):** which verses make one account and how its reference reads are facts over the curated witnesses alone.
- **Served code:** loses the per-request account derivation and its two `panic!`s. **Why:** 27, 27b, and the Haskell bar: a served read must not abort.
- No read is added. **One relation is appended** (OPEN 1a), because "this passage narrates that event" is a domain fact the graph did not hold. No relation exists for a client construct.

**Owner gate first:** OPEN 1, 8 (under 1(c) the compiler part is skipped and only the served deletion runs). **Starts after** Task 3, FOCUS-2 T1 (`references.rs`), FOCUS-3 T4 (`passage_container_id`), and FOCUS-4 T1 (`grounds_of`).

**Files:**
- Modify:
  - `graph-types/src/{edge,graph,sections}.rs`, `graph-types/src/canon/*`, `graph-types/tests/common/mod.rs` (23 directed);
  - `server/atlas-graph/src/{event_world,references,labels,law_check,xref_adapter}.rs` (`xref_adapter` only to pass `&[range]`);
  - `server/atlas-graph/src/sqlite/{ddl,partition,writer}.rs`;
  - `server/atlas-contract/src/graph.rs` (the attests arms; `EdgeEntry`'s description in `wire/graph.rs`);
  - `data/compiled`;
  - `contracts/*` regen, CHANGELOG (AQC major), AGC VERSION (minor: a relation added) and pins;
  - pacts; `client/Exploring/Affordances.cs`.
- Test:
  - `event_world.rs`, `references.rs`, `labels.rs`, `law_check.rs` unit tests;
  - `server/atlas-graph/tests/{sqlite_laws,bible_containers_real_data}.rs`;
  - `server/atlas-contract/tests/graph_api.rs`;
  - `client.Tests/Explore/AffordancesTests.cs`.

- [ ] **Step 1: Failing tests.**
  - `references.rs`:
    - `a_run_set_reads_as_one_reference_with_later_runs_shortened` (`MRK.14.54, 66-72` whole);
    - `a_run_across_chapters_names_both_chapters_once` (`MAT.5.1-7.29`).
  - `event_world.rs`:
    - `each_account_is_one_passage_holding_its_verses_and_narrating_its_event` (fixture event with two witnesses; whole `contains` pages and the event's whole `narrated-in` page);
    - `an_account_whose_verses_another_account_or_a_citation_already_spans_is_the_same_passage`;
    - `every_verse_of_an_account_still_attests_its_event`.
  - `law_check.rs`: FOCUS-3's `a_passage_holds_only_verses_and_is_no_members_only_parent` stays green over account passages.
  - `graph_api.rs`, read from the artifact:
    - `the_sermon_on_the_mount_is_narrated_in_its_two_accounts` (the curated witnesses of `rob_sermon_on_the_mount` as passages, with labels from `runs_reference`);
    - `peters_denial_in_mark_is_one_account_of_two_runs` (`rob_peter_denies`);
    - `an_attestation_page_carries_no_account`;
    - `a_cited_span_and_an_account_share_one_label_rule` (OPEN 8).
  - `grounds_of` walks `RowFamily::ALL` (FOCUS-4's law) and needs the new arm.
- [ ] **Step 2:** red. Record the account count, the passages shared with cited spans, and the passages new.
- [ ] **Step 3: Implement** as in Types.
- [ ] **Step 4: Rebuild and regenerate (critical section `contract`).** Rebuild and re-bless. Cited-span passage labels change under OPEN 8 (record the count). Regenerate; CHANGELOG AQC major; `client.ContractGenerator`. A golden map fixture that moves stops the task for the owner.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh --base <base>`, semver gate (declared), `dotnet build client && dotnet test client.Tests` → green. **Commit:** `accounts: each account of an event is a passage that narrates it, labelled by its runs; attestation pages stop regrouping verses per request (rule 27; F-65 account reference; AQC major)`.

### Task 5: `/api/event` and `/api/narrative/event/{id}` are gone; the event record reads no neighbours (backend deletions; regen)

**Backend change: yes (deletions only, plus `event_detail`).** **Why there (rule 27a):** both routes are view-shaped. Everything they served is now the record plus the generic neighbour read. The record's `located-at` drain is a whole read in served code (27b).

**Starts after** Task 2 (no client caller), Tasks 3–4, and the `contract` lock's earlier holders.

**Files:**
- Delete: `server/atlas-contract/src/events.rs`, `server/atlas-contract/src/wire/events.rs`, `contracts/atlas-query-contract/features/event-page.feature`; `server/atlas-core/src/narrative.rs` (if no reader).
- Modify:
  - `server/atlas-contract/src/{lib,graph,aqc_export}.rs`, `wire/mod.rs`;
  - `server/atlas-graph/src/service.rs`; `server/atlas-core/src/lib.rs` (the module line);
  - `server/atlas-contract/tests/{api,graph_api,contract_api,contract_coverage,contract_pact,no_legacy_event_reads}.rs`;
  - `server/atlas-graph/tests/{port_widening_real_data,narrative_real_data}.rs`;
  - `server/atlas-core/tests/no_atlas_data_in_public_signatures.rs`;
  - `server/atlas-contract/benches/queries.rs`, `server/BENCHMARKS.md`;
  - `contracts/atlas-edge/{events,narratives}.feature`, `contracts/atlas-graph-contract/graph/detail-routes.feature`;
  - `contracts/*` regen, CHANGELOG (AQC major), pacts.
- Test: `graph_api.rs`; `contract_coverage.rs`.

- [ ] **Step 1: Failing tests.**
  - `graph_api.rs`:
    - `an_event_record_is_its_payload_and_its_date` (the whole `EventDetail` of `rob_last_nazareth_visit`, read against the artifact);
    - `the_event_and_narrative_event_routes_answer_not_found`.
  - `contract_coverage.rs`: neither path is published.
- [ ] **Step 2:** red.
- [ ] **Step 3: Delete and implement.** `event_detail` reads the payload and `chrono.resolved` only. `cargo build` decides `narrative.rs` and every helper left without a served reader. A helper whose only reader is a tool (`atlas-cli`) stays, with a FINDING.
- [ ] **Step 4: Regenerate (critical section `contract`):** `export_contract`, `export_aqc_examples`, `--check`; CHANGELOG major; re-bless; `client.ContractGenerator`. `GeneratedUsageTests` must not list a removed type.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace`, `bash scripts/contract-gate.sh --base <base>`, semver gate, `bash scripts/gate-selftest.sh`, `dotnet test client.Tests && dotnet test client.ContractTests` → green. **Commit:** `events: the event and narrative-event routes are gone; an event is its record and its neighbours (rule 27a; AQC major)`.

### Task 6: Re-express the event specs (TypeScript)

**Backend change: no.**

**Files:**
- `tests/ux/{event-timeline,event-timeplace,popover-sections,accounts-and-mentions,provenance,reader-headings,world-pin,world-narrative-focus,w1-passages,w2-passages,w3-passages,w4-passages,w5-passages}.spec.ts`;
- `tests/ux/lib/api.ts` (`event` and `narrativeEventPositions` deleted; callers read `api.node`/`api.nodeEdges`);
- `tests/ux/CONTRACT.md` (the sections listed above).

- [ ] **Step 1:** Re-express by name, every expected value read from the API (F-8), keeping each spec's intent:
  - **Chronology** (`event-timeline` HOTFIX-4, CHRONO-MERGE-1, chain ends, MERGE-1/2/3, AMENDMENT C, TITLE-2, EVT-3/RefsList):
    - walk `popover-next`/`popover-prev` by `data-entry`, comparing with `api.nodeEdges(id, 'follows-in')`;
    - ≥5 hops each way on `gen_binding_isaac`;
    - the first event has no `popover-prev` and the last no `popover-next`;
    - a narrative successor that differs from the timeline's is its own arrow, whose edge step's title names the narrative.
    - RefsList, peek, HOVER-KILL, PEEK-*, EVENT-HOVER-HATCH tests are **deleted** (OPEN 7; O-F6b), listed in the close report.
  - **General kind** (L184, AFFORDANCE-1): a titled passage serves no succession, so no arrows. The heading's quiet class is FOCUS-3's.
  - **Map coherence** (`event-timeline` L310, `world-narrative-focus` L20, OPEN 4a): `data-narrative-focus` set on the event's narratives' arrows and cleared on leaving.
  - **Time and place** (`event-timeplace`):
    - `popover-field-When` = `event.when.label`;
    - places are `popover-link-located-at-Place:{id}` with the served label;
    - EVT-3/Place follows the place and its `popover-chip-map`;
    - EVT-3/Time becomes "the map chip opens the World at the event's window" (4a/b);
    - EVT-META-TOP-1's order assertion becomes the card's field order (closes its F-55 site);
    - NAV-2 (Creation): no `located-at` section.
  - **Accounts** (`popover-sections` EVENT-1/PASSAGE-1, ACCT-COALESCE-1 ×4, single-witness, M-D1, chronological-vs-reading-order; `event-timeline` ACCT-RUNS-1, ACCT-SET-MISMATCH-1; `w1`–`w4` parallels):
    - `popover-section-narrated-in` lists exactly the curated accounts by their compiled labels (`MAT.5.1-7.29`, `LUK.6.17-49`; `MRK.14.54, 66-72`; PSA 14/53 stay two);
    - following one opens the passage, whose `contains` children are its verses;
    - READER-1's focal-range highlight is **deleted** (no inline expand), and so is "expand opens the chapter".
  - **Mentions and analogues** (`accounts-and-mentions` L69, L118, L193):
    - `popover-section-mentioned-in`, `popover-section-analogous-to` with their links;
    - "SIMILAR directly below PARALLEL" becomes served order: the assertion is the whole ordered section list.
  - **Provenance** (L78, L102): `popover-field-Provenance` = the served provenance. Each analogue's provenance is its edge step's record (`popover-entry-edge-analogous-to-{edge id}` → `popover-field-Provenance`). L353 reads the edge records instead of `witnesses_provenance`/`mentions_provenance`.
  - **Map chip** (HATCH-DELIVERABLE-1; `w1` L49, `w2` L82, `w3` L21, `w4` L13, `w5` L13/L58):
    - 4a/b: `popover-chip-map` present on every event, and the World's slider is bounded to the served window (the URL is `/world`; the window is read from the slider);
    - the "no date-places section" assertions become "no `When` field".
  - **Reader headings** (L19, L79): the heading opens the event; the card shows `popover-field-When`.
  - **World traversal** (`world-pin` L96, L121, L154): `popover-link-site-of-{Event}` → `popover-next[data-entry=…]` → Back retraces. TRAVERSAL-3's narrative step is its own arrow.
- [ ] **Step 2:** `npx playwright test` over the files above, plus the "stay green" list → green.
- [ ] **Step 3: Commit:** `tests: the event specs read FocusView's generic presentation: arrows from follows-in, accounts as passages, the card's fields (rulings applied: served order, no peek)`.

### Task 7: Gates, mutation, close

- [ ] **Step 1: Gates.**
  - Stryker configs cover `Affordances.cs`, `Geography.cs`, `Presenter.cs`, `Presentation.cs`, `Explorable.cs` (verify).
  - Mutation, inside the owner's window only (`heavy` with "mutation", `free -g` ≥ 18 GB):
    - `bash scripts/mutants-parallel.sh -n 3 -b <base>` → 100% or equivalents recorded. It covers `populate_chronological_succession`, the account minting, `passage_container_id`, `runs_reference`, `relation_label`'s narrative arm and `event_detail`.
    - Then `dotnet stryker` → 100% or equivalents recorded.
    - Outside the window, `.superpowers/MUTATION-GATE-DEBT.md` names `<base>`.
  - Then (lock `heavy`): `cargo test --workspace && (cd graph-types && cargo test --all-features) && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh --base <base> && bash scripts/timing-gates.sh run && npx playwright test tests/ux` → green, except the carried reds (density smoke, C3-M1, VIEWSTATE-1) and any F-55 flake named by spec.
  - The diff adds no comment line: `git diff <base>.. | grep -E '^\+.*(//|/\*|<!--|#\[doc)'` over application files prints nothing.
- [ ] **Step 2:** Write the close report, `docs/superpowers/reports/2026-10-0x-focus-5-close.md`. It covers:
  - every rule-24 category with its closure and guarantee (below);
  - the FINDINGS raised;
  - the amendments: `Arrows` clamp, `Entry.Narrative`, `Emphasis.Narrative`, the Event World row, `passage_container_id` over runs;
  - the behaviours removed by rulings: story-thread rows, peeks, inline account text, the year view, the section headings.
- [ ] **Step 3:** Push `lane/<agent>/F5-int`; set the queue item to `review` with the commit range. The other agent reviews (14b + 24a); Claude lands under `land`.

**Rule-24 categories and their closures:**

| Category | Closure | Guarantee |
|---|---|---|
| Event served by both mechanisms | deletion law (Event) | compiler + law |
| An event's accounts derived per request (server) and re-grouped and formatted on the client | compiled passages + `Narration`; `EventAccounts` (both sides) deleted | the served code has no account type; `runs_reference` is the one label function (real-data law) |
| Event adjacency computed per request (timeline scan, narrative positions) | the timeline is succession; both routes gone | relation-count law; `every_dated_events_next_in_time_is_a_follows_in_neighbour` |
| A view-shaped event route | routes deleted | `contract_coverage.rs`; vocabulary of the published document |
| A world query composed from a local id (F-54) | `MapFocusHatch` deleted; the map chip goes through the World presentation | partial: other `NavigateWorld` chips stay (FOCUS-9 rows, FINDING) |
| A map row the client compares by composing an id | `SceneArrow.from_node`/`to_node` (4a) | real-data law over a scene window |
| A whole read on the client (F-63, event accounts) | deleted; lists page through the one door | `WholeReadLawTests` ratchet + `RootConsistencyLawTests` |
| A hover-only interaction (peek) | deleted | review rule (no law; FINDING proposes one) |

---

## Wave schedule

Primary is the critical path; the companion is unlike work beside it (rule 23: Rust beside C#). A task starts only when its named inputs have landed on `lane/<agent>/F5-int` and, for a shared file, on the other batch's integration branch.

| Wave | Primary | Companion | Critical sections | Expected red at wave close |
|---|---|---|---|---|
| 0 | owner: OPEN 1–8 (defaults stand if unanswered); Task 0 | — | — | — |
| 1 | Task 1 Part A (Rust; OPEN 4a) | Task 1 Part B tests written against `ServedGraph` (C#) | `contract` regen (AQC minor), re-bless; `heavy` | — |
| 2 | Task 1 Part B (C#) | Task 3 tests written (Rust, not yet built against the rebuild) | — | none (events still legacy) |
| 3 | Task 2 (C#: the switch) | Task 3 (Rust: rebuild #1, regen; after Task 2's client deletion of `YearNode`) | `contract` rebuild + regen; `heavy` | Playwright: every event spec but the "stay green" list |
| 4 | Task 4 (Rust: rebuild #2, regen) | Task 6 drafted against Tasks 1–3 (TS) | `contract` rebuild + regen; `heavy` | as wave 3 |
| 5 | Task 5 (Rust: deletions, regen) | Task 6 finished (TS) | `contract` regen; `heavy` | none of the event specs |
| 6 | Task 7 | — | `heavy` (mutation only in the owner's window); `land` after review | only the carried reds and named F-55 flakes |

**Critical path:** OPEN answers → (F2 T3, F3 T5/T6, F4 T3) Task 1 → (F3 T8) Task 2 → Task 3 → (F3 T4, F4 T1) Task 4 → Task 5 → Task 6 → Task 7.

## FINDINGS this plan expects to raise (for the queue; the owner decides)

- **`parallel` is a declared symmetric relation with no row family** (rule 4: dead vocabulary in `relations!`). Closure: retire it with the next relation change, or give it its rows.
- **A succession edge recorded by two rows names one narrative** (the first row's) on its entry and label: two narratives sharing a step, or a timeline step that is also a narrative step. Closure: per-row meta on the neighbour page, or one edge per (relation, ends, narrative).
- **Narrative membership is not modelled as edges.** A Narrative node has only `shown-on`; a one-leg narrative has no succession row, so its event names no narrative (none exists in the data today; the special case in `events.rs` dies with the route). Closure: a membership relation from a narrative to its legs, compiled.
- **`GraphService.chronology` is held at serve time** for the scene and the heading index (MAPS). Closure: with MAPS, the scene reads compiled windows.
- **FOCUS-3's ratchet labels** name FOCUS-5 for `MiniReaderExpand`, `PassageList` and `VerseTextResolver`, which survive to FOCUS-7/MAPS (relabelled in Task 2).
- **`World.razor` left the previous emphasis in place** for a focus with no World presentation (FOCUS-6; changed by Task 1, so the review checks the places' behaviour).
- **A passage's popover lists its verses by label, not text** (FOCUS-3 OPEN 7's chapter card rule). Its text is one click away on the reader. Owner may want a passage-text section on the popover (a `Presentation.Sequence` on `Surface.Popover` for passages, which needs a container level, FOCUS-3's FINDING).
- **Account notes are curator text in `Narration.justification`** (F-50 category; not shown).
- **F-53 and F-56 now reach events** (an event opened twice from the same heading keeps the trail; Escape after a step).
- **No law forbids hover-only interactions.** Closure: a source law over `.razor` files for `@onpointerenter`/`@onmouseenter` handlers with no click twin.
- **Pre-existing comments in touched server files** (`events.rs` dies whole; `graph.rs`, `service.rs` keep theirs; A-STRIP).

## Self-review against rule 27, the spec and the brief

- **Data-only derivations are compiled:**
  - the timeline's direction (Task 3);
  - accounts as passages with their run references (Task 4);
  - a narrative step's label (Task 3);
  - the passage label rule (Task 4).

  The edge counts the card shows are the compiled `edge_count`.
- **Per-request derivations are bounded index reads:** the element read for an event; one neighbour page per group (20, server-capped). The two routes and their whole reads (`drain_edges` of mentions, succession and analogues; the `chrono.order` scan; `EventAccounts`) are gone.
- **Interaction derivations are the client's:** the Event rows of `Presentation.Of`, the card fields, every successor as an arrow, the map frame and emphasis, collapsed lists, every follow through `Explore`.
- **The graph models the domain, never a view:**
  - `Narrates` is a domain relation (a passage narrates an event), and the timeline's succession was always in the rows;
  - no CHRONOLOGY or PARALLEL ACCOUNTS grouping is built into the graph (rulings applied);
  - no client word enters `server/` or `graph-types/` (`narrates`, `succession` and `passage` are the graph's words).
- **27c/27e:** an event's popover is one element read plus one first page per group, plus two cached pages for the map emphasis (4a). The six legacy fetches, the per-arrow `attested-in` whole reads and the peek's text reads are gone.
- **Coverage of §5's FOCUS-5 row:** the six `Event*Section`s, `EventNode`, `/api/event` and `/api/narrative/event/{id}` are deleted, and `ArrowNav` is replaced by the event's `follows-in`/`precedes-in` (§8's "chronology adjacency served as follows-in" done by Task 3).
- **Type consistency:**
  - `SceneArrow.from_node` (Task 1A) is what `Emphasis.Narrative`'s `map.js` arm compares;
  - `Entry.Narrative` is what `GeographyOf` reads;
  - `Narrates`/`NarratedIn` (Task 4) are what `Affordances.Of` and the specs name;
  - `runs_reference` labels both FOCUS-3's and FOCUS-5's passages.

## Assumptions

**Verified against `b3d7cfa`:**
- The six sections, `RenderArrowNav`, `NarrativeArrow` and `WitnessUnitsResolver` sit at `PopoverSectionProviders.cs:672–1099`. They are registered at `PopoverSections.cs:57–64`. `EventNode` reads `/api/event` and `/api/narrative/event` and `NodeRecord` (for `when`).
- `/api/node/Event:ab_ur` serves groups `attested-in`, `follows-in`, `dated-by`, `dates`, `located-at`, `shown-on`, `temporal-adjacency`. `rob_sermon_on_the_mount` serves `attested-in` 144, one entry per verse. The account's runs and note are computed per request by `EventAccounts::read` (`graph.rs:365`). (Probed on a running server built from a slightly older head; the shapes are unchanged at `b3d7cfa` by code reading.)
- `TemporalAdjacency` rows record `earlier`/`later` (`graph-types/src/edge.rs:433`) but lower to the symmetric `temporal-adjacency` (`graph.rs:321`). `temporal_neighbors_of` scans `chrono.order` per request (`service.rs:479`).
- `Succession` edges carry `EdgeMeta::Narrative`. `MapSuccession` and `CanonSuccession` lower into the same `Succession` relation. Edge ids are `(relation, subject, object)`, and `rows_of_edge` answers several rows.
- `Affordance.Arrows` opens at `ArrowsShown = 1`. FocusView renders arrows from the group's window with an edge step each; no Playwright spec reads `popover-next`/`popover-prev` today.
- `Presentation.Of(Event, World)` is `null`; `HomeSurfaces.Of` is derived from the table; `World.razor` adopts an exploration whose home is the World and presents it.
- `Emphasis` has three arms. `MapInterop.Emphasize` sends ids from `NodeRef`. `SetNarrativeFocus` takes raw narrative and event ids, and `SceneArrow` carries raw `from_event`/`to_event` only.
- `PericopeHeading` opens `new EventNode(Heading.EventId, Heading.Title)`, and its test id is `pericope-heading-{id}`.
- `DECLARED_DIRECTED_RELATIONS` 22, `DECLARED_SYMMETRIC_RELATIONS` 7, `SECTION_SCHEMA_VERSION` 22.

**To verify at execution (each with its fallback):**
- **`TimeSlider` accepts a one-year window** (`from == to`, e.g. AD 31) as `Bounds` (Task 1). Fallback: stop for the controller; do not widen the window on the client (rule 25).
- **FOCUS-3 T4's `passage_container_id(first, last)` landed as specified.** Fallback: Task 4 adds the run-set form beside it and FOCUS-3's becomes a one-run call in the same commit.
- **FOCUS-4 T2's `relation_label`/`ReaderNames` landed.** Fallback: Task 3 introduces them with only its own arm, and FOCUS-4 rebases.
- **Removing `temporal-adjacency` moves no golden map fixture** (the scene does not read it). Fallback: stop for the owner.
- **No AQC scenario outside the event features pins `attested-in` loci.** Fallback: those scenarios are re-expressed in Task 4 under the declared major.
