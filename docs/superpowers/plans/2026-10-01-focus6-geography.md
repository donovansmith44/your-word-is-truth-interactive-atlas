# FOCUS-6 (Geography, with A-EDGES) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

## OPEN — for the owner, before Task 1 starts (each blocks only the task named)

1. **Edge ends as link kinds (Task 1):** append `EdgeSource => "from" / "source-of"` and `EdgeTarget => "to" / "target-of"` to `relations!` (22 → 24 directed), with node cards NOT summarising `source-of`/`target-of` (those members are already the `edge` ids on every page) — yes, or another shape?
2. **Hover on the map (Task 8):** with `PlaceCard` gone, does hovering a place still preview it (the same popover card, transient), or is a click the only way in?
3. **Window-scoped place content (Tasks 3, 8):** the hover card's period name, period blurb, events-in-this-window with their verses, and per-place narrative prev/next — accept the popover's window-free card plus the full `site-of` frontier, or serve a windowed card (`/api/node/{id}?from&to`)?
4. **`/world` with nothing focused (Task 8):** keep today's free slider over all time until a Map/Era is focused, or always open on a Map (needs a served "Map at year")?
5. **Crossing an era (Tasks 7, 8):** does dragging past the focused Map's bound follow `follows-in` by itself, or only an explicit arrow click at the bound (as NAV-UNIFORM-1 does for the next book)?

Defaults this plan builds if unanswered: (1) yes; (2) click only, no hover preview; (3) window-free card; (4) free slider; (5) explicit click.

**Goal:** Make edges explorable (A-EDGES, spec §12 R18) and move Place, Map, Polity and Era onto the new path: the World view presents `Presentation.Geography` (R9/R10/R16 rows on `Surface.World`), a polity focus highlights its territory at the slider's year and shows its reign (R14), the slider scrubs within a focused Map's window and crosses to the next Map by `follows-in` (R10); the four `Place*Section`s, `PlaceNode`, `PlaceCard` and `/api/place` die.

**Architecture:** A-EDGES generalises the FOCUS-1 types from node to *element*: `ElementKind = Node(NodeKind) | Edge(EdgeKind)`, `Link.Target: PositionRef`, `Explorable` over a node card or an edge card, `Follow` total. FOCUS-6 then adds one presentation form, `Presentation.Geography(Frame, Emphasis)`, built by `GraphExplorer` from served card details only. The World view reads the exploration's current node, asks for its `Surface.World` presentation, and applies the frame to the slider and the emphasis to the map. **Map data stays the atlas's existing scene/polity data behind one seam, `IMapSource`** (see "The MAPS seam" below).

**Tech Stack:** Rust (axum, utoipa, `atlas-contract`, `atlas-graph`, `graph-types`); .NET 10 Blazor WebAssembly; xUnit; Playwright; Stryker.NET; cargo-mutants via `scripts/mutants-parallel.sh`.

**Spec:** `docs/superpowers/specs/2026-09-26-focus-exploration-design.md` §5 (FOCUS-6 row), §10 R9/R10, §11 R14, §12 R15–R18. **Queue:** A-EDGES, A-MAPS-SPEC, A-FPLANS; closes F-21, F-22, F-23, F-28, F-29.

**Base:** FOCUS-1 at `3baaeb6` (branch `lane/claude/A-F1`). Every gate in this batch takes `--base 3baaeb6` (PRINCIPLES 22). If A-F1 lands on `worktree-bible-atlas-m1` with fixes, rebase and write the new base into the ledger at Task 1.

## Global Constraints

- `docs/PRINCIPLES.md` binds, especially: rule 9 (no comments in application code — every snippet below has none), 12 (every signature below is for sign-off), 21 (critical sections, below), 24/24a/24b (every fix names its category and closes it; offenders found on the side go to FINDINGS, never fixed on the side), 25 (the client composes over the contract: no year arithmetic, no id composition, no label composition on the client), 26 (the server reads facts from the artifact only).
- Tests: whole-body assertions, one behaviour per test named as a sentence, `// Arrange` `// Act` `// Assert` only, no magic numbers, newspaper order.
- **Total matches, not switches with a discard.** C# cannot prove a closed record hierarchy exhaustive, so every new sum (`ElementKind`, `Frame`, `Emphasis`, `PopoverOpening`) exposes an abstract `Match<T>` implemented once per arm; no `switch` over them anywhere (a source-scan law in Task 2 enforces it). Enums keep the existing exhaustive-switch build error.
- Names are this plan's: `ElementKind.{Node,Edge}`, `Link`, `Explorable`, `IExplorer.{Resolve,Follow,Present}`, `Presentation.{Card,Field,Geography}`, `Frame.{Current,Bounded}`, `Emphasis.{None,Site,Territory}`, `Affordance.{Arrows(ArrowDirection),InlineChildren,UpCrumb,EntryEnd,SectionList}`, `ArrowDirection.{Previous,Next}`, `PopoverOpening.{Explore,Resume,Legacy}`, `IMapSource`, `AtlasMapSource`, `MapLayers`, `Crossing`.
- The Playwright suite is the behaviour gate; the expected-red sets per task are listed in the wave schedule and re-expressed in Task 10. Known reds carried from FOCUS-1: `world-quiet-places:211`, `world-cluster-chooser:213` (O-CHOOSER).
- Build no interaction that works only by hovering (A-F1 note).
- Commit per task; push at the end of the batch to `origin/worktree-bible-atlas-m1`, never force.

## Critical sections (PRINCIPLES 21) — one holder at a time

| Section | Held by | Why |
|---|---|---|
| appending to `relations!` | Task 1 | positional (`graph-types/src/edge.rs`) |
| regenerating `contracts/openapi.yaml` + `aqc.schema.json` (`cargo run -p atlas-contract --bin export_contract`), then `Wire.g.cs` (`dotnet run --project client.ContractGenerator`) | Task 1, then Task 3, then Task 9 — strictly in that order | one generated document |
| re-blessing pacts/fixtures | Task 1, Task 3, Task 9 (right after their regen) | moves the version root |
| the mutation gate | Task 10 only | once per batch (3a) |

## The MAPS seam

`Presentation.Geography`, `Frame`, `Emphasis` and the `Surface.World` rows of `Presentation.Of` are built from `NodeCard` details alone (Task 4) and name no map data. The map's layers come from **`IMapSource`** (Task 5):

```csharp
public interface IMapSource
{
    Task<MapLayers> During(int from, int to);
    Task<MapLayers> For(string scriptureRef);
}

public sealed record MapLayers(Scene Scene, IReadOnlyList<Polity> Polities);
```

