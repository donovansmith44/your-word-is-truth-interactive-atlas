# FOCUS-3 (Container: Chapter, Book, Passage, BoC document/article) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Bible reader and the Book of Concord become one presentation of a Container: a scope (a Bible book, a BoC document) read as one continuous scroll, its chapter-role children (a chapter, an article) titled once and read through, arrows from the graph's `follows-in`/`precedes-in`, a remembered reading position, and one table of contents. Delete what spec §5 and §9 list for FOCUS-3: `ChapterCardSection`, `ChapterNode`, `BookNode`, `PassageNode`, `Reader.ComputeAdjacent`, `_toc`, `Concord.LoadWindowAsync`'s paging and trim, `/api/chapter`, `/api/books`; and what FOCUS-2's OPEN 1a and 6 hand on: `PassageTextSection`, `PassageCrossRefsSection`, `PassageCatechismSection`, `/api/xrefs/{sref}`, `/api/catechism/{sref}`, a passage Container per cited span. The 2026-09-20 notes (NAV-UNIFORM-1, ARTICLE-HEADER-1, PARA-NUM-1, COVER-HEADER-1, ENUM-CONSISTENCY-1, SIDENAV-PAGENUM-1, CONTINUOUS-SCROLL-1, PAGE-BOUNDARY-BUG-1) and rulings R8, R11, R12, R13 are its acceptance criteria.

**Architecture:** Rule 27 splits the work by who knows the inputs.
- **The compiler** writes what the data alone decides: each Concord paragraph's Triglotta citation as its label (R11), and one passage Container per cited span (FOCUS-2 OPEN 6b, as the owner ruled).
- **The server** reads. It gains one bounded, keyset-paged range read: the text units a container holds, `GET /api/node/{id}/text`, the same page shape as the neighbour read (27a "range reads by passage span", 27b). It loses `/api/chapter`, `/api/books`, `/api/xrefs/{sref}` and `/api/catechism/{sref}`. No relation is appended; no field exists for a view.
- **The client** derives the interaction: which container is scrolled (the focus's `member-of` scope, R12/R13), which chapter is the focus as you scroll, the arrows (`Affordances.Of(FollowsIn)`), the remembered position (R8, local storage), and the table of contents (R13). Every page and window goes through the one root-aware paging door (`PageWindow`, F-67/F-68/F-70/F-74).

**Tech Stack:** Rust (axum, utoipa, `atlas-contract`, `atlas-graph`, `atlas-etl`, `graph-types`); .NET 10 Blazor WebAssembly (`BibleAtlas.Client`, `BibleAtlas.Client.Exploring`); xUnit + bUnit; Playwright; Stryker.NET; cargo-mutants via `scripts/mutants-parallel.sh`.

**Spec:** `docs/superpowers/specs/2026-09-26-focus-exploration-design.md` §3.3 (`Sequence`), §5 (FOCUS-3 row and the 2026-09-20 acceptance paragraph), §6 (test ids), §9 (deletions), §11 R11–R13, §10 R8, §12 R15–R20. **Principles:** 4, 9, 12, 14b, 21–23, 24–24b, 25, 26, 26a, 27–27g (rule 27 is on `worktree-bible-atlas-m1`'s `docs/PRINCIPLES.md`, not yet on `F1-int`; Task 0 notes it). **Queue:** A-F3; Amendment A (end of this plan) closes F-78 and the served half of F-79; closes PAGE-BOUNDARY-BUG-1 and the rest of the 2026-09-20 list; the FOCUS-3 sites of F-13, F-63 (PassageList's "all"), F-65 (passage-side `CanonRef` sites), F-66 (`ConcordToc`, `ChapterText` in the core); FOCUS-2's FINDINGS "`TextUnit.ref` restates the node" and "the range routes parse references per request".

**Base:** `origin/lane/claude/F1-int` at `eea9023` (FOCUS-1 + FOCUS-6 + fix waves through ATTEST-1). Assume F-74's semantics (`previous` is null only on the first page; `lane/claude/FIX6`). Every gate takes `--base eea9023` (PRINCIPLES 22). When F1-int lands, rebase onto the landed head and write the new base into the ledger (`.superpowers/sdd/2026-10-02-focus3/progress.md`).

**Owner and pairing:** Claude, Lane A. Runs beside FOCUS-2 (Codex) and ahead of FOCUS-4 (Codex). See "File boundary" below: FOCUS-2 goes first on every shared file.

## OPEN: for the owner, before the task named starts (each blocks only that task)

1. **The reading read (Task 2).** Serve a container's text units as one keyset page, `GET /api/node/{id}/text?cursor&limit` → `TextPage` (one round trip per page, headings and anchors included)? (b) is a `contains` page plus a batched element read (two trips per page, no pericope headings). **Recommend (a).**
2. **The legacy `/api/text?ref=` mode (Task 2).** Keep it, on a ratchet, for its callers that leave later (`MiniReaderExpand`, `VerseTextResolver` → FOCUS-5; `Kretzmann` commentary column → FOCUS-7)? **Recommend yes.**
3. **The BoC "part" level (R13, Tasks 6–7).** The graph has no part level; the ETL flattens the Smalcald Articles' parts into articles. (a) The document plays the book's role everywhere now. (b) The ETL mints part containers now. **Recommend (a).**
4. **Triglotta citations (R11, Task 3).** Compile each Concord paragraph's label as its citation (`Ap IV 48`), from a curated `data/curated/concord-citations.toml`. Where a document's paragraph numbering is not the Triglotta's, (a) cite to the article and list that document in the close report, or (b) stop for curation. **Recommend (a).**
5. **Passage containers (Task 4; your FOCUS-2 answer 6: "(b) in FOCUS-3").** (a) Mint one Container per cited span, `cites` ends at it, and titled passages stay their events (F-13 closed as "a titled passage is its event"). (b) Also mint a Container per titled passage. **Recommend (a).**
6. **Shift-click verse range (`passage-chip`, `PassageNode`, Task 8).** (a) The range selects its verses (the selection tray); there is no range popover. (b) The range opens its first verse. **Recommend (a).**
7. **The chapter card's aggregates (Task 8).** The pericope headings, places, cross-reference total, "Chapter n of N" and the hover peek: (a) dropped, so the chapter's popover is its card and frontier; (b) served as aggregates (view-shaped, against rule 27a). **Recommend (a).**
8. **The BoC numeric "Go to Part/Article/Paragraph" picker (Task 7).** (a) Deleted; the table of contents and the arrows navigate, and `/concord?ref=` deep links stay. (b) Kept, over the contents. **Recommend (a).**
9. **The BoC intro blurb hard-coded in `Concord.razor` (Task 7).** (a) Deleted. (b) Moved to `data/` as the corpus root's description, shown once on the table of contents' cover. **Recommend (b).**
10. **Which Concord paragraphs are clickable (Task 7).** It replaces `ConcordToc.IsExplorablePart(7)`. (a) A unit whose served frontier has any group but its own `member-of`. (b) Every unit. **Recommend (a).**
11. **Kretzmann (Task 6).** (a) FOCUS-3 moves only its chapter arrows, chapter head and verse rows onto the shared pieces (its copy of `ComputeAdjacent` dies), and its continuous scroll waits for FOCUS-7. (b) Kretzmann waits wholly for FOCUS-7, and `/api/books` and `/api/chapter` live until then. **Recommend (a).**
12. **Reading page size (Task 5).** Reading uses the list ruling too: 20 units a page, 40 resident (`Affordances.PageSize`, `PageWindow.ShownEntries`)? **Recommend yes.**
13. **Scrolling into the next chapter (Task 6).** It makes that chapter the focus: the locus atom moves (the map follows), and the URL is replaced, not pushed. **Recommend yes.**

Defaults this plan builds if unanswered: 1a, 2 yes, 3a, 4a, 5a, 6a, 7a, 8a, 9b, 10a, 11a, 12 yes, 13 yes.

## Global Constraints

- `docs/PRINCIPLES.md` binds:
  - rule 4: zero dead code.
  - rule 9: no comments in application code. No snippet below has one; a comment found on a touched line is deleted, never reworded. `Reader.razor` and `Concord.razor` carry many today; lines this batch deletes take theirs with them, and no other comment is touched on the side.
  - rule 12: every signature below is for sign-off.
  - rule 21: the critical sections, below.
  - rules 24, 24a, 24b: every fix names its category and closes it; an offender found on the side goes to FINDINGS.
  - rule 25: the client composes over the contract. It parses no reference, composes no id, numbers nothing, and holds no corpus fact (`ConcordToc` dies).
  - rules 26, 26a: domain facts (abbreviations, numerals, the intro blurb) enter through `data/` and the ETL.
  - **rule 27**: data-only derivations compiled (citations, passage containers); per-request reads indexed and bounded (the text page); interaction derivations the client's (scope, focus-on-scroll, arrows, reading position, the table of contents' expansion).
- Tests:
  - whole-body assertions;
  - one behaviour per test, named as a sentence;
  - `// Arrange` `// Act` `// Assert` only;
  - no magic numbers;
  - newspaper order.
  - Real-data expectations are read from the artifact, never written as literals (F-8).
- **Total matches.** A new closed sum exposes `Match<T>`; `Presentation` is matched by its `Form` enum (build error on a missing arm).
- **The one paging door.** Every page the reader reads opens through `Paging` (`Paging.Window` for neighbours, `Paging.Text` for text), so root renewal (F-70) and bounded windows (F-68) hold by construction. `RootConsistencyLawTests` enumerates `Paging.Text`.
- **The one walk door.** The reader resolves its scope and focus through `Explore.Resume` (one batched element read). No new caller of `IExplorer.Resolve` exists, and the compiler refuses one (R20).
- **No relation is appended.** `DECLARED_DIRECTED_RELATIONS` and `DECLARED_SYMMETRIC_RELATIONS` stay as they are at `eea9023`.
- Build no interaction that works only by hovering (roadmap). The chapter-head hover peek goes (OPEN 7a).
- Commit per task. Push each task's branch to `origin/lane/claude/F3-<task>`, integrate on `lane/claude/F3-int`. Landing on `worktree-bible-atlas-m1` is by cherry-pick, under `land`, after Codex's review. Never force.

## File boundary with FOCUS-2 (Codex) and FOCUS-4 (Codex)

FOCUS-2's plan (`origin/lane/claude/F2-plan`, 994b6ca) touches most files this batch must touch. Two batches editing one file at once is a sequencing risk, so the rule is: **FOCUS-2 goes first on every shared file; FOCUS-3 writes new files in parallel and edits a shared file only after the FOCUS-2 task that edits it has landed on `lane/claude/F2-int`.** The wave schedule below follows it.

