# FOCUS-2 (TextUnit: Verse, ConcordUnit) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move TextUnit (a verse, a Concord paragraph) off the legacy popover onto FocusView and `Presentation`, and delete what spec §5 lists for it: `VerseTextSectionProvider`, `CrossRefsSection`, `CatechismSeamSection`, `VerseParallelsSection`, `VerseEventMembershipSection`, `VersePassageMembershipSection`, `VersePersonsSection` (dead, F-64), `ConcordUnitTextSection`, `ConcordSmallCatechismSection`, `VerseNode`, `ConcordUnitNode`, `/api/verse`, and, by OPEN 1, `/api/xrefs` and `/api/catechism/{sref}` with their `AtlasClient` methods. Fold in what the move closes: F-65 (the client parses reference strings), F-63's verse-side whole reads, F-45 (a live popover offers an edge step), F-42 where text-unit references are composed per request, F-64, F-14, and O-VERSE-LABEL.

**Architecture:** Rule 27 splits the work by who knows the inputs. The **compiler** writes each text unit's reference (`JHN.3.16`, `BoC 7.2.1`) beside the label it already writes, and labels a citation by the passage it cites. The **server** reads: a text unit's record carries its text (`NodeRecord.text: UnitText`), every text row names its node (`TextUnit.node`), and the anchors' character spans come from the compiled token table, not from re-tokenizing the text per request. The **client** derives the interaction: the popover row for TextUnit becomes `Form.Text`, `GraphPresenter` builds `Presentation.Text` from the served `UnitText`, FocusView renders the text with each served anchor as a `Link` followed through `Explore`, and every host opens a verse or a paragraph as `PopoverOpening.Explore` on a served position. Nothing new is appended to `relations!`; nothing in the graph exists for a client construct.

**Tech Stack:** Rust (axum, utoipa, `atlas-contract`, `atlas-graph`, `graph-types`); .NET 10 Blazor WebAssembly (`BibleAtlas.Client`, `BibleAtlas.Client.Exploring`); xUnit + bUnit; Playwright; Stryker.NET; cargo-mutants via `scripts/mutants-parallel.sh`.

