# FOCUS-9 (Author, Year, the old saves, and the end of the legacy popover) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the strangler (spec F7). The last two client-only kinds become what R2 ruled they are: an **Author** is the `Person` a book is `authored-by`, and a **Year** on a place is the `Event` its date claim attests. The old save and selection readers go (R4). The legacy popover machinery goes entirely: `IExplorable`, `Chip`/`ChipTarget`, `PopoverSectionRegistry` and its providers, `LegacyNodes`, `LegacyPresentations`, `PopoverChromeRegistry`, the client's frontier-matrix mirror. The one thing held for MAPS, the polity border-change popover, leaves the legacy family as one small typed view in the MAPS seam (`client/Geography/`), so nothing of the old mechanism survives to serve it.

**What is true after FOCUS-9:**
- A book's card shows its author as a link to the person when the graph links exactly one (OPEN 1); a place's "Established"/"Destroyed" date is a link to the event that dates it when the curated claim names one (R2). The compiler refuses a claim whose event does not happen at that place.
- `client/Legacy/` does not exist. The popover body has exactly two forms: `FocusView` for every position, and `BorderChangeView` for a border change opened from the map. `PopoverOpening` is `Explore | Resume | BorderChange`.
- No client code decides a kind from a string, holds a legacy kind vocabulary (`"Verse"`, `"Author"`, `"Year"`, …), or reads `explorations-v1`, `explorations-v2` or `selection-v1`.
- Spec §5's FOCUS-9 row, §9's table and the deletion law are closed; §12 R20's `client/Legacy/` project boundary is retired.

**Architecture (rule 27, by who knows the inputs):**
- **Compiler:** one data law: a place date claim's `event` is located at that place (26: a curated id that does not resolve, or resolves to the wrong thing, fails the compile).
- **Server:** no change. The wire already serves `DateClaim.event` (`server/atlas-contract/src/wire/time.rs`), `BookDetail.author` and the `authored-by` edges (FOCUS-0 R2).
- **Client:** a card field may open something (a year, or a link); the presenter fills it from served values only; FocusView renders it. The border-change view composes over the map's served polity rows (`IMapSource`, FOCUS-6). Everything else is deletion.

**Tech Stack:** Rust (`atlas-etl`, `atlas-graph` law_check); .NET 10 Blazor WebAssembly (`BibleAtlas.Client`, `BibleAtlas.Client.Exploring`); xUnit + bUnit; Playwright; Stryker.NET; cargo-mutants via `scripts/mutants-parallel.sh`.

**Spec:** `docs/superpowers/specs/2026-09-26-focus-exploration-design.md` §3.3 (`Field`), §3.5 (selection), §3.6 (persistence), §5 (FOCUS-9 row), §9 (what dies), §10 R2, R4, §12 R16, R20. **Principles:** 4, 9, 12, 14b, 21–23, 24–24b, 25, 26, 27–27g. **Queue:** A-F9; F-12 (comments in the deleted files go with them), F-31/F-54 (ids composed on the client), F-57 (presentation retained per trail entry).

**Reconciling the spec's two wordings of Year.** §5's row says "Year → `dated-by` link to an Anchor"; the spec header and §10 R2 say "Year → the Event the claim attests", and R2 governs ("superseded … throughout"). This plan applies R2: a place's date claim links its event. An event's own date needs nothing here: its `dated-by` neighbours (an Anchor or another Event) are already a group in its frontier on FocusView (FOCUS-5).

**Agent:** Claude (roadmap Lane A: "FOCUS-9"), or Codex; nothing below depends on which.

**Base:** the head of `worktree-bible-atlas-m1` after FOCUS-8 has landed (FOCUS-9 is last). Record it in the ledger (`.superpowers/sdd/2026-10-0x-focus9/progress.md`) at Task 0; every gate takes `--base <base>` (PRINCIPLES 22). Written against `b3d7cfa` with the FOCUS-2, 3, 4, 6 and 8 plans read; the FOCUS-5 and FOCUS-7 plans were not yet pushed, and what this plan assumes they deleted is listed under "What the earlier batches leave" and checked by Task 0.

## OPEN: for the owner, before the task named starts (each blocks only that task)

Each is answerable in one line. The plan builds the recommendation if unanswered.

1. **A book's author (Task 2):** for the 14 books whose author the graph links to a person (Genesis → Moses …): (a) the "Author" line on the book's card becomes a link to that person, and the separate "Authored by" list is not shown a second time; (b) keep both, the plain "Author" line and the "Authored by" list. **Recommend (a)** (by analogy to POPOVER-LAW-1: show only non-redundant content).
2. **A place's date with no event behind it (Task 2):** "Established 1003 BC" links to the event that dates it when the curated claim names one (R2). When it names none: (a) plain text, as R2 says; (b) a button that opens the map at that year, like a person's birth year if you approve FOCUS-4's OPEN 5(b). **Recommend (b) if FOCUS-4's 5(b) is approved, else (a).**
3. **Old saved explorations (Task 5):** R4 deletes the reader for the oldest saves (`explorations-v1`). Delete, by the same ruling, the reader for the next-oldest saves (`explorations-v2`, before Sept 30) and for old selections (`selection-v1`)? Someone who has not opened the app since then loses those saves. **Recommend yes.**
4. **The map button on a border-change popover (Task 3):** keep the "show on the map" button, which sets the map's time window to the years of the change? **Recommend keep** (the border-morph specs pin it).
5. **The verses on a border-change popover (Task 3):** keep showing the verse text exactly as today until MAPS replaces the whole popover? **Recommend yes.**