FOCUS-6's only implementation is `AtlasMapSource(AtlasClient atlas)`, over `/api/scene`, `/api/scene/scripture` and `/api/polities`. The MAPS migration replaces `AtlasMapSource` (and `MapLayers`' payload types, and `MapInterop`/`map.js` beneath the World view) with map-generator's registry and renderer. It does not touch `Presentation`, `Frame`, `Emphasis`, `GraphExplorer.Present`, the table rows, `Crossing`, or the exploration. `/api/scene` therefore stays in this batch (the spec's §5 note "MAPS may retire it instead" is taken). `PolityDeltaNode` and the three `PolityDelta*Section`s stay for MAPS (R2).

## Re-anchor table — what the World view touches today (at `3baaeb6`), and where it goes

| Today | Role | FOCUS-6 |
|---|---|---|
| `client/Pages/World.razor` | page shell, split/follow, picker, slider, hover card, popover host | keeps shell/split/follow/picker/scripture mode; loses the card plumbing (Task 8); presents the current Geography focus |
| `client/Components/PlaceCard.razor` (491 lines) | hover/pinned place card: windowed verses, blurb, dates, narrative prev/next | **deleted** (Task 9); its content is the popover `Card` + frontier (OPEN 3) |
| `client/Components/PlaceChooser.razor` | cluster chooser | kept; picks a served `NodeRef` (Task 8); O-CHOOSER unchanged |
| `client/Components/TimeSlider.razor` | eras, window, drag/change | gains `Bounds`, `Band`, `OnCross` (Task 7) |
| `client/Components/Legend.razor` | narrative legend/isolate | unchanged |
| `client/MapInterop.cs` (`IMapEvents`, `MapInterop`, `MapEventsSink`) + `client/wwwroot/js/map.js` | the renderer | gains `Emphasize`, `OnPolityClick`; loses `MeasureCardPlacement`/`CardMeasurement` (Tasks 7, 9) |
| `client/SliderWindow.cs` | labels the slider's window in scripture mode | unchanged |
| `client/AtlasClient.cs` `Eras`, `Landmarks`, `LandMask`, `Polities`, `SceneTime`, `SceneScripture` | map data | `Polities`/`SceneTime`/`SceneScripture` move behind `AtlasMapSource` (Task 5); `Place`, `PlaceHistory` **deleted** (Task 9) |
| `client/Explore/PlaceNode.cs`, `PlaceDates.cs`, `YearNode.cs`, `TimeAndPlaceNode.cs`, `client/Components/PlaceEventsList.razor`, `client/CardPlacement.cs` | legacy place path | **deleted** (Task 9) |
| `PlaceDescriptionSection`, `PlaceDatesSection`, `PlaceBlurbSection`, `PlaceEventsSection`, `YearFrontierSection` (`client/Explore/PopoverSectionProviders.cs`, registry rows in `PopoverSections.cs`) | legacy popover sections | **deleted** (Task 9) |
| `client/Components/MentionScan.razor` | opens `new PlaceNode(...)` from a text mention | opens `PopoverOpening.Explore` (Task 6) |
| `client/Explore/LegacyNodes.cs` `NodeKind.Place` arm | bridge | → `null` (Task 9) |
| `client/Explore/PolityDeltaNode.cs` + three `PolityDelta*Section`s | border-change popover | unchanged (MAPS) |
| `server/atlas-contract/src/places.rs` `/api/place/{id}`; `wire/places.rs` `PlacePage`, `History` | legacy route | **deleted** (Task 9) |
| `server/atlas-contract/src/graph.rs` `node_card`; `wire/graph.rs` | the card | gains `map`/`era`/`polity` details, `place.blurb` (Task 3); edge card (Task 1) |
| `server/atlas-core/src/wire.rs` `ScenePlace`, `QuietPlace`; `server/atlas-contract/src/wire/map.rs` `Era`, `Polity` | map wire | each gains `node: NodeRef` (Task 3) |

**Pulled forward from FOCUS-9 by rule 4 (zero dead code):** `YearNode` is constructed only by `PlaceCard`, and `TimeAndPlaceNode` only by `PlaceNode`/`PlaceEventsList`, so they and `YearFrontierSection`, `PlaceDatesSection` die here. FOCUS-9's row shrinks to `AuthorNode`, the v1 save reader and `PopoverSectionRegistry`.

## Types (for sign-off — PRINCIPLES 12)

### A-EDGES (Tasks 1–2)

Server wire (`server/atlas-contract/src/wire/graph.rs`):
```rust
pub struct EdgeRef { pub id: String, pub kind: EdgeKind, pub label: String }

pub struct EdgeCard {
    pub id: String,
    pub kind: EdgeKind,
    pub label: String,
    pub from: PositionRef,
    pub to: PositionRef,
    pub provenance: String,
    pub loci: Option<Vec<TextSpan>>,
    pub votes: Option<u32>,
    pub narrative: Option<NarrativeId>,
    pub note: Option<String>,
    pub edge_summary: Vec<EdgeSummaryEntry>,
    pub version: String,
}

pub enum EdgeEnd { From, To }
```
`EdgeEntry` gains `pub end: EdgeEnd`.

Routes: `GET /api/edge/{id}` → `EdgeCard`; `GET /api/edge/{id}/edges?kind&cursor&limit` → `EdgePage` (kinds: `from`, `to`, `justified-by`, and any served on `Position::Edge`). `EdgeEntry.end` says whether the page's own node is the edge's `from` or `to` end (so the client steps onto the edge with `source-of` or `target-of` without deciding it). The edge's label is composed on the server from its kind's display label and its ends' labels; each `EdgeKind` gains a served display label in the vocabulary (`x-atlas-relations` carries `label`) — closes F-23.

`graph-types/src/edge.rs` (OPEN 1):
```rust
EdgeSource => "from" / "source-of",
EdgeTarget => "to" / "target-of"
```
appended last; `DECLARED_DIRECTED_RELATIONS` 22 → 24, `DECLARED_EDGE_KINDS` follows by its formula.