**Spec:** `docs/superpowers/specs/2026-09-26-focus-exploration-design.md` §3.3 (the `Text` form), §5 (FOCUS-2 row), §6 (test ids), §9 (deletions), §12 R15–R20. **Principles:** 4, 9, 12, 24–24b, 25, 26, 26a, 27–27g (rule 27 is on `worktree-bible-atlas-m1`'s `docs/PRINCIPLES.md`, not yet on `F1-int`; Task 5 Step 0 notes it). **Queue:** FOCUS-2; closes F-14, F-45, F-64; closes the FOCUS-2 sites of F-42, F-63, F-65 and ratchets the rest; O-VERSE-LABEL.

**Base:** `origin/lane/claude/F1-int` at `caddd75` (FOCUS-1 + FOCUS-6 + the F1 fix wave + F-62 + R19 + O-GOLDEN-R4 + R20), in Codex's re-review. Every gate in this batch takes `--base caddd75` (PRINCIPLES 22). If F1-int lands with fixes, rebase onto the landed head and write the new base into the ledger (`.superpowers/sdd/2026-10-02-focus2/progress.md`) at Task 5.

## OPEN: for the owner, before the task named starts (each blocks only that task)

**Owner answers, 2026-10-02 (binding; they supersede the defaults below):**
1. **(a)** keep the passage path (`PassageNode`) until FOCUS-3.
2. **Drop PARALLELS on a verse only** (`VerseParallelsSection`). An event's own parallel accounts (`attested-in`, `EventAccounts`) stay; they move with Event in FOCUS-4/5.
3. **Events only.** A verse shows the events it attests; no passage-membership group ("the passage you can leave off").
4. **(b)** a "read in context" hatch on FocusView for a verse.
5. **Remove the cross-reference marker** — it duplicates opening the verse.
6. **(a) now; (b) in FOCUS-3** — the label names the span; minting a passage per cited span waits for FOCUS-3.
7. **(a)** rule 27 reaches every served site that composes a text-unit reference (all `encode_node_id` call sites), in FOCUS-2.
8. **(a)** keep the locator ("JHN.3.16").
9. **(a)** accept the AQC major; `TextUnit` nests the served `UnitText`.

1. **A reader's verse range (Tasks 4, 6).** Five of the nine providers also serve `PassageNode` (`AppliesTo: node.Kind is "Verse" or "Passage"`, `PopoverSectionProviders.cs:153, 273, 368, 927, 1184`). A passage's identity is its first verse's TextUnit (F1-3); it has no node (F-13), and it reads `/api/xrefs/{sref}` and `/api/catechism/{sref}` (`PassageNode.cs:41, 43`). (a) Keep the passage path until FOCUS-3: three providers narrow to `"Passage"` and are renamed `PassageTextSection`, `PassageCrossRefsSection`, `PassageCatechismSection`; `/api/xrefs/{sref}`, `/api/catechism/{sref}`, `AtlasClient.Xrefs` and `AtlasClient.Catechism` move to FOCUS-3's §5 row. (b) A range opens its first verse on FocusView now, and the passage path dies in FOCUS-2. That loses the range's text, its cross references gathered over the range, and its catechism.
2. **Parallels (Task 4).** `VerseParallelsSection` works out on the client each event's other accounts: one whole `attested-in` read per event (F-63, 27c, 27e). (a) The section dies. A verse's parallels are one step away: its `attests` event, whose `attested-in` group lists every account with its served runs. (b) Keep a PARALLELS section on FocusView as an `Explore` walk over the first page of each event, at one read per event, which breaks 27c.
3. **Events and passages a verse belongs to (Task 3).** The legacy popover splits the verse's `attests` neighbours into EVENT and PASSAGE by `VerseEvent.kind` from `/api/verse`. (a) One `attests` group, in served order. (b) Two groups split by each neighbour's served `event.kind`, at one batched element read per page (27c).
4. **"Read in context" and "About this book" on a verse (Task 4).** (a) FocusView offers neither. The verse's `member-of` crumb opens its chapter, which stays legacy until FOCUS-3, and the chapter's chips carry `popover-chip-context` and `popover-chip-book`. (b) FocusView gets a reader hatch for a text unit (`popover-chip-context`), like `popover-chip-map` for the world, built from the served `UnitText.locus`.
5. **The cross-reference marker (Task 4).** `VerseNode(xrefEntryPoint: true)` puts the cross references first and unclamped. (a) The marker opens the verse like any other click. (b) It opens the verse with the `cites` group revealed. That needs a reveal hint on `PopoverOpening.Explore`, a new type for sign-off.
6. **A cross reference to a span on FocusView (Task 2, F-65).** The `cites` edge ends at the span's first verse; the row keeps its last (`CrossRef.to_last`, `graph-types/src/edge.rs:440`). (a) The compiler labels the edge by the passage it cites (`GEN.1.1 · Cites · GEN.29.32-GEN.30.24`). The list's link reads its neighbour (the first verse); the edge step and the edge's card read the span. (b) The ETL mints a passage Container for each cited span and `cites` ends at it, so the graph models the citation and the list reads the span. This changes a row family and is F-13's mechanism, which FOCUS-3 would reuse.
7. **How far rule 27 reaches into the text read (Task 1).** (a) Compile each text unit's reference and read it at every served site: `TextUnit.ref`, `TextWindow.next`, and the text-unit arm of `encode_node_id`, which has 20 call sites (`dot_ref` and the `BoC` format leave served code). Read each anchor's characters from the compiled `kjv_token`/`concord_token` tables. (b) Only the two text-window sites and the anchors now; `encode_node_id` waits for F-34. (c) None now; F-42 stays whole for FOCUS-3.
8. **O-VERSE-LABEL (queue 21c).** (a) A verse's label stays its locator (`JHN.3.16`). (b) `John 3:16`, compiled. The book names live in code today (`atlas-core/src/canon.rs`, F-1), so (b) moves them into `data/` first (rule 26).
9. **`TextUnit` gains `node` and nests its text in `body: UnitText` (Task 1).** The semver gate classes the reshape as an **AQC major**. (a) Accept the major: one `UnitText` type, read on `/api/text` and on the record. (b) Keep `TextUnit` flat and restate the four fields in `UnitText`, which is a D.R.Y. finding (14b).

Defaults this plan builds if unanswered: (1) a; (2) a; (3) a; (4) a; (5) a; (6) a; (7) a; (8) a; (9) a.

## Global Constraints

- `docs/PRINCIPLES.md` binds, especially:
  - rule 4: zero dead code.
  - rule 9: no comments in application code. No snippet below has one; a comment found on a touched line is deleted, never reworded.
  - rule 12: every signature below is for sign-off.
  - rule 21: the critical sections, below.
  - rules 24, 24a, 24b: every fix names its category and closes it; an offender found on the side goes to FINDINGS and is never fixed on the side.
  - rule 25: the client composes over the contract; it never parses a reference, composes an id, formats a label or reorders served text.
  - rules 26, 26a: the server reads facts from the artifact; source shapes stay in the tools.
  - **rule 27**: data-only derivations are compiled, per-request ones are indexed reads, interaction ones are the client's; the graph models the domain, never a view.
- Tests:
  - whole-body assertions;
  - one behaviour per test, named as a sentence;
  - `// Arrange` `// Act` `// Assert` only;
  - no magic numbers;
  - newspaper order.
  - Real-data expectations are read from the artifact, never written as literals (F-8).
  - A test name in `server/` or `graph-types/` is an identifier, so the vocabulary gate reads it.
- **Total matches.** A new closed sum exposes `Match<T>`. `ElementKindLawTests` already fails a `switch` over `ElementKind`, `Frame`, `Emphasis` or `PopoverOpening`; `Presentation` is matched by its `Form` enum (exhaustive switch, build error on a missing arm).
- **The one walk door.** Every follow from FocusView goes `OnFollow(Link)` → `ExplorerPopover.FollowAsync` → `Explore.Follow`. No new caller of `IExplorer.Resolve` exists, and the compiler refuses one: `IExplorer.Resolve` is `internal` to `BibleAtlas.Client.Exploring` (R20).
- **No relation is appended.** `DECLARED_DIRECTED_RELATIONS` and `DECLARED_SYMMETRIC_RELATIONS` stay as they are at `caddd75`; the relation-count law is the guard.
- Build no interaction that works only by hovering.
- Commit per task. Push each task's branch to `origin/lane/claude/F2-<task>`, then integrate on `lane/claude/F2-int`. Landing on `worktree-bible-atlas-m1` is by cherry-pick, under `land`, after Codex's review. Never force.

## Critical sections (PRINCIPLES 21): one holder at a time

| Section | Held by | Why |
|---|---|---|
| `contract`: rebuilding `data/compiled` (moves the version root) | Task 1 Part A, then Task 2 | one artifact |
| `contract`: `export_contract`, `export_aqc_examples`, the AQC/AGC features, `client.ContractGenerator` | Task 1 Part B, then Task 6, strictly in that order | one generated document |
| `contract`: re-blessing pacts and fixtures | right after each rebuild or regen above | moves the version root |
| `heavy`: `cargo test --workspace`, full Playwright | Tasks 1, 2, 6, 7 | memory |
| `heavy` with "mutation" in the message | Task 7 only, inside the owner's window (`.superpowers/MUTATION-GATE-DEBT.md`) | once per batch (3a) |
| appending to `relations!` | **nobody** | rule 27 |

## Re-anchor table: what the verse and paragraph popovers touch at `caddd75`, and where each goes

| Today | Role | FOCUS-2 |
|---|---|---|
| `client/Legacy/VerseNode.cs` (55 lines) | verse identity, chips, `DetailAsync` over `/api/verse` | **deleted** (Task 4) |
| `client/Legacy/ConcordUnitNode.cs` (39 lines) | paragraph identity, `TextAsync` over `/api/text` | **deleted** (Task 4) |
| `VerseTextSectionProvider` (`verse-text`) | the verse in its chapter, mentions clickable, text provenance | verse arm → `Presentation.Text` (Tasks 3, 4); the passage arm is renamed `PassageTextSection` (OPEN 1a) |
| `CrossRefsSection` (`xrefs`) | cross references with previews, votes-ranked, provenance | verse arm → FocusView `cites` group (`Affordances.Cites`); passage arm → `PassageCrossRefsSection` (OPEN 1a) |
| `CatechismSeamSection` (`catechism`) | catechism items citing the verse | verse arm → FocusView `catechism-link` group; passage arm → `PassageCatechismSection` (OPEN 1a) |
| `VerseEventMembershipSection`, `VersePassageMembershipSection`, `EventMembershipHeading` | EVENT / PASSAGE rows | → FocusView `attests` group (OPEN 3a); **deleted** with `client.Tests/EventMembershipHeadingTests.cs` |
| `VerseParallelsSection` (`parallels`) | other accounts of the verse's events | **deleted** (OPEN 2a), for passages too |
| `VersePersonsSection` | registered nowhere (F-64) | **deleted** (Task 5) |
| `ConcordUnitTextSection` (`concord-unit-text`) | the paragraph's text | → `Presentation.Text` |
| `ConcordSmallCatechismSection` (`concord-small-catechism`), `CatechismLinks` (whole read, F-63) | catechism items a paragraph cites | → FocusView `catechism-link` group; `CatechismLinks` stays for `CatechismInConcordSection` (FOCUS-7), on the ratchet (Task 5) |
| `ExplorerPopover` `XrefEntryPoint`, `OtherContextSectionCount`'s `"verse-text"`/`"xrefs"` | section ordering for the marker | `XrefEntryPoint` **deleted** (OPEN 5a); `OtherContextSectionCount` keeps only what `PassageCrossRefsSection` reads |
| `client/Contracts/Frontier.cs` (`FocusKind`, `FrontierMatrix`, nine `IHas*` markers) | no production reader; mirrors the `graph-types/src/frontier.rs` that FOCUS-6 deleted | **deleted** with `FrontierMatrixConformanceTests.cs`, `FrontierMatrixRustParityTests.cs`, `tests/ux/frontier-matrix.spec.ts` (Task 5; F-14) |
| `client/Legacy/LegacyNodes.cs` `NodeKind.TextUnit` arm | bridge | → `null` (Task 4) |
| hosts building `new VerseNode(...)`: `VerseLine.razor:44,114`, `Reader.razor:429`, `Kretzmann.razor:573`, `MiniReaderExpand.razor:38`, `ScriptureRefText.razor:34`; `new ConcordUnitNode(...)`: `Concord.razor:381` | open a verse or a paragraph | `PopoverOpening.Explore(new NodePosition(unit.Node))` from the served `TextUnit.node` or `Anchor.node` (Task 4) |
| hosts holding only a served reference string: `PersonMentionsList.razor:27`, `PopoverSectionProviders.cs:859` (event mentions), `:1508` (eternal grounds), `ArrowNav.razor:138`, `PassageList.razor:168,172`, `ConcordUnitList.razor:11` | open a verse or a paragraph from a legacy payload | `LegacyTextUnits.Opening(servedRef)`, the one place a text-unit id is composed (F-31), on the ratchet with each caller's retiring batch (Task 4) |
| `client/AtlasClient.cs` `Verse` | `/api/verse` | **deleted** (Task 4) |
| `server/atlas-contract/src/reading.rs` `verse`, `first_verse_of_target`; `wire/reading.rs` `VerseDetail`, `VerseEvent` | the legacy route | **deleted** (Task 6) |
| `reading.rs` `xrefs`, `catechism.rs` `catechism_for_span`; `wire::CrossRef`, `wire::CatechismRef` | range routes read only by `PassageNode` | stay for FOCUS-3 (OPEN 1a) |
| `graph.rs` `bible_text_units`, `concord_text_units`, `anchors_over`, `unit_at`; `atlas_graph::node_ref::encode_node_id` | references composed per request (`dot_ref`, `format!("BoC …")`), anchors re-tokenized per request | read the compiled reference and the compiled token offsets (Task 1) |
| `server/atlas-graph/src/labels.rs` `edge_label` | `{subject} · {kind} · {object}` | a citation of a span is labelled by the span (Task 2) |

## Types (for sign-off, PRINCIPLES 12)

### The compiled reference and the text on the record (Task 1)

Graph (`graph-types`):
```rust
pub trait GraphQuery {
    fn references(&self, units: &[AnyNodeId]) -> Vec<Option<String>>;
}

pub struct Graph {
    pub references: BTreeMap<AnyNodeId, String>,
}
```
- A text unit's reference is a fact over the data alone (book code, chapter, verse; part, article, paragraph), so the compiler writes it (rule 27). `references` answers one index lookup per unit (27b): `Graph` from `references`, built at index time; `SqliteSnapshot` from a new `reference TEXT NOT NULL` column on the existing `verse` and `concord_unit` tables (`server/atlas-graph/src/sqlite/extras.rs`), keyed by `node_id`.
- A node that is not a text unit answers `None`. A text unit the compile left without a reference is a compile defect: the store law below fails.

Compiler (`server/atlas-graph/src/references.rs`, compile side only):
```rust
pub fn unit_reference(id: &AnyNodeId) -> Option<String>;
pub fn compile(graph: &mut Graph);
```
- `unit_reference` is the one reference function. A verse reads `kjv_adapter::dot_ref`, a Concord paragraph `ConcordTag::cite`. `labels::node_label`'s two text-unit arms call it, so a verse's label and its reference stay one derivation while O-VERSE-LABEL (OPEN 8) keeps them equal.
- `atlas_graph::build` calls `references::compile` before `labels::compile`. The writer fills the two `reference` columns from `Graph.references`.

Served id encoding (`server/atlas-graph/src/node_ref.rs`, OPEN 7a):
```rust
pub fn encode_node_id(id: &AnyNodeId, query: &impl GraphQuery) -> Result<String, UnreferencedUnit>;
pub struct UnreferencedUnit(pub AnyNodeId);
```
- The text-unit arm writes `text-unit:{compiled reference}`; every other kind keeps `{Kind}:{raw}`. **The id's bytes do not change**, so no fixture, pact or version root moves on account of the encoding. The 20 call sites thread the snapshot they already hold. `UnreferencedUnit` maps to `ApiError::internal` naming the unit.

Wire (`server/atlas-contract/src/wire/graph.rs`):
```rust
pub struct UnitText {
    pub locus: TextRef,
    pub text: String,
    pub words_of_christ: Vec<WordsOfChristSpan>,
    pub anchors: Vec<Anchor>,
}

pub struct TextUnit {
    pub r#ref: String,
    pub node: NodeRef,
    pub body: UnitText,
    pub heading: Option<UnitHeading>,
    pub edge_summary: Vec<EdgeSummaryEntry>,
}
```
Added field: `NodeRecord.text: Option<UnitText>`, present exactly for a text unit.
- `UnitText` is what a text unit **is**: where it stands, its words, the words of Christ in it, and what its words stand for. `/api/text` and the record read it through **one** builder:
  ```rust
  pub fn unit_text(graph: &GraphService, snap: &impl GraphQuery, units: &[AnyNodeId]) -> Result<Vec<wire::UnitText>, ApiError>;
  ```
  `bible_text_units` and `concord_text_units` call it, and `read_node_record` calls it for one id. It answers one index read per unit (the rendering in the node payload, the red-letter rows, the anchor rows and their token offsets, the anchors' compiled labels), and the anchors of a window come in one batched read (27c).
- `TextUnit.node` is the unit's own reference with its compiled label, so a reader row opens without composing an id (rule 25; the FOCUS-6 closure "rows the client cannot open", extended to text). `TextUnit.ref` and `TextWindow.next` read the compiled reference (OPEN 7a). The `ref` that restates `node.id`'s local part is a FINDING for FOCUS-3, which owns the reading window's addressing.
- Anchors: `anchors_over` stops re-tokenizing the text. It reads each anchor's first and last token's `char_start`/`char_end` from the compiled `kjv_token` or `concord_token` rows (`extras.rs:71`), keyed by unit and ordinal, so the server does no per-request derivation (OPEN 7a). A token span past the unit's compiled tokens stays an `ApiError::internal` naming the unit, never a panic. The `panic!` at `graph.rs` `anchors_over` is replaced, because a served read must not abort the process.

### A citation is labelled by the passage it cites (Task 2, F-65, OPEN 6a)

Compiler (`server/atlas-graph/src/labels.rs`):
```rust
pub fn edge_label(kind: EdgeKind, subject: &str, object: &str) -> String;
fn object_label(record: &EdgeRecord, graph: &Graph, labels: &BTreeMap<Position, String>) -> Option<String>;
```
- `object_label` is the object's compiled label, except for a `cites` edge whose row records a span (`to_last` is `Some`), where it is the row's `target_display`. `target_display` is data the ETL already canonicalizes (`xref_adapter.rs`, `citations.rs`); the served system never parses it.
- The edge id is `(relation, from, to)`, so two rows from one verse to one first verse with different `to_last` would be one edge with two spans. The compile law `every_citation_edge_records_one_span` fails if any exist; Task 2 Step 2 records the count at `caddd75`.

### The client (Tasks 3–5)

`client/Exploring/Presentation.cs`, `client/Exploring/Presenter.cs`:
```csharp
public sealed record Presentation.Text(UnitText Unit, IReadOnlyList<Field> Fields) : Presentation;
```
- The table row (R16) changes once: `Presentation.Of(ElementKind.Node(NodeKind.TextUnit), Surface.Popover)` is `Form.Text` (it was `Form.Card`). `Surface.Reader` stays `Form.Text` and `Surface.World` stays `null`, so `HomeSurfaces.Of` (TextUnit → Reader) is unchanged.
- §3.3's signed `Text(Locus Locus, string Body, IReadOnlyList<Anchor> Anchors)` is replaced by the served `UnitText`, which carries the same three facts plus the words of Christ. A client record restating them field by field would be a D.R.Y. hit (14b), and rule 25 says the client composes over the contract types. **This amends §3.3 and is for sign-off.**
- `GraphPresenter.PresentAs`: `Form.Text => TextOf(element)`, with `Fields = [Provenance]` read from the served record. A text unit served without `text` is a `ContractBreach` naming the unit, never a silent card. `Form.Sequence` still answers `CardOf` until FOCUS-3.

`client/Views/UnitTextView.razor` (new component):
```csharp
[Parameter, EditorRequired] public UnitText Unit { get; set; }
[Parameter, EditorRequired] public string Handle { get; set; }
[Parameter, EditorRequired] public EventCallback<Link> OnFollow { get; set; }
```
- It renders `AnchoredText.Runs(Unit.Text, Unit.WordsOfChrist, Unit.Anchors)`. A run of the words of Christ is wrapped in `.words-of-christ`. An anchored piece is a button that follows `new Link(anchor.Kind, new NodePosition(anchor.Node))`; the kind is the served anchor's own (`mentions` on a verse, `cites` on a Concord paragraph), never assumed.
- Test ids: `{Handle}-text` on the container; `{Handle}-anchor-{kind wire name}-{node id}-{n}` on the n-th anchor (n counts from 0 in served order).
- FocusView's body gains one arm, `presented.Presentation is Presentation.Text text`: `<UnitTextView Unit="text.Unit" Handle="handle" OnFollow="OnFollow" />` followed by the same field list the card renders (`{handle}-field-{name}`). The card and the text share one `Fields` fragment.

`client/Legacy/LegacyTextUnits.cs` (Task 4; the one bridge, F-31):
```csharp
public static class LegacyTextUnits
{
    public static PopoverOpening Opening(string servedRef);
}
```
- It returns `new PopoverOpening.Explore(new NodePosition(new NodeRef(id: NodeIds.Of(NodeKind.TextUnit, servedRef), kind: NodeKind.TextUnit, label: servedRef)))`. This is the only client site that composes a text-unit id, and it exists only for legacy payloads that carry a reference string and no `NodeRef`. Each caller is listed on the Task 5 ratchet with its retiring batch, and the bridge dies with its last caller.

`client.Tests` laws (Task 5):
```csharp
public sealed class ReferenceParsingLawTests
{
    private static readonly IReadOnlyDictionary<string, string> RetiredBy;
    [Fact] public void No_client_source_parses_or_composes_a_reference_outside_its_listed_sites();
    [Fact] public void Every_listed_site_still_parses_or_composes_a_reference();
}

public sealed class WholeReadLawTests
{
    private static readonly IReadOnlyDictionary<string, string> RetiredBy;
    [Fact] public void No_client_source_reads_a_whole_collection_outside_its_listed_sites();
    [Fact] public void Every_listed_site_still_reads_a_whole_collection();
}
```
- `ReferenceParsingLawTests` scans `client/**/*.cs` and `*.razor` for `CanonRef.`, `NodeIds.LocalPart(` and `NodeIds.Of(`. `RetiredBy` maps each allowed file to the batch that removes it (`"Legacy/PassageNode.cs" → "FOCUS-3"`, `"Components/ArrowNav.razor" → "FOCUS-5"`, …). Its pair law fails when a listed file no longer offends, so the list can only shrink.
- `WholeReadLawTests` does the same for `Paging.Whole(` and `OffersAll="true"`.
- This is a **ratchet**, and the report says so: from the batch's first commit, no new site can be written. The sites FOCUS-2 owns die in Task 4 and leave the list. The rest are enumerated with their batch, and the category is fully closed (24b) when the list is empty.

## Deletion inventory (FOCUS-2 total, with OPEN defaults)

- **Server, Task 6:**
  - `reading.rs` `verse` (`/api/verse/{vref}`) and `first_verse_of_target` (a served parser, rule 26a);
  - `wire/reading.rs` `VerseDetail` and `VerseEvent`;
  - their route registration, `api.rs`/`graph_api.rs`/`contract_api.rs`/`contract_coverage.rs`/`contract_pact.rs` cases, pact interactions, `contracts/atlas-graph-contract/graph/detail-routes.feature` and `transport/http.feature` lines, the `scripts/gate-selftest.sh` reference, and `server/BENCHMARKS.md`'s row.
- **Server, Task 1:** `dot_ref` and the `BoC` reference format in served code (`graph.rs` `bible_text_units`, `concord_text_units`, `unit_at`; `node_ref.rs`), now read compiled; `tokens::tokenize` and `tokens::chars_of` in `anchors_over`.
- **Client, files:**
  - `client/Legacy/VerseNode.cs`, `client/Legacy/ConcordUnitNode.cs`;
  - `client/Contracts/Frontier.cs`;
  - tests `client.Tests/EventMembershipHeadingTests.cs`, `client.Tests/FrontierMatrixConformanceTests.cs`, `client.Tests/FrontierMatrixRustParityTests.cs`;
  - `tests/ux/frontier-matrix.spec.ts`.
- **Client, members:**
  - `VerseEventMembershipSection`, `VersePassageMembershipSection`, `EventMembershipHeading`, `VerseParallelsSection` (and its `Slugify`), `VersePersonsSection`, `ConcordUnitTextSection`, `ConcordSmallCatechismSection`, and their `PopoverSectionRegistry` rows;
  - the verse arms of the three passage-kept providers (renamed per OPEN 1a);
  - `AtlasClient.Verse`;
  - `IPopoverSectionContext.XrefEntryPoint` and `ExplorerPopover.XrefEntryPoint` with the `_sections` reorder;
  - `LegacyNodes.For`'s `TextUnit` arm (→ `null`) and its private `TextUnit(string)`;
  - `CanonRef.TargetSpan`, whose last caller is the verse arm of `CrossRefsSection`; Task 4 verifies, and if the passage arm still calls it, it stays under the ratchet;
  - the `IdentityTests`/`LegacyNodesTests`/`PushViaConformanceTests`/`ChipTests`/`LegacyViews` rows for Verse and ConcordUnit;
  - `AsyncMemoTests`/`AsyncMemoConformanceTests` cases built on `VerseNode`, re-expressed on `PassageNode`;
  - the `Reader.razor`/`Kretzmann.razor` `OpensLegacy(node => node is VerseNode …)` guards (→ position equality through `PositionIdentity.Comparer`);
  - `app.css` rules used only by the deleted sections (`.popover-verse-text` stays while `PassageNode` uses it).
- **Under OPEN 1(b) only:** `PassageNode`, `PassageTextSection`, `PassageCrossRefsSection`, `PassageCatechismSection`, `VerseTextSection.razor`, `AtlasClient.Xrefs`/`Catechism`, `/api/xrefs/{sref}`, `/api/catechism/{sref}`, `wire::CrossRef`, `aggregate_span_xrefs`.
- **Not deleted** (stated so no one "finishes" it):
  - `/api/text` and `IExplorableClient.Reading` (Concord paging, FOCUS-3);
  - `/api/chapter`, `ChapterText`, `PassageBlock`, `PassageList` (FOCUS-3/5/7);
  - `CatechismList`, `CatechismLinks` (FOCUS-7);
  - `/api/node/{id}` and `IExplorableClient.Card` (F-34);
  - `LegacySaves`'s v1 `Verse`/`Passage` translations (FOCUS-9);
  - `drain_edges` (events, FOCUS-5);
  - `persons_at_verse` (`/api/chapter`, FOCUS-3).

---

### Task 1: The text read is compiled; the record serves a unit's text; every text row names its node

**Owner gate first:** OPEN 7, 8 and 9 (or the defaults), recorded in the ledger.

Two commits: Part A compiles the references and the anchors are read; Part B is the wire and its regen.

**Files (Part A):**
- Create:
  - `server/atlas-graph/src/references.rs`.
- Modify:
  - `graph-types/src/{graph,store}.rs` (`Graph.references`, `GraphQuery::references` on `Graph`, `MemSnapshot` and the law fakes);
  - `server/atlas-graph/src/{build,labels,node_ref,service}.rs`;
  - `server/atlas-graph/src/sqlite/{ddl,extras,writer,snapshot}.rs` (the two `reference` columns);
  - `server/atlas-contract/src/{graph,graph_wire,reading,contents,catechism,events,map}.rs` and `src/bins/export_aqc_examples.rs` (the 20 `encode_node_id` sites take the snapshot);
  - `server/atlas-cli/src/commands/{find,verse}.rs`;
  - `server/atlas-contract/tests/no_served_label_composition.rs` (the scan gains `dot_ref`, `tokenize(`, `chars_of(` and the file `server/atlas-graph/src/node_ref.rs`);
  - `data/compiled` (rebuilt), and the pacts the rebuild re-blesses.
- Test:
  - `graph-types/src/store.rs` laws;
  - `server/atlas-graph/tests/sqlite_laws.rs`;
  - `references.rs` unit tests;
  - `server/atlas-contract/tests/graph_api.rs`.

**Files (Part B):**
- Modify:
  - `server/atlas-contract/src/wire/graph.rs` (`UnitText`, `TextUnit.node`, `TextUnit.body`, `NodeRecord.text`);
  - `server/atlas-contract/src/graph.rs` (`unit_text`; `read_node_record` fills `text`);
  - `contracts/*` regen, `contracts/atlas-query-contract/CHANGELOG.md` (AQC major, OPEN 9);
  - the AQC features and `client.ContractTests/Steps/AqcSteps.cs` that read `TextUnit` fields;
  - pacts;
  - the client sites that read `TextUnit.{Locus,Text,WordsOfChrist,Anchors}` (→ `.Body.…`, mechanical, so the client keeps compiling): `client/Exploring/{ChapterText,KretzmannCitationScan}.cs`, `client/Legacy/{PassageBlock,PopoverSectionProviders,ConcordUnitNode}.cs`, `client/Components/{VerseLine,VerseTextSection,MiniReaderExpand}.razor`, `client/Pages/{Reader,Concord,Kretzmann}.razor`, `client.Tests/{ChapterTextTests,KretzmannCitationScanTests,AnchoredTextTests}.cs`, `client.Tests/Explore/ServedGraph.cs`.
- Test:
  - `server/atlas-contract/tests/graph_api.rs`;
  - `server/atlas-contract/tests/contract_generation.rs`.

- [ ] **Step 1 (A): Failing tests.**
  - Store law, both backends through `assert_answers_match`: `every_text_unit_answers_one_compiled_reference_and_no_other_node_answers_one`. 24b: a backend that answers differently fails.
  - `references.rs`: `a_verse_is_referenced_by_its_book_code_chapter_and_verse`; `a_concord_paragraph_is_referenced_by_its_part_article_and_paragraph`; `compiling_references_every_text_unit_the_graph_holds`.
  - `graph_api.rs`, every expectation read from the artifact (F-8):
    - `a_text_window_serves_each_units_compiled_reference_and_the_next_units`: one Bible chapter window and one Concord window; each `unit.ref` equals `references` read through the service for that unit; `next` equals the following unit's.
    - `a_text_unit_id_on_the_wire_carries_its_compiled_reference`.
    - `every_anchor_spans_the_characters_its_compiled_tokens_cover`: for a sample window, each anchor's `start`/`end` equals the `char_start` of its first token and the `char_end` of its last, read from `kjv_token`/`concord_token`.
  - `no_served_label_composition.rs`: `no_served_crate_composes_a_label_or_a_reference` (the scan gains `dot_ref`, `tokenize(`, `chars_of(`, and reads `node_ref.rs`); `a_reference_composer_named_in_code_is_found`.
- [ ] **Step 2 (A):** `cargo test -p atlas-graph -p atlas-contract` and `(cd graph-types && cargo test --all-features)` → red; record the offender count of the source law in the ledger.
- [ ] **Step 3 (A): Implement.**
  - `references::compile` fills `Graph.references` at the end of `build`, before `labels::compile`; the writer writes the two columns; `SqliteSnapshot::references` is one primary-key lookup per unit.
  - `labels::node_label`'s text-unit arms call `references::unit_reference`.
  - `encode_node_id` reads `query.references`.
  - `bible_text_units`/`concord_text_units` and both `unit_at`s read `references`; `anchors_over` reads the compiled token offsets.
  - A text unit with no compiled reference is `ApiError::internal` naming it.
- [ ] **Step 4 (A): Rebuild (critical section `contract`):** rebuild `data/compiled` and re-bless. Expect the version root to move (a new column), with no id and no served JSON changed; `export_contract --check` stays clean. Release the lock.
- [ ] **Step 5 (A):** `cargo test -p atlas-graph -p atlas-contract -p atlas-cli`, graph-types, `bash scripts/contract-gate.sh --base caddd75` → green. **Commit:** `text: every text unit's reference is compiled and read, and anchors read their compiled characters; nothing in a text read is composed per request (rule 27; F-42)`.
- [ ] **Step 6 (B): Failing tests** (`graph_api.rs`):
  - `a_text_units_record_serves_its_text_as_the_text_window_does`: for a verse and for a Concord paragraph, the `text` of `/api/elements?ids=` equals that unit's `body` in `/api/text`, whole `UnitText`.
  - `no_record_but_a_text_units_carries_text`: walks one node of every `NodeKind` the artifact holds.
  - `every_text_row_names_its_node`: walks one Bible chapter and one Concord window; each `unit.node` resolves through `/api/elements` to a `NodeElement` with that id and label. This is the 24b closure for "text rows the client cannot open".
  - `contract_generation.rs`: the regenerated document holds `UnitText` and `TextUnit.node`.
- [ ] **Step 7 (B):** `cargo test -p atlas-contract --test graph_api text` → red.
- [ ] **Step 8 (B): Implement** `UnitText`, `TextUnit { r#ref, node, body, heading, edge_summary }`, `NodeRecord.text`, and the one `unit_text` builder that both reads call.
- [ ] **Step 9 (B): Regenerate (critical section `contract`):**
  - `cargo run -p atlas-contract --bin export_contract`, `--bin export_aqc_examples`, `--check` clean;
  - `CHANGELOG.md` AQC **major** (OPEN 9);
  - re-bless pacts;
  - `dotnet run --project client.ContractGenerator`;
  - the mechanical `.Body.` client edits.
  - Release the lock.
- [ ] **Step 10 (B, lock `heavy`):** `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh --base caddd75`, `bash scripts/contract-semver-gate.sh` (declared major), `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green except the expected reds below. Run `npx playwright test tests/ux --grep "reader|concord|kretzmann"` → the same set as at `caddd75`, since the reshape changes no behaviour. **Commit:** `text: a text unit's record serves its text, one UnitText read on /api/text too, and every text row names its node (rule 25, 27c)`.

**Expected red after Task 1 until Tasks 3 and 4:** `client.ContractTests` `GeneratedUsageTests` for `NodeRecord.Text` (Task 3) and `TextUnit.Node` (Task 4).

### Task 2: A citation is labelled by the passage it cites (F-65, server)

**Owner gate first:** OPEN 6 (or the default).

**Files:**
- Modify:
  - `server/atlas-graph/src/labels.rs` (`object_label`);
  - `server/atlas-graph/src/build.rs` (the compile law);
  - `data/compiled` (rebuilt), and pacts.
- Test:
  - `labels.rs` unit tests;
  - `server/atlas-graph/tests/sqlite_laws.rs`;
  - `server/atlas-contract/tests/graph_api.rs`.

- [ ] **Step 1: Failing tests.**
  - `labels.rs`: `a_citation_of_a_span_is_labelled_by_the_passage_it_cites` (whole string); `a_citation_of_one_verse_is_labelled_by_that_verse`.
  - `build.rs` compile law: `every_citation_edge_records_one_span`.
  - `graph_api.rs`, `the_edge_record_of_a_cross_reference_to_a_span_names_the_span`: it takes RUT 4:11's `cites` page from the artifact, finds an entry whose row records `to_last`, reads its edge through `/api/elements`, and asserts the label against the row's `target_display`, read from the artifact.
- [ ] **Step 2:** `cargo test -p atlas-graph labels` → red. Run the compile law once at `caddd75` and record in the ledger the number of edges carrying more than one span. If it is not zero, **stop**: the edge identity loses data, and the owner decides (a FINDING with the pairs).
- [ ] **Step 3: Implement** `object_label` as specified; `edge_label`'s signature is unchanged.
- [ ] **Step 4: Rebuild (critical section `contract`, after Task 1 Part A's):** rebuild `data/compiled`, re-bless. Labels change for span citations only. If `scene_byte_identity` or a golden fixture moves, the task stops for the owner (O-GOLDEN rule).
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh --base caddd75` → green. **Commit:** `labels: a citation of a span is labelled by the passage it cites, read from its row, never parsed (F-65)`.

### Task 3: `Presentation.Text`: a text unit on the popover is its served text

**Owner gate first:** OPEN 3 (or the default).

**Files:**
- Create:
  - `client/Views/UnitTextView.razor`.
- Modify:
  - `client/Exploring/Presentation.cs` (the `Text` record, the TextUnit popover row);
  - `client/Exploring/Presenter.cs` (`TextOf`);
  - `client/Views/FocusView.razor` (the text arm, the shared fields fragment).
- Test:
  - `client.Tests/Explore/PresentationTests.cs`;
  - `client.Tests/Explore/GraphPresenterTests.cs`;
  - `client.Tests/Views/FocusViewTests.cs`;
  - `client.Tests/Explore/ServedGraph.cs` (gains `UnitTextOf(...)` and `Resolved.Node(...).WithText(...)` builders).

- [ ] **Step 1: Failing tests.**

`client.Tests/Explore/GraphPresenterTests.cs`:
```csharp
[Fact]
public async Task A_verse_on_the_popover_presents_its_served_text_with_its_anchors_and_its_provenance()
{
    // Arrange
    var verse = await Resolved.Node(NodeKind.TextUnit, John316, John316Label).WithText(ServedGraph.UnitTextOf(John316Locus, John316Text, John316Anchors)).Using(new ServedGraph());
    // Act
    var presented = await new GraphPresenter().Present(new PresentationRequest(verse, Surface.Popover));
    // Assert
    Assert.Equal(new Presentation.Text(ServedGraph.UnitTextOf(John316Locus, John316Text, John316Anchors), [new Presentation.Field(ProvenanceField, KjvProvenance)]), presented);
}
```
Also in `GraphPresenterTests`: `A_concord_paragraph_on_the_popover_presents_its_served_text_with_its_citations`; `A_text_unit_served_without_its_text_is_a_contract_breach` (the message names the unit).

`PresentationTests`: the table test now expects `(TextUnit: World null, Reader Text, Popover Text)`; `Every_element_is_offered_on_the_popover` stays green.

`FocusViewTests`:
- `A_text_shows_its_served_words_with_each_anchor_as_a_link_of_its_served_kind` (whole rendered markup of `popover-text`).
- `Following_an_anchor_follows_a_link_of_the_anchors_kind_to_its_node`: the `OnFollow` argument is the whole `Link`.
- `The_words_of_Christ_are_marked_as_served`.
- `A_verse_offers_its_neighbouring_verses_as_arrows_its_chapter_as_a_crumb_and_its_other_neighbours_as_sections`: a served verse with `follows-in`, `precedes-in`, `member-of`, `cites`, `attests` and `catechism-link` groups; asserts the ordered test ids.
- `A_text_shows_its_fields_as_a_card_does`.
- [ ] **Step 2:** `dotnet test client.Tests --filter "GraphPresenterTests|PresentationTests|FocusViewTests"` → red or compile errors.
- [ ] **Step 3: Implement** the types exactly as above.
  - `PresentAs` stays an exhaustive switch on `Form`: `Form.Text => TextOf(element)`, with `element.Kind.Match(node: _ => …, edge: kind => throw new UnreachableException(…))`, since no edge kind has a `Text` row.
  - `UnitTextView` composes over `AnchoredText.Runs`: no offset arithmetic beyond the existing served-scalar-to-UTF-16 mapping, and no kind decided from a string.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green except `GeneratedUsageTests` for `TextUnit.Node` (Task 4).
- [ ] **Step 5: Commit:** `text: a text unit on the popover presents as its served text, with each anchor a link of its served kind (R16, §3.3 amended)`.

### Task 4: Text units open by position, and the legacy verse and paragraph path is gone

**Owner gate first:** OPEN 1, 2, 4 and 5 (or the defaults).

**Files:**
- Create:
  - `client/Legacy/LegacyTextUnits.cs`;
  - `client.Tests/Explore/LegacyTextUnitsTests.cs`.
- Modify:
  - `client/Components/{VerseLine,MiniReaderExpand,ScriptureRefText,PersonMentionsList,ConcordUnitList,PassageList,ArrowNav,ExplorerPopover}.razor`;
  - `client/Pages/{Reader,Kretzmann,Concord}.razor`;
  - `client/Legacy/{PopoverSectionProviders,PopoverSections,LegacyNodes,PassageNode}.cs`;
  - `client/AtlasClient.cs`;
  - `client/wwwroot/css/app.css`;
  - `client.Tests/Explore/{DeletionLawTests,IdentityTests,LegacyNodesTests,PushViaConformanceTests,ChipTests,LegacyViews}.cs`, `client.Tests/{AsyncMemoTests,AsyncMemoConformanceTests,PopoverSectionRegistryTests,ArrowNavTests}.cs`, `client.Tests/Explore/PopoverOpeningTests.cs`;
  - Task 5's two `RetiredBy` lists (entries removed).
- Delete:
  - `client/Legacy/VerseNode.cs`, `client/Legacy/ConcordUnitNode.cs`;
  - `client.Tests/EventMembershipHeadingTests.cs`.
- Test:
  - `client.Tests/Explore/DeletionLawTests.cs`, `LegacyTextUnitsTests.cs`;
  - `tests/ux/explore-edges.spec.ts` (live, F-45).

- [ ] **Step 1: Failing tests.**
  - `DeletionLawTests` adds `NodeKind.TextUnit` to `MigratedKinds`, and replaces the kind-name match with a declared `LegacyNames` (`Place → ["Place"]`, `Polity → ["Polity"]`, `Era → ["Era"]`, `Map → ["Map"]`, `TextUnit → ["Verse", "ConcordUnit"]`), with a pair law `Every_migrated_kind_names_its_legacy_names`, so a kind cannot be migrated without saying what it retired. The held list gains `PassageNode` ("FOCUS-3, OPEN 1a"), beside `PolityDeltaNode` ("MAPS"). Run → red: `VerseNode`, `ConcordUnitNode` and the `"Verse"`/`"ConcordUnit"` `AppliesTo` strings.
  - `LegacyTextUnitsTests.An_opening_on_a_served_reference_explores_that_text_unit` (whole `PopoverOpening`).
  - `explore-edges.spec.ts` gains `a verse opened from the reader offers a step onto each entry's edge, and the step opens the edge under its two ends` (F-45, live). It opens EXO 14:21 by clicking its reader line, takes the first `attests` entry's edge id from `api.nodeEdges`, clicks `popover-entry-edge-attests-{edgeId}`, and asserts the served edge label and both ends. No seeded save is used.
- [ ] **Step 2:** red (law, test, spec).
- [ ] **Step 3: Implement.**
  - Hosts with a served `TextUnit` or `Anchor` open `new PopoverOpening.Explore(new NodePosition(unit.Node))` or `new NodePosition(anchor.Node)`: `VerseLine` (click, Enter, and Ctrl/Cmd selection with `Unit.Node`), `Reader`/`Kretzmann` xref markers, `MiniReaderExpand`, `Concord`, and `ScriptureRefText`, whose `NodeIds.LocalPart` goes away.
  - Hosts with only a served reference string call `LegacyTextUnits.Opening`.
  - The re-click guards compare positions through `PositionIdentity.Comparer`.
  - `LegacyNodes.For` answers `null` for `TextUnit`, so every verse and paragraph, opened or followed, renders on FocusView.
  - Delete the files and members in the inventory, and rename the passage-kept providers (OPEN 1a). Their `AppliesTo` is `node.Kind == "Passage"` and they match `PassageNode` only.
  - `PassageCrossRefsSection`'s cap reads `OtherContextSectionCount` alone.
  - Delete `AtlasClient.Verse`.
  - Remove each deleted site from Task 5's `RetiredBy` lists. The pair laws prove each listed site still offends.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green (`GeneratedUsageTests` now reads `TextUnit.Node`). `npx playwright test tests/ux/explore-edges.spec.ts tests/ux/saved-explorations.spec.ts tests/ux/state-focus.spec.ts` → green. Record the expected-red Playwright set by name in the ledger.
- [ ] **Step 5: Commit:** `text: a verse or a paragraph opens and renders on FocusView from its served position; VerseNode, ConcordUnitNode and their seven sections are gone (deletion law: TextUnit; F-45, F-63, F-65)`.

### Task 5: Closures that need no regen: F-14, F-64, and the two ratchets

Runs first, beside Task 1 (C# beside Rust).

**Files:**
- Create:
  - `client.Tests/ReferenceParsingLawTests.cs`;
  - `client.Tests/WholeReadLawTests.cs`.
- Delete:
  - `client/Contracts/Frontier.cs`;
  - `client.Tests/FrontierMatrixConformanceTests.cs`, `client.Tests/FrontierMatrixRustParityTests.cs`;
  - `tests/ux/frontier-matrix.spec.ts`;
  - `VersePersonsSection` (`PopoverSectionProviders.cs:1182–1253`).

- [ ] **Step 0:** In the ledger, write the base and that rule 27 is read from `worktree-bible-atlas-m1`'s `docs/PRINCIPLES.md`; the doc reaches `F1-int` when F1-int lands. Then verify `client/Contracts/Frontier.cs` has no production reader: `grep -rn "FrontierMatrix\.\|FocusKinds\.\|IHas[A-Z]" client --include=*.cs --include=*.razor` outside `Contracts/Frontier.cs` must print nothing (it prints nothing at `caddd75`). If it prints anything, stop.
- [ ] **Step 1: Failing laws.** Write the two law classes with `RetiredBy` as the offending files at `caddd75`, each mapped to its batch. The verse-side entries are listed under `"FOCUS-2"`, so Task 4 must remove them. Then add a deliberate offender in a scratch copy, and confirm the law names it, before committing without it.
- [ ] **Step 2:** Delete F-14's four files and F-64's class; `dotnet test client.Tests` → green; `npx playwright test tests/ux --grep frontier` → no spec matches (deleted).
- [ ] **Step 3: Commit:** `client: the unread FrontierMatrix and VersePersonsSection are gone; parsing a reference and reading a whole collection are ratcheted to their listed sites (F-14, F-64, F-63, F-65)`.

### Task 6: `/api/verse` is gone

**Files:** the server half of the deletion inventory; `tests/ux/lib/api.ts` (`verse()`); `tests/ux/CONTRACT.md`.

- [ ] **Step 1: Failing test:** `contract_coverage.rs` `no_route_serves_a_verse_by_its_reference` asserts that the published document's path set holds no `/api/verse/{vref}`. Run → red.
- [ ] **Step 2: Delete** the handler, `first_verse_of_target`, `VerseDetail`, `VerseEvent`, the route, and every test, pact interaction, AGC feature line and script reference to them. `cargo build` decides whether `atlas_core::scene::to_scene_event` or `BookMeta` lose their last served reader (they do not at `caddd75`: `narrative.rs:22`, `graph.rs:264`).
- [ ] **Step 3: Regenerate (critical section `contract`, after Task 1 Part B's):** export, `--check`, AQC/AGC per `scripts/contract-semver-gate.sh` (a removed route), re-bless, client generator. Release the lock.
- [ ] **Step 4 (lock `heavy`):** `cargo test --workspace && (cd graph-types && cargo test --all-features) && dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh --base caddd75` → green, with the vocabulary gate and `no_served_label_composition` still green. **Commit:** `text: /api/verse is gone; a verse is read as an element and its neighbours (rule 27a)`.

### Task 7: Re-express the specs, gates, close

- [ ] **Step 1: Re-express** each expected-red Playwright spec under the new behaviour, by name, keeping every surviving test id. The candidates at `caddd75` are every spec reading `verse-text`, `xref*`, `catechism-section-heading`, `verse-event-*`, `verse-parallel*`, `concord-unit-text`, `concord-small-catechism*`, `popover-chip-book` or `popover-chip-context` from a verse or a paragraph:
  - `popover-sections`, `reader`, `reader-xref-anchoring`, `reader-xref-superscripts`, `reader-recursion`, `reader-headings`, `reader-red-letters`, `reader-map`;
  - `concord`, `kretzmann`, `provenance`, `accounts-and-mentions`, `event-timeplace`, `event-timeline`, `world-pin`;
  - `saved-explorations`, `state-focus`, `api-reader`;
  - `w1`…`w4-passages`;
  - `lib/api.ts`, `lib/verse.ts`, `tests/ux/CONTRACT.md`.

  The ledger keeps the actual red set from Task 4 Step 4. Test ids move:

  | Legacy id | FocusView id |
  |---|---|
  | `verse-text`, `popover-section-verse-text`, `concord-unit-text` | `popover-text` |
  | `verse-text-provenance` | `popover-field-Provenance` |
  | `popover-section-xrefs`, `xrefs-section-heading`, `xref-item-*`, `xrefs-more`, `xrefs-collapse` | `popover-section-cites`, `-heading`, `popover-link-cites-{id}`, `-more`, `-collapse` |
  | `popover-section-catechism`, `catechism-section-heading` (verse), `concord-small-catechism*` | `popover-section-catechism-link`, `popover-link-catechism-link-{id}` |
  | `verse-event-{id}`, `event-section-heading` (membership) | `popover-link-attests-{id}` (OPEN 3a) |
  | mention spans in the verse text | `popover-anchor-mentions-{id}-{n}` |

  A spec whose behaviour an OPEN default removed is rewritten to what FocusView offers, and the removal is listed in the close report. That covers PARALLELS (2a), the per-section provenance affordances, the verse-level chips (4a), and the marker's reorder (5a).
- [ ] **Step 2: Gates.**
  - `client.Tests/stryker-config.exploring.json` `mutate` covers `**/Exploring/Presentation.cs` and `**/Exploring/Presenter.cs` (verify both are listed). `client.Tests/stryker-config.json` gains `**/Legacy/LegacyTextUnits.cs`.
  - Mutation: inside the owner's window only (critical section `heavy` with "mutation" in the message, `free -g` ≥ 18 GB):
    1. `bash scripts/mutants-parallel.sh -n 3 -b caddd75` → 100% or equivalents recorded (covers `references`, `labels::object_label`, `unit_text`, the anchor read);
    2. then `dotnet stryker` → 100% or equivalents recorded.

    Outside the window, `.superpowers/MUTATION-GATE-DEBT.md` names `caddd75` as this batch's base.
  - Then `cargo test --workspace && (cd graph-types && cargo test --all-features) && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh --base caddd75 && bash scripts/timing-gates.sh run && npx playwright test tests/ux` → green except the carried reds (`world-quiet-places` density smoke, `world-cluster-chooser` C3-M1) and any F-55 flake named by spec.
- [ ] **Step 3:** Write the close report, `docs/superpowers/reports/2026-10-0x-focus-2-close.md`. It covers:
  - every rule-24 category with its closure and guarantee (below);
  - the FINDINGS raised;
  - the §5 amendment (OPEN 1a moves the two range routes to FOCUS-3's row) and the §3.3 amendment (`Text` over the served `UnitText`).
- [ ] **Step 4:** Commit; push the batch branch; hand to Codex for review (14b + 24a). Land under `land` after review.

**Rule-24 categories and their closures:**

| Category | Closure | Guarantee |
|---|---|---|
| Verse and paragraph served by both mechanisms | the deletion law (TextUnit) with `LegacyNames` | compiler + law |
| A text row the client cannot open | `TextUnit.node`, `every_text_row_names_its_node` | resolve-walk |
| References composed per request (F-42, FOCUS-2's sites) | compiled `reference`; `no_served_label_composition` over `dot_ref`; the store law | source law + store law |
| Anchors re-tokenized per request | compiled token offsets; `every_anchor_spans_the_characters_its_compiled_tokens_cover` | real-data law |
| The client parses a served reference (F-65) | verse sites deleted; `ReferenceParsingLawTests` ratchet; the citation label read from its row | ratchet, closed when the list is empty |
| A whole collection read on the client (F-63) | verse sites deleted; `WholeReadLawTests` ratchet | ratchet |
| No live popover offers an edge step (F-45) | the live `explore-edges` spec | Playwright |

---

## Wave schedule

Primary is the lane's critical-path work; the companion is unlike work paired beside it (rule 23: Rust beside C#). A task starts only when the tasks it names as inputs have landed on `lane/claude/F2-int`.

| Wave | Primary | Companion | Critical sections held | Expected red at wave close (the ledger records names) |
|---|---|---|---|---|
| 0 | owner: OPEN 1–9 (defaults stand if unanswered) | Task 5 (C#: laws, F-14, F-64) | — | — |
| 1 | Task 1 (Rust: compiled references and anchors, then the wire; rebuild #1, regen #1) | Task 5 lands; Task 3's tests written against `ServedGraph` | `contract` rebuild #1, regen #1, re-bless; `heavy` | `GeneratedUsageTests`: `NodeRecord.Text`, `TextUnit.Node` |
| 2 | Task 3 (C#: `Presentation.Text`, `UnitTextView`) | Task 2 (Rust: citation labels; rebuild #2) | `contract` rebuild #2, re-bless; `heavy` | `GeneratedUsageTests`: `TextUnit.Node` |
| 3 | Task 4 (C#: openings, deletions) | Task 6's server half written, not regenerated | — | Playwright: Task 7 Step 1's list (actual names in the ledger) |
| 4 | Task 6 (Rust: `/api/verse` deleted; regen #2) | Task 7 Step 1 (TypeScript: re-express) | `contract` regen #2, re-bless; `heavy` | Playwright as wave 3, shrinking as specs are re-expressed |
| 5 | Task 7 Steps 2–4 (gates, close, push) | — | `heavy` (mutation only in the owner's window); `land` after review | only the carried reds and named F-55 flakes |

**Critical path:** OPEN answers → Task 1 → Task 3 → Task 4 → Task 6 → Task 7. Task 5 rides beside Task 1 and must land before Task 4, which shrinks its lists. Task 2 rides beside Task 3 and must land before Task 7.

## FINDINGS this plan expects to raise (for the queue; the owner decides)

- **`TextUnit.ref` restates `TextUnit.node.id`'s local part** (D.R.Y., 14b).
  - Closure: FOCUS-3 addresses the reading window by position (`/api/text?from={id}`), and `ref` and `next` become `NodeRef`s.
- **Request decoding of a text-unit id parses book codes against the canon table in code** (`server/atlas-contract/src/reference.rs`; F-1 residue).
  - Closure: decoding is a lookup of the compiled reference (a UNIQUE index), and the canon moves to `data/` (F-1).
- **A cross reference's span is not in the graph's model** (F-65's remainder, OPEN 6b).
  - Closure: a cited span is a passage Container, and `cites` ends at it. FOCUS-3's titled passages (F-13) would reuse the mechanism.
- **The range routes parse references per request** (rule 26a): `/api/xrefs` (`aggregate_span_xrefs`, the in-memory `cross_refs_for_span`) and `/api/catechism/{sref}`.
  - Closure: they retire with `PassageNode` in FOCUS-3 (OPEN 1a), or now under OPEN 1b.
- **`drain_edges` reads whole collections in served code** (`limit: usize::MAX`, 27b): `events.rs` after `/api/verse` dies; also `persons_at_verse` (`/api/chapter`).
  - Closure: FOCUS-5 and FOCUS-3 read bounded pages.
- **Words-of-Christ spans are keyed by a dot-ref string built at load** (`serve.rs:84`, `red_letter_spans.rs`), not by unit id. Load-time, not per request, but a composed key.
  - Closure: key by unit node id.
- **Per-section provenance affordances are gone on FocusView** ("Sources for these cross references"). Provenance is per edge, on the edge's card. A UX loss the owner may want back as a served per-group provenance read.
- **`FrontierMatrixRustParityTests` transcribed a Rust file that FOCUS-6 deleted** (review gap). Closed here by deletion; the category is "a test pinned to a deleted source", proposed closure a law that every path a test names exists.
- **Pre-existing non-AAA comments in files this batch touches** (F-12 residue, e.g. `reading.rs`, `PopoverSectionProviders.cs`, `ExplorerPopover.razor`). Lines this batch deletes take theirs with them; no other comment is touched on the side, and each task's diff adds none (the review greps it).

## Self-review against rule 27, the spec and the brief

- **Data-only derivations are compiled:**
  - a text unit's reference (Task 1; it was `dot_ref` and `format!` per request);
  - an anchor's characters (Task 1; it was `tokenize` per request);
  - a citation's label (Task 2);
  - a verse's label stays compiled (FOCUS-6), with its text-unit arms now calling the one reference function.

  The text itself was already compiled into the node payload (`window::render` reads `renderings`, `window.rs:47`) and stays so.
- **Per-request derivations are bounded index reads:**
  - `unit_text` for one unit or one window (one lookup per unit, anchors batched per window, 27c);
  - the element read for a verse;
  - the neighbour read for each group.

  Nothing in the served path computes from the data alone.
- **Interaction derivations are the client's:** the TextUnit row on the popover, `Presentation.Text`, the anchor `Link`s, the arrows and crumbs from `Affordances.Of`, and every follow through `Explore`.
- **The graph models the domain, never a view:**
  - no relation appended (the count law);
  - no node, field or element added for a client construct (`UnitText` is a text unit's own content, and `TextUnit.node` its own reference);
  - the PARALLELS view and the EVENT/PASSAGE split are not built into the graph (OPEN 2, 3).
- **The client walks through `Explore`:** FocusView's `OnFollow` is the one door; anchors are `Link`s; no new `Resolve` caller.
- **Rule 25:** after Task 4 no client site parses or composes a text-unit reference except `LegacyTextUnits` and the files on the ratchet, each named with its batch.
- **27b/27c/27e:** the verse popover reads one element and one first page per group; no whole read remains on the verse path (`VerseParallelsSection`'s and `ConcordSmallCatechismSection`'s whole reads are deleted).
- **Coverage of §5's FOCUS-2 row:** every provider and node named is deleted (or narrowed to Passage under OPEN 1a, which amends the row); `/api/verse` is deleted; `/api/xrefs`, `/api/catechism/{sref}` follow OPEN 1.
- **Type consistency:** `UnitText` (Task 1) is what `Presentation.Text` (Task 3) holds and `UnitTextView` renders; `TextUnit.node` (Task 1) is what Task 4's hosts open; `LegacyTextUnits` (Task 4) is what Task 5's ratchet names; the passage-kept providers keep `PassageNode`'s contract unchanged.

## Assumptions

**Verified against `caddd75`:**
- The five verse providers also apply to `"Passage"` (`PopoverSectionProviders.cs:153, 273, 368, 927, 1184`). `PassageNode` reads `/api/xrefs` and `/api/catechism/{sref}` (`PassageNode.cs:41, 43`); `VerseParallelsSection` reads `/api/verse` for a passage (`:941`).
- `VersePersonsSection` is not in `PopoverSectionRegistry` (`PopoverSections.cs`).
- `client/Contracts/Frontier.cs` has no reader outside its own tests, and `graph-types/src/frontier.rs` no longer exists.
- A `cites` row holds `to`, `to_last` and `target_display` (`graph-types/src/edge.rs:440`). The neighbour page serves `loci` only for attestations and mentions (`graph.rs` `node_edges`). The edge label is `{subject} · {kind} · {object}` over compiled labels (`labels.rs:55`).
- `TextUnit` has no node reference and `NodeRecord` no text (`wire/graph.rs`).
- `TextUnit.ref` and `next` are composed per request with `dot_ref` and `format!("BoC …")` (`graph.rs` `bible_text_units`, `concord_text_units`). The text-unit wire id is composed the same way (`atlas-graph/src/node_ref.rs:5`).
- `no_served_label_composition.rs` does not list `dot_ref`.
- `anchors_over` re-tokenizes the text per request, while the artifact already holds `kjv_token` with `char_start`/`char_end` (`extras.rs:71`). A unit's text is a stored rendering (`window.rs:47`).
- `AtlasClient.Verse` is read only by `VerseNode` and `VerseParallelsSection`. Every `new VerseNode`/`new ConcordUnitNode` site is listed in the re-anchor table.
- `LegacyNodes.For` returns the legacy node for TextUnit; `DeletionLawTests` matches legacy classes by the kind's own name, which does not catch `VerseNode`. Hence `LegacyNames`.
- `Presentation.Of(TextUnit, Popover)` is `Card` and `GraphPresenter` answers `Text` with `CardOf`.
- `drain_edges` survives `/api/verse` (`events.rs`).

**To verify at execution (each with its fallback):**
- **`concord_token` holds character offsets like `kjv_token`.** If not, Task 1 reports and reads the Concord anchors as `/api/text` does today, with a FINDING.
- **The anchor rows key tokens by ordinal so a span's first and last token are one lookup each.** If not, Task 1 adds the index the read needs, with its law.
- **The client generator emits a nested `UnitText` on `TextUnit` and an optional `NodeRecord.Text`, as it does `PlaceDetail`.**
- **Threading the snapshot through the 20 `encode_node_id` sites touches no published byte** (`export_contract --check` clean after Part A).
- **No `cites` edge carries two spans** (Task 2 Step 2 stops otherwise).
- **`red_letter_span` offsets are characters** (`serve.rs:84` says so).
- **`PassageNode`'s behaviour is unchanged by the renames** (`npx playwright test tests/ux --grep "passage|w1|w2|w3|w4"` against `caddd75`'s set).
