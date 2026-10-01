# FOCUS-6 (Geography, with A-EDGES) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Re-planned 2026-09-30 against PRINCIPLES 27** (owner: "frontier and explorable are front end constructs derived from the graph"; "revert that stuff you shouldn't have done to the graph"). The first plan's Tasks 1–2 (`lane/claude/F6-t1`, `F6-t2`: `86baad7`, `54079c4`, `7f1ef67`) are withdrawn and never land: they put an edge card endpoint, `EdgeSource`/`EdgeTarget` relations and per-request edge labels into the backend. Only their client half is salvaged, reworked in Task 3. Task 6 (`IMapSource`, `lane/claude/F6-t5`, `4e7a77e`) is kept as built.

## OPEN — for the owner, before the task named starts (each blocks only that task)

1. ~~**Edge ends as link kinds (Task 1):**~~ **WITHDRAWN** (owner, 2026-09-30, PRINCIPLES 27): no `from`/`to` relations. An edge's ends are fields of the edge, read by the element read (Task 2).
2. **Hover on the map (Task 9):** with `PlaceCard` gone, does hovering a place still preview it (the same popover card, transient), or is a click the only way in?
3. **Window-scoped place content (Tasks 4, 9), revised by rule 27a:** the first plan's option of a windowed card (`/api/node/{id}?from&to`) is a view-shaped read and is withdrawn. The choice now: (a) the popover's window-free record plus the full `site-of` neighbours, now; or (b) hold the windowed content (period name and blurb, events in this window with their verses, per-place narrative prev/next) until a generic time-window range read exists (27a; F-34 schedules it).
4. **`/world` with nothing focused (Task 9):** keep today's free slider over all time until a Map/Era is focused, or always open on a Map (needs a served "Map at year")?
5. **Crossing an era (Tasks 8, 9):** does dragging past the focused Map's bound follow `follows-in` by itself, or only an explicit arrow click at the bound (as NAV-UNIFORM-1 does for the next book)?
6. **The vocabulary gate's reach (Task 1):** the gate scans identifiers, comments, doc comments and file paths in `server/` and `graph-types/`, plus the published contract text; it does **not** scan Rust string literals, because domain prose lives in them today (`atlas-core/src/event_merge.rs` holds the curated merge table, whose reasons quote "Temple Presentation of Jesus"). (a) This reach, with the event-merge table filed as a rule-26 FINDING (F-35, below); or (b) scan string literals too, and widen Task 1 to move `EVENT_MERGE_PAIRS`/`EVENT_DISTINCT_PAIRS` into `data/curated/` first?
7. **`graph-types/src/frontier.rs` (Task 1):** the `FocusKind` × `Capability` matrix (with its owner-reviewed verse row) is read by nothing in the served system, only by its own tests and `server/atlas-graph/tests/frontier_falsifiability.rs`; the client owns the same decisions in `Presentation.Of` and `Affordances.Of`. Delete it, or keep it under new names?
8. **Renaming the AQC features (Task 1):** `exploration-roundtrip.feature` → `descriptor-roundtrip.feature` (and "card" out of `focus-query.feature`'s prose). The semver gate classes a renamed feature file as **AQC major**. Accept the major, or keep the old file name?
9. **"focus" (Task 1):** rule 27's list is explore, explorable, frontier, card, popover, presentation. "focus" also names a client construct (`FocusQuery`, the `focus-*.json` AQC fixtures, `FocusView`). Add it to the gate's list (more renames, all AQC), or leave the list as the rule states it?
10. **An edge's label (Task 2):** the compiler writes one label per edge. What does it say? And O-LABELS (queue 21: derived or curated edge-kind display labels) feeds into it.
11. **The neighbour read at an edge (Task 2):** widen `/api/node/{id}/edges` to accept an edge id (the path says "node" until F-34 renames the read set), or add the generic `/api/position/{id}/edges` now and retire the node path in the same task (a removed route: AQC major)?
12. **Counts on an element (Task 2):** each element record carries its `edge_summary` (one index lookup, saves a round trip, 27c), or the count per edge kind becomes its own read now (27a lists it separately; F-34 calls the bundled summary view-shaped)?
13. **Attestation and mention loci on an edge record (Task 2):** the neighbour page computes `loci`/`note` per request (`EventAccounts::read`, mention spans). Leave them on the page only (the edge record carries its row's votes/narrative/parentage and provenance), or compile them and serve them on the edge record too?

Defaults this plan builds if unanswered: (2) click only, no hover preview; (3) (a); (4) free slider; (5) explicit click; (6) (a); (7) delete; (8) rename and take the major; (9) leave the list as stated; (10) `"{subject label} · {edge-kind display label} · {object label}"`, with the display label derived from the wire name ("attested-in" → "Attested in") until O-LABELS rules; (11) widen in place; (12) bundled; (13) the page only.

**Goal:** Make edges explorable (A-EDGES, spec §12 R18 as amended 2026-09-30) with the derivation on the client, close F-33 (the backend names the client's vocabulary), and move Place, Map, Polity and Era onto the new path: the World view presents `Presentation.Geography` (R9/R10/R16 rows on `Surface.World`), a polity focus highlights its territory at the slider's year and shows its reign (R14), the slider scrubs within a focused Map's window and crosses to the next Map by `follows-in` (R10); the four `Place*Section`s, `PlaceNode`, `PlaceCard` and `/api/place` die.

**Architecture:** Rule 27 splits the work by who knows the inputs. The **compiler** writes every label (node and edge) and every data-only derivation (a polity's reign, a place's default blurb) into the artifact. The **server** reads: one generic element read (nodes and edges by id, many per call; an edge's ends are fields) and the existing neighbour read, widened to an edge's position. The **client** derives everything about the interaction: `ElementKind = Node(NodeKind) | Edge(EdgeKind)`, `Link.Target: PositionRef`, an edge's two ends as `Link`s, `Follow` total, `Presentation.Geography(Frame, Emphasis)` built by `GraphExplorer` from served record details only. The World view reads the exploration's current element, asks for its `Surface.World` presentation, and applies the frame to the slider and the emphasis to the map. **Map data stays the atlas's existing scene/polity data behind one seam, `IMapSource`** (see "The MAPS seam" below).

**Tech Stack:** Rust (axum, utoipa, `atlas-contract`, `atlas-graph`, `graph-types`); .NET 10 Blazor WebAssembly; xUnit; Playwright; Stryker.NET; cargo-mutants via `scripts/mutants-parallel.sh`.

**Spec:** `docs/superpowers/specs/2026-09-26-focus-exploration-design.md` §5 (FOCUS-6 row), §10 R9/R10, §11 R14, §12 R15–R18 (R18 as amended 2026-09-30). **Principles:** 25, 26, 26a, 27–27g. **Queue:** A-EDGES (REWORK), A-MAPS-SPEC, A-FPLANS; closes F-21, F-22, F-23, F-28, F-29, F-33; feeds F-34.

**Base:** FOCUS-1 at `3baaeb6` (branch `lane/claude/A-F1`). Every gate in this batch takes `--base 3baaeb6` (PRINCIPLES 22). If A-F1 lands on `worktree-bible-atlas-m1` with fixes, rebase and write the new base into the ledger at Task 1. Task 6 is already built on `3baaeb6` (`lane/claude/F6-t5`, `4e7a77e`) and awaits review only.

## Global Constraints

- `docs/PRINCIPLES.md` binds, especially: rule 4 (zero dead code), 9 (no comments in application code — every snippet below has none; a comment that names a client word is deleted, not reworded), 12 (every signature below is for sign-off), 21 (critical sections, below), 24/24a/24b (every fix names its category and closes it; offenders found on the side go to FINDINGS, never fixed on the side), 25 (the client composes over the contract: no year arithmetic, no id composition, no label composition on the client), 26/26a (the server reads facts from the artifact only; source shapes stay in the tools), **27 (derivation belongs to whoever knows its inputs: data-only derivations are compiled, per-request ones are indexed reads, interaction ones are the client's; the graph models the domain, never a view; no new relation, field or element serves a client construct)**.
- Tests: whole-body assertions, one behaviour per test named as a sentence, `// Arrange` `// Act` `// Assert` only, no magic numbers, newspaper order. A test name in `server/` or `graph-types/` is an identifier: it may not contain a client word (Task 1's gate reads it).
- **Total matches, not switches with a discard.** C# cannot prove a closed record hierarchy exhaustive, so every new sum (`ElementKind`, `Frame`, `Emphasis`, `PopoverOpening`) exposes an abstract `Match<T>` implemented once per arm; no `switch` over them anywhere (a source-scan law in Task 3 enforces it). Enums keep the existing exhaustive-switch build error.
- Names are this plan's. Server/graph: `adjacency`, `Adjacent`, `Adjacency`, `adjacency_at`, `NeighbourRefusals`, `NodeRecord`, `EdgeRecord`, `Element`, `ElementPage`, `ElementRefusals`, `ElementId`, `GraphQuery::{labels, edge}`, `atlas_graph::labels`. Client: `ElementKind.{Node,Edge}`, `Positions`, `Link`, `Entry`, `Explorable.{Ends, Entries}`, `IExplorer.{Resolve,Follow,Present}`, `IExplorableClient.Elements`, `Presentation.{Card,Field,Geography}`, `Frame.{Current,Bounded}`, `Emphasis.{None,Site,Territory}`, `Affordance.{Arrows(ArrowDirection),InlineChildren,UpCrumb,SectionList}`, `ArrowDirection.{Previous,Next}`, `PopoverOpening.{Explore,Resume,Legacy}`, `IMapSource`, `AtlasMapSource`, `MapLayers`, `Crossing`.
- **No relation is appended in this batch.** `DECLARED_DIRECTED_RELATIONS` stays 22; the relation-count law is the guard.
- The Playwright suite is the behaviour gate; the expected-red sets per task are listed in the wave schedule and re-expressed in Task 11. Known reds carried from FOCUS-1: `world-quiet-places:211`, `world-cluster-chooser:213` (O-CHOOSER).
- Build no interaction that works only by hovering (A-F1 note).
- Commit per task; push at the end of the batch to `origin/worktree-bible-atlas-m1`, never force.

## Critical sections (PRINCIPLES 21) — one holder at a time

| Section | Held by | Why |
|---|---|---|
| `contract`: regenerating `contracts/openapi.yaml` + `aqc.schema.json` (`cargo run -p atlas-contract --bin export_contract`), the AQC features, then `Wire.g.cs` (`dotnet run --project client.ContractGenerator`) | Task 1, then Task 2, then Task 4, then Task 10 — strictly in that order | one generated document |
| `contract`: rebuilding `data/compiled` (moves the version root) | Task 2 (the `label` table), Task 4 (the reign and blurb sidecars) | one artifact |
| `contract`: re-blessing pacts/fixtures | Tasks 1, 2, 4, 10 (right after their regen) | moves the version root |
| `heavy`: `cargo test --workspace`, full Playwright | Tasks 1, 2, 4, 10, 11 | memory |
| `heavy` ("mutation"): the mutation gate | Task 11 only | once per batch (3a) |
| appending to `relations!` | **nobody** | rule 27 |

## The MAPS seam

`Presentation.Geography`, `Frame`, `Emphasis` and the `Surface.World` rows of `Presentation.Of` are built from `NodeRecord` details alone (Task 5) and name no map data. The map's layers come from **`IMapSource`** (Task 6, built):

```csharp
public interface IMapSource
{
    Task<MapLayers> During(int from, int to);
    Task<MapLayers> For(string scriptureRef);
}

public sealed record MapLayers(Scene Scene, IReadOnlyList<Polity> Polities);
```

FOCUS-6's only implementation is `AtlasMapSource(AtlasClient atlas)`, over `/api/scene`, `/api/scene/scripture` and `/api/polities`. The MAPS migration replaces `AtlasMapSource` (and `MapLayers`' payload types, and `MapInterop`/`map.js` beneath the World view) with map-generator's registry and renderer (pre-tiled geometry, 27g). It does not touch `Presentation`, `Frame`, `Emphasis`, `GraphExplorer.Present`, the table rows, `Crossing`, or the exploration. `/api/scene` therefore stays in this batch (the spec's §5 note "MAPS may retire it instead" is taken; it is one of F-34's view-shaped endpoints). `PolityDeltaNode` and the three `PolityDelta*Section`s stay for MAPS (R2).

## Re-anchor table — what the World view touches today (at `3baaeb6`), and where it goes

| Today | Role | FOCUS-6 |
|---|---|---|
| `client/Pages/World.razor` | page shell, split/follow, picker, slider, hover card, popover host | keeps shell/split/follow/picker/scripture mode; loses the card plumbing (Task 9); presents the current Geography focus |
| `client/Components/PlaceCard.razor` (491 lines) | hover/pinned place card: windowed verses, blurb, dates, narrative prev/next | **deleted** (Task 10); its content is the popover `Card` + neighbours (OPEN 3) |
| `client/Components/PlaceChooser.razor` | cluster chooser | kept; picks a served `NodeRef` (Task 9); O-CHOOSER unchanged |
| `client/Components/TimeSlider.razor` | eras, window, drag/change | gains `Bounds`, `Band`, `OnCross` (Task 8) |
| `client/Components/Legend.razor` | narrative legend/isolate | unchanged |
| `client/MapInterop.cs` (`IMapEvents`, `MapInterop`, `MapEventsSink`) + `client/wwwroot/js/map.js` | the renderer | gains `Emphasize`, `OnPolityClick`; loses `MeasureCardPlacement`/`CardMeasurement` (Tasks 8, 10) |
| `client/SliderWindow.cs` | labels the slider's window in scripture mode | unchanged |
| `client/AtlasClient.cs` `Eras`, `Landmarks`, `LandMask`, `Polities`, `SceneTime`, `SceneScripture` | map data | `Polities`/`SceneTime`/`SceneScripture` behind `AtlasMapSource` (Task 6, built); `Place`, `PlaceHistory` **deleted** (Task 10) |
| `client/Explore/PlaceNode.cs`, `PlaceDates.cs`, `YearNode.cs`, `TimeAndPlaceNode.cs`, `client/Components/PlaceEventsList.razor`, `client/CardPlacement.cs` | legacy place path | **deleted** (Task 10) |
| `PlaceDescriptionSection`, `PlaceDatesSection`, `PlaceBlurbSection`, `PlaceEventsSection`, `YearFrontierSection` (`client/Explore/PopoverSectionProviders.cs`, registry rows in `PopoverSections.cs`) | legacy popover sections | **deleted** (Task 10) |
| `client/Components/MentionScan.razor` | opens `new PlaceNode(...)` from a text mention | opens `PopoverOpening.Explore` (Task 7) |
| `client/Explore/LegacyNodes.cs` `NodeKind.Place` arm | bridge | → `null` (Task 10) |
| `client/Explore/PolityDeltaNode.cs` + three `PolityDelta*Section`s | border-change popover | unchanged (MAPS) |
| `server/atlas-contract/src/places.rs` `/api/place/{id}`; `wire/places.rs` `PlacePage`, `History` | legacy route | **deleted** (Task 10) |
| `server/atlas-contract/src/graph.rs` `node_card`; `wire/graph.rs` `NodeCard` | the node read | renamed `node_record`/`NodeRecord` (Task 1); its builder shared with the element read (Task 2); gains `map`/`era`/`polity` details and `place.blurb` (Task 4) |
| `server/atlas-contract/src/graph_wire.rs` `describe_node`, `text_unit_label`, `node_label` | labels composed per request | read the compiled `label` table (Task 2); the two composers die |
| `server/atlas-core/src/wire.rs` `ScenePlace`, `QuietPlace`; `server/atlas-contract/src/wire/map.rs` `Era`, `Polity` | map wire | each gains `node: NodeRef` (Task 4) |

**Pulled forward from FOCUS-9 by rule 4 (zero dead code):** `YearNode` is constructed only by `PlaceCard`, and `TimeAndPlaceNode` only by `PlaceNode`/`PlaceEventsList`, so they and `YearFrontierSection`, `PlaceDatesSection` die here. FOCUS-9's row shrinks to `AuthorNode`, the v1 save reader and `PopoverSectionRegistry`.

## Types (for sign-off — PRINCIPLES 12)

### F-33: the backend speaks the graph's words (Task 1)

Renames (behaviour-free; every site at `3baaeb6` found by `grep -rniw 'explor[a-z]*\|frontiers\?\|cards\?\|popovers\?\|presentations\?' server graph-types --include=*.rs` and by the gate's own first red run, which lists 289 identifier hits, 41 comment hits and the file paths):

| At `3baaeb6` | Becomes |
|---|---|
| `graph-types/src/explore.rs`, `pub mod explore` | `graph-types/src/adjacency.rs`, `pub mod adjacency` |
| trait `Explorable` (`impl Explorable for PositionRef`) | trait `Adjacent` |
| struct `Frontier` (`of_rows`, `append`, `edge_count`, `edges`, `page`) | struct `Adjacency` (same methods) |
| `fn frontier_at` | `fn adjacency_at` |
| `edge.rs` `fn frontiers`, `BiIndex.{fwd,inv}: BTreeMap<Position, Frontier>` | `fn adjacencies`, `BTreeMap<Position, Adjacency>` |
| `lib.rs` `pub use explore::{Explorable, Holdings}` | `pub use adjacency::{Adjacent, Holdings}` |
| `id::PositionKind::Exploration` (addresses the in-memory graph's version, `store.rs:189`) | `PositionKind::Version` |
| `node::Card`, `node::card()` | **deleted**; `pub fn node::label(n: &dyn NodeData) -> String` (its one real caller, `graph_wire::node_label`, reads `.label` only; Task 2 moves the label into the compiler) |
| `graph-types/src/present.rs` (`PresentationContext`, `Presentation`, `Presentable`) | **deleted** — no reader (rule 4) |
| `graph-types/src/frontier.rs` (`FocusKind`, `FocusBacking`, `FrontierEdge`, `Capability`, `CoreSection`, `Section`, `SectionOutcome`, `allows`) and `server/atlas-graph/tests/frontier_falsifiability.rs` | **deleted** (OPEN 7) |
| `atlas-contract/src/error.rs` `FrontierRefusals` | `NeighbourRefusals` (not in the published document) |
| `wire::NodeCard` (schema `NodeCard`) | `wire::NodeRecord` (schema `NodeRecord`; JSON unchanged) |
| `graph.rs` `node_card` (handler of `/api/node/{id}`) | `node_record` |
| `atlas-cli` `ResolvedCard`; help and tutorial text ("card", "frontier", "Bible Explorer") | `ResolvedNode`; "record", "neighbours", "Bible Atlas" |
| `aqc_export::exploration_roundtrip_feature`, `Feature: ExplorationRoundTrip`, `exploration-roundtrip.feature`; "card" in `focus-query.feature`'s prose | `descriptor_roundtrip_feature`, `DescriptorRoundTrip`, `descriptor-roundtrip.feature`; "record" (OPEN 8) |
| published descriptions: `document.rs` graph tag ("one node's card, one frontier"), `EdgeEntry.edge` ("can be explored"), `wire_form.rs` `EDGE_KIND` ("one frontier"), `wire/reading.rs` ("to explore"), `NodeCard.version` ("this card") | reworded in the graph's words, as `#[schema(description = "…")]` where they were `///` |
| test functions (`node_card_*`, `*_card_*`, `*_frontier*`, `edge_summary_counts_match_frontiers`, `paging_semantics_match_explore_rs_*`, `cites_row_is_queryable_through_the_generic_explorable_machinery`, …) | the same sentence with record/neighbours/adjacency |
| client: `NodeCard` in 17 files (`client/`, `client.Tests/`, `client.ContractTests/`) | `NodeRecord` (regenerated `Wire.g.cs`; mechanical) |

**Card-shaped names deferred to F-34, listed so no one "finishes" them here:** the route `/api/node/{id}` (a single-id node read that the element read supersedes), its bundled `edge_summary`, `NodeRecord`'s per-kind optional details, `NodeRecord.version`, and every view-shaped route (`/api/scene*`, `/api/chapter`, `/api/verse`, `/api/event`, `/api/narrative/event`, `/api/catechism*`, `/api/kretzmann`, `/api/xrefs`, `/api/place` — the last dies in Task 10). The gate forces only the names above: `NodeCard`/`node_card` are identifiers in `server/`, so they rename now; the route path, its shape and the other routes contain no client word and wait for F-34's audit.

The gate (`server/atlas-contract/tests/backend_vocabulary.rs`, the 24b closure):
```rust
const CLIENT_WORDS: [&str; 6] = ["explore", "explorable", "frontier", "card", "popover", "presentation"];

fn words_of(text: &str) -> Vec<String>;
fn is_client_word(word: &str) -> bool;
fn scanned_text(source: &str) -> Vec<(usize, String)>;
fn backend_sources() -> Vec<PathBuf>;
fn published_contract() -> Vec<PathBuf>;
```
- `words_of` splits on every non-alphanumeric and at camel-case boundaries and lower-cases (`NodeRecord` → `node`, `record`); `is_client_word` holds for a word in `CLIENT_WORDS` or a form of one (`explor*`, plural `s`), never for a word that merely contains one (`jaccard`, `discard`).
- `scanned_text` yields identifiers, `//`/`/* */` comments and doc comments with their line, and skips string and char literals (OPEN 6).
- `backend_sources` walks `server/**/*.rs` and `graph-types/**/*.rs`; `published_contract` is `contracts/openapi.yaml`, `contracts/atlas-query-contract/aqc.schema.json` (whole text) and the `Feature:`/`Scenario:`/`#` lines of `contracts/atlas-query-contract/features/*.feature`.
- The client words appear only inside string literals in this file, so the gate passes over itself.

### A-EDGES: the generic element read and compiled labels (Task 2)

Graph (`graph-types`):
```rust
pub struct EdgeRecord {
    pub id: EdgeId,
    pub kind: EdgeKind,
    pub subject: Position,
    pub object: Position,
    pub meta: EdgeMeta,
}

pub trait GraphQuery {
    fn labels(&self, at: &[Position]) -> Vec<Option<String>>;
    fn edge(&self, id: &EdgeId) -> Option<EdgeRecord>;
}

pub struct Graph {
    pub labels: BTreeMap<Position, String>,
    pub edges_by_id: BTreeMap<EdgeId, EdgeRecord>,
}
```
- `EdgeRecord` exists at `3baaeb6` (`id`, `kind`, `subject`, `object`); it gains `meta`. `kind` is the direction it is recorded in: `Directed(rel, Forward)` or `Symmetric(rel)`; a symmetric edge's `subject` is the lesser end.
- Both reads are one index lookup per item (27b): `Graph` answers from `labels` and `edges_by_id`, both built at index time (no scan); `SqliteSnapshot` from the new `label` table (primary key) and `edge_by_id` with `dir = forward` (or the symmetric row whose subject is the lesser).

Compiler (`server/atlas-graph/src/labels.rs`, compile side only):
```rust
pub fn node_label(id: &AnyNodeId, node: &Node) -> String;
pub fn edge_label(kind: EdgeKind, subject: &str, object: &str) -> String;
pub fn compile(graph: &mut Graph);
```
- `node_label` is the one node-label function: a verse by its reference (`kjv_adapter::dot_ref`), a Concord paragraph by `ConcordTag::cite`, every other kind by `node::label`. `edge_label` is OPEN 10's form over `EdgeKind::display_label`. `compile` fills `Graph.labels` for every held position and `edges_by_id` for every edge; `atlas_graph::build` calls it last.
- SQLite: `CREATE TABLE label (position TEXT PRIMARY KEY, label TEXT NOT NULL) WITHOUT ROWID;` written by the writer from `Graph.labels`. The unread `node.label` column and `writer::node_label` are deleted (they were the second, partial label function).
- `EdgeKind::display_label(self) -> String` (graph-types); `x-atlas-relations` gains `label` per direction (closes F-23).

Wire (`server/atlas-contract/src/wire/graph.rs`):
```rust
pub struct EdgeRef { pub id: String, pub kind: EdgeKind, pub label: String }

pub struct EdgeRecord {
    pub id: String,
    pub kind: EdgeKind,
    pub label: String,
    pub subject: PositionRef,
    pub object: PositionRef,
    pub provenance: String,
    pub votes: Option<u32>,
    pub narrative: Option<NarrativeId>,
    pub parentage: Option<Parentage>,
    pub edge_summary: Vec<EdgeSummaryEntry>,
}

pub enum Element {
    Node { node: NodeRecord },
    Edge { edge: EdgeRecord },
    Missing { id: String },
}

pub struct ElementPage { pub elements: Vec<Element>, pub version: String }
```
- `EdgeEntry.edge: String` becomes `EdgeEntry.edge: EdgeRef` (a shape change: AQC major by the semver gate). An `EdgePosition` neighbour now carries its edge's kind and compiled label (27c).
- `Element` is written by the existing `union::tagged_by` machinery like `PositionRef`: tag `element`, cases `NodeElement` (`"node"`), `EdgeElement` (`"edge"`), `MissingElement` (`"missing"`). `elements[i]` answers `id[i]`, so the answer is total over the request.
- `EdgeRecord.provenance` is the first row's (`row_provenance`); its `edge_summary` is `edge_summary(&Position::Edge(id))` (OPEN 12).

Server (`server/atlas-contract/src/`):
```rust
#[utoipa::path(get, path = "/api/elements", params(ElementsQuery), responses((status = 200, body = wire::ElementPage), ElementRefusals), tag = "graph")]
pub async fn elements(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Contract(asked): Contract<ElementsQuery>,
) -> Result<Json<wire::ElementPage>, ApiError>;

pub struct ElementsQuery { pub id: Vec<String> }
const MAX_ELEMENTS: usize = MAX_EDGE_LIMIT;

pub enum ElementId { Node(AnyNodeId), Edge(EdgeId) }
pub fn decode_element_id(s: &str) -> Option<ElementId>;
pub struct PositionReference(pub ElementId);

fn node_record(id: &AnyNodeId, data: &AtlasData, graph: &GraphService, snap: &impl GraphQuery) -> Result<Option<wire::NodeRecord>, ApiError>;
fn edge_record(id: &EdgeId, snap: &impl GraphQuery) -> Option<wire::EdgeRecord>;
```
- `error.rs`: `ElementRefusals { BadRef, TooMany }`. One malformed id refuses the whole read (`bad_ref`); more than `MAX_ELEMENTS` ids, or none, is `too_many`/`bad_ref`; an id that decodes but names nothing is a `MissingElement`, not a refusal.
- `decode_element_id` decides by prefix: a node kind's wire prefix (`decode_node_id`) or a relation's name (`RelationId`/`SymRelationId::name`). A law proves the two prefix sets disjoint.
- `/api/node/{id}` and `/api/elements` build a node through the one `node_record` (DRY).
- `/api/node/{id}/edges` takes a `PositionReference` (OPEN 11): a node id or an edge id; `not_found` when neither resolves.

### A-EDGES: the client (Task 3)

Client (`client/Explore/`, `client/`):
```csharp
public abstract record ElementKind
{
    private ElementKind() { }
    public abstract T Match<T>(Func<NodeKind, T> node, Func<EdgeKind, T> edge);
    public static ElementKind Of(PositionRef position);
    public sealed record Node(NodeKind Kind) : ElementKind;
    public sealed record Edge(EdgeKind Kind) : ElementKind;
}

public static class Positions
{
    public static (ElementKind Kind, string Id, string Label) Of(PositionRef position);
}

public sealed record Link(EdgeKind Kind, PositionRef Target);
public sealed record Entry(Link Neighbour, Link Edge);

public sealed class Explorable
{
    internal Explorable(NodeRecord node, IExplorableClient graph);
    internal Explorable(EdgeRecord edge, IExplorableClient graph);
    public ElementKind Kind { get; }
    public string Id { get; }
    public string Label { get; }
    public PositionRef Identity { get; }
    public IReadOnlyList<FrontierGroup> Groups { get; }
    public IReadOnlyList<Link> Ends { get; }
    public Task<Page<Link>> Links(EdgeKind kind, int? cursor = null);
    public Task<Page<Entry>> Entries(EdgeKind kind, int? cursor = null);
}

public interface IExplorer
{
    Task<Explorable> Resolve(PositionRef target);
    Task<IReadOnlyList<Explorable>> Resolve(IReadOnlyList<PositionRef> targets);
    Task<Explorable> Follow(Link link);
    Task<Presentation?> Present(Explorable element, Surface surface);
}

public interface IExplorableClient
{
    const int DefaultPageSize = 20;
    Task<NodeRecord> Card(string id);
    Task<IReadOnlyList<Element>> Elements(IReadOnlyList<string> ids);
    Task<EdgePage> Edges(string positionId, EdgeKind kind, int? cursor = null, int limit = DefaultPageSize);
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
    public sealed record SectionList(SectionStyle Style, int InitialClamp, SectionOrder Order) : Affordance;
}
public enum ArrowDirection { Previous, Next }

public sealed record SavedExploration(string Id, string Name, DateTimeOffset CreatedUtc, PositionRef Start, IReadOnlyList<Link> Steps);
```
- **The edge's frontier is derived here (rule 27).** An edge's `Groups` are its served `edge_summary` (what is positioned on the edge: `justified-by`, …); its `Ends` are `[Link(edge.Kind.Dual(), edge.Subject), Link(edge.Kind, edge.Object)]`; a node's `Ends` are empty. Stepping onto an edge from a page of kind `K` is `Link(K, EdgePosition(entry.Edge))`. With these kinds, `Exploration.Back` and the breadcrumb's retrace rule (`step.Kind == crumbs[^1].Kind.Dual()`) already return to the end the edge was entered from — no new Back logic.
- `IExplorableClient.Card` stays for the legacy section providers only (F-34 retires `/api/node/{id}`); `GraphExplorer` reads `Elements` only. `Resolve(targets)` is one element read, so resuming a save costs one request (27c).
- `Explorable.Links` no longer filters edge positions (`Neighbours.Nodes()` leaves it; F1-12's filter retires): every served neighbour is a `Link`. `Follow` is total.
- `Affordances.Of`: `FollowsIn` → `Arrows(Next)`, `PrecedesIn` → `Arrows(Previous)` (F-22). No edge kind maps to an edge's ends (there is none); FocusView renders `Ends` as up-crumbs and offers `Entry.Edge` as a step control on every entry. The signed-off `Affordance.EntryEnd` and the `From`/`To` → `UpCrumb` rows fall with OPEN 1.
- `Presentation.Of(ElementKind.Edge _, Popover)` = `Card` for every edge kind; `World`/`Reader` = `null`. So `Offers` is total on the popover (a law), and F-28 closes: a section heading shows its served count **only** on `Surface.Popover`.
- Selection stays node-only: `ToggleSelection` takes a `NodeRef`; FocusView offers select only on `NodePosition` targets (`Positions.Of(...).Kind.Match(node: _ => true, edge: _ => false)`).
- Saves: storage key `explorations-v3`; `LegacySaves` gains the v2 → v3 translation (a v2 `NodeRef` becomes a `NodePosition`; every v2 save maps whole, R4). The v1 reader is unchanged (FOCUS-9 deletes v1 and v2 readers together).

### FOCUS-6 (Tasks 4–10)

Server wire additions (`server/atlas-contract/src/wire/graph.rs`, `wire/map.rs`, `server/atlas-core/src/wire.rs`):
```rust
pub struct MapDetail { pub window: TimeRange }
pub struct EraDetail { pub window: TimeRange }
pub struct PolityDetail { pub reign: TimeRange }
```
Added fields: `NodeRecord.map: Option<MapDetail>`, `NodeRecord.era: Option<EraDetail>`, `NodeRecord.polity: Option<PolityDetail>`, `PlaceDetail.blurb: Option<String>`, and `node: NodeRef` on `ScenePlace`, `QuietPlace`, `Polity`, `Era`.
- `MapDetail.window` and `EraDetail.window` read the node payloads' `from_year`/`to_year` (`NodePayload::Map`, `NodePayload::Era`).
- **Compiled, not computed per request (rule 27):** a polity's reign (the span of its eras) and a place's default blurb are derivations over data alone, so the compiler writes them: sidecars `polity_reign (polity_id TEXT PRIMARY KEY, from_year INTEGER NOT NULL, to_year INTEGER NOT NULL)` and `place_blurb (place_id TEXT PRIMARY KEY, blurb TEXT NOT NULL)` in `server/atlas-graph/src/sqlite/extras.rs`, with their `SceneSource`/`GraphService` readers. The server reads them by key.
- `node: NodeRef` on the map wire lets the World view open what was clicked without composing an id (rule 25; the F-31 category). It is the graph's own reference, read from the compiled label table; `/api/scene` itself stays view-shaped until MAPS retires it (F-34).
- Each `TimeRange` carries its served `label`; the client never formats a year. (That label is still formatted per request by `wire::TimeRange::of`; FINDING F-36 below.)

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
| every edge kind | `null` | Label, Provenance (and the two ends as crumbs) |

`PopoverOpening.Legacy` exists only for callers that still hold a legacy node; its last users (`PolityDeltaNode` for MAPS, the legacy kinds for FOCUS-2…9) remove it. Closes F-29.

## Deletion inventory (FOCUS-6 total)

Graph and server, Task 1: `graph-types/src/present.rs`; `graph-types/src/frontier.rs` and `server/atlas-graph/tests/frontier_falsifiability.rs` (OPEN 7); `node::Card`; every comment in `server/`/`graph-types/` that names a client word (41 sites, rule 9).
Graph and server, Task 2: `writer::node_label` and the `node.label` column; `graph_wire::text_unit_label` and `graph_wire::node_label` (labels are read, never composed).
Client — files: `client/Explore/PlaceNode.cs`, `client/Explore/PlaceDates.cs`, `client/Explore/YearNode.cs`, `client/Explore/TimeAndPlaceNode.cs`, `client/Components/PlaceCard.razor`, `client/Components/PlaceEventsList.razor`, `client/CardPlacement.cs`; tests `client.Tests/PlaceDatesTests.cs`, `client.Tests/YearNodeEventTimeTests.cs`, `client.Tests/CardPlacementTests.cs`, `client.Tests/ServedYearsTests.cs`.
Client — members: `PlaceDescriptionSection`, `PlaceDatesSection`, `PlaceBlurbSection`, `PlaceEventsSection`, `YearFrontierSection` and their `PopoverSectionRegistry` rows; `AtlasClient.Place`, `AtlasClient.PlaceHistory`; `MapInterop.MeasureCardPlacement`, `CardMeasurement`, map.js's card-measure export; `LegacyNodes.For`'s `Place` arm (→ `null`); `ExplorerPopover.Root`/`.Saved` (→ `Opening`); in `World.razor`: `_hoverPlace`, `_hoverX`, `_hoverY`, `_closeCts`, `_pinnedPlaceId`, `_pointerOverCard`, `QuietAsScenePlace`, `OpenPlaceFromCard`, `OpenExploreNode`, `ClosePinnedCard`, `GoToAdjacent`, `CloseCardNow`, `ScheduleCardClose`, `CardPointerEnter`, `CardPointerLeave`, `CloseCardAfterDelay`, the `PlaceCard` element, and the direct `Atlas.SceneTime`/`SceneScripture`/`Polities` calls; the `IdentityTests`/`LegacyNodesTests`/`PushViaConformanceTests` rows for Place, TimeAndPlace, Year; `stryker-config.json`'s `**/Explore/PlaceDates.cs`; the `app.css` rules for `.place-card*`, `.popover-place-*`, `.popover-event-row*` that only these used; `Explorable.Links`' use of `Neighbours.Nodes()`.
Server, Task 10: `server/atlas-contract/src/places.rs` (`/api/place/{id}`, `PlacePeriod`), `wire/places.rs` `PlacePage` and `History` (if nothing else reads them — the task verifies), their route registration and pact interactions.
Never built (withdrawn with F6-t1/t2, so nothing to delete): `/api/edge/{id}`, `/api/edge/{id}/edges`, `EdgeCard`, `EdgeEnd`, `EdgeEntry.end`, `RelationId::{EdgeSource, EdgeTarget}`, `IExplorableClient.{EdgeCard, EdgeEdges}`, `Affordance.EntryEnd`.
Not deleted (stated so no one "finishes" it): `/api/node/{id}` and `IExplorableClient.Card` (F-34), `/api/scene`, `/api/scene/scripture`, `/api/polities`, `/api/eras`, `/api/landmarks`, `/api/land-mask`, `PolityDeltaNode`, `PolityDelta*Section`s, `MapFocusHatch` (Event sections, FOCUS-5), `PlaceChooser`, `Neighbours.Nodes()` (legacy providers still use it).

---

### Task 1: F-33 — the backend speaks the graph's words, and a gate keeps it so

**Owner gate first:** show OPEN 6–9 to the owner (or take the defaults); record the answers in the ledger.

**Files:**
- Create: `server/atlas-contract/tests/backend_vocabulary.rs`; `graph-types/src/adjacency.rs` (by `git mv` of `explore.rs`)
- Delete: `graph-types/src/present.rs`, `graph-types/src/frontier.rs`, `server/atlas-graph/tests/frontier_falsifiability.rs` (OPEN 7)
- Modify: every file in the rename table above (`graph-types/src/{lib,edge,graph,id,node,store,tests,wire_form}.rs`, `graph-types/tests/wire_form.rs`; `server/atlas-cli/src/commands/{edges,help,node,tutorial}.rs`, `server/atlas-cli/tests/cli.rs`; `server/atlas-contract/src/{aqc_export,contents,document,error,events,graph,graph_wire,load,reading}.rs`, `src/bins/export_aqc_examples.rs`, `src/wire/{graph,reading}.rs`, `benches/queries.rs`; `server/atlas-contract/tests/{api,aqc_corpus_generation,aqc_cucumber,contract_generation,graph_api,no_legacy_event_reads,perf_smoke,regenerated_aqc_corpus,sources_api}.rs`; `server/atlas-core/src/narrative.rs` (assertion message); `server/atlas-graph/src/{bible_container_adapter,build,catechism_adapter,concord_adapter,event_world,kretzmann_adapter,law_check,legacy,peoples_adapter,person_adapter,provenance,red_letter_adapter,scene_source,service}.rs`, `src/sqlite/{partition,snapshot,writer}.rs`; `server/atlas-graph/tests/{bible_containers_real_data,concord_sc_overlap_real_data,justified_by_real_data,lexicon_real_data,lexicon_section_real_data,provenance_registry_real_data,reload_real_data,sqlite_laws}.rs`), `contracts/openapi.yaml`, `contracts/atlas-query-contract/{aqc.schema.json,features/*,CHANGELOG.md}`, pacts, `client/Contract/Wire.g.cs` (regenerated) and the 17 client files naming `NodeCard`.
- Test: the gate; every existing test is the rename's regression net.

- [ ] **Step 1: Failing gate** (`backend_vocabulary.rs`), names free of client words:
  - `no_backend_source_borrows_a_client_word` — every `scanned_text` line of every `backend_sources()` file, and every path component, has no `is_client_word` word; the failure lists `path:line: word` for every hit.
  - `the_published_contract_borrows_no_client_word` — the same over `published_contract()`.
  - `a_camel_case_name_is_split_into_its_words` (`"NodeRecord"` → `["node", "record"]`, `"adjacency_at"` → `["adjacency", "at"]`).
  - `a_word_that_merely_contains_a_client_word_is_not_one` (`"jaccard"`, `"discard"`).
  - `a_string_literal_is_not_scanned_but_a_doc_comment_is` (a source holding both, whole `scanned_text` result).
- [ ] **Step 2:** `cargo test -p atlas-contract --test backend_vocabulary` → red; record the hit count in the ledger (at `3baaeb6`: 289 identifiers, 41 comments, plus file paths and the contract's descriptions).
- [ ] **Step 3: Rename** per the table, in this order so each step compiles: graph-types module and types (`git mv` keeps history); deletions (`present.rs`, `frontier.rs` and its test); `node::label`; server identifiers, CLI, test names; comments deleted; published descriptions reworded. `PositionKind::Version`: if the non-`canon-ids` version hash encodes the variant's name, the in-memory version stamp moves — check `sections::version_root` (the `canon-ids` build, which serves) does not, and say so in the ledger.
- [ ] **Step 4: Regenerate (critical section `contract`):** `cargo run -p atlas-contract --bin export_contract`; `cargo run -p atlas-contract --bin export_aqc_examples`; `--check` clean; `CHANGELOG.md` AQC **major** (OPEN 8: a renamed feature file); re-bless pacts; `dotnet run --project client.ContractGenerator`; rename `NodeCard` → `NodeRecord` in the 17 client files. Release the lock.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace` and `(cd ../graph-types && cargo test --all-features)` → green, gate included; `bash scripts/contract-gate.sh`; `bash scripts/contract-semver-gate.sh` (declared major); `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green.
- [ ] **Step 6: Commit** `graph: the backend names adjacency and records, never the client's exploration words; a gate fails on any (F-33, rule 27)`.

### Task 2: A-EDGES, server — compiled labels and the generic element read

**Owner gate first:** OPEN 10–13 (or the defaults), in the ledger.

Two commits: Part A (labels are compiled) lands first; Part B (the read) builds on it.

**Files (Part A):**
- Create: `server/atlas-graph/src/labels.rs`, `server/atlas-contract/tests/no_served_label_composition.rs`, and the scanner moved from `backend_vocabulary.rs` into `server/atlas-contract/tests/common/source_scan.rs` so both laws read sources one way
- Modify: `graph-types/src/{edge,graph,store}.rs` (`EdgeRecord.meta`, `Graph.{labels, edges_by_id}`, `GraphQuery::{labels, edge}` on `Graph`, `MemSnapshot` and the law fakes), `graph-types/src/edge.rs` (`EdgeKind::display_label`), `server/atlas-graph/src/build.rs` (calls `labels::compile`), `server/atlas-graph/src/sqlite/{ddl,writer,snapshot}.rs` (`label` table; `node.label` and `writer::node_label` deleted), `server/atlas-graph/src/service.rs`, `server/atlas-contract/src/graph_wire.rs` (`describe_node`, `describe_nodes`, `describe_position`, `node_ref` read `labels`; `text_unit_label`, `node_label` deleted), `server/atlas-contract/src/wire/graph.rs` (`EdgeRef { id, kind, label }`, `EdgeEntry.edge: EdgeRef`), `server/atlas-contract/src/graph.rs` (entries carry `EdgeRef`), `server/atlas-contract/src/document.rs` (`x-atlas-relations` `label`), `data/compiled` (rebuilt), contracts, pacts, and the client sites that read `EdgeEntry.Edge` as a string (→ `.Id`, mechanical, so the client keeps compiling).
- Test: `graph-types/src/store.rs` laws, `server/atlas-graph/tests/sqlite_laws.rs`, `server/atlas-graph/src/labels.rs` unit tests, `server/atlas-contract/tests/graph_api.rs`, `server/atlas-contract/tests/contract_generation.rs`.

**Files (Part B):**
- Modify: `server/atlas-contract/src/wire/graph.rs` (`EdgeRecord`, `Element`, `ElementPage`), `server/atlas-contract/src/graph.rs` (`elements`, `ElementsQuery`, `MAX_ELEMENTS`, `node_record`/`edge_record` builders; `node_edges` takes `PositionReference`), `server/atlas-contract/src/reference.rs` (`ElementId`, `decode_element_id`, `PositionReference`), `server/atlas-contract/src/error.rs` (`ElementRefusals`), the router and `document.rs` path list, contracts, AQC, pacts.
- Test: `server/atlas-contract/tests/graph_api.rs`, `server/atlas-contract/tests/perf_smoke.rs`.

- [ ] **Step 1 (A): Failing tests.**
  - store law (both backends through `assert_answers_match`): `every_held_position_answers_one_compiled_label` and `every_edge_is_read_by_its_id_with_its_ends_and_its_meta` — the law already walks every position; it now compares `labels` and `edge` too (24b: a backend that answers either differently fails).
  - `labels.rs`: `a_verse_is_labelled_by_its_reference`, `a_concord_paragraph_is_labelled_by_its_citation`, `an_edge_is_labelled_by_its_subject_its_kind_and_its_object` (OPEN 10's form, whole string), `compiling_labels_every_position_the_graph_holds`.
  - `graph_api.rs`: `every_neighbour_names_its_edge_with_its_kind_and_compiled_label` (walks one page of every kind in a sample verse's `edge_summary`; each `entry.edge.label` equals the element read's label for that id in Part B — in Part A, equals `labels` read through the service).
  - `contract_generation.rs`: `every_edge_kind_has_a_served_display_label` (walks `EdgeKind::ALL` against `x-atlas-relations`; F-23).
  - a source law, `server/atlas-contract/tests/no_served_label_composition.rs` (sharing `backend_vocabulary.rs`'s scanner, one declaration): `no_served_crate_composes_a_label` — `atlas-contract`/`atlas-server` sources call none of `labels::`, `node::label(`, `dot_ref(`, `ConcordTag::cite(` (the closure of "labels composed per request"; it stands in for F-4's crate boundary until F-4 lands).
  - graph-types: the relation-count law still reads 22 (unchanged: no new relation).
- [ ] **Step 2 (A):** `cargo test -p atlas-graph -p atlas-contract` and `(cd ../graph-types && cargo test --all-features)` → red.
- [ ] **Step 3 (A): Implement.** `labels::compile` fills `Graph.labels` and `Graph.edges_by_id` at the end of `build`; the writer writes the `label` table; `SqliteSnapshot::labels` is one primary-key lookup per position, `SqliteSnapshot::edge` one `edge_by_id` lookup. `describe_*` read `labels`; a held position with no label is `ApiError::internal` naming it (a compile defect, never a fallback string).
- [ ] **Step 4 (A): Rebuild and regenerate (critical section `contract`):** rebuild `data/compiled`; export, `--check`, AQC **major** (`EdgeEntry.edge` changes shape), re-bless, client generator, client `.Id` sites. Release.
- [ ] **Step 5 (A):** `cargo test -p atlas-graph -p atlas-contract`, graph-types, `bash scripts/contract-gate.sh`, `dotnet build client && dotnet test client.Tests` → green. **Commit** `graph: every node and edge label is compiled into the artifact and read, never composed per request (rule 27, 27c; F-23)`.
- [ ] **Step 6 (B): Failing tests** (`graph_api.rs`), every expectation read from the artifact, never a literal (F-8):
  - `the_element_read_answers_each_id_in_order_with_its_node_its_edge_or_its_absence` — ids: a verse, the edge of its first `attests` entry, an unknown `Person:` id → `[NodeElement, EdgeElement, MissingElement]`, whole `ElementPage`.
  - `an_edge_record_names_its_kind_its_label_its_two_ends_and_its_provenance` — the ends equal the page's own node and the entry's neighbour (in recorded direction), the label equals the entry's `edge.label`.
  - `an_edge_records_counts_are_its_own_neighbours` — a justified edge: its `edge_summary` `justified-by` count equals the length of `/api/node/{edge}/edges?kind=justified-by`.
  - `the_neighbour_read_at_an_edge_lists_what_justifies_it`.
  - `the_node_route_and_the_element_read_serve_one_node_record` (DRY: equal JSON for the same id).
  - `an_element_read_beyond_the_cap_is_refused`, `an_element_read_of_no_id_is_refused`, `one_malformed_id_refuses_the_whole_element_read`.
  - `no_relation_name_is_a_node_kind_prefix` (walks `RelationId::ALL`, `SymRelationId::ALL` against every `NodeKind`'s wire prefix; 24b for the decode).
  - `perf_smoke.rs`: `an_element_read_at_the_cap_answers_inside_the_read_budget` (the existing per-read budget; 27f's 10× synthetic graph is not in this batch, FINDING F-37).
- [ ] **Step 7 (B):** `cargo test -p atlas-contract --test graph_api element` → red (route absent).
- [ ] **Step 8 (B): Implement.** `elements` decodes every id first (one `bad_ref` refuses all), then reads nodes through `node_record` and edges through `edge_record`, each one index lookup; no relation, field or element is added to the graph. `node_edges` resolves a `PositionReference` (`snap.node` or `snap.edge`).
- [ ] **Step 9 (B): Regenerate (critical section `contract`):** export, `--check`, AQC minor (a new route and schemas), re-bless, client generator. Release.
- [ ] **Step 10 (B, lock `heavy`):** `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh` → green. **Commit** `graph: one element read answers nodes and edges by id, many per call; the neighbour read takes an edge (rule 27a, 27c; F-21)`.

**Expected red after Task 2 until Task 3:** `client.ContractTests` `GeneratedUsageTests` for `Element`, `NodeElement`, `EdgeElement`, `MissingElement`, `ElementPage`, `EdgeRecord`, `EdgeRef.Kind`, `EdgeRef.Label`.

### Task 3: A-EDGES, client — `ElementKind`, `Link.Target: PositionRef`, `Follow` total, the edge's frontier derived

Salvaged from `54079c4`/`7f1ef67` (read them for `ElementKind.cs`, `Positions.cs`, `Link`/`Entry`, the FocusView edits and `ServedGraph`), reworked onto the element read: no `EdgeCard`, no `EdgeEnd`, no `From`/`To`/`SourceOf`/`TargetOf` kinds.

**Files:**
- Create: `client/Explore/ElementKind.cs`, `client/Explore/Positions.cs`
- Modify: `client/Explore/{Explorable,Link,Explorer,Presentation,Affordances,Surface,Exploration,LegacySaves}.cs`, `client/IExplorableClient.cs`, `client/GraphExplorableClient.cs`, `client/Views/FocusView.razor`, `client/SavedExplorationsService.cs`, `client/Components/ExplorerPopover.razor` (identity type only), `client/Components/ExplorationListItem.razor`, `tests/ux/lib/{api,edges}.ts`
- Test: `client.Tests/Explore/{ExplorableTests,GraphExplorerTests,PresentationTests,AffordancesTests,LegacySavesTests,ExplorationTests}.cs`, `client.Tests/Explore/ServedGraph.cs` (serves elements), `client.Tests/GraphExplorableClientTests.cs`, `client.Tests/Views/FocusViewTests.cs`, new `client.Tests/Explore/ElementKindLawTests.cs`, `tests/ux/explore-edges.spec.ts`

- [ ] **Step 1: Failing tests.**

`client.Tests/Explore/GraphExplorerTests.cs` (additions):
```csharp
[Fact]
public async Task Following_a_link_to_an_edge_resolves_it_with_its_two_ends_and_its_own_neighbour_groups()
{
    // Arrange
    var graph = new ServedGraph().Serving(ServedGraph.EdgeRecordOf(AttestedIn, ExodusEvent, Exodus14, new EdgeSummaryEntry(EdgeKind.JustifiedBy, 1)));
    var explorer = new GraphExplorer(graph);
    // Act
    var edge = await explorer.Follow(new Link(EdgeKind.Attests, ServedGraph.AtEdge(AttestedIn)));
    // Assert
    Assert.Equal(
        (new ElementKind.Edge(EdgeKind.AttestedIn) as ElementKind, AttestedIn.Id, AttestedIn.Label,
            new[] { new FrontierGroup(EdgeKind.JustifiedBy, 1) },
            new[] { new Link(EdgeKind.Attests, ServedGraph.At(ExodusEvent)), new Link(EdgeKind.AttestedIn, ServedGraph.At(Exodus14)) }),
        (edge.Kind, edge.Id, edge.Label, edge.Groups.ToArray(), edge.Ends.ToArray()));
}

[Fact]
public async Task A_page_that_leads_to_edges_yields_links_to_those_edges()
{
    // Arrange
    var justifier = await Resolved.Node(NodeKind.Source, SourceId, SourceLabel, new FrontierGroup(EdgeKind.Justifies, 1)).Using(graphServingAJustifiesPage);
    // Act
    var page = await justifier.Links(EdgeKind.Justifies);
    // Assert
    Assert.Equal(new Page<Link>([new Link(EdgeKind.Justifies, ServedGraph.AtEdge(JustifiedEdge))], null), page);
}
```
Plus: `Every_entry_offers_a_step_onto_its_served_edge` (`Entries` → whole `Page<Entry>`); `A_missing_element_is_a_contract_breach` (`Resolve` of a target the read answers `MissingElement` throws `ContractBreach` naming the id); `Resolving_many_targets_is_one_element_read` (the `ServedGraph` counts calls: one).
`client.Tests/Explore/ExplorationTests.cs`: `Stepping_onto_an_edge_and_back_out_of_the_end_it_was_entered_from_retraces` — a law over every `EdgeKind` in both orientations: for an edge entered from its subject (page kind `k`) and from its object (page kind `k.Dual()`), following the matching `Ends` link collapses the breadcrumb to the start (24b: an end link that does not retrace fails for that kind).
`client.Tests/Explore/PresentationTests.cs`: the table test walks `Enum.GetValues<NodeKind>()` **and** `Enum.GetValues<EdgeKind>()` as `ElementKind`s; asserts every edge kind is `(null, null, Card)`; plus `Every_element_is_offered_on_the_popover` (the F-28 law).
`client.Tests/Explore/AffordancesTests.cs`: `Following_is_the_next_arrow_and_preceding_the_previous` (F-22).
`client.Tests/Views/FocusViewTests.cs`: `An_edge_shows_its_two_ends_as_crumbs` (`{Handle}-end-{targetId}`, served labels), `Every_entry_offers_a_step_onto_its_edge` (`{Handle}-entry-edge-{kind}-{edgeId}`), `A_heading_reads_the_served_display_label` (F-23), `A_heading_counts_only_on_the_popover` (F-28).
`client.Tests/Explore/ElementKindLawTests.cs`: `No_client_source_switches_over_a_closed_sum` — scans `client/**/*.cs` and `*.razor` for `switch` arms naming `ElementKind.`, `Frame.`, `Emphasis.`, `PopoverOpening.` (24b closure of the partial-match category).
`client.Tests/Explore/LegacySavesTests.cs`: `A_v2_save_reads_as_v3_with_every_node_as_a_node_position` (whole `SavedExploration`).
`tests/ux/explore-edges.spec.ts` (A-EDGES' done-when): take EXO 14:21's first `attests` entry from the live page (`api.nodeEdges`), read its edge with `api.elements([entry.edge.id])`; open the verse's popover, click `popover-entry-edge-attests-{edgeId}`, assert `popover-card-title` is the served edge label and `popover-end-{subjectId}`/`popover-end-{objectId}` show the served end labels; click the event's end; assert `popover-title` is the event's served label; Back returns to the verse.

- [ ] **Step 2:** `dotnet test client.Tests --filter "GraphExplorerTests|ExplorationTests|PresentationTests|AffordancesTests|ElementKindLawTests|LegacySavesTests|FocusViewTests"` → compile errors.
- [ ] **Step 3: Implement** the types exactly as above. `GraphExplorer.Resolve(targets)` sends every target's id in one `Elements` call and matches each `Element` (`NodeElement` → `Explorable(node)`, `EdgeElement` → `Explorable(edge)`, `MissingElement` → `ContractBreach`); `Resolve(target)` is `Resolve([target])[0]`; `Follow(link) => Resolve(link.Target)`. `Explorable(EdgeRecord)` builds `Ends` from `Subject`/`Object` and the kind's served dual (`EdgeKinds.Dual`, read from the vocabulary document — no client arithmetic). `Explorable.Links`/`Entries` page `Edges(Id, kind, cursor)` for nodes and edges alike and map every entry (no filter). `Presentation.Of(ElementKind, Surface)` delegates by `kind.Match(node: n => OfNode(n, surface), edge: _ => OfEdge(surface))`, both private tables exhaustive switches over the enums. `HomeSurfaces.Of(ElementKind)`. FocusView: the arrow branch reads `Arrows.Direction`; headings read the served display label and show `(count)` only on `Surface.Popover`; `Ends` render as up-crumbs; each entry renders its edge step control. Retype every `Explorable.Identity`/`NodeRef` consumer in `ExplorerPopover`, saves and selection per the types block; storage key `explorations-v3`.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green (`GeneratedUsageTests` now reads every Task-2 member). `npx playwright test tests/ux/explore-edges.spec.ts tests/ux/saved-explorations.spec.ts tests/ux/state-focus.spec.ts` → green.
- [ ] **Step 5: Commit** `edges: an Explorable is a node or an edge; Link targets a position; an edge's ends are derived on the client; Follow is total (R18; F-21, F-22, F-23, F-28; F1-12 retires)`.

### Task 4: Geography on the wire — map/era/polity details, place blurb, `node` on map rows

**Files:**
- Modify: `server/atlas-contract/src/wire/graph.rs` (`MapDetail`, `EraDetail`, `PolityDetail`, `NodeRecord` fields, `PlaceDetail.blurb`), `server/atlas-contract/src/graph.rs` (`node_record` fills them), `server/atlas-graph/src/sqlite/{extras,ddl}.rs` (`polity_reign`, `place_blurb` sidecars, written by the compiler), their `SceneSource`/`GraphService` readers, `server/atlas-core/src/wire.rs` (`ScenePlace.node`, `QuietPlace.node`), `server/atlas-contract/src/wire/map.rs` (`Era.node`, `Polity.node`) and their builders, `data/compiled` (rebuilt), `contracts/*` regen, pacts, `client.Tests/Explore/ServedGraph.cs` (`Record(...)` gains the three nulls)
- Test: `server/atlas-contract/tests/graph_api.rs`, the scene/polity/era API tests, `server/atlas-graph/tests/sqlite_laws.rs`

- [ ] **Step 1: Failing tests:** `a_map_record_serves_its_window` (`Map:era-conquest` → whole `map` detail with served label); `an_era_record_serves_its_window`; `a_polity_record_serves_its_compiled_reign_as_the_span_of_its_eras` (expected span computed in the test from the artifact's `polity_era` rows, never a literal); `a_place_record_serves_its_compiled_default_blurb_where_one_is_recorded` (and `None` where not); `every_polity_has_one_compiled_reign` and `every_place_with_a_history_has_its_default_blurb_compiled` (compile laws, 24b); `every_scene_place_and_quiet_place_names_its_node` (walks the whole scene for a window: each `node.id` resolves through `/api/elements`); `every_polity_row_and_era_names_its_node` (same walk). The resolve-walks are the 24b closure for "map rows the client cannot open".
- [ ] **Step 2:** `cargo test -p atlas-contract -p atlas-graph` → red.
- [ ] **Step 3: Implement** from the artifact only (rules 26, 27): payload years through `wire::TimeRange::of`; reign and default blurb **written by the compiler** into their sidecars and read by key (the server computes neither); which period is a place's default is decided where `place_history` is compiled, not per request; `node` via `node_ref(&id, snap)`, which reads the compiled label.
- [ ] **Step 4: Rebuild and regenerate (critical section `contract`, after Task 2's):** rebuild `data/compiled`, export, `--check`, AQC minor, re-bless, client generator. `cargo test -p atlas-contract -p atlas-core -p atlas-graph && bash scripts/contract-gate.sh` → green.
- [ ] **Step 5: Commit** `geography: map, era and polity records serve their windows; reigns and default blurbs are compiled; map rows name their nodes (R10, R14; rule 27)`.

**Expected red after Task 4 until Task 5/9:** `client.ContractTests` `GeneratedUsageTests` for `MapDetail`, `EraDetail`, `PolityDetail`, `PlaceDetail.Blurb`, `ScenePlace.Node`, `QuietPlace.Node`, `Polity.Node`, `Era.Node` (each is first read in Task 5 or 9).

### Task 5: `Presentation.Geography` — the `Surface.World` rows and the geographic cards

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
Plus: `An_era_on_the_world_is_bounded_like_its_map`; `A_place_on_the_world_keeps_the_current_frame_and_marks_its_site`; `A_polity_on_the_world_keeps_the_current_era_and_marks_its_territory_with_its_reign` (R14: `Frame.Current`, never `Bounded`); `A_first_map_has_no_previous_end`; `A_place_card_lists_only_the_served_fields` (canonical/established/destroyed/blurb/provenance, absent ones omitted); `A_polity_card_shows_its_reign_label`; `A_map_card_shows_its_window_label`; `A_non_geographic_node_is_not_presented_on_the_world` (returns `null`); `An_edge_is_never_presented_on_the_world`.
`CrossingTests`: `A_window_inside_the_bounds_crosses_nothing`; `A_window_reaching_past_the_end_crosses_to_the_next`; `A_window_reaching_before_the_start_crosses_to_the_previous`; `Touching_a_bound_is_not_crossing_it`.
- [ ] **Step 2:** `dotnet test client.Tests --filter "GeographyPresentationTests|CrossingTests"` → compile errors.
- [ ] **Step 3: Implement.** `Present` switches on `Presentation.Of(element.Kind, surface)` (the `Form` enum, exhaustive): `Card` → `CardOf(element)` (fields from the served record detail, field names as named constants beside `ProvenanceField`); `Geography` → `GeographyOf(element)` reading `map`/`era`/`polity`/`place` from the node record and the first `PrecedesIn`/`FollowsIn` link for `Bounded`; `Sequence`/`Text` → `CardOf(element)` until FOCUS-2/3. A geographic node whose detail is absent is a served-contract breach: `GeographyOf` throws `ContractBreach` naming the node (not a silent default). `Crossing.Of` compares served `Year` values only; no year arithmetic.
- [ ] **Step 4:** `dotnet test client.Tests && dotnet test client.ContractTests` → green except the Task-4 expected-red members still unread (`ScenePlace.Node`, `QuietPlace.Node`, `Polity.Node`, `Era.Node`).
- [ ] **Step 5: Commit** `geography: Map, Era, Place and Polity present on the world as a frame and an emphasis (R10, R14, R16)`.

### Task 6: The MAPS seam — `IMapSource` (BUILT: `lane/claude/F6-t5`, `4e7a77e`, on `3baaeb6`)

Rule 27 does not touch it: `AtlasMapSource` composes over existing reads and derives nothing. **Remaining:** Codex review (14b + 24a), then land in wave order. If Task 1's `NodeCard` → `NodeRecord` regen touches any file it changed, rebase it onto Task 1 and re-run its Step 4.

**Files (as built):**
- Create: `client/Geography/IMapSource.cs` (`IMapSource`, `MapLayers`), `client/Geography/AtlasMapSource.cs`
- Modify: `client/Program.cs` (register `IMapSource` → `AtlasMapSource`), `client/Pages/World.razor` (`DebouncedLoadScene`, `LoadPolitiesFor`, `DebouncedLoadScriptureScene` read `IMapSource` only)
- Test: `client.Tests/Geography/AtlasMapSourceTests.cs`, `client.Tests/FetchLayerConformanceTests.cs` (a law: `World.razor` names no `Atlas.Scene*`/`Atlas.Polities`)

- [x] **Step 1: Failing tests:** `The_layers_during_a_window_are_that_windows_scene_and_its_polities` (fake HTTP serving both; whole `MapLayers`); `The_layers_for_a_scripture_are_its_scene_with_no_polities` (today's behaviour: polities hidden in scripture mode); the conformance law.
- [x] **Step 2:** red. **Step 3:** implemented; `World.razor` keeps its request series and debounce, calling `Maps.During(from, to)` once for scene+polities (`MapLayers`), which also removes the second request race in `LoadPolitiesFor`. `Polities(-4004, 100)` roster fetch moves behind the same source as `Maps.During(TimelineStart, TimelineEnd)` with the two years as the named constants they already are in `World.razor`.
- [x] **Step 4:** `dotnet test client.Tests` green; `npx playwright test tests/ux --grep "world"` → same set as at the base (no behaviour change).
- [x] **Step 5: Commit** `geography: the world reads its layers through IMapSource -- the seam the MAPS migration replaces`.
- [ ] **Step 6:** review; land.

### Task 7: `PopoverOpening` — open the popover on a position, a save, or a legacy node

**Files:**
- Create: `client/Explore/PopoverOpening.cs`
- Modify: `client/Components/ExplorerPopover.razor` (`Opening` replaces `Root`/`Saved`), every host (`client/Pages/{Reader,World,Concord,Kretzmann}.razor`, `client/Layout/MainLayout.razor`, the saved-explorations page), `client/Components/MentionScan.razor`
- Test: `client.Tests/State/ExplorationOwnershipHandoffTests.cs` (fake popover retyped), `client.Tests/Explore/PopoverOpeningTests.cs`

- [ ] **Step 1: Failing tests:** `Opening_on_a_position_resolves_it_and_opens_there`; `Opening_on_a_save_reseeds_its_whole_trail_in_one_element_read`; `Opening_on_a_legacy_node_resolves_its_identity_and_remembers_its_rendering` (today's `Root` path, unchanged in behaviour); the five ownership laws, retyped.
- [ ] **Step 2:** red. **Step 3:** implement; `OnInitializedAsync` is one `Opening.Match(...)`. Hosts pass `new PopoverOpening.Legacy(node)` where they hold a legacy node and `new PopoverOpening.Explore(position)` where they hold a reference. `Resume` reseeds through `IExplorer.Resolve(targets)` (one request, 27c). `MentionScan` opens `Explore` with the mention's served `NodeRef` — if its piece carries only a local place id, record a FINDING under F-31 (the id is composed through `NodeIds.Of` as today; not fixed on the side).
- [ ] **Step 4:** `dotnet test client.Tests`; `npx playwright test tests/ux --grep "popover|saved-explorations|state-focus|mention"` → green.
- [ ] **Step 5: Commit** `focus: the popover opens on one PopoverOpening -- a position, a save, or a legacy node (F-29)`.

### Task 8: Slider and map primitives — `Bounds`, `Band`, `OnCross`, `Emphasize`, `OnPolityClick`

**Files:**
- Modify: `client/Components/TimeSlider.razor`, `client/MapInterop.cs` (`Emphasize`, `IMapEvents.OnPolityClick`, `MapEventsSink.OnPolityClick`), `client/wwwroot/js/map.js` (`setEmphasis(id, emphasis)`: pan+ring a site, outline+raise a polity at the drawn year; polity-territory click → `OnPolityClick`), `client/wwwroot/css/app.css`
- Test: `client.Tests/Components/TimeSliderTests.cs` (bUnit, if present at base; else Playwright only), `tests/ux/world-geography.spec.ts` (new, primitives part)

- [ ] **Step 1: Failing tests:** slider — `A_bounded_slider_cannot_be_dragged_past_its_bounds_and_reports_the_crossing` (drag the end handle beyond `Bounds.To` → `OnCross(Next)` fires once, window clamps; OPEN 5 default: the crossing is offered as the `world-cross-next` button at the bound, and dragging only clamps); `A_band_is_drawn_under_the_window` (`world-slider-band` spans the served reign); map — Playwright `world-geography.spec.ts` "clicking a polity's territory reports that polity" via a test sink.
- [ ] **Step 2:** red. **Step 3:** implement; `Bounds == null` is today's slider exactly. Test ids: `world-slider-bounds`, `world-slider-band`, `world-cross-previous`, `world-cross-next`, `world-emphasis-site`, `world-emphasis-territory`.
- [ ] **Step 4:** `dotnet test client.Tests`; `npx playwright test tests/ux --grep "slider|world-border-morph|world-map"` green.
- [ ] **Step 5: Commit** `geography: the slider takes bounds, a band and a crossing; the map emphasises a site or a territory and reports polity clicks`.

### Task 9: The World view presents the Geography focus

**Files:**
- Modify: `client/Pages/World.razor`, `client/Components/PlaceChooser.razor` (picks `ScenePlace.Node`/`QuietPlace.Node`), `client/Views/FocusView.razor` (the `popover-chip-map` hatch when the current element's home surface is World and the host is not)
- Test: `tests/ux/world-geography.spec.ts` (behaviour part), `client.Tests` for any extracted pure piece

- [ ] **Step 1: Failing Playwright** (`world-geography.spec.ts`), each asserting served labels, never literals composed in the test:
  - "clicking a place focuses it: the popover shows its record and neighbours, the map marks its site, the slider keeps its window";
  - "clicking a polity keeps the era, outlines its territory at the slider year, and bands its reign on the slider" (R14);
  - "focusing a Map bounds the slider to its window; the next arrow at the bound follows `follows-in` to the next Map and re-bounds" (R10);
  - "Back from the next Map returns to the previous one and its bounds" (R17);
  - "following a Place link in the reader's popover offers `popover-chip-map`, which opens /world with the exploration carried and the place focused" (R9);
  - "selecting a place with its toggle adds its served node to the tray" (selection via `ScenePlace.Node`).
- [ ] **Step 2:** red. **Step 3: Implement.** `World.razor` subscribes to `ExplorationState`; on change, when `HomeSurfaces.Of(current.Kind)` is `World`, `await Explorer.Present(current, Surface.World)` and apply: `Frame.Match(current: keep slider, bounded: set Bounds and the two crossing links)`, `Emphasis.Match(none: clear, site: Maps.Emphasize, territory: Emphasize + Band)`. Map events: `OnPlaceClick` → the clicked scene row's `Node` → `Open` (via `PopoverOpening.Explore`); `OnPolityClick` → the roster row's `Node`; slider `OnCross(direction)` → `Follow` the stored link for that direction. Delete the card plumbing listed in the deletion inventory and the `PlaceCard` element. `PlaceChooser.OnPick` opens the picked row's `Node`. Scripture mode, follow mode, split and `PolityDelta` clicks are unchanged. OPEN 2 default: no hover preview; hover keeps only the marker highlight map.js already draws. OPEN 3 default (a): the place's popover is its window-free record plus its `site-of` neighbours; nothing asks the server for a window-scoped place view.
- [ ] **Step 4:** `dotnet test client.Tests`; `npx playwright test tests/ux/world-geography.spec.ts` green. The expected-red set (below) is recorded in the ledger by name.
- [ ] **Step 5: Commit** `geography: the world view presents its focus -- places, polities, eras and maps open, frame and emphasise (R9, R10, R14)`.

### Task 10: Deletions — the legacy place path and `/api/place`

**Files:** everything in the deletion inventory not already gone; `client.Tests/Explore/DeletionLawTests.cs`.

- [ ] **Step 1: Failing law:** `DeletionLawTests.MigratedKinds = [NodeKind.Place, NodeKind.Polity, NodeKind.Era, NodeKind.Map]`; add `No_section_provider_names_a_migrated_kind` (scans `client/Explore/PopoverSectionProviders.cs` for `node.Kind == "<kind>"` for each migrated kind's legacy string, derived from the enum) and `No_legacy_node_class_exists_for_a_migrated_kind` (reflection: no `IExplorable` whose name starts with a migrated kind's name, except `PolityDeltaNode` named as MAPS' in the test's one declared exception list). Run → red.
- [ ] **Step 2: Delete** the client files and members; `git rm` the four client test files; remove the `IdentityTests`/`LegacyNodesTests`/`PushViaConformanceTests` rows; drop `**/Explore/PlaceDates.cs` from `stryker-config.json`; delete the CSS rules.
- [ ] **Step 3: Server:** delete `/api/place/{id}` (`places.rs`, route registration, `PlacePage`/`History` if unreferenced — `cargo build` decides), its tests and pact interactions. **Regenerate (critical section `contract`, after Task 4's):** export, `--check`, AQC **major** if the contract policy classes a removed route as breaking (else minor; the semver gate `scripts/contract-semver-gate.sh` decides, not the author), re-bless, client generator.
- [ ] **Step 4 (lock `heavy`):** `cargo test --workspace && (cd ../graph-types && cargo test --all-features) && dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh` → green (GeneratedUsageTests now reads every Task-4 member; the vocabulary gate stays green).
- [ ] **Step 5: Commit** `geography: PlaceNode, PlaceCard, the place sections, YearNode, TimeAndPlaceNode and /api/place are gone (deletion law: Place, Polity, Era, Map)`.

### Task 11: Re-express the world specs; gates; mutation; push

- [ ] **Step 1: Re-express** each expected-red Playwright spec under the new behaviour, by name, keeping every surviving test id: `world-hover-text`, `world-pin`, `world-place-history`, `world-quiet-places` (not :211), `world-kjv-names`, `world-labels`, `world-map`, `world-narrative-focus`, `world-same-place`, `world-hover-resolution`, `world-existence`, `popover-sections` (place sections → FocusView card fields `popover-field-*`), `selection-tray`, `split-view`, `reader-map`, `w1`…`w5-passages` (the one `hover-verse`/`place-card` step each), `lib/hovercard.ts`, `lib/hoverSafety.ts`, `tests/ux/CONTRACT.md`. A spec whose behaviour OPEN 3 removed is rewritten to the popover's window-free record and its `site-of` neighbours, and the removal is listed in the close report.
- [ ] **Step 2: Gates.** `client.Tests/stryker-config.json` `mutate` gains `**/Explore/ElementKind.cs`, `**/Explore/Positions.cs`, `**/Explore/Geography.cs`, `**/Explore/PopoverOpening.cs`, `**/Geography/AtlasMapSource.cs`. Mutation (critical section `heavy` with "mutation" in the lock message, inside the owner's window, `free -g` ≥ 18 GB): `bash scripts/mutants-parallel.sh -n 3 -b 3baaeb6` → 100% or equivalents recorded (covers `atlas_graph::labels`, the element read, `decode_element_id`, the reign/blurb compile); then `cd client.Tests && dotnet stryker` → 100% or equivalents recorded (after the Rust shards, never beside them). Then `cargo test --workspace && (cd ../graph-types && cargo test --all-features) && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh && npx playwright test tests/ux` → green except the two carried reds.
- [ ] **Step 3:** close report `docs/superpowers/reports/2026-10-0x-focus-6-close.md`: every rule-24 category with its closure and guarantee (F-33: the vocabulary gate; labels composed per request: the compiled `label` table, the store law and `no_served_crate_composes_a_label`; partial matches over closed sums: `ElementKindLawTests`; an end link that does not retrace: the Exploration law; map rows the client cannot open: the resolve-walks); FINDINGS raised (below, plus any `MentionScan` F-31 site); the spec §5 FOCUS-9 row amendment (Year/TimeAndPlace/PlaceDates moved here).
- [ ] **Step 4: Commit and push** `git push origin worktree-bible-atlas-m1` (holding `land`).

**FINDINGS this plan expects to raise (for the queue; the owner decides):**
- **F-35 (rule 26): the event-merge tables are curated data in code.** `server/atlas-core/src/event_merge.rs` pins `EVENT_MERGE_PAIRS` (94 pairs with prose reasons) and `EVENT_DISTINCT_PAIRS`. Closure: a curated file under `data/curated/` with each pair's grounds, read by the ETL; OPEN 6 (b) would widen the vocabulary gate to string literals after it.
- **F-36 (rule 27): served year and range labels are formatted per request** (`wire::Year::of`, `wire::TimeRange::of`). Closure: the compiler writes them beside the years it already holds.
- **F-37 (rule 27f): no budget gate runs at ten times the artifact.** `perf_smoke` gates today's graph. Closure: a synthetic 10× graph and p95/size/frame budgets in the gate.
- **NodeRecord's per-kind optionals are a product where a sum belongs** (the Haskell bar; proposed closure a `oneOf` detail), and `NodeRecord.version` is restated by `ElementPage.version` — both with F-34's migration of `/api/node/{id}`.
- **The neighbour page derives `loci`/`note` per request** (`EventAccounts::read`, mention spans) — OPEN 13; with F-34.

---

## Wave schedule

Primary = the lane's critical-path work; companion = unlike work paired beside it (rule 23: Rust beside C#). A task starts only when the tasks it names as inputs have landed on the batch branch.

| Wave | Primary | Companion | Critical sections held | Expected red at wave close (ledger records names) |
|---|---|---|---|---|
| 0 | owner: OPEN 2–13 (defaults stand if unanswered) | Task 6 review (built) | — | — |
| 1 | Task 1 (Rust: renames + gate; regen #1) | Task 7 (C#: `PopoverOpening`; rebased onto Task 1's `NodeRecord` regen before it lands) | `contract` regen #1, re-bless #1; `heavy` | — |
| 2 | Task 2 (Rust: compiled labels, element read; rebuild + regen #2) | Task 8 (Razor/JS: slider + map.js; disjoint files) | `contract` rebuild + regen #2 (×2, A then B), re-bless #2; `heavy` | `GeneratedUsageTests` (`Element`, `NodeElement`, `EdgeElement`, `MissingElement`, `ElementPage`, `EdgeRecord`, `EdgeRef.Kind`, `EdgeRef.Label`) until Task 3 |
| 3 | Task 3 (C#: A-EDGES client) | Task 4 (Rust: geography wire; its rebuild and regen wait for Task 2's to land) | `contract` rebuild + regen #3 (Task 4) | `GeneratedUsageTests` for the Task-4 members |
| 4 | Task 5 (C#: `Explore/`) | Task 10's server half written, not regenerated | — | `GeneratedUsageTests` (`*.Node` on map rows) |
| 5 | Task 9 (World view) | — | — | Playwright: every spec in Task 11 Step 1's list; `client.ContractTests` unchanged |
| 6 | Task 10 (deletions; server regen) | — | `contract` regen #4, re-bless #4; `heavy` | Playwright as wave 5 |
| 7 | Task 11 (re-express, gates, push) | — | `heavy` ("mutation"); `land` | only `world-quiet-places:211`, `world-cluster-chooser:213` |

**Critical path:** OPEN answers → Task 1 → Task 2 → Task 3 → Task 5 → Task 9 → Task 10 → Task 11. Tasks 4, 6, 7, 8 ride beside it and must land before the primary that consumes them (4 before 5; 6, 7 and 8 before 9).

## Self-review against the spec, rule 27 and the brief

- **Rule 27, by derivation:** data-only derivations are compiled — node and edge labels (Task 2), a polity's reign and a place's default blurb (Task 4), edge-kind display labels in the vocabulary document (Task 2). Per-request derivations are bounded index reads — the element read (one lookup per id, capped), the neighbour read at an edge's position. Interaction derivations are the client's — `ElementKind`, an edge's ends as `Link`s, `Follow`, Back, `Presentation`, `Frame`, `Emphasis`, `Crossing` (Tasks 3, 5). Nothing derived per request that the data alone determines is added; F-36 and OPEN 13 name the ones that remain.
- **Rule 27, the graph models the domain:** no relation is appended (the count law stays 22); no field or element exists for a client construct (an edge's ends are its own subject and object; the edge's frontier is assembled on the client). The withdrawn `EdgeSource`/`EdgeTarget`, `/api/edge/*`, `EdgeEnd` and per-request edge labels are listed as never built.
- **Rule 27a/27c:** one generic element read answers nodes and edges alike, many per call; every neighbour carries id, kind and compiled label (`EdgeRef` gains `kind` and `label`); resuming a save is one request.
- **F-33's closure (rule 27's last sentence):** the gate fails the build on any client word in `server/` or `graph-types/` identifiers, comments, doc comments and paths, and in the published contract; OPEN 6 and 9 settle its reach.
- **Coverage:** R18/A-EDGES (Tasks 2–3; F-21 closed by the element read, F1-12's filter retired, `Follow` total by a law over every served position); R16 rows for Map/Place/Polity/Era on `Surface.World` and for every edge kind (Tasks 3, 5); R10 bounded slider and `follows-in` crossing (Tasks 5, 8, 9); R14 polity focus (Tasks 4, 5, 8, 9); R9 home surface navigation (Task 9); §5 FOCUS-6 deletions (Task 10) with `/api/scene` deliberately left to MAPS; F-22/F-23/F-28/F-29 closed where this batch touches them.
- **The MAPS seam** is `IMapSource` (+ the renderer beneath the World view); nothing in `Presentation` names map data.
- **Principle 25:** every window, reign, date and label is served (Tasks 2, 4); the client compares served years only in `Crossing`, reads the served dual for an edge's ends, and takes ids from served refs, never composed.
- **Type consistency:** `ElementKind`/`Link`/`Entry`/`PositionRef` (Task 3) are what Tasks 5, 7, 9 use; `NodeRecord` (Task 1) is what Tasks 2–5 extend and read; `Frame`/`Emphasis` (Task 5) are what Tasks 8–9 consume; `IMapSource` (Task 6) is what Task 9 calls; `PopoverOpening` (Task 7) is what Task 9 and `MentionScan` open with.
- **Assumptions to verify at execution:** `PlacePage`/`History` have no reader outside `/api/place` (Task 10 lets `cargo build` decide); a bUnit harness exists for `TimeSlider` (else Task 8's slider tests are Playwright); the verse→attests→event fixture in Task 3's spec uses an edge the served graph has (take it from a live `attests` page, not a literal); `crate::query::Contract` reads a repeated `id` parameter into `Vec<String>` (else Task 2 extends it, with a test, rather than parsing a query map by hand); `client.ContractGenerator` generates a three-case tagged union as it does `PositionRef`; a place's default period is a compiled fact (else Task 4 moves its choice into the compiler, reported as a rule-27 category); the in-memory `version_of` is unaffected by `PositionKind::Version` in the `canon-ids` build.