Client (`client/Explore/`):
```csharp
public abstract record ElementKind
{
    private ElementKind() { }
    public abstract T Match<T>(Func<NodeKind, T> node, Func<EdgeKind, T> edge);
    public static ElementKind Of(PositionRef position);
    public sealed record Node(NodeKind Kind) : ElementKind;
    public sealed record Edge(EdgeKind Kind) : ElementKind;
}

public sealed class Explorable
{
    internal Explorable(NodeCard card, IExplorableClient graph);
    internal Explorable(EdgeCard card, IExplorableClient graph);
    public ElementKind Kind { get; }
    public string Id { get; }
    public string Label { get; }
    public PositionRef Identity { get; }
    public IReadOnlyList<FrontierGroup> Groups { get; }
    public Task<Page<Link>> Links(EdgeKind kind, int? cursor = null);
}

public sealed record Link(EdgeKind Kind, PositionRef Target);

public static class Positions
{
    public static (ElementKind Kind, string Id, string Label) Of(PositionRef position);
}

public interface IExplorer
{
    Task<Explorable> Resolve(PositionRef target);
    Task<Explorable> Follow(Link link);
    Task<Presentation?> Present(Explorable node, Surface surface);
}

public interface IExplorableClient
{
    const int DefaultPageSize = 20;
    Task<NodeCard> Card(string id);
    Task<EdgeCard> EdgeCard(string id);
    Task<EdgePage> Edges(string id, EdgeKind kind, int? cursor = null, int limit = DefaultPageSize);
    Task<EdgePage> EdgeEdges(string edgeId, EdgeKind kind, int? cursor = null, int limit = DefaultPageSize);
    Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible);
}

public static Form? Presentation.Of(ElementKind kind, Surface surface);
public static bool Presentation.Offers(Link link, Surface surface);
public static Surface HomeSurfaces.Of(ElementKind kind);

public abstract record Affordance
{
    public sealed record Arrows(ArrowDirection Direction) : Affordance;
    public sealed record InlineChildren : Affordance;
    public sealed record UpCrumb : Affordance;
    public sealed record EntryEnd : Affordance;
    public sealed record SectionList(SectionStyle Style, int InitialClamp, SectionOrder Order) : Affordance;
}
public enum ArrowDirection { Previous, Next }

public sealed record SavedExploration(string Id, string Name, DateTimeOffset CreatedUtc, PositionRef Start, IReadOnlyList<Link> Steps);
```
- `Explorable.Links` no longer filters edge positions (`Neighbours.Nodes()` leaves it; F1-12's filter retires): every served neighbour is a `Link`. `Follow` is total.
- `Affordances.Of`: `FollowsIn` → `Arrows(Next)`, `PrecedesIn` → `Arrows(Previous)` (F-22); `EdgeSource`, `EdgeTarget` → `UpCrumb` (an edge card shows its two ends as crumbs); `SourceOf`, `TargetOf` → `EntryEnd` (offered as a small "this connection" control on each entry of another group, using the entry's served `end`, never as a section).
- `Presentation.Of(ElementKind.Edge _, Popover)` = `Card` for every edge kind; `World`/`Reader` = `null` for every edge kind. So `Offers` is total on the popover (a law), and F-28 closes: a section heading shows its served count **only** on `Surface.Popover`, where the law guarantees every link is offered.
- Selection stays node-only: `ToggleSelection` takes a `NodeRef`; FocusView offers select only on `NodePosition` targets (`Positions.Of(...).Kind.Match(node: _ => true, edge: _ => false)`).
- Saves: storage key `explorations-v3`; `LegacySaves` gains the v2 → v3 translation (a v2 `NodeRef` becomes a `NodePosition`; every v2 save maps whole, R4). The v1 reader is unchanged (FOCUS-9 deletes v1 and v2 readers together).

### FOCUS-6 (Tasks 3–9)

Server wire additions (`server/atlas-contract/src/wire/graph.rs`, `wire/map.rs`, `server/atlas-core/src/wire.rs`):
```rust
pub struct MapDetail { pub window: TimeRange }
pub struct EraDetail { pub window: TimeRange }
pub struct PolityDetail { pub reign: TimeRange }
```
Added fields: `NodeCard.map: Option<MapDetail>`, `NodeCard.era: Option<EraDetail>`, `NodeCard.polity: Option<PolityDetail>`, `PlaceDetail.blurb: Option<String>`, and `node: NodeRef` on `ScenePlace`, `QuietPlace`, `Polity`, `Era`.
- `MapDetail.window` and `EraDetail.window` from the node payloads' `from_year`/`to_year` (`NodePayload::Map`, `NodePayload::Era`); `PolityDetail.reign` is the span of the polity's eras (`NodePayload::Polity.eras`), computed on the server; `PlaceDetail.blurb` is the place's default-period `History.blurb` (OPEN 3 may widen this).
- `node: NodeRef` on the map wire lets the World view open what was clicked without composing an id (rule 25; the F-31 category).
- Each `TimeRange` carries its served `label`; the client never formats a year.

Client (`client/Explore/Presentation.cs`, `client/Explore/Geography.cs`, `client/Geography/`, `client/Components/`):
```csharp
public sealed record Presentation.Geography(Frame Frame, Emphasis Emphasis) : Presentation;

public abstract record Frame
{
    private Frame() { }
    public abstract T Match<T>(Func<T> current, Func<TimeRange, Link?, Link?, T> bounded);
    public sealed record Current : Frame;
    public sealed record Bounded(TimeRange Window, Link? Previous, Link? Next) : Frame;
}

public abstract record Emphasis
{
    private Emphasis() { }
    public abstract T Match<T>(Func<T> none, Func<NodeRef, double, double, T> site, Func<NodeRef, TimeRange, T> territory);
    public sealed record None : Emphasis;
    public sealed record Site(NodeRef Place, double Lat, double Lon) : Emphasis;
    public sealed record Territory(NodeRef Polity, TimeRange Reign) : Emphasis;
}

public static class Crossing
{
    public static ArrowDirection? Of(TimeRange bounds, int from, int to);
}

public interface IMapSource
{
    Task<MapLayers> During(int from, int to);
    Task<MapLayers> For(string scriptureRef);
}
public sealed record MapLayers(Scene Scene, IReadOnlyList<Polity> Polities);
public sealed class AtlasMapSource(AtlasClient atlas) : IMapSource;

public abstract record PopoverOpening
{
    private PopoverOpening() { }
    public abstract T Match<T>(Func<PositionRef, T> explore, Func<SavedExploration, T> resume, Func<IExplorable, T> legacy);
    public sealed record Explore(PositionRef Target) : PopoverOpening;
    public sealed record Resume(SavedExploration Saved) : PopoverOpening;
    public sealed record Legacy(IExplorable Node) : PopoverOpening;
}

public Task MapInterop.Emphasize(Emphasis emphasis);
void IMapEvents.OnPolityClick(string polityId);

[Parameter] public TimeRange? TimeSlider.Bounds { get; set; }
[Parameter] public TimeRange? TimeSlider.Band { get; set; }
[Parameter] public EventCallback<ArrowDirection> TimeSlider.OnCross { get; set; }
```
The table rows (R16) this batch fills:

| Kind | `Surface.World` | `Surface.Popover` card fields (served) |
|---|---|---|
| Map | `Geography(Bounded(map.window, precedes-in link, follows-in link), None)` | Window (`map.window.label`), Provenance |
| Era | `Geography(Bounded(era.window, precedes-in link, follows-in link), None)` | Window (`era.window.label`), Provenance |
| Place | `Geography(Current, Site(ref, place.lat, place.lon))` | Canonical name, Established, Destroyed (`DateClaim.label`), Blurb, Provenance — each only where served |
| Polity | `Geography(Current, Territory(ref, polity.reign))` — R14: stay in the current era, no jump | Reign (`polity.reign.label`), Provenance |

`PopoverOpening.Legacy` exists only for callers that still hold a legacy node; its last users (`PolityDeltaNode` for MAPS, the legacy kinds for FOCUS-2…9) remove it. Closes F-29.

## Deletion inventory (FOCUS-6 total)

Client — files: `client/Explore/PlaceNode.cs`, `client/Explore/PlaceDates.cs`, `client/Explore/YearNode.cs`, `client/Explore/TimeAndPlaceNode.cs`, `client/Components/PlaceCard.razor`, `client/Components/PlaceEventsList.razor`, `client/CardPlacement.cs`; tests `client.Tests/PlaceDatesTests.cs`, `client.Tests/YearNodeEventTimeTests.cs`, `client.Tests/CardPlacementTests.cs`, `client.Tests/ServedYearsTests.cs`.
Client — members: `PlaceDescriptionSection`, `PlaceDatesSection`, `PlaceBlurbSection`, `PlaceEventsSection`, `YearFrontierSection` and their `PopoverSectionRegistry` rows; `AtlasClient.Place`, `AtlasClient.PlaceHistory`; `MapInterop.MeasureCardPlacement`, `CardMeasurement`, map.js's card-measure export; `LegacyNodes.For`'s `Place` arm (→ `null`); `ExplorerPopover.Root`/`.Saved` (→ `Opening`); in `World.razor`: `_hoverPlace`, `_hoverX`, `_hoverY`, `_closeCts`, `_pinnedPlaceId`, `_pointerOverCard`, `QuietAsScenePlace`, `OpenPlaceFromCard`, `OpenExploreNode`, `ClosePinnedCard`, `GoToAdjacent`, `CloseCardNow`, `ScheduleCardClose`, `CardPointerEnter`, `CardPointerLeave`, `CloseCardAfterDelay`, the `PlaceCard` element, and the direct `Atlas.SceneTime`/`SceneScripture`/`Polities` calls; the `IdentityTests`/`LegacyNodesTests`/`PushViaConformanceTests` rows for Place, TimeAndPlace, Year; `stryker-config.json`'s `**/Explore/PlaceDates.cs`; the `app.css` rules for `.place-card*`, `.popover-place-*`, `.popover-event-row*` that only these used; `Explorable.Links`' use of `Neighbours.Nodes()`.
Server: `server/atlas-contract/src/places.rs` (`/api/place/{id}`, `PlacePeriod`), `wire/places.rs` `PlacePage` and `History` (if nothing else reads them — the task verifies), their route registration and pact interactions.
Not deleted (stated so no one "finishes" it): `/api/scene`, `/api/scene/scripture`, `/api/polities`, `/api/eras`, `/api/landmarks`, `/api/land-mask`, `PolityDeltaNode`, `PolityDelta*Section`s, `MapFocusHatch` (Event sections, FOCUS-5), `PlaceChooser`, `Neighbours.Nodes()` (legacy providers still use it).

---

### Task 1: A-EDGES, server half — the edge card and the edge's frontier

**Owner gate first:** show the A-EDGES types above and OPEN 1 to the owner; record the sign-off in the ledger. No code before it.

**Files:**
- Modify: `graph-types/src/edge.rs` (`relations!` append — critical section), `graph-types/tests/common/mod.rs` (`DECLARED_DIRECTED_RELATIONS = 24`), `server/atlas-contract/src/wire/graph.rs` (`EdgeRef` kind+label, `EdgeCard`, `EdgeEnd`, `EdgeEntry.end`), `server/atlas-contract/src/graph.rs` (`edge_card`, `edge_edges` handlers; `EdgeRef` construction), `server/atlas-contract/src/graph_wire.rs` (`describe_edge`), the router and `document.rs` path list, `contracts/openapi.yaml`, `contracts/atlas-query-contract/aqc.schema.json` + `CHANGELOG.md` (AQC minor), pacts.
- Test: `server/atlas-contract/tests/graph_api.rs`, `server/atlas-contract/tests/contract_generation.rs` (relations manifest), `graph-types` law tests.

- [ ] **Step 1: Failing tests** (`graph_api.rs`):
  - `an_edge_card_names_its_kind_its_ends_its_provenance_and_its_frontier` — fetch a verse's `attests` page, take `entries[0].edge`, `GET /api/edge/{id}`; assert the whole `EdgeCard` (kind `attested-in`… as served orientation, `from` the event position, `to` the verse position, label `"<kind label>: <from label> → <to label>"` as served, `edge_summary` containing `from: 1`, `to: 1`).
  - `an_edges_from_page_leads_to_its_source_node` / `..._to_page_leads_to_its_target_node`.
  - `a_justified_edge_lists_what_justifies_it` — an edge with `justified-by` rows serves them on `/api/edge/{id}/edges?kind=justified-by`.
  - `every_edge_entry_says_which_end_the_page_node_is` — whole-page assertion on a `mentions` page: every entry's `end` is `from`; on its dual page `to`.
  - `an_unknown_edge_id_is_not_found`; `a_malformed_edge_id_is_bad_ref`.
  - `every_edge_ref_carries_its_kind_and_served_label` (a `justifies` page).
  - `every_edge_kind_has_a_served_display_label` — walks `EdgeKind::ALL` against the vocabulary (24b: a new kind without a label fails).
  - graph-types: the relation-count law reads 24.
- [ ] **Step 2:** `cargo test -p atlas-contract --test graph_api edge` → red (routes absent).
- [ ] **Step 3: Implement.** Append the two relations (hold the lock). `edge_card` reads the edge through `snap` by its id (`Position::Edge`), its ends, provenance, loci/votes/narrative/note from its row, `edge_summary(&Position::Edge(id))` plus one `from` and one `to`; `edge_edges` pages them in server order. `describe_edge` composes the label from `RelationId::label` / the served display label and `describe_node` of both ends. The node card's `edge_summary` excludes `source-of`/`target-of` (OPEN 1).
- [ ] **Step 4: Regenerate (critical section):** `cargo run -p atlas-contract --bin export_contract`; `cargo run -p atlas-contract --bin export_contract -- --check` clean; AQC minor in `CHANGELOG.md`; re-bless pacts; `dotnet run --project client.ContractGenerator`. Release the lock.
- [ ] **Step 5:** `cargo test -p atlas-contract -p graph-types -p atlas-graph` → green; `bash scripts/contract-gate.sh` → passes.
- [ ] **Step 6: Commit** `edges: an edge has a card and a frontier -- /api/edge/{id}, from/to, served labels (R18; F-21, F-23)`.

### Task 2: A-EDGES, client half — `ElementKind`, `Link.Target: PositionRef`, `Follow` total

**Files:**
- Create: `client/Explore/ElementKind.cs`, `client/Explore/Positions.cs`
- Modify: `client/Explore/{Explorable,Link,Explorer,Presentation,Affordances,Surface,Exploration,LegacySaves}.cs`, `client/IExplorableClient.cs`, the `IExplorableClient` implementation, `client/Views/FocusView.razor`, `client/SavedExplorationsService.cs`, `client/Components/ExplorerPopover.razor` (identity type only)
- Test: `client.Tests/Explore/{ExplorableTests,GraphExplorerTests,PresentationTests,AffordancesTests,LegacySavesTests,ExplorationTests}.cs`, `client.Tests/Explore/ServedGraph.cs` (serves edge cards), new `client.Tests/Explore/ElementKindLawTests.cs`, `tests/ux/explore-edges.spec.ts`

- [ ] **Step 1: Failing tests.**

`client.Tests/Explore/GraphExplorerTests.cs` (additions):
```csharp
[Fact]
public async Task Following_a_link_to_an_edge_resolves_the_edge_with_its_ends_as_its_frontier()
{
    // Arrange
    var graph = new ServedGraph()
        .Serving(ServedGraph.EdgeCardOf(EdgeKind.AttestedIn, AttestsEdge, AttestsLabel, ServedGraph.At(ExodusEvent), ServedGraph.At(Exodus14)))
        .Serving(ServedGraph.Card(NodeKind.TextUnit, Exodus14.Id, Exodus14.Label, new FrontierGroup(EdgeKind.Attests, 1)));
    var explorer = new GraphExplorer(graph);
    // Act
    var edge = await explorer.Follow(new Link(EdgeKind.SourceOf, ServedGraph.AtEdge(AttestsEdge, EdgeKind.AttestedIn, AttestsLabel)));
    // Assert
    Assert.Equal(
        (new ElementKind.Edge(EdgeKind.AttestedIn) as ElementKind, AttestsEdge, AttestsLabel, new[] { new FrontierGroup(EdgeKind.EdgeSource, 1), new FrontierGroup(EdgeKind.EdgeTarget, 1) }),
        (edge.Kind, edge.Id, edge.Label, edge.Groups.ToArray()));
}

[Fact]
public async Task A_page_that_leads_to_edges_yields_links_to_those_edges()
{
    // Arrange
    var justifier = await Resolved.Node(NodeKind.Source, SourceId, SourceLabel, new FrontierGroup(EdgeKind.Justifies, 1)).Using(graphServingAJustifiesPage);
    // Act
    var page = await justifier.Links(EdgeKind.Justifies);
    // Assert
    Assert.Equal(new Page<Link>([new Link(EdgeKind.Justifies, ServedGraph.AtEdge(JustifiedEdge, EdgeKind.Mentions, JustifiedLabel))], null), page);
}
```
`client.Tests/Explore/PresentationTests.cs`: the table test walks `Enum.GetValues<NodeKind>()` **and** `Enum.GetValues<EdgeKind>()` as `ElementKind`s; asserts every edge kind is `(null, null, Card)`; plus `Every_element_is_offered_on_the_popover` (the F-28 law).
`client.Tests/Explore/AffordancesTests.cs`: `Following_is_the_next_arrow_and_preceding_the_previous` (F-22), `An_edge_shows_its_two_ends_as_crumbs`, `Stepping_onto_an_edge_is_an_entry_control_not_a_section`.
`client.Tests/Explore/ElementKindLawTests.cs`: `No_client_source_switches_over_a_closed_sum` — scans `client/**/*.cs` and `*.razor` for `switch` arms naming `ElementKind.`, `Frame.`, `Emphasis.`, `PopoverOpening.` (24b closure of the partial-match category).
`client.Tests/Explore/LegacySavesTests.cs`: `A_v2_save_reads_as_v3_with_every_node_as_a_node_position` (whole `SavedExploration`).
`tests/ux/explore-edges.spec.ts` (A-EDGES' done-when): open EXO 14:21's popover, follow its `attests` entry's connection control (`popover-entry-end-attests-{edgeId}`), assert `popover-card-title` is the served edge label and `popover-up-from-*`/`popover-up-to-*` exist, follow `popover-up-from-*` to the event, assert its title.

- [ ] **Step 2:** `dotnet test client.Tests --filter "GraphExplorerTests|PresentationTests|AffordancesTests|ElementKindLawTests|LegacySavesTests"` → compile errors.
- [ ] **Step 3: Implement** the types exactly as above. `GraphExplorer.Resolve(PositionRef)` matches the wire subtype (`NodePosition` → `Card`, `EdgePosition` → `EdgeCard`); `Follow(link) => Resolve(link.Target)`. `Explorable.Links` maps every entry's `neighbour` to a `Link` (no filter). `Presentation.Of(ElementKind, Surface)` delegates by `kind.Match(node: n => OfNode(n, surface), edge: e => OfEdge(e, surface))`, both private tables exhaustive switches over the enums. `HomeSurfaces.Of(ElementKind)`. FocusView: the arrow branch reads `Arrows.Direction` (no `group.Kind == FollowsIn`); heading text reads the served edge-kind label (F-23) and shows `(count)` only when `Surface == Surface.Popover` (F-28); each entry of a `SectionList`/`InlineChildren` group renders an `EntryEnd` control `{Handle}-entry-end-{kind}-{edgeId}` when the page's `end` is served. Retype every `Explorable.Identity`/`NodeRef` consumer in `ExplorerPopover`, saves and selection per the types block; storage key `explorations-v3`.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green (GeneratedUsageTests now reads `EdgeCard`, `EdgeEnd`). `npx playwright test tests/ux/explore-edges.spec.ts tests/ux/saved-explorations.spec.ts tests/ux/state-focus.spec.ts` → green.
- [ ] **Step 5: Commit** `edges: an Explorable is a node or an edge; Link targets a position; Follow is total (R18; F-21, F-22, F-28; F1-12 retires)`.

### Task 3: Geography on the wire — map/era/polity details, place blurb, `node` on map rows

**Files:**
- Modify: `server/atlas-contract/src/wire/graph.rs` (`MapDetail`, `EraDetail`, `PolityDetail`, `NodeCard` fields, `PlaceDetail.blurb`), `server/atlas-contract/src/graph.rs` (`node_card` fills them from `node.payload`), `server/atlas-core/src/wire.rs` (`ScenePlace.node`, `QuietPlace.node`), `server/atlas-contract/src/wire/map.rs` (`Era.node`, `Polity.node`) and their builders, `contracts/*` regen, pacts, `client.Tests/Explore/ServedGraph.cs` (`Card(...)` gains the three nulls)
- Test: `server/atlas-contract/tests/graph_api.rs`, the scene/polity/era API tests

- [ ] **Step 1: Failing tests:** `a_map_card_serves_its_window` (`Map:era-conquest` → whole `map` detail with served label); `an_era_card_serves_its_window`; `a_polity_card_serves_its_reign_as_the_span_of_its_eras`; `a_place_card_serves_its_default_blurb_where_one_is_recorded` (and `None` where not); `every_scene_place_and_quiet_place_names_its_node` (walks the whole scene for a window: each `node.id` resolves on `/api/node/{id}`); `every_polity_row_and_era_names_its_node` (same walk). The resolve-walks are the 24b closure for "map rows the client cannot open".
- [ ] **Step 2:** `cargo test -p atlas-contract` → red.
- [ ] **Step 3: Implement** from the artifact only (rule 26): payload years through `wire::TimeRange::of`; reign = min `from` / max `to` over `NodePayload::Polity.eras`; blurb from `data.place_history_for(&place.id)` default period; `node` via `node_ref(&id, snap)`.
- [ ] **Step 4: Regenerate (critical section, after Task 1's):** export, `--check`, AQC minor, re-bless, client generator. `cargo test -p atlas-contract -p atlas-core && bash scripts/contract-gate.sh` → green.
- [ ] **Step 5: Commit** `geography: map, era and polity cards serve their windows; map rows name their nodes (R10, R14)`.

**Expected red after Task 3 until Task 4/8:** `client.ContractTests` `GeneratedUsageTests` for `MapDetail`, `EraDetail`, `PolityDetail`, `PlaceDetail.Blurb`, `ScenePlace.Node`, `QuietPlace.Node`, `Polity.Node`, `Era.Node` (each is first read in Task 4 or 8).

### Task 4: `Presentation.Geography` — the `Surface.World` rows and the geographic cards

**Files:**
- Create: `client/Explore/Geography.cs` (`Frame`, `Emphasis`, `Crossing`)
- Modify: `client/Explore/Presentation.cs` (`Geography` arm), `client/Explore/Explorer.cs` (`Present` by form)
- Test: `client.Tests/Explore/GeographyPresentationTests.cs`, `client.Tests/Explore/CrossingTests.cs`, `client.Tests/Explore/ServedGraph.cs` (gains `MapWindow`/`EraWindow`/`Reign`/`PlaceAt` detail builders and `Resolved...With(detail).Using(graph)`, the fixtures the tests below name)

- [ ] **Step 1: Failing tests** (each asserts the whole `Presentation`):
```csharp
public sealed class GeographyPresentationTests
{
    private const string Conquest = "Map:era-conquest";
    private const string ConquestLabel = "The Conquest";

    [Fact]
    public async Task A_map_on_the_world_is_bounded_by_its_window_with_its_neighbouring_maps_at_either_end()
    {
        // Arrange
        var map = await Resolved.Node(NodeKind.Map, Conquest, ConquestLabel, new FrontierGroup(EdgeKind.PrecedesIn, 1), new FrontierGroup(EdgeKind.FollowsIn, 1))
            .With(ServedGraph.MapWindow(ConquestWindow))
            .Using(graphServingPatriarchsBeforeAndJudgesAfter);
        // Act
        var presented = await new GraphExplorer(graph).Present(map, Surface.World);
        // Assert
        Assert.Equal(new Presentation.Geography(new Frame.Bounded(ConquestWindow, ToPatriarchs, ToJudges), new Emphasis.None()), presented);
    }
}
```
Plus: `An_era_on_the_world_is_bounded_like_its_map`; `A_place_on_the_world_keeps_the_current_frame_and_marks_its_site`; `A_polity_on_the_world_keeps_the_current_era_and_marks_its_territory_with_its_reign` (R14: `Frame.Current`, never `Bounded`); `A_first_map_has_no_previous_end`; `A_place_card_lists_only_the_served_fields` (canonical/established/destroyed/blurb/provenance, absent ones omitted); `A_polity_card_shows_its_reign_label`; `A_map_card_shows_its_window_label`; `A_non_geographic_node_is_not_presented_on_the_world` (returns `null`).
`CrossingTests`: `A_window_inside_the_bounds_crosses_nothing`; `A_window_reaching_past_the_end_crosses_to_the_next`; `A_window_reaching_before_the_start_crosses_to_the_previous`; `Touching_a_bound_is_not_crossing_it`.
- [ ] **Step 2:** `dotnet test client.Tests --filter "GeographyPresentationTests|CrossingTests"` → compile errors.
- [ ] **Step 3: Implement.** `Present` switches on `Presentation.Of(node.Kind, surface)` (the `Form` enum, exhaustive): `Card` → `CardOf(node)` (fields from the served detail, field names as named constants beside `ProvenanceField`); `Geography` → `GeographyOf(node)` reading `map`/`era`/`polity`/`place` from the card and the first `PrecedesIn`/`FollowsIn` link for `Bounded`; `Sequence`/`Text` → `CardOf(node)` until FOCUS-2/3. A geographic node whose detail is absent is a served-contract breach: `GeographyOf` throws `ContractBreach` naming the node (not a silent default). `Crossing.Of` compares served `Year` values only; no year arithmetic.
- [ ] **Step 4:** `dotnet test client.Tests && dotnet test client.ContractTests` → green except the Task-3 expected-red members still unread (`ScenePlace.Node`, `QuietPlace.Node`, `Polity.Node`, `Era.Node`).
- [ ] **Step 5: Commit** `geography: Map, Era, Place and Polity present on the world as a frame and an emphasis (R10, R14, R16)`.

### Task 5: The MAPS seam — `IMapSource`

**Files:**
- Create: `client/Geography/IMapSource.cs` (`IMapSource`, `MapLayers`), `client/Geography/AtlasMapSource.cs`
- Modify: `client/Program.cs` (register `IMapSource` → `AtlasMapSource`), `client/Pages/World.razor` (`DebouncedLoadScene`, `LoadPolitiesFor`, `DebouncedLoadScriptureScene` read `IMapSource` only)
- Test: `client.Tests/Geography/AtlasMapSourceTests.cs`, `client.Tests/FetchLayerConformanceTests.cs` (a law: `World.razor` names no `Atlas.Scene*`/`Atlas.Polities`)

- [ ] **Step 1: Failing tests:** `The_layers_during_a_window_are_that_windows_scene_and_its_polities` (fake HTTP serving both; whole `MapLayers`); `The_layers_for_a_scripture_are_its_scene_with_no_polities` (today's behaviour: polities hidden in scripture mode); the conformance law.
- [ ] **Step 2:** red. **Step 3:** implement; `World.razor` keeps its request series and debounce, calling `Maps.During(from, to)` once for scene+polities (`MapLayers`), which also removes the second request race in `LoadPolitiesFor`. `Polities(-4004, 100)` roster fetch moves behind the same source as `Maps.During(TimelineStart, TimelineEnd)` with the two years as the named constants they already are in `World.razor`.
- [ ] **Step 4:** `dotnet test client.Tests` green; `npx playwright test tests/ux --grep "world"` → same set as at the base (no behaviour change).
- [ ] **Step 5: Commit** `geography: the world reads its layers through IMapSource -- the seam the MAPS migration replaces`.

### Task 6: `PopoverOpening` — open the popover on a position, a save, or a legacy node

**Files:**
- Create: `client/Explore/PopoverOpening.cs`
- Modify: `client/Components/ExplorerPopover.razor` (`Opening` replaces `Root`/`Saved`), every host (`client/Pages/{Reader,World,Concord,Kretzmann}.razor`, `client/Layout/MainLayout.razor`, the saved-explorations page), `client/Components/MentionScan.razor`
- Test: `client.Tests/State/ExplorationOwnershipHandoffTests.cs` (fake popover retyped), `client.Tests/Explore/PopoverOpeningTests.cs`

- [ ] **Step 1: Failing tests:** `Opening_on_a_position_resolves_it_and_opens_there`; `Opening_on_a_save_reseeds_its_whole_trail`; `Opening_on_a_legacy_node_resolves_its_identity_and_remembers_its_rendering` (today's `Root` path, unchanged in behaviour); the five ownership laws, retyped.
- [ ] **Step 2:** red. **Step 3:** implement; `OnInitializedAsync` is one `Opening.Match(...)`. Hosts pass `new PopoverOpening.Legacy(node)` where they hold a legacy node and `new PopoverOpening.Explore(position)` where they hold a reference. `MentionScan` opens `Explore` with the mention's served `NodeRef` — if its piece carries only a local place id, record a FINDING under F-31 (the id is composed through `NodeIds.Of` as today; not fixed on the side).
- [ ] **Step 4:** `dotnet test client.Tests`; `npx playwright test tests/ux --grep "popover|saved-explorations|state-focus|mention"` → green.
- [ ] **Step 5: Commit** `focus: the popover opens on one PopoverOpening -- a position, a save, or a legacy node (F-29)`.

### Task 7: Slider and map primitives — `Bounds`, `Band`, `OnCross`, `Emphasize`, `OnPolityClick`

**Files:**
- Modify: `client/Components/TimeSlider.razor`, `client/MapInterop.cs` (`Emphasize`, `IMapEvents.OnPolityClick`, `MapEventsSink.OnPolityClick`), `client/wwwroot/js/map.js` (`setEmphasis(id, emphasis)`: pan+ring a site, outline+raise a polity at the drawn year; polity-territory click → `OnPolityClick`), `client/wwwroot/css/app.css`
- Test: `client.Tests/Components/TimeSliderTests.cs` (bUnit, if present at base; else Playwright only), `tests/ux/world-geography.spec.ts` (new, primitives part)

- [ ] **Step 1: Failing tests:** slider — `A_bounded_slider_cannot_be_dragged_past_its_bounds_and_reports_the_crossing` (drag the end handle beyond `Bounds.To` → `OnCross(Next)` fires once, window clamps; OPEN 5 default: the crossing is offered as the `world-cross-next` button at the bound, and dragging only clamps); `A_band_is_drawn_under_the_window` (`world-slider-band` spans the served reign); map — Playwright `world-geography.spec.ts` "clicking a polity's territory reports that polity" via a test sink.
- [ ] **Step 2:** red. **Step 3:** implement; `Bounds == null` is today's slider exactly. Test ids: `world-slider-bounds`, `world-slider-band`, `world-cross-previous`, `world-cross-next`, `world-emphasis-site`, `world-emphasis-territory`.
- [ ] **Step 4:** `dotnet test client.Tests`; `npx playwright test tests/ux --grep "slider|world-border-morph|world-map"` green.
- [ ] **Step 5: Commit** `geography: the slider takes bounds, a band and a crossing; the map emphasises a site or a territory and reports polity clicks`.

### Task 8: The World view presents the Geography focus

**Files:**
- Modify: `client/Pages/World.razor`, `client/Components/PlaceChooser.razor` (picks `ScenePlace.Node`/`QuietPlace.Node`), `client/Views/FocusView.razor` (the `popover-chip-map` hatch when the current node's home surface is World and the host is not)
- Test: `tests/ux/world-geography.spec.ts` (behaviour part), `client.Tests` for any extracted pure piece

- [ ] **Step 1: Failing Playwright** (`world-geography.spec.ts`), each asserting served labels, never literals composed in the test:
  - "clicking a place focuses it: the popover shows its card and frontier, the map marks its site, the slider keeps its window";
  - "clicking a polity keeps the era, outlines its territory at the slider year, and bands its reign on the slider" (R14);
  - "focusing a Map bounds the slider to its window; the next arrow at the bound follows `follows-in` to the next Map and re-bounds" (R10);
  - "Back from the next Map returns to the previous one and its bounds" (R17);
  - "following a Place link in the reader's popover offers `popover-chip-map`, which opens /world with the exploration carried and the place focused" (R9);
  - "selecting a place with its toggle adds its served node to the tray" (selection via `ScenePlace.Node`).
- [ ] **Step 2:** red. **Step 3: Implement.** `World.razor` subscribes to `ExplorationState`; on change, when `HomeSurfaces.Of(current.Kind)` is `World`, `await Explorer.Present(current, Surface.World)` and apply: `Frame.Match(current: keep slider, bounded: set Bounds and the two crossing links)`, `Emphasis.Match(none: clear, site: Maps.Emphasize, territory: Emphasize + Band)`. Map events: `OnPlaceClick` → the clicked scene row's `Node` → `Open` (via `PopoverOpening.Explore`); `OnPolityClick` → the roster row's `Node`; slider `OnCross(direction)` → `Follow` the stored link for that direction. Delete the card plumbing listed in the deletion inventory and the `PlaceCard` element. `PlaceChooser.OnPick` opens the picked row's `Node`. Scripture mode, follow mode, split and `PolityDelta` clicks are unchanged. OPEN 2 default: no hover preview; hover keeps only the marker highlight map.js already draws.
- [ ] **Step 4:** `dotnet test client.Tests`; `npx playwright test tests/ux/world-geography.spec.ts` green. The expected-red set (below) is recorded in the ledger by name.
- [ ] **Step 5: Commit** `geography: the world view presents its focus -- places, polities, eras and maps open, frame and emphasise (R9, R10, R14)`.

### Task 9: Deletions — the legacy place path and `/api/place`

**Files:** everything in the deletion inventory; `client.Tests/Explore/DeletionLawTests.cs`.

- [ ] **Step 1: Failing law:** `DeletionLawTests.MigratedKinds = [NodeKind.Place, NodeKind.Polity, NodeKind.Era, NodeKind.Map]`; add `No_section_provider_names_a_migrated_kind` (scans `client/Explore/PopoverSectionProviders.cs` for `node.Kind == "<kind>"` for each migrated kind's legacy string, derived from the enum) and `No_legacy_node_class_exists_for_a_migrated_kind` (reflection: no `IExplorable` whose name starts with a migrated kind's name, except `PolityDeltaNode` named as MAPS' in the test's one declared exception list). Run → red.
- [ ] **Step 2: Delete** the client files and members; `git rm` the four client test files; remove the `IdentityTests`/`LegacyNodesTests`/`PushViaConformanceTests` rows; drop `**/Explore/PlaceDates.cs` from `stryker-config.json`; delete the CSS rules.
- [ ] **Step 3: Server:** delete `/api/place/{id}` (`places.rs`, route registration, `PlacePage`/`History` if unreferenced — `cargo build` decides), its tests and pact interactions. **Regenerate (critical section, after Task 3's):** export, `--check`, AQC **major** if the contract policy classes a removed route as breaking (else minor; the semver gate `scripts/contract-semver-gate.sh` decides, not the author), re-bless, client generator.
- [ ] **Step 4:** `cargo test --workspace && dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh` → green (GeneratedUsageTests now reads every Task-3 member).
- [ ] **Step 5: Commit** `geography: PlaceNode, PlaceCard, the place sections, YearNode, TimeAndPlaceNode and /api/place are gone (deletion law: Place, Polity, Era, Map)`.

### Task 10: Re-express the world specs; gates; mutation; push

- [ ] **Step 1: Re-express** each expected-red Playwright spec under the new behaviour, by name, keeping every surviving test id: `world-hover-text`, `world-pin`, `world-place-history`, `world-quiet-places` (not :211), `world-kjv-names`, `world-labels`, `world-map`, `world-narrative-focus`, `world-same-place`, `world-hover-resolution`, `world-existence`, `popover-sections` (place sections → FocusView card fields `popover-field-*`), `selection-tray`, `split-view`, `reader-map`, `w1`…`w5-passages` (the one `hover-verse`/`place-card` step each), `lib/hovercard.ts`, `lib/hoverSafety.ts`, `tests/ux/CONTRACT.md`. A spec whose behaviour OPEN 3 removed is rewritten to the popover's window-free card and its `site-of` frontier, and the removal is listed in the close report.
- [ ] **Step 2: Gates:** `client.Tests/stryker-config.json` `mutate` gains `**/Explore/ElementKind.cs`, `**/Explore/Positions.cs`, `**/Explore/Geography.cs`, `**/Explore/PopoverOpening.cs`, `**/Geography/AtlasMapSource.cs`; `cd client.Tests && dotnet stryker` → 100% or equivalents recorded; `bash scripts/mutants-parallel.sh -n 4 -b 3baaeb6` → 100% or equivalents recorded (the one mutation gate, critical section). Then `cargo test --workspace && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh && npx playwright test tests/ux` → green except the two carried reds.
- [ ] **Step 3:** close report `docs/superpowers/reports/2026-10-0x-focus-6-close.md`: every rule-24 category with its closure and guarantee; FINDINGS raised (at least: NodeCard's per-kind optionals are a product where a sum `NodeDetail` belongs — the Haskell bar, proposed closure a `oneOf` detail; any `MentionScan` F-31 site); the spec §5 FOCUS-9 row amendment (Year/TimeAndPlace/PlaceDates moved here).
- [ ] **Step 4: Commit and push** `git push origin worktree-bible-atlas-m1`.

---

## Wave schedule

Primary = the lane's critical-path work; companion = unlike work paired beside it (rule 23: Rust beside C#). A task starts only when the tasks it names as inputs have landed on the batch branch.

| Wave | Primary | Companion | Critical sections held | Expected red at wave close (ledger records names) |
|---|---|---|---|---|
| 0 | owner sign-off of A-EDGES types + OPEN 1–5 | — | — | — |
| 1 | Task 1 (Rust: relations, edge card) | Task 6 then Task 5 (C#: `PopoverOpening`, `IMapSource`; disjoint from Task 1) | `relations!`; regen #1; re-bless #1 | `client.Tests`/`client.ContractTests` compile against the regenerated wire: `GeneratedUsageTests` (`EdgeCard`, `EdgeEnd`, `EdgeEntry.End`) until Task 2 |
| 2 | Task 2 (C#: A-EDGES client) | Task 3 (Rust: geography wire) — its regen waits for Task 1's to land | regen #2; re-bless #2 (Task 3) | `GeneratedUsageTests` for the Task-3 members |
| 3 | Task 4 (C#: `Explore/`) | Task 7 (Razor/JS: slider + map.js; disjoint files) | — | `GeneratedUsageTests` (`*.Node` on map rows) |
| 4 | Task 8 (World view) | Task 9's server half written, not regenerated | — | Playwright: every spec in Task 10 Step 1's list; `client.ContractTests` unchanged |
| 5 | Task 9 (deletions; server regen) | — | regen #3; re-bless #3 | Playwright as wave 4 |
| 6 | Task 10 (re-express, gates, push) | — | the mutation gate | only `world-quiet-places:211`, `world-cluster-chooser:213` |

**Critical path:** sign-off → Task 1 → Task 2 → Task 4 → Task 8 → Task 9 → Task 10. Tasks 3, 5, 6, 7 ride beside it and must land before the primary that consumes them (3 before 4; 5 and 6 before 8; 7 before 8).

## Self-review against the spec and the brief

- **Coverage:** R18/A-EDGES (Tasks 1–2; F-21 closed by the edge card, F1-12's filter retired, `Follow` total by a law over every served position); R16 rows for Map/Place/Polity/Era on `Surface.World` and for every edge kind (Tasks 2, 4); R10 bounded slider and `follows-in` crossing (Tasks 4, 7, 8); R14 polity focus (Tasks 3, 4, 7, 8); R9 home surface navigation (Task 8); §5 FOCUS-6 deletions (Task 9) with `/api/scene` deliberately left to MAPS; F-22/F-23/F-28/F-29 closed where this batch touches them.
- **The MAPS seam** is `IMapSource` (+ the renderer beneath the World view); nothing in `Presentation` names map data.
- **Principle 25:** every window, reign, date and label is served (Task 3); the client compares served years only in `Crossing`; ids come from `node: NodeRef` on the map wire, never composed.
- **Type consistency:** `ElementKind`/`Link`/`PositionRef` (Task 2) are what Tasks 4, 6, 8 use; `Frame`/`Emphasis` (Task 4) are what Tasks 7–8 consume; `IMapSource` (Task 5) is what Task 8 calls; `PopoverOpening` (Task 6) is what Task 8 and `MentionScan` open with.
- **Assumptions to verify at execution:** `PlacePage`/`History` have no reader outside `/api/place` (Task 9 lets `cargo build` decide); a bUnit harness exists for `TimeSlider` (else Task 7's slider tests are Playwright); the verse→attests→event fixture in Task 2's spec uses an edge the served graph has (take it from a live `attests` page, not a literal).