**Rulings applied by analogy, not re-asked:**
- **R2 (the client-only kinds are graph nodes)** decides Author and Year (above) and that `TimeAndPlace` is already gone (FOCUS-6 deleted `TimeAndPlaceNode`).
- **R4 (translate what maps, drop what doesn't, report once; the v1 reader is deleted in FOCUS-9)** is OPEN 3's ground; the "dropped steps" notice goes with the reader it reported for.
- **FOCUS-4's `Field.At` (a served year on a card opens the World)** is the precedent the field-target sum generalises (Types).
- **FOCUS-6's MAPS seam** (`IMapSource`, `client/Geography/`): the border change moves there, and the MAPS migration deletes it with `AtlasMapSource`.
- **FOCUS-2 ruling 7 (reach every site of the category in the batch):** Task 6's string-kind law covers every client file, not only the popover.

## Global Constraints

- `docs/PRINCIPLES.md` binds, especially: rule 4 (zero dead code), 9 (no comments in application code; every snippet below has none; comments in deleted files go with them), 12 (every signature below is for sign-off), 24/24a/24b, 25 (no id composition, no kind from a string, no year formatting on the client), 26, **27** (the compiler checks the data; the server is unchanged; the client composes).
- Tests: whole-body assertions; one behaviour per test, named as a sentence; `// Arrange` `// Act` `// Assert` only; no magic numbers; newspaper order; real-data expectations read from the artifact or the API (F-8).
- **Total matches.** `FieldOpens` and `PopoverOpening` expose an abstract `Match<T>` implemented once per arm; no `switch` over them (the FOCUS-6 source-scan law covers new sums).
- **The one walk door and the one paging door** (R19, R20, F-59, F-70): a field link follows through `OnFollow(Link)` → `Explore.Follow`; the author's link is read through `Paging.FirstLink` (the door `Bounded` already uses).
- **No relation is appended; no route changes.**
- Build no interaction that works only by hovering.
- Every task in its own worktree (`git -c core.autocrlf=false worktree add -b lane/<agent>/F9-t<n> ~/w/F9-t<n> <base>`), `CARGO_TARGET_DIR=~/mut/<agent>-F9-t<n>`, `nice -n 10 cargo -j 4`; data copied, never linked. Integrate on `lane/<agent>/F9-int`; the other agent reviews; Claude lands one squashed commit per task under `land`. Never force.

## Critical sections (PRINCIPLES 21): one holder at a time

| Section | Held by | Why |
|---|---|---|
| `contract`: rebuilding `data/compiled` | Task 1 only if the law finds a claim to correct in `data/curated/place-history.toml` | the version root moves |
| `heavy`: `cargo test --workspace`, full Playwright | Tasks 1, 7, 8 | memory |
| `heavy` with "mutation" in the message | Task 8, inside the owner's window | once per batch (3a) |
| appending to `relations!`; regenerating `openapi.yaml` | **nobody** | no wire change |

## What the earlier batches leave (the inventory FOCUS-9 inherits)

Grounded at `b3d7cfa` by grep; "after" is what the plans say. Task 0 checks the "after" column at the base.

| Piece (`b3d7cfa`) | Callers at `b3d7cfa` | After FOCUS-2…8 | FOCUS-9 |
|---|---|---|---|
| `client/Legacy/IExplorable.cs` (`Title`, `Kind`, `Identity`, `ExploreAsync`, `BodyAsync`) | 14 implementations | `BodyAsync` gone (FOCUS-8); one implementation, `PolityDeltaNode` | **deleted** |
| `client/Legacy/Chip.cs` (`Chip`, `ChipTarget.{Push,NavigateWorld,NavigateReader}`) | 12 node classes; `ExplorerPopover.Activate`; `ChipTests`, `PopoverChromeConformanceTests`, `AuthorNodeTests` | authored only by `PolityDeltaNode` (`NavigateWorld`) | **deleted** with its last author (spec §9) |
| `client/Exploring/PopoverChromeRegistry.cs` (string-keyed by legacy kind: 12 rows) | `PopoverChromeConformanceTests` only | rows for `PolityDelta` (and any a batch forgot) | **deleted** |
| `client/Legacy/PopoverSections.cs` (`IPopoverSectionContext`, `PopoverSection`, `IPopoverSectionProvider`, `PopoverSectionRegistry`: 29 entries) | `ExplorerPopover` implements the context | three `PolityDelta*` entries | **deleted** (spec §5: "`PopoverSectionRegistry` once only `PolityDelta*` remain") |
| `client/Legacy/PopoverSectionProviders.cs` (1,616 lines, 31 providers) | the registry | `PolityDeltaEventSection`, `PolityDeltaScripturesSection`, `PolityDeltaGroundingSection` | **deleted**; their three bodies become `BorderChangeView`'s three parts, same test ids |
| `client/Legacy/PolityDeltaNode.cs` (identity composed by `NodeIds.Of(NodeKind.Polity, …)`; `DeltaKind` read by nothing) | `World.razor:974` | unchanged (spec: "leave with MAPS") | **replaced** by `BorderChange` (Types); the polity's ref is the served roster row's `Node`, not composed |
| `client/Legacy/PopoverOpening.cs` (`Explore`, `Resume`, `Legacy(IExplorable)`, `OpensLegacy`) | 20 host sites | `Legacy` only at `World.razor` (FOCUS-8 law) | `Legacy` → `BorderChange`; `OpensLegacy` deleted; the file moves to `client/Components/` |
| `client/Legacy/LegacyPresentations.cs` (a view retained per trail entry) | `ExplorerPopover` | retains only an arriving `PolityDeltaNode` (FOCUS-8 removed `LegacyNodes.For`) | **replaced** by `BorderChangeTrail` (typed, F-57's per-entry retention) |
| `client/Legacy/LegacyNodes.cs` (`For`, `BookContainerId`, `ChapterContainerId`) | `LegacySaves`, `AuthorNode`, `BookNode`, `ChapterNode`, `LegacyPresentations` | `For` gone (FOCUS-8); the two id composers read only by `LegacySaves` | **deleted** |
| `client/Legacy/LegacySaves.cs` (`V1Node`, `V1Exploration`, `V2Link`, `V2Exploration`, `Translated<T>`, three keys) | `SavedExplorationsService`, `AppServices.StoredSelection` | unchanged | **deleted** (R4, OPEN 3) |
| `client/Legacy/AuthorNode.cs`, `YearNode.cs`, `YearFrontierSection` | chips of FOCUS-2/3's nodes; `EventDateAndPlacesSection` (FOCUS-5) | **deleted in FOCUS-8 Task 5** (no constructor left) | Author and Year *replacements* only (Task 2) |
| `PlaceDatesSection`, `TimeAndPlaceNode` | — | deleted in FOCUS-6 | nothing |
| `client/Legacy/PassageBlock.cs` (`PassageBlock`, `VerseTextResolver`), `client/Components/PassageList.razor` | event accounts, `ArrowNav`, `PolityDeltaScripturesSection` | FOCUS-5 and FOCUS-7 may keep or delete them (assumed: kept only if a reader remains) | if `BorderChangeView` is their only reader: they move to `client/Geography/` beside it, unchanged (OPEN 5); else they stay where they are |
| `client/Contracts/Frontier.cs` (`FocusKind`, `FocusKinds.Parse`, nine `IHas*` markers, `FrontierMatrix`) | `FrontierMatrixConformanceTests`, `FrontierMatrixRustParityTests` only (its Rust authority died in FOCUS-6) | unchanged (FOCUS-8 FINDING) | **deleted** with both tests |
| `client/Exploring/MapFocusHatch.cs` | `PopoverSectionProviders` (event place buttons, F-54) | FOCUS-5 (assumed deleted) | deleted if present |
| `client/Exploring/Contract/NodeIds.cs` (`Of`, `LocalPart`: client id composition, rule 25) | 16 files, 10 of them legacy | the legacy callers gone | **deleted** if no reader remains; otherwise each survivor is listed as a FINDING under F-31 with its owner |
| `SavedExplorationsService.Dropped`, `MainLayout._hamburgerNotice`'s dropped-steps text | v1 translation | unchanged | **deleted** with the reader |
| `client.Tests/stryker-config.json` entries `**/Legacy/Chip.cs`, `**/Legacy/LegacyNodes.cs`, `**/Legacy/LegacySaves.cs`, `**/Exploring/RenderedLegacyNodes.cs` (the last names no file) | — | — | **removed**; the Geography files added |

## Types (for sign-off, PRINCIPLES 12)

### A card field may open something (Task 2)

`client/Exploring/Presentation.cs` (amends §3.3 and FOCUS-4's `Year? At`):
```csharp
public sealed record Field(string Name, string Value, FieldOpens? Opens = null);

public abstract record FieldOpens
{
    private FieldOpens()
    {
    }

    public abstract T Match<T>(Func<Year, T> year, Func<Link, T> link);

    public sealed record AtYear(Year Year) : FieldOpens
    {
        public override T Match<T>(Func<Year, T> year, Func<Link, T> link) => year(Year);
    }

    public sealed record Along(Link Link) : FieldOpens
    {
        public override T Match<T>(Func<Year, T> year, Func<Link, T> link) => link(Link);
    }
}

public sealed record Card(string Title, IReadOnlyList<Field> Fields) : Presentation
{
    public bool Shows(EdgeKind kind);
}
```
- FOCUS-4's `Year? At` becomes `FieldOpens.AtYear`; its readers migrate (a mechanical rename at each `At` site: `GraphPresenter.LifeOf`, FocusView's year button). A field cannot both open a year and follow a link: one optional sum, not two optionals (the Haskell bar).
- `Card.Shows(kind)` holds when some field is `Along` a link of that kind.
- `GraphPresenter.CardOf` (async, as `Bounded` already is for geography):
  - **Established / Destroyed** (`NodeRecord.Place.Established`/`.Destroyed`): `Along(new Link(EdgeKind.SiteOf, new NodePosition(claim.Event)))` when `claim.Event` is served; otherwise, per OPEN 2, `AtYear(claim.When.From)` or no target. The value stays `claim.Label`.
  - **Author** (`NodeRecord.Book.Author`, the field FOCUS-3 adds): when the element's `Groups` hold `AuthoredBy` with a count of one, `Along(await Paging.FirstLink(element, EdgeKind.AuthoredBy))`; otherwise no target. The value stays the served `Author` text (OPEN 1).
- FocusView: a field with `AtYear` renders `{handle}-year-{Name}` (FOCUS-4); with `Along`, `<button data-testid="{handle}-field-link-{Name}">` invoking `OnFollow(link)`. A group whose kind the card `Shows` and whose count is one is not listed again (OPEN 1a). No per-kind override table: the rule reads the card.

### A place's claimed event happens there (Task 1, compiler)

`server/atlas-graph/src/law_check.rs`:
```rust
fn every_dated_claim_names_an_event_at_its_place(graph: &Graph, history: &[PlaceHistory]) -> Result<(), LawViolation>;
```
- For every `PlaceHistory` claim with `event: Some(e)`: `e` is an `Event` node and a `LocatedAt { event: e, place }` row exists for the claim's place. A violation names the place, the field (`established`/`destroyed`) and the event. Called from the compile's law pass beside the existing `located_at` checks.

### The border change (Task 3)

`client/Geography/BorderChange.cs`:
```csharp
public sealed record BorderChange(NodeRef Polity, Year From, Year To, PolityDelta? Delta)
{
    public string Title { get; }
}
```
- `Polity` is the served `NodeRef` of the clicked polity, taken from the `IMapSource` roster row whose id the map reports (FOCUS-6 Task 4 named every polity row's `Node`: `wire::Polity.node`); the event text, verses and grounding come from the same served row's `transition` or `fall` delta, which the map's payload already mirrors, so the view reads one served record; the `NodeIds.Of(NodeKind.Polity, …)` composition dies. `Delta` is the served row's `transition` or `fall` (the generated contract type `PolityDelta`: event, verses, grounding note), or none for a boundary with no curated delta; the three loose strings the map passes today are not restated. `PolityDeltaNode.DeltaKind` is not carried (read by nothing, rule 4).
- `Title` is today's `PolityDeltaNode.Title` (`"{Polity.Label}, {From.Label} → {To.Label}"`), unchanged; a FINDING proposes it served with MAPS.

`client/Geography/BorderChangeView.razor`:
```csharp
[Parameter, EditorRequired] public BorderChange Change { get; set; }
[Parameter, EditorRequired] public EventCallback<PopoverOpening> OnOpen { get; set; }
[Parameter] public EventCallback<string> OnShowOnMap { get; set; }
```
- Renders the three parts with today's ids: `popover-section-polity-delta-event`, `popover-section-polity-delta-scriptures` (with `polity-delta-verse-*` rows, rendered by the component `PolityDeltaScripturesSection` uses at the base; OPEN 5), `popover-section-polity-delta-grounding`; and, per OPEN 4, the `popover-chip-map` button calling `OnShowOnMap(WorldQuery.Between(From, To))`.

`client/Exploring/WorldQuery.cs` (FOCUS-4's composer) gains:
```csharp
public static string Between(Year from, Year to);
```

`client/Geography/BorderChangeTrail.cs` (replaces `LegacyPresentations`; F-57: retained per trail entry):
```csharp
public sealed class BorderChangeTrail
{
    public BorderChange? Present(Exploration exploration);
    public void Arrive(Exploration exploration, BorderChange change);
}
```
- Same retention rule as `LegacyPresentations.Retain` at the base: an entry keeps the change it arrived with while the trail prefix up to it is unchanged; any other entry has none.

`client/Components/PopoverOpening.cs` (moved from `client/Legacy/`, namespace `BibleAtlas.Client.Components`):
```csharp
public abstract record PopoverOpening
{
    public abstract T Match<T>(Func<PositionRef, T> explore, Func<SavedExploration, T> resume, Func<BorderChange, T> borderChange);

    public sealed record Explore(PositionRef Target) : PopoverOpening;
    public sealed record Resume(SavedExploration Saved) : PopoverOpening;
    public sealed record BorderChange(Geography.BorderChange Change) : PopoverOpening;
}
```
- `ExplorerPopover`: `OnInitializedAsync` opens a border change at `new NodePosition(change.Polity)` and arrives it on the trail; the body is `_borderChange is { } change ? <BorderChangeView …/> : <FocusView …/>`. `_legacy`, `_usingSectionRegistry`, `_sections`, `_chips`, `LoadLegacy`, `LoadSections`, `Activate`, `ChipGlyph`, `PushLegacyAsync`, the `IPopoverSectionContext` implementation (`PushAsync`, `ToggleSelectAsync`, `OtherContextSectionCount`, `XrefEntryPoint`, `Graph`, `RenewAsync`, `NavigateWorldAsync`) and the head-actions block go. `NavigateWorld(string query)` stays (FOCUS-4's `ShowYear` and the border change's map button read it).

## Deletion inventory (FOCUS-9 total)

- **Client files:** `client/Legacy/{IExplorable,Chip,PopoverSections,PopoverSectionProviders,PolityDeltaNode,LegacyNodes,LegacyPresentations,LegacySaves}.cs`; `client/Legacy/PopoverOpening.cs` (moved); `client/Legacy/PassageBlock.cs` (moved or kept per the inventory row); the `client/Legacy/` directory; `client/Exploring/PopoverChromeRegistry.cs`; `client/Contracts/Frontier.cs`; `client/Exploring/MapFocusHatch.cs` and `client/Exploring/Contract/NodeIds.cs` if present and unread.
- **Client tests:** `client.Tests/{PopoverChromeConformanceTests,PopoverSectionRegistryTests,FrontierMatrixConformanceTests,FrontierMatrixRustParityTests,MapFocusHatchTests}.cs`; `client.Tests/Explore/{ChipTests,LegacyNodesTests,LegacyPresentationsTests,LegacySavesTests,LegacyViews,PushViaConformanceTests,IdentityTests,DeletionLawTests}.cs` (each replaced where a law still has a subject: `DeletionLawTests` by Task 6's laws, `LegacyPresentationsTests` by `BorderChangeTrailTests`); the v1/v2 cases of `client.Tests/SavedExplorationsTests.cs` and `client.Tests/State/SelectionStoreTests.cs`; `client.Tests/Components/OpeningCallbackLawTests.cs`'s `Legacy` arm.
- **Client members:** `SavedExplorationsService`'s v2/v1 branches and `Dropped`; `MainLayout`'s dropped-steps notice; `AppServices.StoredSelection`'s v1 branch; `ExplorerPopover`'s members listed under Types; `PopoverOpening.Legacy` and `OpensLegacy`; `World.razor`'s `new PolityDeltaNode(...)`; `client.Tests/stryker-config.json`'s four stale entries.
- **Server:** none (Task 1 adds a law).
- **Not deleted** (stated so no one "finishes" it): `BookDetail.author` (the free text stays: 25 books have no linked author); `/api/node/{id}` (F-34); `/api/scene*`, `/api/polities` and `AtlasMapSource` (MAPS); `BorderChange` and its view (MAPS deletes them with the atlas's polities); `LocalStore`'s `explorations-v1-probe` key name (a storage probe, not a reader); the old keys' stored bytes in browsers (inert; no cleanup code, rule 4).

---

### Task 0: Ledger, base, and the inherited inventory (no code)

**Backend change: no.**

- [ ] **Step 1:** Write the ledger: the base, the landing commits of FOCUS-2…8, the OPEN answers.
- [ ] **Step 2:** Check the inventory's "After FOCUS-2…8" column by grep at the base, row by row: `grep -rn "class .* : IExplorable" client` → only `PolityDeltaNode`; `grep -rn "new Chip(" client` → only `PolityDeltaNode.cs`; `grep -rn "AppliesTo" client/Legacy` → only the three `PolityDelta*` providers; `grep -rn "PopoverOpening.Legacy(" client` → only `World.razor`; `grep -rln "VerseTextResolver\|PassageList" client` and `grep -rln "NodeIds\.\(Of\|LocalPart\)" client` (record the survivors with their owning batch); `grep -n "At\b" client/Exploring/Presentation.cs` (FOCUS-4's `Field.At` landed or not); `grep -n "Author" client/Exploring/Presenter.cs` (FOCUS-3's book fields). A miss in a row marked "deleted in FOCUS-n" is a FINDING against that batch, and the piece joins this batch's inventory (rule 24: every site of the category).
- [ ] **Step 3:** From the base's API, record: the books with exactly one `authored-by` neighbour (expected 14 by FOCUS-0 §7.4, read, not assumed); every place with a date claim and whether its claim names an event (7 claims over 4 places at FOCUS-0's grounding).

### Task 1: A place's claimed event happens at that place (compiler law)

**Backend change: yes (a compile-time law over curated data; no served code, no wire change). Why there (rule 27, 26):** whether a curated claim's event is located at the claimed place is a fact about the data alone; the compiler checks it once, so the client's link (`site-of` from the place to the event, Task 2) is always a real edge.

**Files:**
- Modify: `server/atlas-graph/src/law_check.rs` (and the compile's law pass that calls it).
- Test: `server/atlas-graph/src/law_check.rs` tests; `server/atlas-graph/tests/geography_real_data.rs`.

- [ ] **Step 1: Failing tests:** `a_claim_whose_event_is_elsewhere_fails_the_compile` (a two-place fixture; the whole violation); `a_claim_whose_event_is_at_its_place_passes`; real data: `every_curated_place_date_claim_names_an_event_at_that_place` (expected set read from `data/curated/place-history.toml` by the test).
- [ ] **Step 2:** `cargo test -p atlas-graph law_check && cargo test -p atlas-graph --test geography_real_data` → red (the law does not exist).
- [ ] **Step 3:** Implement. If the real-data law finds a curated claim whose event is not at its place, stop: that is a data question (the claim or the event's location is wrong). Record it under OWNER QUESTIONS with the place, the claim's verses and the event's `located-at`, and do not edit curated data on the side.
- [ ] **Step 4 (lock `heavy`):** `cargo test --workspace && (cd graph-types && cargo test --all-features)` → green.
- [ ] **Step 5: Commit:** `graph: a place's dated claim names an event that happens at that place, or the compile fails`.

### Task 2: Author and Year are links on the card (client)

**Backend change: no.** The facts (`DateClaim.event`, `authored-by`) are served; following one is the client's interaction (rule 27).

**Owner gate first:** OPEN 1, 2.

**Files:**
- Modify: `client/Exploring/{Presentation,Presenter,WorldQuery}.cs`, `client/Views/FocusView.razor`, `client/wwwroot/css/app.css` (`.focus-field-link`).
- Test: `client.Tests/Explore/{GraphPresenterTests,PresentationTests,ServedGraph}.cs`, `client.Tests/Views/FocusViewTests.cs`, `client.Tests/Explore/WorldQueryTests.cs`.

- [ ] **Step 1: Failing tests** (whole values):
  - `GraphPresenterTests`:
    - `A_place_dated_by_a_claimed_event_presents_the_date_as_a_link_to_that_event` (a Jerusalem-like fixture: `Established` → `Along(SiteOf, sam2_jerusalem_captured-like ref)`);
    - `A_place_date_with_no_claimed_event_presents_per_the_ruling` (OPEN 2: `AtYear` or no target);
    - `A_book_with_one_linked_author_presents_the_author_as_a_link_to_that_person`;
    - `A_book_with_no_linked_author_presents_the_author_as_text`;
    - `A_book_with_two_linked_authors_presents_the_author_as_text` (the group lists them).
  - `PresentationTests.A_card_shows_a_kind_when_one_of_its_fields_follows_a_link_of_that_kind`.
  - `FocusViewTests`: `A_field_that_follows_a_link_is_a_button_that_follows_it` (the `OnFollow` argument is the whole `Link`); `A_group_of_one_already_shown_by_a_field_is_not_listed_again`; `A_group_of_one_not_shown_by_a_field_is_listed`; FOCUS-4's year-button tests, retyped to `AtYear`.
  - `WorldQueryTests.A_span_of_years_opens_the_world_on_that_span` (whole string).
- [ ] **Step 2:** `dotnet test client.Tests --filter "GraphPresenterTests|PresentationTests|FocusViewTests|WorldQueryTests"` → red.
- [ ] **Step 3: Implement** the Types. `CardOf` becomes `Task<Card>` only if the author link needs it (it does: one `Paging.FirstLink` read, the door `Bounded` uses); callers already await `PresentAs`.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green.
- [ ] **Step 5: Commit:** `focus: a book's author and a place's dated claim are links on the card (R2: Author is the authored-by person, Year is the claimed event)`.

### Task 3: The border change is one typed view in the MAPS seam (client)

**Backend change: no.**

**Owner gate first:** OPEN 4, 5.

**Files:**
- Create: `client/Geography/{BorderChange,BorderChangeTrail}.cs`, `client/Geography/BorderChangeView.razor`; tests `client.Tests/Geography/{BorderChangeTests,BorderChangeTrailTests,BorderChangeViewTests}.cs`.
- Modify: `client/Pages/World.razor` (`OnPolityDeltaClick` builds a `BorderChange` from the roster row's served `Node`), `client/Components/ExplorerPopover.razor` (the `BorderChange` arm, beside the legacy machinery until Task 4), `client/Legacy/PopoverOpening.cs` (gains the `BorderChange` arm; `Legacy` stays until Task 4), `client/Exploring/WorldQuery.cs` (`Between`, if Task 2 has not landed it), `client/MapInterop.cs` only if `OnPolityDeltaClick`'s signature must carry the polity row's id unchanged (it does today: `polityId`).
- Test: `client.Tests/Explore/PopoverOpeningTests.cs`, `client.Tests/Components/ExplorerPopoverTests.cs`.

- [ ] **Step 1: Failing tests:**
  - `BorderChangeTests.A_border_change_is_titled_by_its_polity_and_its_years`;
  - `BorderChangeViewTests` (bUnit, whole markup per part): `A_border_change_with_an_event_verses_and_grounding_shows_all_three`; `A_border_change_with_none_shows_only_the_map_button`; `Opening_a_verse_from_a_border_change_opens_it_by_its_served_position`;
  - `BorderChangeTrailTests`: today's `LegacyPresentationsTests` cases, retyped (`Back_to_a_border_change_shows_it_again`, `A_new_branch_forgets_the_border_change_it_left`);
  - `PopoverOpeningTests.Opening_a_border_change_opens_its_polity_and_shows_the_change`;
  - `ExplorerPopoverTests.A_border_change_renders_its_view_and_not_the_polity_card`.
- [ ] **Step 2:** red. **Step 3: Implement.** `World.razor` looks the clicked polity up in the current `MapLayers.Polities` by the id the map reports and takes its `Node`; a click on a polity not in the roster is a `ContractBreach` (the map drew what the roster did not serve). The three part bodies move from the providers verbatim in markup and ids; the scriptures part opens a verse with `PopoverOpening.Explore` (as FOCUS-2 left `PassageList`).
- [ ] **Step 4:** `dotnet test client.Tests`; `npx playwright test tests/ux/world-border-morph.spec.ts` → green with no spec change (the ids and texts are unchanged).
- [ ] **Step 5: Commit:** `maps: a border change opens one typed view in the MAPS seam, on its polity's served node`.

### Task 4: The legacy popover machinery is gone (client)

**Backend change: no.**

**Files:**
- Delete: the inventory's `client/Legacy/{IExplorable,Chip,PopoverSections,PopoverSectionProviders,PolityDeltaNode,LegacyPresentations}.cs`, `client/Exploring/PopoverChromeRegistry.cs`, `client/Exploring/MapFocusHatch.cs` (if present) and their tests.
- Move: `client/Legacy/PopoverOpening.cs` → `client/Components/PopoverOpening.cs` (namespace `BibleAtlas.Client.Components`; every host's `@using` follows); `PassageBlock.cs`/`PassageList.razor` per the inventory row.
- Modify: `client/Components/ExplorerPopover.razor` (the Types list), `client/_Imports.razor`, every host of `PopoverOpening`, `client.Tests/Components/OpeningCallbackLawTests.cs`, `client.Tests/NamespaceLawTests.cs`, `client.Tests/Explore/ExploringBoundaryLawTests.cs` (the `BibleAtlas.Client.Legacy` boundary row), `client.Tests/stryker-config.json`.

- [ ] **Step 1: Failing tests:** `PopoverOpeningTests.A_popover_opening_is_an_exploration_a_resume_or_a_border_change` (the `Match` arms, whole); `NamespaceLawTests.No_type_lives_in_the_legacy_namespace` (reflection: the app assembly has no type in `BibleAtlas.Client.Legacy`). Red.
- [ ] **Step 2:** Delete and move per the inventory; `PopoverOpening.Legacy` and `OpensLegacy` go; the compiler lists every remaining site; each is a host passing `Explore`, `Resume` or `BorderChange` already, or a deleted test.
- [ ] **Step 3:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green. `npx playwright test tests/ux/world-border-morph.spec.ts tests/ux/popover-sections.spec.ts tests/ux/explore-edges.spec.ts tests/ux/node-kinds.spec.ts` → green.
- [ ] **Step 4: Commit:** `focus: the legacy popover is gone -- IExplorable, chips, the section registry and its providers, PolityDeltaNode and the chrome registry (spec F7 closed)`.

### Task 5: The old save and selection readers are gone (client)

**Backend change: no.**

**Owner gate first:** OPEN 3. Under "no", only `explorations-v1` goes (R4) and `V2*`/`selection-v1` stay, with `LegacySaves` renamed `OlderSaves` and moved to `client/State/`.

**Files:**
- Delete: `client/Legacy/{LegacySaves,LegacyNodes}.cs`, `client.Tests/Explore/LegacySavesTests.cs`.
- Modify: `client/SavedExplorationsService.cs`, `client/AppServices.cs`, `client/Layout/MainLayout.razor`, `client.Tests/SavedExplorationsTests.cs`, `client.Tests/State/SelectionStoreTests.cs`, `client.Tests/stryker-config.json`.

- [ ] **Step 1: Failing tests:** `SavedExplorationsTests.A_store_with_only_older_saves_opens_empty` and `.The_service_reads_only_the_current_key` (the in-memory local storage's whole read log equals `[explorations-v3]`); `SelectionStoreTests.A_store_with_only_an_older_selection_opens_empty`. Red.
- [ ] **Step 2:** Delete the branches, `Dropped`, the notice, the records and both files. `LegacyNodes.BookContainerId`/`ChapterContainerId` die with their last reader.
- [ ] **Step 3:** `dotnet test client.Tests` → green.
- [ ] **Step 4: Commit:** `state: saved explorations and the selection read only their current keys (R4: the older readers are retired)`.

### Task 6: No legacy kind vocabulary remains, and none can be written (client laws)

**Backend change: no.**

**Files:**
- Delete: `client/Contracts/Frontier.cs`, `client.Tests/{FrontierMatrixConformanceTests,FrontierMatrixRustParityTests}.cs`; `client/Exploring/Contract/NodeIds.cs` and its tests if Task 0 found no reader left (else the survivors are a FINDING).
- Create: `client.Tests/Explore/KindVocabularyLawTests.cs`.
- Modify: `client.Tests/stryker-config.json` (the four stale entries out; `**/Geography/BorderChange.cs`, `**/Geography/BorderChangeTrail.cs`, `**/Exploring/WorldQuery.cs` in, if not already).

- [ ] **Step 1: Failing laws** (`KindVocabularyLawTests`), the 24b closures for "a kind decided from a string" (rule 25):
  - `No_client_source_compares_a_kind_to_a_string` — scans `client/**/*.cs` and `*.razor` for `Kind ==`/`Kind is`/`Kind !=` followed by a string literal; the failure lists `path:line`;
  - `No_client_source_names_a_retired_kind` — the retired kinds are derived, not listed: every quoted identifier in a source that is not a `NodeKind`/`EdgeKind` member but was a legacy `IExplorable.Kind` value is found by scanning for `"(Verse|Passage|Chapter|Book|Author|Year|TimeAndPlace|PolityDelta|ConcordUnit|Catechism)"` as a whole string literal; the pattern lives only in this test;
  - `Every_stryker_glob_matches_a_file` (FOCUS-8's FINDING closure).
  Red on `Frontier.cs` and the stale stryker entry.
- [ ] **Step 2:** Delete per the files list; re-run until green.
- [ ] **Step 3:** `dotnet build client && dotnet test client.Tests` → green.
- [ ] **Step 4: Commit:** `client: no kind is decided from a string and no legacy kind is named; the frontier-matrix mirror is gone (rule 25 closed)`.

### Task 7: Re-express the specs (Playwright)

**Backend change: no.**

**Files:** `tests/ux/saved-explorations.spec.ts`, `tests/ux/state-focus.spec.ts`, `tests/ux/reader-map.spec.ts` (its `YearNode`/`EventNode` comment lines go with the lines they describe), `tests/ux/node-kinds.spec.ts` (FOCUS-8: the book-author and place-date tests below), `tests/ux/CONTRACT.md`.

- [ ] **Step 1: Re-express** by name, every expected value from the API (F-8):
  - `saved-explorations.spec.ts`: the drill that pushed an `AuthorNode` via "About this book" follows the chapter's `member-of` link to the book, then the book's author link (`popover-field-link-Author`) to the person, saves, continues, and walks Back; `EXPLORE-TRAIL-1` (a seeded v1 trail) becomes "an older save is not read: the hamburger lists no exploration and shows no dropped-steps notice" (OPEN 3; under "no", the test stays as it is).
  - `state-focus.spec.ts`: the "About this book" drill as above; `ST-3/R2` (cold-start `selection-v1`) becomes "an older selection is not read" (OPEN 3).
  - `node-kinds.spec.ts` gains: "a book with one linked author offers its author as a link that opens the person, and does not list it twice" (Genesis-like: the first book from the API with one `authored-by` neighbour); "a place's dated claim opens the event that dates it" (Jerusalem-like: the first place whose record's `established.event` is served; `popover-field-link-Established`; title = that event's served label; Back returns to the place).
  - `CONTRACT.md`: the chip ids (`popover-chip-book`, `popover-chip-context`, `popover-chip-year-*`) are recorded as retired with their replacements (the book's `member-of` crumb, the chapter's `contains`, `popover-year-*`); `popover-chip-map` stays (FocusView's hatch and the border change).
- [ ] **Step 2:** `npx playwright test tests/ux/saved-explorations.spec.ts tests/ux/state-focus.spec.ts tests/ux/node-kinds.spec.ts tests/ux/world-border-morph.spec.ts tests/ux/reader-map.spec.ts` → green.
- [ ] **Step 3: Commit:** `tests: the saved-exploration and focus specs walk Author and Year as links; older saves are not read`.

### Task 8: Gates, mutation, close, and the spec's last rows

- [ ] **Step 1: Gates.** Mutation inside the owner's window only (critical section `heavy` with "mutation" in the message, `free -g` ≥ 18 GB): `bash scripts/mutants-parallel.sh -n 3 -b <base>` (covers the new law) → 100% or equivalents recorded; then `dotnet stryker` (and the `exploring` config) → 100% or equivalents recorded. Outside the window, `.superpowers/MUTATION-GATE-DEBT.md` names `<base>`. Then (lock `heavy`): `cargo test --workspace && (cd graph-types && cargo test --all-features) && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh --base <base> && bash scripts/timing-gates.sh run && npx playwright test tests/ux` → green except the carried reds. The diff adds no comment line (`git diff <base>.. -- server graph-types client | grep -E '^\+.*(//|/\*|<!--)'` prints nothing). `client/` line count before and after is recorded (rule 25: less client code is the direction; this batch should remove well over 2,000 lines).
- [ ] **Step 2: Spec amendment** (`docs/superpowers/specs/2026-09-26-focus-exploration-design.md`, a §13 "FOCUS closed" entry rather than edits to signed text): §5's FOCUS-9 row as built (Year → the claimed event, per R2; `YearNode`, `AuthorNode`, `TimeAndPlaceNode` deleted earlier by rule 4); §9 every row marked with the batch that did it; §3.3 `Field.Opens`; §12 R20's `client/Legacy/` boundary retired; the border change in the MAPS seam.
- [ ] **Step 3:** Close report `docs/superpowers/reports/2026-10-0x-focus-9-close.md`: the rule-24 table, the FINDINGS, behaviour removed by rulings (older saves; the dropped-steps notice; the chip glyphs in the popover head).
- [ ] **Step 4:** Push `lane/<agent>/F9-int`; queue item to `review`; the other agent reviews (14b + 24a); Claude lands under `land` and rewrites the queue STATUS (FOCUS complete).

**Rule-24 categories and their closures:**

| Category | Closure | Guarantee |
|---|---|---|
| A kind served by both mechanisms (spec F7) | the legacy family deleted; `PopoverOpening` has three arms | compiler (`Match`), `No_type_lives_in_the_legacy_namespace` |
| A kind decided from a string (rule 25) | `Frontier.cs`, the chrome registry, every `AppliesTo` gone | `No_client_source_compares_a_kind_to_a_string`, `No_client_source_names_a_retired_kind` |
| A client-only kind with no node (R2) | Author → the `authored-by` person; Year → the claimed event | `FieldOpens.Along` over served refs only; the compile law guarantees the event is at the place |
| An id composed on the client (F-31/F-54) | `PolityDeltaNode`'s composed polity id replaced by the roster's served `Node`; `LegacyNodes`' composers gone; `NodeIds` gone if unread | compiler (no `NodeIds`), or a FINDING naming every survivor |
| A presentation lost on Back (F-57) | `BorderChangeTrail`, typed per entry | `BorderChangeTrailTests` |
| Two optionals where one sum belongs (`Field.At` + a link) | `FieldOpens` | compiler (`Match`) |
| A mutation config naming a file that does not exist | stale entries removed | `Every_stryker_glob_matches_a_file` |

---

## Wave schedule

Primary is the critical path; the companion is unlike work beside it (rule 23). A task starts when the tasks it names as inputs have landed on `lane/<agent>/F9-int`.

| Wave | Primary | Companion | Critical sections | Expected red at wave close |
|---|---|---|---|---|
| 0 | owner: OPEN 1–5 (defaults stand if unanswered); Task 0 | — | — | — |
| 1 | Task 2 (C#) | Task 1 (Rust) | `heavy` (Task 1) | Playwright: `saved-explorations`/`state-focus` drills that clicked the retired chip, if FOCUS-8 left them (named in the ledger) |
| 2 | Task 3 (C#) | Task 5 (C#; disjoint files) | — | as wave 1 |
| 3 | Task 4 (C#) | — | — | as wave 1 |
| 4 | Task 6 (C#) | Task 7 (TypeScript) | — | — |
| 5 | Task 8 | — | `heavy` (mutation only in the owner's window); `land` | only the carried reds |

**Critical path:** OPEN answers → Task 2 → Task 3 → Task 4 → Task 6 → Task 8. Task 1 lands before Task 7's place-date test; Task 5 before Task 7's older-save tests.

## FINDINGS this plan expects to raise (for the queue; the owner decides)

- **A border change's title is composed on the client** (`"{polity}, {from} → {to}"`, rule 25). Closure: served with the map's `ChangeEvent` (MAPS, R2's PolityDelta row).
- **A border change's verses are reference strings, not served positions** (`"EZR.1.1-3"` from `/api/polities`). Closure: MAPS serves them as text-unit refs.
- **`BookDetail.author` is free text beside the `authored-by` edge** for the 14 linked books (the same fact twice, D.R.Y.). Closure: the compiler serves the free text only for books with no linked author, or the 25 unlinked authors are curated (`data/curated/books.toml` `author_ids`); the owner chooses.
- **Survivors of `NodeIds.Of`** (if Task 0 finds any), each with the batch that owns it (F-31).
- **The rule-4 pull-forward** (FOCUS-8's finding) is closed by this batch's deletion of `IExplorable`: no legacy class can outlive its constructor again.

## Self-review against rule 27, the spec and the brief

- **Rule 27:** the only backend change is a compile-time law over curated data (Task 1). No route, field, relation or element is added; the server derives nothing. The client composes the two links from served values (`DateClaim.event`, the `authored-by` page) through the one walk and paging doors.
- **Spec §5 FOCUS-9:** Author → `authored-by` Person (Task 2); TimeAndPlace → the Event (done in FOCUS-6, confirmed by Task 0); Year → R2's claimed event (Task 2, the §5 wording reconciled above); `YearFrontierSection`, `PlaceDatesSection`, `AuthorNode`, `TimeAndPlaceNode`, `YearNode` gone (FOCUS-6/8, confirmed); the v1 save reader (Task 5); `PopoverSectionRegistry` (Task 4), with the `PolityDelta*` sections leaving the registry as one typed view held for MAPS rather than keeping the registry alive for three entries.
- **Spec §9:** `Chip`/`ChipTarget` die with their last author (Task 4); `LegacyNodes` (FOCUS-8: `For`; Task 5: the rest); the v1 save/selection translation (Task 5); `IExplorable.ExploreAsync`/`BodyAsync` (FOCUS-8: `BodyAsync`; Task 4: the interface); the registry and `IPopoverSectionProvider` (Task 4).
- **The brief's end state:** no legacy popover machinery remains; the border change lives in `client/Geography/` beside `AtlasMapSource`, and MAPS deletes both.
- **Assumptions to verify at execution (Task 0):** FOCUS-4's `Field.At` and `WorldQuery` landed (else Task 2 introduces `FieldOpens` with `AtYear` unused by any field except OPEN 2(b), and `WorldQuery` with `Between` only); FOCUS-3's `Author` field landed on the book card (else Task 2 adds it, from `NodeRecord.Book.Author`); FOCUS-6's polity roster rows carry `Node` (they do at `b3d7cfa`: `server/atlas-contract/src/wire/map.rs` `Polity.node`); FOCUS-5 and FOCUS-7 deleted `MapFocusHatch`, `EventNode`, `INarrativeAware`, `CatechismNode`, `CommentaryItemNode` (else they join this batch's inventory as FINDINGS against those batches).