| Shared file | FOCUS-2 task (first) | FOCUS-3 task (after) | What each changes |
|---|---|---|---|
| `server/atlas-contract/src/wire/graph.rs` | T1B (`UnitText`, `TextUnit.node`/`.body`, `NodeRecord.text`) | T2 (`TextPage`) | disjoint types, one regen each |
| `server/atlas-contract/src/graph.rs` | T1A/T1B (`unit_text`, references), T6 | T2 (`node_text`) | T2 calls F2's `unit_text` |
| `server/atlas-graph/src/labels.rs`, `references.rs` (new in F2) | T1A, T2 | T3 (the Concord arm becomes the citation) | T3 edits one arm |
| `server/atlas-contract/src/{reading,catechism,contents}.rs` | T1A (`encode_node_id` threading), T6 | T9 (route deletions) | |
| `contracts/*`, `data/compiled`, pacts | T1A, T1B, T2, T6 | T2, T3, T4, T9 | serialized by the `contract` lock; F2's first |
| `client/Exploring/{Presentation,Presenter}.cs`, `client/Views/FocusView.razor`, `client.Tests/Explore/{PresentationTests,GraphPresenterTests,ServedGraph}.cs`, `client.Tests/Views/FocusViewTests.cs` | T3 (`Presentation.Text`) | T5 (`Presentation.Sequence`, `Arrows` extraction) | adjacent arms |
| `client/Pages/{Reader,Concord,Kretzmann}.razor`, `client/Components/{VerseLine,MiniReaderExpand,PassageList,ScriptureRefText,ExplorerPopover}.razor` | T4 (openings by position) | T6, T7, T8 (bodies rewritten) | F2 edits opening lines; F3 rewrites bodies |
| `client/Legacy/{PopoverSectionProviders,PopoverSections,LegacyNodes,PassageNode}.cs`, `client/AtlasClient.cs` | T4 (renames the passage arms, deletes `Verse`) | T8 (deletes the passage path) | |
| `client.Tests/Explore/{DeletionLawTests,LegacyNodesTests,ChipTests,LegacyViews,PushViaConformanceTests,IdentityTests}.cs` | T4 (`LegacyNames`) | T8 (`Container`) | F3 adds one row |
| `client.Tests/{ReferenceParsingLawTests,WholeReadLawTests}.cs` (new in F2) | T5 (create) | T8 (remove FOCUS-3's entries) | ratchets shrink |
| `tests/ux/{reader,concord,popover-sections,kretzmann,…}.spec.ts`, `tests/ux/lib/api.ts`, `tests/ux/CONTRACT.md` | T7 | T10 | re-expressions on top of F2's |

**FOCUS-4 (Person)** shares `client/Legacy/{PopoverSectionProviders,PopoverSections,LegacyNodes}.cs`, `client/Exploring/Presenter.cs`, `client.Tests/Explore/DeletionLawTests.cs` and the legacy test fixtures. FOCUS-3 and FOCUS-4 edit disjoint members: Chapter, Book and Passage versus Person. Whoever lands second rebases. FOCUS-3 deletes `/api/chapter`, whose `persons` field is the only served reader of `persons_at_verse` (`reading.rs:68`). FOCUS-4's person mentions read `mentions` pages, so this does not block it. `tests/ux/reader-persons.spec.ts` is re-expressed by FOCUS-3 (Task 10).

**Files FOCUS-3 alone touches:** `client/Reading/*` (new), `client/Views/{SequenceView,ReadingArrows,ConcordUnitRow}.razor` (new), `client/Components/{ContentsTree,ContentsPanel,ScripturePicker,PericopeHeading}.razor`, `client/Components/ContentsTreeModel.cs`, `client/Exploring/{ConcordToc,ChapterText}.cs`, `client/Legacy/{ChapterNode,BookNode,AuthorNode}.cs`, `client/ViewStateService.cs`, `client/State/{Locus,PaneScope}.cs` (read only, unless Task 6 needs `SetLocus` origin), `server/atlas-etl/src/{concord,compile}.rs`, `server/atlas-graph/src/{concord_adapter,law_check,xref_adapter}.rs`, `data/curated/{concord-citations,concord-corpus}.toml` (new), `graph-types/src/{edge,sections}.rs` (Task 4 only).

## Critical sections (PRINCIPLES 21): one holder at a time

| Section | Held by | Why |
|---|---|---|
| `contract`: `export_contract`, AQC/AGC, `client.ContractGenerator` | Task 2 (regen #1), then Task 9 (regen #2); each only after FOCUS-2's regen in flight has released | one generated document |
| `contract`: rebuilding `data/compiled` | Task 3 (rebuild #1, carrying Task 3a's parser, Amendment A), then Task 4 (rebuild #2); each after FOCUS-2's T1A/T2 rebuilds | one artifact |
| `contract`: regenerating for `UnitText.parts` | Task 3a, in the same hold as rebuild #1 (Amendment A) | one generated document |
| `contract`: re-blessing pacts and fixtures | right after each rebuild or regen above | moves the version root |
| `heavy`: `cargo test --workspace`, full Playwright | Tasks 2, 3, 4, 9, 10 | memory |
| `heavy` with "mutation" | Task 10 only, inside the owner's window | once per batch |
| appending to `relations!` | **nobody** | rule 27 |

## Re-anchor table: what the readers touch at `eea9023`, and where each goes

| Today | Role | FOCUS-3 |
|---|---|---|
| `Reader.razor` `_toc` (10 uses), `ComputeAdjacent` (l.697–733), `BookName`, `ChapterPeekPositionText` | prev/next and names from `/api/books` | **deleted**; arrows from the focus's `follows-in`/`precedes-in` (`ReadingArrows`), names are served labels (Task 6) |
| `Reader.razor` `LoadChapter` (`Atlas.Chapter` + `Atlas.ChapterText`, l.385) | one chapter's verses | `SequenceView` over the book's `contains` window, each chapter's text through `Paging.Text` (Tasks 5, 6) |
| `Reader.razor` `ApplyScriptureRef` (l.659–683) | splits `"GEN.1.5"` (rule 25) | **deleted**; the picker and the contents hand a served `ContentsChild` (Task 6) |
| `Reader.razor` `OpenChapter` → `ChapterNode` (l.493–506), `chapter-head-peek` | chapter popover, hover peek | `PopoverOpening.Explore(focus.Identity)` → FocusView card + frontier; peek **deleted** (OPEN 7a) |
| `Reader.razor` `OpenPassage` → `PassageNode` (l.631–643), `passage-chip` | range popover | range → selection (OPEN 6a) (Task 8) |
| `Reader.razor` `OnScroll` → `ViewState.Reader` (in memory, pixel `ScrollY`) | scroll memory | `ReadingPositions` (R8): per container, the topmost unit, local storage (Task 5) |
| `Kretzmann.razor` `_toc` (10), own `ComputeAdjacent` (l.340–376), `ChapterNode` (l.529), `PassageNode` (l.562), `Atlas.Chapter` (l.419) | the same, copied | `ReadingArrows`, `ReadingAddress`, TextUnit rows; its commentary column stays (FOCUS-7) (Task 6, OPEN 11a) |
| `MiniReaderExpand.razor` `Atlas.Chapter` (l.81) | verse list + headings beside `ChapterText` | reads `ChapterText` units only (headings are on `TextUnit.heading`) (Task 6) |
| `VerseLine.razor` `Verse` parameter (legacy `wire::Verse`) | xref count, places, persons | renders from `TextUnit` alone: xref letters from its `cites` count in `edge_summary`, mentions from `anchors` (Task 6) |
| `ScripturePicker.razor` `Atlas.Books()` (l.70) | book/chapter/verse lists | reads `Atlas.Contents(Corpus.Bible)`; emits `ScriptureChoice` (Task 6); `World.razor:76` takes the choice's served `Chapter.Ref` |
| `Concord.razor` `LoadWindowAsync` (l.309–361, 7 calls), `PageSize`, `DefaultRef`, `RenderRows`, `_prevRef`/`_nextRef`, backward trim (l.337–341) | 20-unit corpus-offset windows (the bleed) | **deleted**; `SequenceView` over the document's `contains` window, each article's text by `Paging.Text` (Task 7) |
| `Concord.razor` `<h1>The Book of Concord</h1>` + intro (l.66–70), `concord-position` | a cover on every page; the internal ref shown | the corpus title and description once, on the table of contents' cover (OPEN 9b); `concord-position` **deleted** |
| `Concord.razor` `concord-article-heading-*` "Article N" | dead header | the article's served label, a button that opens the article (Task 7) |
| `Concord.razor` `concord-unit-ref` (`BoC 7.2.1`) | internal code | the compiled citation in the margin (R11, Tasks 3, 7) |
| `Concord.razor` picker (l.18–32), `JumpToAsync`, `JumpToPartAsync` | numeric navigation | **deleted** (OPEN 8a) |
| `client/Exploring/ConcordToc.cs` (32 lines: ten titles, `StartRef`, `ExplorableParts {7}`), `client.Tests/ConcordTocTests.cs` | corpus facts on the client (rule 25) | **deleted** (Task 7) |
| `ContentsTreeModel.Root`/`Child` (restate `ContentsRoot`/`ContentsChild`), `contents-count`, `ExpandPathTo(sref)` | the ToC | composes over the wire records; current by container id; no counts (SIDENAV-PAGENUM-1) (Task 1) |
| `client/Legacy/ChapterNode.cs` (64), `BookNode.cs` (46), `PassageNode.cs` (56) | legacy identities | **deleted** (Task 8) |
| `PopoverSectionProviders.cs` `ChapterCardSection` (l.9–141) | `chapter-card-*` | **deleted** (Task 8, OPEN 7a) |
| `PassageTextSection`, `PassageCrossRefsSection`, `PassageCatechismSection` (FOCUS-2 renames) | range popover | **deleted** (Task 8) |
| `PassageList.razor` `ExploreNodeOf` → `PassageNode` (l.159–172) | open a range from an event account | `LegacyTextUnits.Opening(block.FirstVref)` (FOCUS-2's bridge), on its ratchet to FOCUS-5 (Task 8) |
| `AuthorNode.cs:16` `new BookNode(...)` (identity only) | Author → Book (R2) | `LegacyNodes.BookContainerId` direct; `AuthorNode` leaves in FOCUS-9 (Task 8) |
| `LegacyNodes.cs` Container arm (l.24, 38–51) | bridge | → `null` (Task 8) |
| `AtlasClient` `Books` (l.70), `Chapter` (l.74–85), `Xrefs` (l.109), `Catechism` (l.121) | legacy reads | **deleted** (Task 8) |
| `AtlasClient.ChapterText` (l.87–98), `IExplorableClient.Reading` | ref-addressed text | `Reading` **deleted** with Concord's paging (Task 7); `ChapterText` stays for `MiniReaderExpand`, `VerseTextResolver` (FOCUS-5) and Kretzmann (FOCUS-7) on the ratchet (OPEN 2) |
| `reading.rs` `books` (l.24–27), `chapter` (l.34–84), `xrefs` (l.217–230); `catechism.rs` `catechism_for_span` (l.21–30); `wire/reading.rs` `Chapter`, `Verse`, `PlaceRef`, `PersonRef`, `CrossRef`; `wire/catechism.rs` `CatechismRef`; `reference.rs` `ChapterReference`, `VerseSpan` | legacy routes | **deleted** (Task 9) |
| `graph.rs` `text_window` (l.463–556), `TextWindowQuery` (l.562–573) | `/api/text?ref=&n=&dir=&scope=&corpus=` | stays for the ratcheted callers (OPEN 2); `GET /api/node/{id}/text` beside it (Task 2) |

## Types (for sign-off, PRINCIPLES 12)

### The container text read (Task 2; backend change, OPEN 1)

Wire (`server/atlas-contract/src/wire/graph.rs`):
```rust
pub struct TextPage {
    pub container: NodeRef,
    pub units: Vec<TextUnit>,
    pub previous: Option<u32>,
    pub next: Option<u32>,
    pub version: String,
}

pub struct ContainerTextQuery {
    pub cursor: Option<u32>,
    pub limit: Option<usize>,
}
```
Handler (`server/atlas-contract/src/graph.rs`):
```rust
#[utoipa::path(get, path = "/api/node/{id}/text", params(("id" = String, Path), ContainerTextQuery), responses((status = 200, body = wire::TextPage), TextRefusals), tag = "graph")]
pub async fn node_text(State(graph): State<GraphService>, Path(id): Path<String>, Query(query): Query<ContainerTextQuery>) -> Result<Json<wire::TextPage>, ApiError>;
```
- **Why it is the server's (rule 27):** the read depends on one request (this container, this cursor), so it is an indexed read. It is one keyset lookup on the `contains` index from the container plus the page (27b), clamped to `LARGEST_PAGE` like every page (F-62: the cap is the server's). `previous`/`next` follow `EdgePage`'s cursor and first-page convention exactly (F-74), through the same cursor helpers.
- `units` are the container's `contains` neighbours that are text units, in `contains` order, each built by FOCUS-2's one `unit_text` builder. So `TextUnit.node`, `.body`, `.heading` and `.edge_summary` are the same bytes `/api/text` serves. A container whose `contains` neighbours are containers (a book, a document, a corpus root) answers an empty page, never an error.
- Refusals: an unknown id is `not_found`; a non-container id is `not_a_container`; a cursor past the end is `bad_cursor`. These are the three `TextRefusals` (`server/atlas-contract/src/error.rs`), a closed enum.
- **This is the closure of PAGE-BOUNDARY-BUG-1's category,** "a reading window addressed by a corpus offset". A text page is addressed by its container, so it cannot cross an article or chapter boundary. The client never trims a duplicate, because `previous` names the page before.
- It also closes FOCUS-2's FINDING "`TextUnit.ref` restates the node" for every reader-side caller: the cursor is a cursor, never a reference. `TextWindow` and its `ref` mode stay only for the callers OPEN 2 ratchets.

Client (`client/Exploring/IExplorableClient.cs`, `client/GraphExplorableClient.cs`, `client/Exploring/Paging.cs`):
```csharp
Task<TextPage> Text(string containerId, int? cursor = null, int limit = DefaultPageSize);

public static Task<PageWindow<TextUnit>> Text(Explorable scope, Link container);
```
- `Paging.Text` is the one door for text. It opens a `PageWindow<TextUnit>` whose read is `graph.Text(container id, cursor, limit)`. It checks each page's `version` against `scope.Root` (`ArtifactMoved` otherwise, as `ServedPages.Read` does) and reports `Moved` from `scope.Moved`. Its step is `Affordances.PageSize` (OPEN 12). Pages are cached in the scope's `ServedPages` store, keyed `(root, container id, cursor, limit)`; `PageStore` gains a second value type or a sibling store, whichever keeps one eviction bound (`ServedPages.Resident`).

### Concord citations, compiled (Task 3; compiler and data, R11, OPEN 4)

Data, `data/curated/concord-citations.toml`:
```toml
[[document]]
key = "apology"
abbreviation = "Ap"

[[article]]
document = "apology"
article = 4
designation = "IV"
```
- Every document has one `abbreviation`; every article that the Triglotta numbers has a `designation`; an unnumbered article (a preface, an appendix) has none and its paragraphs cite as `{abbreviation} {paragraph}`. Provenance: the Concordia Triglotta (1921, public domain) citation convention; the file names it.

Compiler (`server/atlas-graph/src/citations.rs`, compile side):
```rust
pub struct ConcordCitations { pub abbreviations: BTreeMap<String, String>, pub designations: BTreeMap<(String, u16), String> }
pub fn cite(citations: &ConcordCitations, paragraph: &ConcordParagraphRef) -> Result<String, UncitedParagraph>;
pub struct UncitedParagraph(pub ConcordParagraphRef);
```
- `labels::node_label`'s Concord text-unit arm calls `cite`, so the label **is** the citation (`Ap IV 48`, `SC I 2`). FOCUS-2's `references::unit_reference` keeps `BoC 4.4.48` as the reference, used for ids and never rendered. **This amends FOCUS-2's T1A**, which made both arms call `unit_reference`; the verse arm is untouched (O-VERSE-LABEL).
- Compile laws: `every_concord_paragraph_has_a_citation` (a document without an abbreviation fails the compile, naming it); `every_curated_citation_names_a_document_and_article_the_corpus_holds`.
- The ETL reads the file beside `concord-titles.toml` (`server/atlas-etl/src/concord.rs:207`). `DOCUMENTS` (`concord.rs:20–31`) stays the ETL's.
- `ConcordTag::cite` (`graph-types/src/text.rs:67`, `format!("BoC …")`) has served readers only through labels and `contents.rs:86`. `contents.rs:86` reads the compiled label (Task 3). Then `no_served_label_composition` adds `"BoC "` to its scan.

### Passage containers for cited spans (Task 4; compiler, OPEN 5a)

Graph (`graph-types/src/edge.rs`, `server/atlas-graph/src/xref_adapter.rs`):
```rust
pub fn passage_container_id(first: &TextLocus, last: &TextLocus) -> ContainerNodeId;
```
- For each `CrossRef` with `to_last: Some(last)`, the compiler mints `Container:passage-{first ref}-{last ref}` once, titled by the row's `target_display` (already canonical; never parsed), with one `contains` row per verse from `to` to `last` in canon order. It rewrites the row's `to` to the passage, so the `cites` edge ends at the passage, and keeps `to_last` for provenance. Two rows citing one span share one passage.
- **The forest law is restated, not weakened:** `container_containment_is_a_forest` (`law_check.rs:152`) holds over the containers reachable from a corpus root, and a new law, `a_passage_holds_only_verses_and_is_no_members_only_parent`, says that a container no root reaches holds only text units, each of which also has a root-reached parent. Overlap is legal (the container algebra), and the reader's one decisive title per grouping is unchanged (passages are not `member-of` targets the reader scrolls).
- `SECTION_SCHEMA_VERSION` 22 → 23 only if the row shape changes. It does not; the minted nodes are rows of the existing `Container` and `Contains` families. `graph-types` minor for `passage_container_id`. AGC minor.
- FOCUS-2's Task 2 (`object_label` reading `target_display` for a span citation) becomes the passage's compiled label. Task 4 deletes that special case: the object is a node with a label like any other.

### The client: presentation, reading, position, arrows (Tasks 5–7)

`client/Exploring/Presentation.cs`:
```csharp
public sealed record Sequence(string Title) : Presentation;
```
- §3.3 signed `Sequence(string Title, IReadOnlyList<Link> Children)`. A container's children are its `contains` group, which the frontier already pages. Restating them as a list would duplicate the frontier (14b), and a whole list breaks 27e. **This amends §3.3 and is for sign-off.**
- `GraphPresenter.PresentAs`: `Form.Sequence => new Presentation.Sequence(element.Label)`. The table is unchanged: Container is `Sequence` on the Reader and `Card` on the popover.
- `GraphPresenter.CardOf` gains the book fields the legacy `BookNode` showed, read from `NodeRecord.book`: `Author`, `Written`.

`client/Reading/Reading.cs` (app project, not the core; F-66):
```csharp
public sealed record Reading(Explorable Scope, Explorable Focus);

public static class Readings
{
    public static Task<Outcome<Reading>> Open(IExplorer explorer, ReadingPlace place);
}
```
- `Open` is `Explore.Resume(explorer, place.Scope node, [new Link(EdgeKind.Contains, place.Focus node)])`, which is one batched element read for both (R19). It returns `(Path[0], Current)`. **R12/R13 as one rule:** the reader scrolls the scope and arrows step the focus. The scope is the contents root (a book, a document) and the focus is its child (a chapter, an article), both served by `/api/contents` (OPEN 3a).

`client/Reading/ReadingAddress.cs`:
```csharp
public sealed record ReadingPlace(ContentsRoot Scope, ContentsChild Focus);

public static class ReadingAddress
{
    public static ReadingPlace? Bible(Contents contents, BookId book, int chapter);
    public static ReadingPlace? Concord(Contents contents, int part, int article);
    public static ReadingPlace? Of(Contents contents, string containerId);
    public static string Route(ReadingPlace place);
}
```
- This is the only client file that turns a route into a place or a place into a route. It looks places up by the served `locus` and `id` and composes no id. Route segments are decoded against the generated `BookId` enum and integers only. `Route` writes `/read/{book}/{chapter}` from a `BibleRef` locus and `/concord?ref={part}.{article}.{paragraph}` from a `ConcordRef` locus.
- Law `RouteLawTests.No_client_file_but_the_address_builds_or_reads_a_reading_route` scans for `"/read/"`, `"/concord?ref="` and `"?ref="` outside `ReadingAddress.cs` and `ViewRegistrySetup.cs` (whose `/read/{locus}` link moves here). `Route(Of(c, id))` round-trips every child of both served trees (a real-data law).

`client/Reading/ReadingPositions.cs` (R8):
```csharp
public sealed record ReadingPosition(string Container, string Unit);

public sealed class ReadingPositions
{
    public const int Remembered = 64;
    public const string Key = "reading-positions-v1";
    public ReadingPosition? Of(string container);
    public void Read(ReadingPosition position);
}
```
- §10 R8 signed `ReadingPosition(Explorable Container, Locus Locus)`. A store keeps ids, not live elements, and the unit's id already names its locus, so it is `(container id, unit id)`. **For sign-off.**
- It is written (debounced, no timer race: the latest write wins) when the topmost visible unit changes, read when a container becomes the focus with no explicit target (`#v{n}`, or an arrow, which lands at the chapter's head). It is least-recently-read, bounded at `Remembered` (27e), and never sent to the server (27d).
- It replaces `ReaderViewState`'s pixel `ScrollY` (`ViewStateService.cs`). The reference-counted mounted-chapter set stays; `MiniReaderExpand` reads it.

`client/Views/SequenceView.razor`:
```csharp
[Parameter, EditorRequired] public Reading Reading { get; set; }
[Parameter, EditorRequired] public string Handle { get; set; }
[Parameter, EditorRequired] public EventCallback<PopoverOpening> OnExplore { get; set; }
[Parameter, EditorRequired] public EventCallback<Explorable> OnFocusScrolled { get; set; }
[Parameter, EditorRequired] public RenderFragment<TextUnit> Row { get; set; }
```
- It renders `Presentation.Sequence(scope).Title` once (`{Handle}-scope-title`). Then it renders the scope's `contains` window (`Paging.Window(new PresentationRequest(scope, Surface.Reader), EdgeKind.Contains)`). For each child it renders the child's served label once, as a button that opens the child (`PopoverOpening.Explore`). The focused child's title carries `chapter-head` (Bible) or `concord-article-head` (Concord); every child title carries `{Handle}-container-head-{id}`. Then it renders the child's units through `Paging.Text(scope, child)`, one `Row(unit)` each.
- Continuous scroll: a sentinel at the window's end asks `More()`, and one at its start asks `Fewer()` only while `Position.Less` holds. Both are intersection-observed, never timed (F-55). A child's text window opens when its head enters the viewport margin and stops (`PageWindow.Stop`) when the child leaves the parent window. So resident units are bounded by `ShownEntries` per open child, and open children by the viewport: rule 27e, with a law (Task 5).
- At the scope's end, the scope's `follows-in` link (the next book or document) renders as one explicit link, `{Handle}-scope-next` (R12, R13: "you have to click to get to the next part").
- The focus is the child whose head is topmost-visible; when it changes, `OnFocusScrolled` fires (OPEN 13).

`client/Views/ReadingArrows.razor` (extracted from FocusView's arrow block, D.R.Y.):
```csharp
[Parameter, EditorRequired] public Explorable Focus { get; set; }
[Parameter, EditorRequired] public string Handle { get; set; }
[Parameter, EditorRequired] public Func<Link, string?> Href { get; set; }
[Parameter] public EventCallback<Link> OnStep { get; set; }
```
- It renders `{Handle}-prev`/`{Handle}-next` for each `Affordance.Arrows` group of `Focus` (`Paging.FirstLink`), labelled `‹ {label}` / `{label} ›` from the served label. FocusView renders its arrows through it. The Reader passes `Handle="reader"`, Kretzmann `"kretzmann"`, Concord `"concord"`. Arrows on the Concord come from the graph's `follows-in` (FOCUS-0 §7.2): NAV-UNIFORM-1.

`client/Views/ConcordUnitRow.razor`: the Concord row. Its margin is the unit's served label (the compiled citation, R11), small and muted (`concord-unit-cite`), and its body is FOCUS-2's `UnitTextView`. It is clickable by OPEN 10.

`client/Components/ScripturePicker.razor`:
```csharp
public sealed record ScriptureChoice(ContentsChild Chapter, int? Verse);
[Parameter, EditorRequired] public EventCallback<ScriptureChoice> OnApply { get; set; }
```

## Deletion inventory (FOCUS-3 total, with OPEN defaults)

- **Client files:** `client/Legacy/ChapterNode.cs`, `BookNode.cs`, `PassageNode.cs`; `client/Exploring/ConcordToc.cs`; `client.Tests/ConcordTocTests.cs`; `client/Components/TitledPassageEntry.razor` if `PassageList` no longer reads it (Task 8 verifies).
- **Client members:**
  - `Reader.razor`: `_toc`, `ComputeAdjacent`, `AdjacentChapter`, `PrevTarget`/`NextTarget`, `BookName`, `ChapterPeekPositionText`, `ChapterPeekHeadings`, the peek timers, `LoadChapter`, `ApplyScriptureRef`, `OpenChapter`'s `ChapterNode`, `OpenPassage`, `PassageRef`, `_passageRange`, `OnScroll`'s `ViewState.Reader` writes, and `VerseFragment` (moves to `ReadingAddress`);
  - `Kretzmann.razor`: the same `_toc`/`ComputeAdjacent` copy, `ChapterNode`/`PassageNode` sites, `Atlas.Chapter`;
  - `Concord.razor`: `PageSize`, `DefaultRef`, `ConcordRenderRow` and its three records, `RenderRows`, `LoadWindowAsync`, `GoForward`/`GoBackward`, `JumpToAsync`, `JumpToPartAsync`, `CitationOfQuery`, `Slug`, `CurrentPart`, `PartOf`, the picker, the hard-coded cover, `concord-position`, `_scrollResetPending`;
  - `ChapterCardSection`, `PassageTextSection`, `PassageCrossRefsSection`, `PassageCatechismSection` and their `PopoverSectionRegistry` rows;
  - `AtlasClient.Books`, `.Chapter`, `.Xrefs`, `.Catechism`; `IExplorableClient.Reading`, `GraphExplorableClient.Reading`;
  - `ReaderViewState` (`ViewStateService.cs`), with `ConformanceTests.SweepExemption_A2` amended;
  - `ContentsTreeModel.Root`/`Child`/`ExpandPathTo(string)`; `contents-count`;
  - `LegacyNodes.For`'s Container arm (→ `null`) and `ChapterContainerId`'s callers but `LegacySaves` (FOCUS-9);
  - `CanonRef` call sites on the passage path (`PassageList.FocalFromOf`/`FocalToOf` stay for `MiniReaderExpand`, FOCUS-5);
  - the `ChipTests`/`LegacyViews`/`LegacyNodesTests`/`PushViaConformanceTests`/`IdentityTests` rows for Chapter, Book, Passage.
- **Server (Task 9):**
  - `reading.rs` `books`, `chapter`, `xrefs` and their routes;
  - `catechism.rs` `catechism_for_span` and its route;
  - `wire/reading.rs` `Chapter`, `Verse`, `PlaceRef`, `PersonRef`, `CrossRef`; `wire/catechism.rs` `CatechismRef`;
  - `reference.rs` `ChapterReference`, `VerseSpan` (served parsers, rule 26a);
  - `atlas-core` `xrefs::aggregate_span_xrefs` and `data::catechism_items_for_span` if no reader remains (`cargo build` decides; `atlas-cli` is a tool and may keep a reader, noted);
  - `GraphService::cross_refs_for_span`;
  - their tests, pact interactions, AGC feature lines, `scripts/gate-selftest.sh`'s `/api/xrefs` line, and `server/BENCHMARKS.md` rows.
  - `persons_at_verse` loses its served reader. It stays for `atlas-cli verse` (a tool), with a FINDING.
- **Not deleted** (stated so no one "finishes" it):
  - `/api/text?ref=` and `AtlasClient.ChapterText` (OPEN 2: `MiniReaderExpand`, `VerseTextResolver` → FOCUS-5; Kretzmann → FOCUS-7);
  - `/api/contents/{corpus}` (it is the reader's address book and the ToC);
  - `AuthorNode`, `YearNode`, `TimeAndPlaceNode` (FOCUS-9);
  - `LegacySaves`' v1 `Chapter`/`Book`/`Passage` translations (FOCUS-9);
  - `PassageList`, `PassageBlock`, `ArrowNav` (FOCUS-4/5/7);
  - `KretzmannChapter` (FOCUS-7).

---

### Task 0: Ledger, base, and the facts the plan assumes

- [ ] **Step 1:** Write the ledger. Record the base `eea9023`, the F-74 dependency, that rule 27 is read from `worktree-bible-atlas-m1`, and FOCUS-2's integration head at the time of writing.
- [ ] **Step 2:** Verify each "to verify at execution" item below that needs no build (greps), and record the results. Stop on any that fails without its fallback.

### Task 1: One table of contents over the served tree (R13, SIDENAV-PAGENUM-1)

No regen; runs beside FOCUS-2's Task 1 (C# beside Rust). No backend change.

**Files:**
- Modify:
  - `client/Components/ContentsTreeModel.cs` (holds `ContentsRoot`/`ContentsChild`; `Current(string containerId)`; `Root`/`Child`/`ExpandPathTo(string)` deleted);
  - `client/Components/ContentsTree.razor` (no `contents-count`; current by id);
  - `client/Components/ContentsPanel.razor` (current by id; persists through `LocalStore`, not raw JS);
  - `client/Pages/Concord.razor` (the sidebar fallback list removed; nothing else);
  - `client/Pages/Reader.razor` (`CurrentRef` becomes the focus id; one line).
- Create:
  - `client/Reading/ReadingAddress.cs`;
  - `client.Tests/Reading/ReadingAddressTests.cs`;
  - `client.Tests/Reading/RouteLawTests.cs`.
- Test:
  - `client.Tests/ContentsTreeModelTests.cs` (rewritten over wire records).

- [ ] **Step 1: Failing tests.**
  - `ContentsTreeModelTests`: `The_tree_holds_the_served_roots_and_children_as_served`; `The_current_entry_is_the_one_with_the_focus_id_and_its_root_is_expanded`; `A_toggle_twice_restores_the_rows` (the fast-check property, moved from Playwright to bUnit as well).
  - `client.Tests/Components/ContentsTreeTests.cs` (new, bUnit): `No_row_shows_a_count` (whole markup of a two-root tree).
  - `ReadingAddressTests`: `A_bible_route_finds_the_book_and_chapter_the_contents_serve`; `A_concord_route_finds_the_document_and_article_the_contents_serve`; `A_container_id_finds_its_place`; `A_route_the_contents_do_not_hold_finds_nothing`; `Every_served_child_round_trips_through_its_route` (over `ServedGraph`'s two trees).
  - `RouteLawTests.No_client_file_but_the_address_builds_or_reads_a_reading_route`, with a planted offender confirmed in a scratch copy.
- [ ] **Step 2:** `dotnet test client.Tests --filter "ContentsTree|ReadingAddress|RouteLaw"` → red.
- [ ] **Step 3: Implement.** The route law starts with today's offenders (`Reader.razor`, `Kretzmann.razor`, `Concord.razor`, `ViewRegistrySetup.cs`, `ContentsPanel.razor`, `CompositionSplit.razor`'s `?q`) listed under `RetiredBy` → `"FOCUS-3 Task 6/7"`, and a pair law that each still offends. So the list only shrinks, and it is empty at Task 7's close.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests` → green. `npx playwright test tests/ux/contents-tree.spec.ts` → green, except the `contents-count` assertions, which are re-expressed (none exist at `eea9023`; Step 0 verified).
- [ ] **Step 5: Commit:** `contents: one table of contents over the served tree, current by container id, no counts; reading routes have one address (R13, SIDENAV-PAGENUM-1)`.

### Task 2: The container text read (backend, OPEN 1, 2)

**Owner gate first:** OPEN 1 and 2. **Starts after** FOCUS-2's T1B lands (it builds on `unit_text`, `TextUnit.node`, `TextUnit.body`).

**Backend change, flagged:** one new generic read. Why rule 27 puts it on the server: it depends on one request (the container and the cursor), and it is an indexed keyset read bounded by the page cap. Nothing in it is a view: it is the text units a container holds, in the order the graph holds them.

**Files:**
- Modify:
  - `server/atlas-contract/src/wire/graph.rs` (`TextPage`, `ContainerTextQuery`);
  - `server/atlas-contract/src/error.rs` (`TextRefusals`, beside `NeighbourRefusals` and `ReadingWindowRefusals`);
  - `server/atlas-contract/src/graph.rs` (`node_text`, its route);
  - `contracts/*` (regen), `contracts/atlas-query-contract/CHANGELOG.md` (AQC minor: a route added);
  - pacts; `contracts/atlas-graph-contract/graph/*.feature` (a `text` scenario per corpus);
  - `client/Exploring/IExplorableClient.cs`, `client/GraphExplorableClient.cs` (`Text`);
  - `client/Exploring/Paging.cs`, `client/Exploring/ServedPages.cs` (`Paging.Text`).
- Test:
  - `server/atlas-contract/tests/graph_api.rs`;
  - `client.Tests/GraphExplorableClientTests.cs`, `client.Tests/Explore/PageWindowTests.cs`, `client.Tests/Explore/RootConsistencyLawTests.cs`.

- [ ] **Step 1: Failing tests** (`graph_api.rs`, every expectation read from the artifact):
  - `a_chapters_text_pages_are_its_verses_in_order_and_nothing_else`: walk GEN 1's pages to the end; the units' `node`s equal its `contains` neighbours, whole.
  - `an_articles_text_pages_never_reach_the_next_article`: the PAGE-BOUNDARY-BUG-1 red. For the Small Catechism's Ten Commandments article, the union of its pages equals its `contains` neighbours, whole, and holds no unit of the Creed. The same check runs over every article of the corpus, the 24b closure walk.
  - `a_text_unit_on_a_text_page_is_the_unit_the_text_window_serves`: whole `TextUnit` equality against `/api/text`.
  - `every_text_page_names_the_page_before_it_and_only_the_first_has_none` (F-74's protocol, both ways, with the same cursor helpers as `node_edges`).
  - `a_container_of_containers_answers_an_empty_text_page`; `a_text_page_of_a_non_container_is_refused`; `an_unknown_container_is_not_found`; `a_text_page_is_clamped_to_the_servers_page_cap`.
  - Client: `GraphExplorableClientTests.Text_reads_the_containers_page_at_the_cursor`; `PageWindowTests.A_text_window_slides_forward_and_back_over_a_containers_units_at_three_hundred_three_thousand_and_thirty_thousand` (the F-68 growth law over `Paging.Text`); `RootConsistencyLawTests` enumerates `Paging.Text`.
- [ ] **Step 2:** `cargo test -p atlas-contract --test graph_api text_page` → red; the client tests do not compile (`Text` missing).
- [ ] **Step 3: Implement.** `node_text` reads the `contains` keyset from the container, keeps text-unit neighbours, builds units through `unit_text`, and pages with `node_edges`' cursor helpers. No `format!` of a reference; the vocabulary gate and `no_served_label_composition` stay green.
- [ ] **Step 4: Regenerate (critical section `contract`, after FOCUS-2's T1B regen has released):** `export_contract`, `export_aqc_examples`, `--check`, AQC minor, re-bless, `client.ContractGenerator`. Release.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh --base eea9023`, `bash scripts/contract-semver-gate.sh`, `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green except `GeneratedUsageTests` for `TextPage` (read in Task 6). **Commit:** `text: a container's text units are one keyset page, addressed by the container, so a window cannot cross it (rule 27a/27b; PAGE-BOUNDARY-BUG-1 category closed)`.

### Task 3: Concord citations, compiled (compiler and data, R11, OPEN 4)

**Owner gate first:** OPEN 4. **Starts after** FOCUS-2's T1A lands (`references.rs`, the labels arms).

**Backend change, flagged:** compiler and data only, plus `contents.rs:86` reading a compiled label instead of composing `"BoC …"`. Why rule 27 puts it in the compiler: a citation depends on the data alone (the document, its article, the paragraph, and the convention's abbreviations), so it is compiled once (26a). The served system composes nothing.

**Files:**
- Create:
  - `data/curated/concord-citations.toml` (ten documents; the Triglotta article designations);
  - `server/atlas-graph/src/citations.rs`.
- Modify:
  - `server/atlas-etl/src/concord.rs` (reads the file);
  - `server/atlas-graph/src/labels.rs` (the Concord arm);
  - `server/atlas-contract/src/contents.rs` (`ref` reads the compiled label);
  - `server/atlas-contract/tests/no_served_label_composition.rs` (scans `"BoC "`);
  - `LICENSES.md` (the Triglotta, PD);
  - `data/compiled` (rebuilt), pacts.
- Test:
  - `citations.rs` unit tests; `server/atlas-graph/tests/sqlite_laws.rs`; `server/atlas-contract/tests/graph_api.rs`.

- [ ] **Step 1: Failing tests.**
  - `citations.rs`: `an_apology_paragraph_cites_as_ap_article_paragraph`; `a_small_catechism_chief_part_paragraph_cites_with_its_numeral`; `an_unnumbered_article_cites_by_document_and_paragraph`; `a_document_without_an_abbreviation_fails_the_compile_naming_it`.
  - Compile laws: `every_concord_paragraph_has_a_citation`; `every_curated_citation_names_a_document_and_article_the_corpus_holds`.
  - `graph_api.rs`: `a_concord_paragraphs_label_is_its_citation_and_never_its_internal_code` (walk one page of every document; no label starts `BoC `).
- [ ] **Step 2:** red. Record in the ledger, per document, whether our paragraph numbering matches the Triglotta's (spot-check three paragraphs per document against the PD text). Apply OPEN 4 to any that do not.
- [ ] **Step 3: Implement** as specified.
- [ ] **Step 4: Rebuild (critical section `contract`, after FOCUS-2's T1A/T2 rebuilds):** rebuild `data/compiled`, re-bless. Labels change for Concord paragraphs only. If a golden fixture moves, stop for the owner.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh --base eea9023` → green. **Commit:** `concord: a paragraph is labelled by its Triglotta citation, compiled from curated data; the internal code is never served as a label (R11, PARA-NUM-1)`.

### Task 4: Passage containers for cited spans (compiler, OPEN 5)

**Owner gate first:** OPEN 5. **Starts after** FOCUS-2's T2 lands (it supersedes its span label).

**Backend change, flagged:** compiler and graph-types only; served code is unchanged (a passage is a node read by the generic reads). Why rule 27 puts it in the compiler: which spans are cited, and the nodes and `contains` rows for them, follow from the data alone.

**Files:**
- Modify:
  - `graph-types/src/edge.rs` (`passage_container_id`), `graph-types/Cargo.toml` (minor), `graph-types/CHANGELOG` line;
  - `server/atlas-graph/src/xref_adapter.rs` (mint, rewrite `to`);
  - `server/atlas-graph/src/law_check.rs` (the forest law restated, the passage law);
  - `server/atlas-graph/src/labels.rs` (FOCUS-2's `object_label` span case deleted);
  - `contracts/atlas-graph-contract/VERSION` (minor), AGC pins for a passage;
  - `data/compiled` (rebuilt), pacts.
- Test:
  - `law_check.rs` laws; `server/atlas-graph/tests/bible_containers_real_data.rs`; `server/atlas-contract/tests/graph_api.rs`.

- [ ] **Step 1: Failing tests.**
  - `a_cross_reference_to_a_span_ends_at_the_passage_that_holds_it` (RUT 4:11's citation of GEN 29:32–30:24, read from the artifact: the edge's object is a Container whose `contains` page is those verses, in order, whole).
  - `a_cited_span_is_minted_once_however_many_rows_cite_it`.
  - `container_containment_is_a_forest_over_the_containers_a_corpus_root_reaches`; `a_passage_holds_only_verses_and_is_no_members_only_parent`.
  - `a_cross_reference_to_one_verse_still_ends_at_the_verse`.
- [ ] **Step 2:** red. Record the count of distinct cited spans in the ledger.
- [ ] **Step 3: Implement** as specified.
- [ ] **Step 4: Rebuild (critical section `contract`, after Task 3's):** rebuild, re-bless; edge ids of span citations change (their `to` changed), so the pins and pacts that name them move. If a golden map fixture moves, stop for the owner.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh --base eea9023` → green. **Commit:** `passages: each cited span is a passage Container the citation ends at; the forest law holds over the canon and a passage only gathers verses (F-65 remainder, F-13 for cited spans)`.

### Task 5: `Presentation.Sequence`, the reading walk, the sequence view, the remembered position

**Starts after** FOCUS-2's T3 lands (`Presentation.cs`, `Presenter.cs`, `FocusView.razor`). Task 2's client half must be in.

**No backend change.** Everything here is an interaction derivation (rule 27): which container is scrolled, which child is the focus, what is remembered.

**Files:**
- Create:
  - `client/Reading/{Reading,ReadingPositions}.cs`;
  - `client/Views/{SequenceView,ReadingArrows}.razor`;
  - `client.Tests/Reading/{ReadingsTests,ReadingPositionsTests}.cs`;
  - `client.Tests/Views/{SequenceViewTests,ReadingArrowsTests}.cs`.
- Modify:
  - `client/Exploring/Presentation.cs` (`Sequence`);
  - `client/Exploring/Presenter.cs` (`Form.Sequence`, book fields);
  - `client/Views/FocusView.razor` (its arrows render through `ReadingArrows`);
  - `client/AppServices.cs` (`ReadingPositions` registered);
  - `client.Tests/Explore/{GraphPresenterTests,PresentationTests,ServedGraph}.cs` (`ServedGraph` gains a two-level corpus with text pages);
  - `client.Tests/Views/FocusViewTests.cs`.

- [ ] **Step 1: Failing tests.**
  - `GraphPresenterTests`: `A_container_on_the_reader_presents_its_title_once`; `A_book_on_the_popover_presents_its_author_and_when_it_was_written`.
  - `ReadingsTests`: `Opening_a_place_reads_its_scope_and_focus_in_one_element_read`; `A_failed_read_opens_nothing_and_asks_nothing_more`.
  - `SequenceViewTests` (bUnit over `ServedGraph`):
    - `The_scope_title_shows_once_and_each_child_title_once` (whole ordered test ids; COVER-HEADER-1);
    - `Each_child_title_opens_that_child` (the `OnExplore` argument whole; ARTICLE-HEADER-1);
    - `A_childs_units_are_exactly_its_own` (bleed, at the view);
    - `Scrolling_past_the_end_asks_for_more_and_past_the_start_asks_for_less_only_when_there_is_less`;
    - `The_end_of_the_scope_offers_its_next_scope_as_one_link` (R12/R13);
    - `The_topmost_child_becomes_the_focus` (OPEN 13);
    - `Reading_a_whole_scope_never_holds_more_than_the_window_bound` (27e: walk a scope of 300 / 3,000 / 30,000 units; resident units and DOM rows are equal across the three).
  - `ReadingArrowsTests`: `The_arrows_are_the_focus_follows_in_and_precedes_in_with_their_served_labels`; `A_focus_with_no_successor_has_no_next_arrow`.
  - `ReadingPositionsTests`: `A_read_position_is_found_again_for_its_container`; `The_least_recently_read_position_is_forgotten_past_the_bound`; `An_unavailable_store_remembers_nothing_and_never_throws`.
  - `FocusViewTests`: its arrow tests stay green through `ReadingArrows` (same ids).
- [ ] **Step 2:** `dotnet test client.Tests --filter "GraphPresenter|Readings|SequenceView|ReadingArrows|ReadingPositions|FocusView"` → red.
- [ ] **Step 3: Implement** the types exactly as above. `SequenceView` composes over `Paging.Window` and `Paging.Text` only; it reads no route and parses nothing.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests` → green. **Commit:** `reading: a container presents as a sequence, the reader scrolls its scope and steps its focus, and the place you read is remembered per container (§3.3 amended, R8, R12, R13)`.

### Task 6: The Bible reader is the sequence; Kretzmann's navigation joins it (OPEN 11, 13)

**Starts after** FOCUS-2's T4 lands (the openings in `Reader`, `Kretzmann`, `VerseLine`, `MiniReaderExpand`). **No backend change.**

**Files:**
- Modify:
  - `client/Pages/Reader.razor`, `client/Pages/Kretzmann.razor`;
  - `client/Components/{VerseLine,MiniReaderExpand,ScripturePicker,PericopeHeading,ContentsPanel}.razor`;
  - `client/Pages/World.razor` (the picker's choice, one line);
  - `client/Views/ViewRegistrySetup.cs` (the reader link through `ReadingAddress`);
  - `client/ViewStateService.cs` (`ReaderViewState` deleted);
  - `client.Tests/State/ConformanceTests.cs` (`SweepExemption_A2`, the Reader allow-list line);
  - `client.Tests/Reading/RouteLawTests.cs` (entries removed).
- Create:
  - `client.Tests/Components/{VerseLineTests,ScripturePickerTests}.cs`.
- Test:
  - `tests/ux/reader.spec.ts` gains `READ-SCROLL-1` (scroll from GEN 1 into GEN 2: `chapter-head` reads "Genesis 2", the URL is `/read/GEN/2`, and the history length is unchanged); `READ-SCROLL-2` (the end of Genesis shows `reader-scope-next` "Exodus", and clicking it lands on Exodus 1); `READ-POS-1` (read to verse 20 of a long chapter, go to the world and back: verse 20's line is the topmost visible).

- [ ] **Step 1: Failing tests:** the three specs; `VerseLineTests.A_verse_line_renders_from_its_text_unit_alone` (whole markup, xref letters from the served `cites` count); `ScripturePickerTests.The_picker_offers_the_served_books_and_chapters_and_hands_back_a_served_chapter`.
- [ ] **Step 2:** red.
- [ ] **Step 3: Implement.**
  - **Reader:** route → `ReadingAddress.Bible(contents, book, chapter)` → `Readings.Open` → `SequenceView` with `Row = VerseLine`.
    - Arrows: `ReadingArrows Handle="reader"` with `Href = link => ReadingAddress.Route(Of(contents, id))`. Each `reader-prev`/`reader-next` stays an `<a>` with an `href`, so NAV-2…NAV-STUTTER-2 hold.
    - `chapter-head` opens `PopoverOpening.Explore(focus.Identity)`.
    - On focus scrolled: dispatch `SetLocus` from the focus's served locus and replace the URL.
    - The arrival target is `#v{n}` when present, else the remembered position for the focus, else the focus's head.
    - The split guest (`SetLocus` from the host) resolves through the same address.
  - **Kretzmann (OPEN 11a):** `ReadingArrows Handle="kretzmann"`, `ReadingAddress`, and `VerseLine` from `ChapterText` units; its `_toc` and `ComputeAdjacent` are deleted. Its commentary column is untouched.
  - **MiniReaderExpand:** reads `ChapterText` only.
  - **VerseLine:** loses `Verse`, `Book` and `Chapter`.
  - **ScripturePicker:** reads the contents.
  - Remove the Reader's and Kretzmann's entries from `RouteLawTests`.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` (`TextPage` now read) → green. Run `npx playwright test tests/ux/reader.spec.ts tests/ux/kretzmann.spec.ts tests/ux/split-view.spec.ts tests/ux/split-pairs.spec.ts tests/ux/state-sync.spec.ts tests/ux/state-window.spec.ts tests/ux/composition.spec.ts` and record the red set by name in the ledger. The expected reds: `chapter-card`, `chapter-head-peek`, `passage-chip`, VIEWSTATE-1 (pixel `scrollY`, re-expressed in Task 10 as unit position).
- [ ] **Step 5: Commit:** `reader: a Bible book is one scroll, its chapters titled once, arrows from follows-in, the place remembered; Kretzmann steps chapters the same way (R8, R12; NAV-UNIFORM-1, CONTINUOUS-SCROLL-1)`.

### Task 7: The Book of Concord is the sequence (OPEN 3, 8, 9, 10)

**Starts after** Task 6 (shared `SequenceView` behaviour settles on one corpus first), Task 3 (citations), and FOCUS-2's T4 (`Concord.razor:381`).

**Data change only (OPEN 9b), flagged:** `data/curated/concord-corpus.toml` holds the corpus root's description, read by the ETL into `Container:concord`'s `description` (an existing `NodeRecord` field). No served code changes.

**Files:**
- Create:
  - `client/Views/ConcordUnitRow.razor`;
  - `data/curated/concord-corpus.toml`.
- Modify:
  - `client/Pages/Concord.razor` (rewritten: sidebar ToC with the corpus cover; `SequenceView` with `Row = ConcordUnitRow`; `ReadingArrows Handle="concord"`);
  - `server/atlas-etl/src/concord.rs` (the root description), `data/compiled` (rebuilt with Task 4's rebuild if adjacent, else its own under `contract`);
  - `client/Exploring/IExplorableClient.cs`, `client/GraphExplorableClient.cs` (`Reading` deleted), `client.Tests/GraphExplorableClientTests.cs`, `client.Tests/Components/ExplorerPopoverTests.cs`, `client.Tests/Explore/ExplorationInterleavingTests.cs`, `client.Tests/Views/FocusViewTests.cs` (fakes lose `Reading`);
  - `client.Tests/Reading/RouteLawTests.cs` (last entries removed; the list is empty).
- Delete:
  - `client/Exploring/ConcordToc.cs`, `client.Tests/ConcordTocTests.cs`.
- Test:
  - `tests/ux/concord.spec.ts` gains:
    - `BOC-BLEED-1`: open the Ten Commandments. The article's units are exactly its served `contains` neighbours (from `api.nodeEdges`), and none of the Creed's. Then click `concord-next`: the Creed's head is the focus. This is PAGE-BOUNDARY-BUG-1's red, written and seen red at `eea9023`'s behaviour before the rewrite.
    - `BOC-NAV-1`: `concord-next`/`concord-prev` step articles. At a document's last article there is no `concord-next`, and `concord-scope-next` names the next document.
    - `BOC-COVER-1`: "The Book of Concord" appears exactly once on the page (the ToC cover), and "The Small Catechism" exactly once (its scope title).
    - `BOC-CITE-1`: every visible margin citation matches the served label, and no visible text matches `/BoC \d/`.
    - `BOC-ENUM-1`: the Small Catechism's six chief parts are titled I–VI, and its preface and appendices are unnumbered.
    - `BOC-HEAD-1`: every article head is a button that opens its article.
    - `BOC-POS-1`: the remembered position, as `READ-POS-1`.

- [ ] **Step 1: Failing specs** as listed. Run them at the pre-rewrite page and record which are red (`BOC-BLEED-1` must be red). PAGE-BOUNDARY-BUG-1's red/green evidence goes in the ledger, as the owner asked ("confirm it's actually gone, don't assume").
- [ ] **Step 2: Implement.**
  - Route `/concord?ref=` → `ReadingAddress.Concord` → `Readings.Open`. With no `ref`, the place is the remembered position's container, else the first document's first article (`contents.Roots[0].Children[0]`, served).
  - The sidebar is `ContentsTree` under the corpus cover: root label + description, served, once.
  - Clickable units follow OPEN 10a, through the served `edge_summary`.
  - The numeric picker and `concord-position` are deleted (OPEN 8a).
- [ ] **Step 3:** `dotnet build client && dotnet test client.Tests` → green; `npx playwright test tests/ux/concord.spec.ts tests/ux/contents-tree.spec.ts` → the new specs green. Record the old ones re-expressed in Task 10 by name: CONCORD-2, -3, -4, -5, -8, -9b, -9d, -10, BOC-CLICK-1, BOC-SCROLL-1.
- [ ] **Step 4: Commit:** `concord: the Book of Concord reads as documents of articles, one scroll each, arrows from follows-in, citations in the margin, one cover; corpus facts leave the client (R11, R13; the 2026-09-20 notes; PAGE-BOUNDARY-BUG-1 verified gone)`.

### Task 8: The legacy container and passage path is gone (OPEN 6, 7)

**Starts after** Tasks 6, 7 and FOCUS-2's T4. **No backend change.**

**Files:**
- Delete:
  - `client/Legacy/{ChapterNode,BookNode,PassageNode}.cs`.
- Modify:
  - `client/Legacy/{PopoverSectionProviders,PopoverSections,LegacyNodes,AuthorNode}.cs`;
  - `client/Components/{PassageList,ExplorerPopover}.razor`, `client/Pages/{Reader,Kretzmann}.razor` (shift-click → selection);
  - `client/AtlasClient.cs`;
  - `client/wwwroot/css/app.css` (rules only the deleted sections used);
  - `client.Tests/Explore/{DeletionLawTests,LegacyNodesTests,ChipTests,LegacyViews,PushViaConformanceTests,IdentityTests}.cs`, `client.Tests/{AtlasClientTests,AsyncMemoTests,AsyncMemoConformanceTests,PopoverSectionRegistryTests}.cs`, `client.Tests/Components/PlaceOpeningTests.cs`;
  - FOCUS-2's `ReferenceParsingLawTests`/`WholeReadLawTests` `RetiredBy` (FOCUS-3 entries removed).

- [ ] **Step 1: Failing tests.**
  - `DeletionLawTests` adds `NodeKind.Container` to `MigratedKinds` with `LegacyNames` `Container → ["Chapter", "Book", "Passage"]` (FOCUS-2's mechanism). Red: `ChapterNode`, `BookNode`, `PassageNode` and the `"Chapter"`/`"Passage"` `AppliesTo` strings. `PassageNode` leaves the held list.
  - `client.Tests/SelectionTests.cs`: `A_shift_click_range_selects_each_of_its_verses` (OPEN 6a).
- [ ] **Step 2:** red.
- [ ] **Step 3: Implement.**
  - Delete the files and members in the inventory.
  - `PassageList` opens `LegacyTextUnits.Opening(block.FirstVref)`, on the ratchet to FOCUS-5.
  - `AuthorNode`'s identity reads `LegacyNodes.BookContainerId`.
  - `LegacyNodes.For` answers `null` for Container, so a chapter, a book, a document, an article or a passage, opened or followed, renders on FocusView.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green. `npx playwright test tests/ux/popover-sections.spec.ts tests/ux/saved-explorations.spec.ts tests/ux/state-focus.spec.ts tests/ux/selection-tray.spec.ts` → record the reds.
- [ ] **Step 5: Commit:** `containers: a chapter, a book, a passage open on FocusView from their served position; ChapterNode, BookNode, PassageNode and the chapter card are gone (deletion law: Container)`.

### Task 9: `/api/chapter`, `/api/books`, `/api/xrefs/{sref}`, `/api/catechism/{sref}` are gone (backend deletions)

**Backend change, flagged:** deletions only. Rule 27a: the four are view-shaped legacy reads whose content is now the generic reads (a container's text page, a passage container's `contains`, a verse's `cites` and `catechism-link` groups).

**Files:** the server half of the deletion inventory; `tests/ux/lib/api.ts` (`books`, `chapter`, `xrefs`, `catechism`), `tests/ux/lib/canon.ts` (`loadToc` reads `/api/contents/bible`); `tests/ux/CONTRACT.md`.

- [ ] **Step 1: Failing test:** `contract_coverage.rs` `no_route_serves_a_chapter_the_book_list_or_a_span_by_its_reference` asserts that the published path set holds none of the four. Run → red.
- [ ] **Step 2: Delete** the handlers, the wire types, the parsers, the routes, and every test, pact interaction, feature line and script reference to them. `cargo build` decides which `atlas-core` helpers lose their last reader.
- [ ] **Step 3: Regenerate (critical section `contract`, after Task 2's and after FOCUS-2's T6):** export, `--check`, AQC (routes removed: major per `scripts/contract-semver-gate.sh`), AGC per its features, re-bless, client generator. Release.
- [ ] **Step 4 (lock `heavy`):** `cargo test --workspace && (cd graph-types && cargo test --all-features) && dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh --base eea9023` → green. **Commit:** `reading: /api/chapter, /api/books and the range routes are gone; a reader reads containers through the generic reads (rule 27a; FOCUS-2 FINDING closed)`.

### Task 10: Re-express the specs, gates, close

- [ ] **Step 1: Re-express** each recorded red spec under the new behaviour, by name, keeping every surviving test id (§6). Candidates at `eea9023`:
  - `reader`, `popover-sections` (CHAP-HOVER-1, CHAPTER-CARD-1, PLACE-ONE-VIEW), `kretzmann`, `split-view` (VIEWSTATE-1 ×2 → unit position), `split-pairs`, `state-sync`, `state-window`, `composition`;
  - `concord` (CONCORD-2, -3, -4, -5, -8, -9b, -9d, -10, BOC-CLICK-1, BOC-SCROLL-1), `contents-tree`;
  - `reader-headings`, `reader-recursion`, `reader-red-letters`, `reader-xref-anchoring`, `reader-xref-superscripts`, `reader-persons`, `reader-map`, `selection-tray`, `provenance`, `event-timeline`, `w1`…`w5-passages`, `api-reader`, `smoke`, `frontier-matrix` (if FOCUS-2 has not deleted it);
  - `lib/api.ts`, `lib/canon.ts`, `CONTRACT.md`.

  Test ids move:

  | Legacy id | After |
  |---|---|
  | `chapter-card`, `chapter-card-*` | `popover-section-card`, `popover-field-*`, `popover-children-contains`, `popover-up-member-of-*` (OPEN 7a drops headings, places and the xref total) |
  | `chapter-head-peek*` | removed (OPEN 7a; no hover-only affordance) |
  | `passage-chip` | removed; the range is in `selection-tray` (OPEN 6a) |
  | `concord-position`, `concord-picker-*` | removed (OPEN 8a) |
  | `concord-part-heading-{n}` | `concord-scope-title` |
  | `concord-article-heading-{p}-{a}` | `concord-container-head-{id}` |
  | `concord-unit-ref` | `concord-unit-cite` |
  | `concord-toc-part-{n}` | `contents-node-{slug}` on the ToC (one vocabulary for both trees) |
  | `concord-unit-BoC-{p}-{a}-{n}` | `concord-unit-{node id slug}` |

  A spec whose behaviour an OPEN default removed is rewritten to what the reader offers, and the removal is listed in the close report.
- [ ] **Step 2: Gates.**
  - Stryker `mutate` covers `**/Reading/*.cs`, `**/Views/SequenceView.razor` (its code-behind) and `**/Exploring/{Presentation,Presenter,Paging}.cs`.
  - Mutation runs inside the owner's window only (`heavy` with "mutation", `free -g` ≥ 18 GB):
    1. `bash scripts/mutants-parallel.sh -n 3 -b eea9023` (covers `node_text`, `citations`, the passage minting, the laws);
    2. then `dotnet stryker`.

    Outside the window, `.superpowers/MUTATION-GATE-DEBT.md` names `eea9023` as this batch's base.
  - Then `cargo test --workspace && (cd graph-types && cargo test --all-features) && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh --base eea9023 && bash scripts/timing-gates.sh run && npx playwright test tests/ux` → green, except the carried reds (`world-quiet-places:211`, `split-view:293` if still carried, `reader-xref-anchoring:201` F-55) named by spec.
  - The timing gates gain the text page: p95 of `/api/node/{id}/text` at a full page, against the 10× synthetic graph (27f). If no 10× fixture exists yet, it is a FINDING and the gate runs at 1×, said so in the report.
- [ ] **Step 3:** Close report `docs/superpowers/reports/2026-10-0x-focus-3-close.md`. It covers:
  - every rule-24 category below, with its guarantee;
  - the FINDINGS;
  - the amendments: §3.3 (`Sequence(Title)` over the frontier's `contains` window), §10 R8 (`ReadingPosition` by ids), FOCUS-2 T1A's Concord label arm (now the citation) and T2's span label (now the passage);
  - PAGE-BOUNDARY-BUG-1's red/green evidence.
- [ ] **Step 4:** Commit; push the batch branch; hand to Codex for review (14b + 24a). Land under `land` after review.

**Rule-24 categories and their closures:**

| Category | Closure | Guarantee |
|---|---|---|
| A reading window addressed by a corpus offset (PAGE-BOUNDARY-BUG-1) | the text page is addressed by its container; `previous` per page; Concord's trim deleted | `an_articles_text_pages_never_reach_the_next_article` walks every article; `BOC-BLEED-1` |
| Navigation computed on the client from a book list (`ComputeAdjacent` ×2, `_toc`) | arrows are `follows-in`/`precedes-in` through `ReadingArrows` | deletion of `/api/books`; the route law |
| Corpus facts on the client (`ConcordToc`, the cover blurb, `ExplorableParts`) | served titles, the root description, the served frontier | file deleted; rule-25 scan |
| Internal codes shown to readers (`BoC 7.2.1`) | compiled citation labels; `"BoC "` in `no_served_label_composition` | compile law + Playwright `BOC-CITE-1` |
| Reading routes parsed in many places (`ApplyScriptureRef` ×2, Concord query, `ViewRegistrySetup`, `?q`) | `ReadingAddress` the one door | `RouteLawTests` (empty list) |
| Containers served by both mechanisms | deletion law (Container, `LegacyNames`) | compiler + law |
| A cited span that is not in the graph (F-65 remainder) | passage containers; the forest law over root-reached containers | compile laws |
| The ToC restating the wire (`ContentsTreeModel.Root`/`Child`) | the model holds the wire records | compiler |

---

## Wave schedule

Primary is the lane's critical-path work; the companion is unlike work paired beside it (rule 23). A task starts only when the tasks it names have landed on `lane/claude/F3-int` and, for a shared file, the FOCUS-2 task named in the file boundary has landed on `lane/claude/F2-int`.

| Wave | Primary (FOCUS-3) | Beside it | Critical sections | Expected red at wave close |
|---|---|---|---|---|
| 0 | owner: OPEN 1–13; Task 0 | FOCUS-2 wave 0 (its Task 5) | — | — |
| 1 | Task 1 (C#: ToC, `ReadingAddress`, route law) | FOCUS-2 Task 1 (Rust) | — (F2 holds `contract`) | route-law pair entries (listed) |
| 2 | Task 2 (Rust: text page; regen #1 after F2's) | FOCUS-2 Task 3 (C#) | `contract` regen, re-bless; `heavy` | `GeneratedUsageTests`: `TextPage` |
| 3 | Task 3a (Rust/data: the parser keeps every source text node; Amendment A), then Task 3 (Rust/data: citations); one regen and rebuild #1 after F2's | Task 5 written against `ServedGraph` (lands after F2 T3) | `contract` rebuild; `heavy` | as wave 2 |
| 4 | Task 5 lands; Task 6 (C#: Bible reader) after F2 T4 | Task 4 (Rust: passages; rebuild #2) | `contract` rebuild #2; `heavy` | Playwright: Task 6 Step 4's set |
| 5 | Task 7 (C#: Concord) | Task 9's server half written, not regenerated | — | + Task 7's set |
| 6 | Task 8 (C#: deletions) | Task 9 (Rust: regen #2 after F2 T6) | `contract` regen #2; `heavy` | + Task 8's set |
| 7 | Task 10 (specs, gates, close) | FOCUS-4 may start its shared-file tasks after Task 8 lands | `heavy`; mutation in the owner's window; `land` after review | carried reds and named F-55 flakes only |

**Critical path:** OPEN answers → F2 T1B → Task 2 → (F2 T3) Task 5 → (F2 T4) Task 6 → Task 7 → Task 8 → Task 9 → Task 10. Tasks 1, 3a, 3 and 4 ride beside it.

## FINDINGS this plan expects to raise (for the queue; the owner decides)

- **A container has no kind; served code decodes it from the id prefix** (`decode_book_container`, `bible_container_adapter.rs:55`; `book_detail`). This is a Haskell-bar finding.
  - Closure: `NodePayload::Container { title, level: ContainerLevel }`, a closed enum (corpus, book, chapter, document, article, passage), compiled; `ContentsRootKind`/`ContentsChildKind` read from it.
- **`/api/contents` derives `ref`, `locus` and `title` per request** from first members and payloads (`contents.rs:57, 86, 105–110`). These are data-only derivations (rule 27, the F-72 category).
  - Closure: compile them with the container.
- **`persons_at_verse` keeps only a tool reader** (`atlas-cli verse`) after `/api/chapter` dies.
  - Closure: the CLI reads `mentions` pages.
- **`/api/text?ref=` is a second addressing of text** while its ratcheted callers remain (OPEN 2).
  - Closure: FOCUS-5 and FOCUS-7 move them to the text page, and the mode is deleted.
- **`ContentsPanel` persisted through raw `localStorage` JS** (closed in Task 1 by `LocalStore`). The category is "browser storage outside `LocalStore`".
  - Proposed closure: a law that `localStorage` appears only in `LocalStore.cs`.
- **No 10× synthetic graph fixture for the text page's timing gate** (if Task 10 Step 2 finds none; 27f).
- **The Kretzmann commentary column is not yet a sequence** (OPEN 11a). It is FOCUS-7's.

## Self-review against rule 27, the spec and the brief

- **Data-only derivations are compiled:**
  - Concord citations (Task 3);
  - passage containers for cited spans (Task 4);
  - the corpus description (Task 7, data).

  Nothing in the served path composes a reference or a label.
- **Per-request derivations are bounded index reads:**
  - the text page: one keyset lookup plus the page, with the cap the server's;
  - the element read: scope and focus in one call;
  - neighbour pages for the scope's children and the arrows.
- **Interaction derivations are the client's:**
  - the scope/focus rule (R12/R13);
  - the focus on scroll;
  - the arrows (`Affordances.Of`);
  - the remembered position (local storage, never wire, 27d);
  - the ToC's expansion and current entry.
- **The graph models the domain, never a view:**
  - no relation appended;
  - passages are domain objects (a cited span), not a reader construct;
  - the text page serves the units a container holds;
  - no client word enters `server/` or `graph-types/` (the vocabulary gate).
- **27c:** a chapter arrival is one element read (scope + focus), one first `contains` page of the scope, and one text page of the focus, plus a cached first neighbour page per arrow kind.
- **27e:** the reader's resident units and DOM rows are bounded by the window and the viewport, with a growth law at 300/3,000/30,000.
- **Coverage of §5's FOCUS-3 row and §9:**
  - `ChapterCardSection`, `ChapterNode`, `BookNode`, `PassageNode`, `ComputeAdjacent`, `_toc` and Concord's `LoadWindowAsync` paging and trim are deleted;
  - `/api/chapter` is deleted, and so is `/api/books` (wholly: the picker reads the contents);
  - the reader and Concord bodies are `Sequence`s;
  - FOCUS-2's handed-on passage path and range routes are deleted.
- **The 2026-09-20 notes:**
  - NAV-UNIFORM-1: `ReadingArrows`, `BOC-NAV-1`;
  - ARTICLE-HEADER-1: `BOC-HEAD-1`;
  - PARA-NUM-1: `BOC-CITE-1`;
  - COVER-HEADER-1: `BOC-COVER-1`;
  - ENUM-CONSISTENCY-1: `BOC-ENUM-1`, over FOCUS-0's curated titles;
  - SIDENAV-PAGENUM-1: no `contents-count`;
  - CONTINUOUS-SCROLL-1: `READ-SCROLL-1`, `READ-POS-1`, `BOC-POS-1`;
  - PAGE-BOUNDARY-BUG-1: `BOC-BLEED-1` and the real-data law, red before and green after.

## Assumptions

**Verified against `eea9023`:**
- Chapter `CanonSuccession` runs across the whole canon, so GEN 50 `follows-in` EXO 1 (`bible_container_adapter.rs:150`). Books succeed books (`:153`). Concord documents succeed documents, and articles succeed articles only within a document (`concord_adapter.rs:146–149`).
- The graph's containers:
  - Corpus roots: `Container:bible` "The Holy Bible" and `Container:concord` "The Book of Concord" (`corpus_root.rs`).
  - Bible: `bible-book-{CODE}`, `bible-chapter-{CODE}-{n}`.
  - Concord: `concord-doc-{key}`, `concord-art-{key}-{n}`.
  - There is no part level; the Smalcald parts are flattened (`atlas-etl/src/concord.rs:52–75`).
- A container's record carries `label` (its title), `edge_summary` and, for books, `book` (author, written).
- `/api/contents/{corpus}` serves roots and children with container ids, titles, `ref`, structured `locus` and `count` (`wire/contents.rs`).
- `contains` pages are served in canonical order (`snapshot.rs:221–224`; `bible_containers_real_data.rs:94–121`).
- `/api/text` takes `ref`, `n`, `dir`, `scope`, `corpus`; `TextWindow` has `next` only (`graph.rs:463–577`). Concord units carry no heading.
- `Presentation.Of(Container, Reader)` is `Form.Sequence`, which `GraphPresenter` answers with `CardOf`. No `Sequence` record exists. `FocusView` is mounted only on `Surface.Popover` (`ExplorerPopover.razor:60`), and its handle for `Surface.Reader` is `reader`, so `reader-prev`/`reader-next` are its arrow ids.
- `ConcordToc.cs` holds the ten titles, `StartRef` and `ExplorableParts {7}`.
- `ContentsTree` shows `contents-count`.
- No Triglotta citation data exists anywhere in `data/` or code.
- The callers:
  - `AtlasClient.Books`: Reader, Kretzmann, `ScripturePicker`, `BookNode`.
  - `AtlasClient.Chapter`: Reader, Kretzmann, `MiniReaderExpand`, `ChapterNode`.
  - `AtlasClient.Xrefs` and `AtlasClient.Catechism`: `PassageNode` only.
- No spec asserts `concord-article-heading-*`, `concord-unit-ref`, `concord-title`, `concord-intro`, Roman numerals or bleed. The new specs are the first.

**To verify at execution (each with its fallback):**
- **Our Concord paragraph numbering matches the Triglotta's per document** (Task 3 Step 2). Fallback: OPEN 4.
- **`PageStore` can hold text pages under the same resident bound.** Fallback: a sibling store with its own bound, and a law that the two sum to `ServedPages.Resident`.
- **Intersection observers work in the Playwright browsers.** Fallback: the scroll sentinels are buttons too ("More"/"Less"), so nothing depends only on scrolling.
- **The `contains` index supports a keyset start at an ordinal** (as the neighbour read does). Fallback: add the index the read needs, with its law.
- **`cites` rows' `to_last` ranges stay inside one corpus.** Fallback: Task 4 stops and reports.
- **FOCUS-2's T4 leaves `Reader.razor`'s opening lines as the plan expects.** Fallback: re-anchor this plan's Task 6 against F2-int before starting it.

---

## Amendment A (F-78): the Concord parser keeps every source text node

Added 2026-10-02. It adds **Task 3a** and two owner questions (OPEN A1, A2). Nothing above changes except the critical-section and wave rows that name Task 3a. The measurements below were read from `data/raw/concord` and the served artifact at root `69fefc39…`, without modifying either.

### A.1 What is lost today

The parser (`server/atlas-etl/src/concord.rs`) turns an article body into strings by excision. A text node that no excision rule keeps is gone, with no disclosure. Four mechanisms drop text:

| Mechanism | Where | What it drops |
|---|---|---|
| M1. `strip_complete_headings` excises every complete `<hN>…</hN>` | `clean_paragraph_text` | Every heading with its words. Because a group's gaps are concatenated **before** excision, an `<h4>` that wraps a marker becomes "complete" across two gaps, and its body text is excised too. So 7.3.1–3 lose "I believe in God the Father Almighty, Maker of heaven and earth." and the other two articles of the Creed. |
| M2. `strip_standalone_strong_paragraphs` | `clean_paragraph_text` | `<p><strong>` … `</strong></p>`, whose close is searched past the paragraph it opened. 7.6.4 loses everything from the servant's confession ("I, a poor sinner, confess myself…") through the master's ("…short measure."), including "A master or mistress may say thus:". The eight Daily Prayers (7.8) lose their prayers. |
| M3. Text before an article's first marker is in no gap | `group_and_number_paragraphs` | Luther's salutation to the Small Catechism ("Martin Luther, to All Faithful…", "Grace, Mercy, and Peace…"); the "As the head of the family…" subtitles; Apology XXIV's opening "At the outset we must again make the preliminary statement that we"; the Treatise's subscription preamble; Smalcald III XIII's subtitle. |
| M4. The marker-less fallback reads `<p>` only | `split_on_paragraph_tags` | Headings in the Ecumenical Creeds ("Written against the Arians."). |

**Measured on the real source:**
- The ten pages and 23 Smalcald sub-pages hold 137 `<h4>` nodes.
- **88 `<h4>` nodes and one `<h5>`** in served articles have words that land in no served unit:

  | Document | `<h4>` dropped |
  |---|---|
  | Small Catechism | 25 |
  | Epitome | 37 |
  | Large Catechism | 13 |
  | Preface | 6 |
  | Smalcald Articles | 3 |
  | Solid Declaration | 2 |
  | Augsburg Confession | 1 |
  | Ecumenical Creeds | 1 |
- **28 `<p>` runs** are dropped by M2 and M3: 23 in the Small Catechism, 2 in the Treatise and 1 in the Apology.
- In all, about **1,700 words** of source text reach no reader.
- The 48 `<h4>` nodes that survive are the ones whose marker sits inside them with no second marker following (e.g. "Thou shalt have no other gods."). They survive by accident, as plain text.

**Classified against the Triglot scan** (archive.org `concordiatriglot00unse`, public domain):

| Class | Nodes | Examples | Must it be served? |
|---|---|---|---|
| **Body text: Luther's or the Confessions' own words** | about 30 | the three Creed articles (7.3.1–3); the absolution dialogue ("Proceed!", "God be merciful to thee…", "Dost thou believe that my forgiveness is God's forgiveness?", "As thou believest, so be it done unto thee… Depart in peace.") and both confessions in 7.6.4; the eight Daily Prayers (7.8); the salutation of Luther's preface; Smalcald III's opening ("Concerning the following articles we may treat with learned and reasonable men…"); Apology XXIV's opening clause; the Treatise's subscription preamble | **Yes.** It is the 1921 text, and it is missing. |
| **Titles and subtitles the Triglot prints** | about 85 | the commandment, petition and Creed-article titles; "Secondly." / "Thirdly." / "Fourthly." in Baptism; the five "As the head of the family…" subtitles; the 37 Epitome section heads (STATUS CONTROVERSIAE, AFFIRMATIVE THESES, NEGATIVE THESES, the Anabaptist error lists); the 13 Large Catechism heads; "ARTICLES IN WHICH ARE REVIEWED THE ABUSES…" (AC) | **Yes, as headings.** They are the Triglot's own words, and the reader needs them to find its place. |
| **Not the Triglot (F-79)** | 7 headings plus runs | five modern headings in the Preface ("The Naumburg Conference Failed", "The Torgau Conference of 1576"…); the Apology `<h5>` "Shouldn't this be V (IV II) – in Tappert and Kolb…"; the Christian Questions' bracket note | **No.** They are excluded by name (A.2), never served. |

**F-79, which this amendment must meet** (the full report is the controller's `f79.md`): the served corpus already carries non-Triglot text:
- **7.10.0–7.10.20** ("Christian Questions with Their Answers", 21 units) is a modern translation. The same vendored page carries the notice "© 1986 Concordia Publishing House. All rights reserved"; the Triglot prints no such text.
- **bookofconcord.org's own notes:** 7.6.4's closing footnote "* These questions may not have been composed by Luther himself…", 2.1.4, and 4.5.212's "The following, through paragraph 213, are left out of the Readers Edition."
- **Site furniture:** 2.1.5 ("Biblical references … can be found here ."), and 4.17.70 and 4.17.106. The last two are phantom units: a link "(see [AP IV 1](http://bocl.org?AP+IV+1) and [AP IV 106](…))" whose rendered spans the parser takes as markers 1 and 106.

A parser that keeps every node would start serving the other non-Triglot nodes, so the closure must name what it leaves out.

### A.2 The closure (rules 24, 24a, 24b)

- **Category:** "the parser decides what text exists by excising strings". The failed abstraction is the **source text node**: today it has no type, so dropping one is invisible.
- **Side:** the tool layer (rule 26a). The ETL alone reads the source's shape.
- **Sites, all migrated:** M1–M4 above, and `is_skipped_article`. The latter is a domain list in code (rule 26), and it moves to data.

**Closure:**
1. **One door reads an article body.** A private `source_nodes(body)` is the only function in `concord.rs` that turns an article body into text. It yields every text node in source order with its block (heading, bold-only paragraph, paragraph) and the marker it follows, if any. `strip_complete_headings`, `strip_standalone_strong_paragraphs`, `split_on_paragraph_tags` and `clean_paragraph_text` are deleted. `strip_tags` and `decode_entities` become private helpers of `source_nodes`. No string excision remains in which to write an offender.
2. **Every node is assigned by one total function, and it never mints or splits a unit:**
   - a node belongs to the group of the nearest marker at or before it;
   - nodes before an article's first marker lead its first unit;
   - in a marker-less article, each paragraph block starts a unit, and a heading leads the next unit (the last unit, if none follows).
3. **A unit's text is composed from its pieces, and its parts are a partition by type.** `ConcordParagraph.text` is deleted. The compiler composes the rendering from the pieces through `Parts::compose`, so text and parts cannot disagree (rule 6). `Parts` has a private field and two constructors, so a non-partition cannot be built.
4. **What is not served is named in data, with a reason from a closed enum.** `data/curated/concord-source.toml` lists each exclusion (a whole article, or one exact run of source text that must occur exactly once in its article) and each role correction (OPEN A1). An entry that names nothing fails the read. Excluded text is counted in `ConcordStats.excluded`, never dropped silently. A marker group whose every node is excluded mints no unit.

**Real-data law, written first and red on the current parser.** It goes in `server/atlas-etl/tests/concord_real_data.rs`:

```rust
#[test]
fn every_source_text_node_lands_in_exactly_one_served_unit_or_one_named_exclusion()
```

- **Oracle, per article of all ten pages and the 23 Smalcald sub-pages:** the article body's words, with every tag stripped by the test's naive `<[^>]*>` rule and the marker labels removed, in source order.
- **Compared against:** the words of that article's units' pieces, interleaved with its exclusions, also in source order.
- **Assertion:** the list of missing or extra runs equals `vec![]`, as a whole-body assertion. Order makes it "exactly one": a duplicated node is an extra run.
- **The oracle is deliberately not the parser's code.** An independent oracle is the point of the law, so this repetition is not a D.R.Y. finding (14b).
- **Red at the base:** the 117 nodes and runs above, about 1,700 words, in seven documents. The ledger records the list.

Further laws and unit tests, each red first:
- `concord_real_data.rs`:
  - `every_curated_exclusion_and_role_names_exactly_one_source_node`;
  - `the_creeds_first_article_keeps_i_believe_in_god_the_father_almighty`: 7.3.1's pieces, whole, against the Triglot wording;
  - `the_confession_form_keeps_both_confessions_and_the_absolution`: 7.6.4, whole.
- `concord.rs` unit tests over fixtures:
  - `a_heading_that_wraps_a_marker_keeps_its_words_in_that_markers_unit` (the Creed's `1b`);
  - `a_bold_paragraph_never_reaches_past_its_own_paragraph` (7.6.4's shape);
  - `text_before_the_first_marker_leads_the_first_unit`;
  - `a_heading_in_an_unmarked_article_leads_the_next_unit_and_mints_none`;
  - `an_excluded_run_is_counted_and_never_served`;
  - `a_marker_group_of_only_excluded_text_mints_no_unit`.
- `graph-types` canon vectors: `a_rendering_with_one_whole_text_part_encodes_as_its_text_alone` (A.4).

**Guarantee:** every character of an article body is served in exactly one unit or excluded under a named, counted reason. There is no third path. The law walks every article of every page.

### A.3 Signatures (for sign-off, rule 12)

Graph types (`graph-types/src/text.rs`), with the names and offsets of the catechism model spec §3.2:

```rust
pub enum TextPartRole {
    Heading,
    Text,
}

pub struct TextPart {
    pub role: TextPartRole,
    pub start: u32,
    pub end: u32,
}

pub struct Parts(Vec<TextPart>);

impl Parts {
    pub fn whole(text: &str) -> Parts;
    pub fn compose(pieces: &[(TextPartRole, &str)]) -> (String, Parts);
    pub fn iter(&self) -> impl Iterator<Item = &TextPart>;
}

pub struct Rendering {
    pub text: String,
    pub parts: Parts,
}

pub type LayerMap = BTreeMap<TranslationId, Rendering>;
```

- **Offsets** are characters, half-open, the convention of `WordsOfChristSpan`.
- **`compose`** joins pieces with one space and merges adjacent pieces of one role into one part.
- **Every reader of `renderings` reads `.text`.** That is about 20 files at `eea9023`, and `cargo build` enumerates them.
- **A Bible verse is `Parts::whole`.** On disk, and in the canonical encoding that content-addresses a node, a rendering whose parts are one whole `Text` part encodes as its text alone. So no Bible node's pid changes, and the KJV section does not grow.

ETL (`server/atlas-etl/src/concord.rs`; `read_all`'s signature is unchanged):

```rust
pub struct ConcordParagraph {
    pub paragraph: u16,
    pub source_label: String,
    pub pieces: Vec<SourcePiece>,
}

pub struct SourcePiece {
    pub role: TextPartRole,
    pub text: String,
}

pub struct ConcordExclusion {
    pub document: String,
    pub article: String,
    pub scope: ExclusionScope,
    pub reason: ExclusionReason,
}

pub enum ExclusionScope {
    WholeArticle,
    Run(String),
}

pub enum ExclusionReason {
    SiteFurniture,
    EditorialNote,
    NotPublicDomain,
}

pub struct ConcordRoleCorrection {
    pub document: String,
    pub article: String,
    pub text: String,
    pub role: TextPartRole,
}

pub struct ExcludedText {
    pub document: &'static str,
    pub article: String,
    pub reason: ExclusionReason,
    pub words: usize,
}

pub fn parse_concord_source(input: &str) -> Result<(Vec<ConcordExclusion>, Vec<ConcordRoleCorrection>)>;

struct SourceNode {
    block: SourceBlock,
    marker: Option<usize>,
    text: String,
}

enum SourceBlock {
    Heading,
    BoldParagraph,
    Paragraph,
}

fn source_nodes(body: &str) -> Vec<SourceNode>;
```

- **`ConcordStats`:** `skipped_articles: usize` becomes `excluded: Vec<ExcludedText>`.
- **The default role is the markup's:**
  - a node in a `Heading` or `BoldParagraph` block that carries no marker is `Heading`;
  - every other node is `Text`.
- **A `ConcordRoleCorrection` overrides it** where the source's markup disagrees with the Triglot's typesetting (OPEN A1).
- **Pieces stay at source-node granularity, one piece per text node, and the ETL never merges them.** So the catechism batch can re-role a node without re-parsing (A.7).

Data (`data/curated/concord-source.toml`, CC0, provenance: the vendored pages checked against the 1921 Triglot scan):

```toml
[[exclude]]
document = "small-catechism"
article = "/small-catechism/prefaratory-notes/"
reason = "site-furniture"

[[exclude]]
document = "small-catechism"
article = "/small-catechism/how-christians-confess/"
run = "* These questions may not have been composed by Luther himself but reflect his teachings and were included in editions of the Small Catechism during his lifetime."
reason = "editorial-note"

[[role]]
document = "small-catechism"
article = "/small-catechism/how-christians-confess/"
text = "God be merciful to thee and strengthen thy faith! Amen."
role = "text"
```

The compiler (`server/atlas-graph/src/concord_adapter.rs`):

```rust
pub fn paragraph_node(unit: ConcordRef, pieces: &[SourcePiece]) -> Node;
```

Wire (`server/atlas-contract/src/wire/graph.rs`, on FOCUS-2's `UnitText`):

```rust
pub struct UnitText {
    pub locus: TextRef,
    pub text: String,
    pub parts: Vec<TextPart>,
    pub words_of_christ: Vec<WordsOfChristSpan>,
    pub anchors: Vec<Anchor>,
}

pub struct TextPart {
    pub role: TextPartRole,
    pub start: usize,
    pub end: usize,
}

pub enum TextPartRole {
    Heading,
    Text,
}
```

- **One builder fills `parts`:** FOCUS-2's `unit_text` reads `Rendering.parts` and computes nothing.
- **A `Heading` part is not `TextUnit.heading` (`UnitHeading`), and this is not a D.R.Y. finding.** A Heading part is the source's own printed words inside the unit. A `UnitHeading` is a link to an event node whose label heads a verse. They are different facts.

Client:
- FOCUS-2's `UnitTextView` renders a unit by its served parts: a `Heading` part as its own line, `unit-part-heading`, and a `Text` part as body.
- It reads the role and never the words (rule 25).
- This lands in Task 7, after FOCUS-2's T3. Until then, `GeneratedUsageTests` stays red for `TextPart`, as it does for `TextPage`.

### A.4 Schema, contract and re-bless

| Item | Change | Why |
|---|---|---|
| `SECTION_SCHEMA_VERSION` | 22 → 23 | the TextUnit payload's rendering becomes `Rendering`; moves the version root |
| `graph-types` | major | a public field's type changes (`LayerMap`) |
| AQC | minor | `UnitText.parts`, `TextPart` and `TextPartRole` are added; if `scripts/contract-semver-gate.sh` classes a new required response field as major, it is major, and the gate decides |
| AGC | minor | the feature `concord/parts.feature`: "a Small Catechism Creed paragraph holds its article's words as a Text part, after its title as a Heading part" |
| `data/compiled` | rebuilt in Task 3's rebuild #1 | one `contract` hold covers the regen, the rebuild and one re-bless |

The re-bless:
- **Pacts and fixtures** that quote Concord text whose words grew: 7.1, 7.2, 7.3, 7.6.4, 7.8, the Large Catechism, the Epitome and the Solid Declaration heads, AC XXI, Ap XXIV, Smalcald III, and the Treatise.
- **AGC pins of Concord `cites` edges.** `citations::cite_scripture` re-scans the longer text, so a citation after an inserted piece gets a new token span and edge id. New text may add citations; the ledger records the count before and after.
- **No geography moves.** If a golden map fixture moves, stop for the owner.

`LICENSES.md`: the Concord row names its exclusions (OPEN A2).

### A.5 Paragraph numbering against the Triglot

- **Kept text never changes a number.** Pieces join existing marker groups (A.2, point 2), so every `(part, article, paragraph)` at the base survives with the same number. The only moves come from exclusions:

| Exclusion | Effect |
|---|---|
| Ap XVIII's link residue (OPEN A2) | Removes the phantom markers 1 and 106. Source labels 70–76 are served today as 4.17.107–113, because the parser remapped them after the phantom 106. After the exclusion they are served at their own labels, 4.17.70–76, which is a correction toward the Triglot. The phantom units 4.17.70 (old) and 4.17.106 disappear. The seven old ids stop resolving; the close report lists them, and FOCUS-9's `LegacySaves` owns id migration. |
| 2.1.4 and 2.1.5 | The last two units of the Apostles' Creed article go; no other number moves. |
| 7.10, the Christian Questions (OPEN A2) | The article goes, with its 21 units. The Small Catechism keeps nine articles and 70 paragraphs; it is the last article, so nothing renumbers. `concord_real_data.rs`'s counts and `SMALL_CATECHISM_TITLES_AS_SERVED` move with it. |

- **Task 3 Step 2's spot-check** runs after Task 3a. It adds three named checks to its three per document:
  - Ap XVIII 70 ("Nor, indeed, do we deny liberty to the human will");
  - SC II 1, which must now hold "I believe in God the Father Almighty…";
  - SC V, the whole form against the scan's section V.

### A.6 Task 3a and its place in the waves

**Task 3a: the Concord parser keeps every source text node** (Rust and data; OPEN A1, A2).

- **Starts after:** FOCUS-2's T1B lands (`UnitText`, `unit_text`).
- **Order:** before Task 3, in wave 3, because both edit `concord.rs`, and Task 3's numbering check must see the corrected numbers.
- **Paired with:** C#, Task 5 (rule 23).
- **Backend change, flagged:** tool layer and compiler, plus one wire field filled by FOCUS-2's builder. Rule 27 puts it in the compiler because which words a paragraph holds, and their roles, depend on the source alone.

**Files:**
- Modify:
  - `server/atlas-etl/src/concord.rs`;
  - `server/atlas-etl/src/curated.rs` (reads `concord-source.toml` beside `concord-titles.toml`);
  - `graph-types/src/{text,node}.rs` and the canonical encoding (`graph-types/src/canon/node.rs`);
  - every `renderings` reader;
  - `server/atlas-graph/src/concord_adapter.rs`;
  - `server/atlas-contract/src/{wire/graph,graph}.rs`;
  - `contracts/*`, AGC;
  - `LICENSES.md`;
  - `data/compiled`, pacts.
- Create:
  - `data/curated/concord-source.toml`;
  - `contracts/atlas-graph-contract/concord/parts.feature`.
- Test:
  - `server/atlas-etl/tests/concord_real_data.rs`;
  - `concord.rs` unit tests;
  - `graph-types/tests/canon_vectors.rs`;
  - `server/atlas-contract/tests/graph_api.rs`, with `a_creed_paragraph_is_served_with_its_title_as_a_heading_part_and_its_article_as_text`, whole `UnitText`, read from the artifact.

**Steps:**
1. Write the failing tests (A.2). Run `cargo test -p atlas-etl --test concord_real_data every_source_text_node` → red. Record the missing runs in the ledger, by article.
2. Implement A.2 and A.3. Delete the four excision functions and `is_skipped_article`. The comments on lines this task rewrites go with them (rule 9); no other comment is touched.
3. Under the `contract` lock, in one hold with Task 3's rebuild #1 and after FOCUS-2's regen has released:
   - regen;
   - AQC and AGC bumps;
   - rebuild;
   - one re-bless;
   - `client.ContractGenerator`.

   Then release.
4. Under the `heavy` lock: `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh --base eea9023` → green, except `GeneratedUsageTests` for `TextPart`.
5. **Commit:** `concord: every source text node is served in exactly one paragraph or excluded by name; the Creed's articles, the absolution and the daily prayers are read again; a paragraph's parts are typed Heading or Text (F-78; F-79's served text excluded)`.

**Task 7 additions:**
- `UnitTextView` renders parts by role.
- Playwright `BOC-PARTS-1`: the Creed's first article shows "I believe in God the Father Almighty, Maker of heaven and earth.", and Confession shows "As thou believest, so be it done unto thee.", both read from the served text.

**Rule-24 table addition:**

| Category | Closure | Guarantee |
|---|---|---|
| The parser decides what text exists by excising strings (F-78) | one `source_nodes` door; a total assignment; `Parts` as a typed partition; exclusions by name in data | `every_source_text_node_lands_in_exactly_one_served_unit_or_one_named_exclusion` over every article of every page |

### A.7 Compatibility with the catechism model spec (CAT, draft)

The amendment matches the spec's model and precludes none of its parts:

| Point | How Amendment A meets the spec |
|---|---|
| **Names and offsets** | `TextPart`, `TextPartRole`, and character offsets as in §3.2. Amendment A ships `Heading` and `Text`. CAT-1 adds `Question`, `Answer` and `Bracket` by splitting `Text` parts, which keeps the partition. A variant added to the enum breaks every match until it is handled, which is the point of total matches. |
| **Where parts live** | CAT §3.2 put `parts` on `TextUnit`, which predates FOCUS-2's T1B. They belong on `UnitText`, beside `words_of_christ`. The spec should follow. |
| **CAT's partition law** | `every_text_unit_is_partitioned_into_parts` holds by type (`Parts`). CAT keeps it as a deserialization test. |
| **CAT's red law** | `every_small_catechism_chief_part_paragraph_has_text_and_answer` needs `Answer`, so it stays CAT-1's. This amendment's red test is the Text-part half, 7.3.1. |
| **Luther's questions** | Luther's questions ("What does this mean?" in `<em>`, "–Answer:") stay inside `Text` pieces at node granularity. CAT-1 re-roles them from the same `source_nodes` blocks (it may add an `Emphasis` block) or from `concord-source.toml` corrections, with no second parser. |
| **Brackets** | The Triglot's brackets lie inside text nodes. CAT-1 splits them at `[`…`]`, inside the tool layer. |
| **Not closed here** | §2.1 item 3's markup residue (`**`, `_…_`, a stray `*` reference mark, spaces before `:` and `?`) is character-level inside nodes. It is a separate category, F-CAT-d, and stays open. |
| **CAT's §2.6 counts** | The spec's "91 Small Catechism paragraphs" becomes 70 under OPEN A2. |

**Found while writing this amendment:** Task 3 says "Create `server/atlas-graph/src/citations.rs`", but that file exists at `eea9023`. It holds the Concord Scripture-citation scanner (`cite_scripture`, `scan`). Task 3's Triglotta citation module needs another name, `server/atlas-graph/src/triglotta.rs`, and its `cite` must not shadow the scanner. This is corrected here; Task 3's text otherwise stands.

### A.8 Owner questions

- **OPEN A1 (Task 3a). How should a kept text node be roled?** The source's markup and the Triglot's typesetting disagree in about 16 places:
  - "The First/Second/Third Article." carries a paragraph marker, so markup says body text, but it is a title;
  - the three absolution lines and the eight Daily Prayers are typeset in heading or bold-only blocks, so markup says heading, but they are prayer and dialogue;
  - the master's confession and Smalcald III's opening are the same.

  (a) Role each node by its markup, with about 16 curated corrections in `data/curated/concord-source.toml`, each checked against the Triglot scan. (b) Serve every kept node as `Text` now, and leave roles to the catechism batch.

  **Recommend (a).** Under (b) the reader shows titles as body text and runs headings into prose. CAT-1 needs the same roles anyway.
- **OPEN A2 (Task 3a). Should the non-Triglot text be excluded now?** The closure law makes every source node either served or excluded by name. Exclude in this rebuild:
  - the Small Catechism's "Christian Questions with Their Answers" (7.10, 21 units), a © 1986 Concordia Publishing House translation that the Triglot does not contain;
  - bookofconcord.org's own notes: 7.6.4's footnote, 2.1.4, 4.5.212's "Readers Edition" line, the Preface's modern headings, and the Apology `<h5>`;
  - the site furniture: 2.1.5, Apology XVIII's link residue, and the two skipped articles.

  Correct `LICENSES.md` to say so.

  **Recommend yes.** They fail the licensing rule, or are not the 1921 text. Apology XVIII's paragraphs 70–76 return to their own numbers.
