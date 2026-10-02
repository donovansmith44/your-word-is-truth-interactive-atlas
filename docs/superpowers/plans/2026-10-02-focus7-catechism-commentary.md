# FOCUS-7 (CatechismItem, CommentaryItem) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Executor: Claude or Codex.** The plan is written so either can run it with no context beyond it. Touch only the files a task lists. Anything else found on the way goes to FINDINGS in the queue, never into the diff (rules 24a, AGENTS.md).

**Goal:** Move the two Lutheran-confessional kinds off the legacy popover onto FocusView and the generic presentation, and finish what FOCUS-3 handed on: the Kretzmann commentary column and its continuous scroll (FOCUS-3 OPEN 2 and 11). Delete what spec §5 lists for FOCUS-7 (`CatechismTextSection`, `CatechismExplanationSection`, `CatechismWhereWrittenSection`, `CatechismScripturesSection` — the fifth `Catechism*Section`, `CatechismSeamSection`, is FOCUS-2's — `CatechismInConcordSection`, `CommentaryItemProseSection`, `CatechismNode`, `CommentaryItemNode`, `/api/catechism/item/{id}`), and with them everything only they used: `CatechismSectionRendering`, `CatechismLinks`, `CatechismList`, `ConcordUnitList`, `KretzmannCitationScan`, `AtlasClient.CatechismItem`, `AtlasClient.KretzmannChapter`, `/api/kretzmann/chapter/{cref}` and `kretzmann_adapter::chapter_commentary`. Fold in what the move closes: F-63's last two whole reads (`CatechismLinks`), the FOCUS-7 entries of FOCUS-2's two ratchets (F-65), the client's citation scanner (rule 25; a second copy of the compiler's abbreviation table, 14b), a served read that walks every verse of a chapter with an unbounded page and parses ids for order (27b, 26a), the per-item prose fetch on the Kretzmann page (27c), F-51's commentary placeholder label, and the client literal "Where is this written?" (rules 25, 26).

**Architecture:** Rule 27 splits the work by who knows the inputs.
- **The compiler** writes what the data alone decides: Kretzmann's own structure (his books and chapters, each chapter holding its comments in the order he wrote them, each chapter `comments-on` the Bible chapter it explains), every Scripture citation inside his prose (the same scanner that already cites the Book of Concord), the section heading each comment opens, and each comment's label. The catechism's two headings ("What does this mean?", "Where is this written?") move from code into `data/curated/catechism.toml`.
- **The server** reads. A commentary item's record serves its prose as FOCUS-2's `UnitText` (its compiled citations as anchors), and FOCUS-3's container text page serves a Kretzmann chapter's comments exactly as it serves a Bible chapter's verses. It loses `/api/catechism/item/{id}` and `/api/kretzmann/chapter/{cref}`. No relation is appended; no field exists for a view.
- **The client** derives the interaction. A catechism item is FocusView's card (its served words as fields) and frontier (its `catechism-link` group, paged 20). A commentary item is FocusView's `Text`. The Kretzmann page is FOCUS-3's `SequenceView` over the Bible book (R12) with each verse's comments laid under it, read through the one paging door.

**Tech Stack:** Rust (axum, utoipa, `atlas-etl`, `atlas-graph`, `atlas-contract`, `graph-types`); .NET 10 Blazor WebAssembly (`BibleAtlas.Client`, `BibleAtlas.Client.Exploring`); xUnit + bUnit; Playwright; Stryker.NET; cargo-mutants via `scripts/mutants-parallel.sh`.

**Spec:** `docs/superpowers/specs/2026-09-26-focus-exploration-design.md` §3.3 (`Card`, `Text`; home surfaces: "Kretzmann for the commentary"), §5 (FOCUS-7 row), §6 (test ids `catechism*`), §9 (deletions), §11 R12/R13, §12 R15–R20. **Principles:** 4, 9, 12, 14b, 15–18, 21–23, 24–24b, 25, 26, 26a, 27–27g. **Owner directives:** the KJV is the text, never edited, no textual criticism anywhere (Kretzmann's prose is shown as written; nothing here edits or hedges Scripture); the Small Catechism, the Book of Concord and Kretzmann are first-class. **Queue:** FOCUS-7; closes F-63 (last sites), the FOCUS-7 sites of F-65 and F-66 (`KretzmannCitationScan`), F-51's commentary placeholder; the queue's "Kretzmann as generic text anchored on text" direction (first step: the commentary is text with compiled anchors; words-as-base stays deferred); feeds O-CATECHISM (OPEN 2).

**Base:** verified against `b3d7cfa` (trunk: FOCUS-1 + FOCUS-6). **FOCUS-7 starts on the head FOCUS-3 lands at** (FOCUS-3 itself starts after FOCUS-2). Task 0 writes that head into the ledger (`.superpowers/sdd/2026-10-0x-focus7/progress.md`) as this batch's `--base` (PRINCIPLES 22); every gate takes it. Every signature below either exists at `b3d7cfa` or is marked with the batch that introduces it: **[F2]** FOCUS-2 (`docs/superpowers/plans/2026-10-02-focus2-textunit.md`, `lane/claude/F2-plan`), **[F3]** FOCUS-3 (`2026-10-02-focus3-containers.md`, `lane/claude/F3-plan`), **[F4]** FOCUS-4 (`2026-10-02-focus4-person.md`, `lane/claude/F4-plan`), **[F7]** this plan.

## OPEN: for the owner, before the task named starts (each blocks only that task)

1. **Proof verses on a catechism item (Task 2):** list each verse by its reference, its words one click away (as FOCUS-2 did for cross references), or also print each verse's words in the list (one more server read per 20 verses)? **Recommend: by reference.**
2. **The question headings over proof verses ("What does Baptism give or profit?" groups) (Task 2):** drop them (the graph already merges them, and their source has no license yet, O-CATECHISM), or rebuild them into the graph? **Recommend: drop.**
3. **Arrows between catechism items (Task 2):** none (the item links to its own Small Catechism paragraph, where the Book of Concord reader's arrows already step in order), or add item-to-item arrows? **Recommend: none.**
4. **Kretzmann as a work of its own in the graph (Tasks 3, 6):** file his comments under his own books and chapters, in the order he wrote them, so his commentary reads like the Bible and the Book of Concord and the special Kretzmann chapter read retires; or keep that read? **Recommend: file them.**
5. **A comment's title (popover title, trail, saved explorations) (Task 3):** name it by what it explains ("Kretzmann on GEN.1.1-2", the verse form following your answer to O-VERSE-LABEL), or keep its section heading ("Commentary" when it has none)? **Recommend: what it explains.**
6. **Citations in Kretzmann's prose that the shared scanner cannot place (Task 3):** leave them as plain text and list them in the close report, or stop the batch for curation? **Recommend: plain text, listed.**

Defaults this plan builds if unanswered: 1 by reference; 2 drop; 3 none; 4 file them; 5 what it explains; 6 plain text, listed.

**Rulings applied by analogy, not re-asked:**
- **FOCUS-2 ruling 3 (one group per edge kind, served order, no client split; FOCUS-4 applied it the same way):** an item's verses and its Small Catechism paragraphs are one `catechism-link` group. The "IN THE BOOK OF CONCORD (n)" list and its client filter by the `"BoC "` id prefix (rule 25) go.
- **FOCUS-2 ruling 4(b) (a "read in context" hatch on FocusView for a verse):** a commentary item gets the same hatch (`popover-chip-context`), opening the Kretzmann page at its chapter.
- **FOCUS-2 ruling 7 (reach every site of the category in the batch):** citations are compiled for every prose corpus (Concord and Kretzmann) through one scanner; the client scanner dies.
- **FOCUS-2 ruling 9 (accept the AQC major):** the catechism record's reshape (Task 1) and the commentary locus and heading (Task 4) are AQC majors.
- **R12/R13 and FOCUS-3 OPEN 6, 11, 12, 13 (as FOCUS-3's owner answers stand at Task 0):** the Kretzmann page scrolls a whole Bible book with chapter heads inline, the next book is an explicit click, a verse range selects its verses, pages are 20 with 40 resident, and scrolling into the next chapter moves the locus. If FOCUS-3's answers differ from its defaults, Task 0 records the difference and Task 6 follows the answer.
- **Paging rulings:** 20 per page, More appends to a 40 window then slides, Less goes back 20 (`Affordances.PageSize`); the server's page cap is the server's alone; typed `Cursor`, `previous` null only on the first page (F-74).

## Global Constraints

- `docs/PRINCIPLES.md` binds, especially:
  - rule 4: zero dead code;
  - rule 9: no comments in application code (no snippet below has one; the review greps the diff for added comment lines; a comment on a deleted line goes with it, no other comment is touched);
  - rule 12: every signature below is for sign-off;
  - rules 24, 24a, 24b: every fix names its category and closes it; an offender found on the side goes to FINDINGS;
  - rule 25: the client composes over the contract; it parses no reference, composes no id, scans no text, splits no paragraph and decides no kind from a string;
  - rules 26, 26a: domain facts (headings, abbreviations, structure) live in `data/` or are compiled by the tools; the served system never scans text;
  - **rule 27**: data-only derivations compiled (Kretzmann's structure, citations, headings, labels); per-request reads indexed and bounded (the record, the text page, the neighbour page); interaction derivations the client's (which verse a comment sits under, the scroll, the hatch). The graph models the domain, never a view. No client exploration word in `server/` or `graph-types/` (the vocabulary gate).
- Tests: whole-body assertions; one behaviour per test, named as a sentence; `// Arrange` `// Act` `// Assert` only; no magic numbers; newspaper order. Real-data expectations are read from the artifact, never written as literals (F-8). A test name in `server/` or `graph-types/` is an identifier the vocabulary gate reads.
- **Total matches.** A new closed sum exposes `Match<T>` (C#) or an exhaustive `match` with no wildcard (Rust); `Presentation` is matched by its `Form` enum.
- **The one walk door / the one paging door.** Every follow goes `OnFollow(Link)` → `ExplorerPopover.FollowAsync` → `Explore.Follow`; no new caller of `IExplorer.Resolve` (internal, R20). Every list opens through `Paging.Window`, every text page through `Paging.Text` [F3]; `RootConsistencyLawTests` stays green.
- **No relation is appended.** `DECLARED_DIRECTED_RELATIONS` and `DECLARED_SYMMETRIC_RELATIONS` do not change. New row families (Task 3) belong to existing relations (`contains`, `comments-on`, `cites`).
- Build no interaction that works only by hovering.
- Commit per task on `lane/<agent>/F7-t<n>`; integrate on `lane/<agent>/F7-int`. Landing on `worktree-bible-atlas-m1` is by cherry-pick (squashed per task), under `land`, after the other agent's review (14b + 24a). Never force.
- Each task: its own worktree (`git -c core.autocrlf=false worktree add -b lane/<agent>/F7-t<n> ~/w/F7-t<n> <base>`), `CARGO_TARGET_DIR=~/mut/<agent>-F7-t<n>`, `nice -n 10 cargo -j 4`, `data/raw` and `data/cache` copied never linked; `. ~/.bible-atlas-env` before any toolchain command.

## Critical sections (PRINCIPLES 21): one holder at a time

| Section | Held by | Why |
|---|---|---|
| `contract`: rebuilding `data/compiled` (moves the version root) | Task 1 (catechism headings), then Task 3 (Kretzmann structure and citations) | one artifact |
| `contract`: `export_contract`, `export_aqc_examples`, AQC/AGC features, `client.ContractGenerator` | Task 1, then Task 4, then Task 8, strictly in that order | one generated document |
| `contract`: re-blessing pacts and fixtures, re-pinning `graph-types/tests/canon_vectors.rs` | right after each rebuild or regen above | moves the version root |
| `heavy`: `cargo test --workspace`, full Playwright | Tasks 1, 3, 4, 8, 9 | memory |
| `heavy` with "mutation" in the message | Task 9 only, inside the owner's window (`.superpowers/MUTATION-GATE-DEBT.md`) | once per batch (3a) |
| appending to `relations!` | **nobody** | rule 27 |

## Re-anchor table: what the two kinds touch at `b3d7cfa`, and where each goes

| At `b3d7cfa` | Role | FOCUS-7 |
|---|---|---|
| `client/Legacy/CatechismNode.cs` (33 lines; `Kind => "Catechism"`, `DetailAsync` over `/api/catechism/item/{id}`) | catechism identity and detail | **deleted** (Task 2) |
| `client/Legacy/CommentaryItemNode.cs` (27 lines; a placeholder `"Commentary"` title; a comment that says it is unreachable) | commentary identity | **deleted** (Task 7) |
| `CatechismTextSection`, `CatechismExplanationSection`, `CatechismWhereWrittenSection` (`PopoverSectionProviders.cs:443–519`), `CatechismSectionRendering` (`:420–441`, splits served text on `"\n\n"`, holds the literal "Where is this written?") | the item's words | FocusView card fields from the served `CatechismDetail` (Tasks 1, 2); **deleted** |
| `CatechismScripturesSection` (`:521–585`): parses each `vref` (`CanonRef.ParseVerse`), fetches one whole chapter per distinct chapter (`api.ChapterText`, a fan-out FOCUS-3's OPEN 2 list omits), groups by question | "THE SCRIPTURES" | FocusView `catechism-link` group (OPEN 1, 2); **deleted** (Task 2) |
| `CatechismInConcordSection` (`:1328–1365`): `CatechismLinks.AllTargetsAsync` (`Paging.Whole`, F-63), filtered by `NodeIds.LocalPart(n).StartsWith("BoC ")` (rule 25) | "IN THE BOOK OF CONCORD (n)" | the same `catechism-link` group (ruling 3 by analogy); **deleted** (Task 2) |
| `CatechismLinks` (`:1440–1444`) | the whole catechism-link read | **deleted** with its last caller (Task 2; FOCUS-2 deletes `ConcordSmallCatechismSection`, the other caller) |
| `client/Components/CatechismList.razor` (47 lines; opens `new CatechismNode(...)`) | the verse-side list | **deleted** (Task 2), after FOCUS-2/3 delete its two callers (`CatechismSeamSection`'s verse arm [F2 T4], `PassageCatechismSection` [F3 T8]) |
| `client/Components/ConcordUnitList.razor` (23 lines; opens `new ConcordUnitNode(...)`; FOCUS-2 T4 retargets line 11 to `LegacyTextUnits.Opening`) | the item's Concord list | **deleted** (Task 2) |
| `CommentaryItemProseSection` (`:1299–1326`, `ctx.Graph.Card(...)` → `card.Description`) | the comment's prose | FocusView `Presentation.Text` over the served `UnitText` (Tasks 4, 5); **deleted** (Task 7) |
| `client/Pages/Kretzmann.razor` (579 lines at `b3d7cfa`; after FOCUS-3 T6: `ReadingArrows Handle="kretzmann"`, `ReadingAddress`, verse rows from `Atlas.ChapterText`) — its commentary column: `Atlas.KretzmannChapter` (l.419), `CommentaryItemRow`, `ShowChapter`'s client-side section-start by comparing headings (l.430–455, F-51's `"Commentary"` default at l.441), lazy prose through `lazyProse.js` and one `Graph.Card(itemId)` per item (l.478, 27c), `KretzmannCitationScan.Anchors` (l.505), `OnExplore` → `new CommentaryItemNode(NodeIds.LocalPart(...))` (l.507) | the commentary column | `SequenceView` [F3] over the Bible book with each verse's comments under it, comments read through `Paging.Text` [F3] from the Kretzmann chapter; **all of it deleted** (Task 6) |
| `client/Exploring/KretzmannCitationScan.cs` (90 lines; a regex over book names and a 69-entry alias table that restates `server/atlas-graph/src/citations.rs`'s `ABBREVIATIONS`; composes text-unit ids with `NodeIds.Of`) | client-side citation scanning (rule 25, 14b, F-65, F-66) | **deleted** (Task 7); citations compiled (Task 3) |
| `client/wwwroot/js/lazyProse.js` | the per-item prose observer | **deleted** (Task 7) |
| `client/AtlasClient.cs` `KretzmannChapter` (l.106–107), `CatechismItem` (l.133–134) | legacy reads | **deleted** (Tasks 7, 2) |
| `client/Legacy/LegacyNodes.cs` `CatechismItem`/`CommentaryItem` arms (l.27–28) | bridge | → `null` (Tasks 2, 7) |
| `client/Exploring/PopoverChromeRegistry.cs` `["Catechism"]`, `["CommentaryItem"]` (l.28, 30) | chrome rows | **deleted** (Tasks 2, 7) |
| `client/Legacy/LegacySaves.cs:75–76` | the v1 save translation | stays (FOCUS-9) |
| `server/atlas-contract/src/catechism.rs` `catechism_item` (`/api/catechism/item/{id}`, l.32–83: walks the item's verses and questions per request, reads each verse's text) | legacy route | **deleted** (Task 8); the file goes with it once FOCUS-3 T9 has deleted `catechism_for_span` |
| `server/atlas-contract/src/wire/catechism.rs` `CatechismItem`, `CatechismProofVerse` | its wire | **deleted** (Task 8); the file goes once FOCUS-3 has deleted `CatechismRef` |
| `server/atlas-contract/src/graph.rs` `catechism_detail` (l.248–259), `wire::CatechismDetail` (`wire/graph.rs:118–126`) | the record's catechism words | reshaped with served headings (Task 1) |
| `server/atlas-contract/src/graph.rs` `node_description`'s `CommentaryItem` arm (l.281) | prose as `description` | **deleted**; the prose is `NodeRecord.text` [F2] (Task 4) |
| `server/atlas-contract/src/reading.rs` `kretzmann_chapter` (`/api/kretzmann/chapter/{cref}`, l.86–112); `wire/reading.rs` `KretzmannChapter`, `KretzmannChapterVerse`, `KretzmannChapterItem` (l.62–82) | view-shaped route (27a) | **deleted** (Task 8) |
| `server/atlas-graph/src/kretzmann_adapter.rs` `chapter_commentary` (l.104–145: every verse of a chapter, each with `limit: usize::MAX` (27b), order parsed from the id's last segment, `ordinal_of` (26a)) and `ChapterCommentaryRow` | served read | **deleted** (Task 8) |
| `server/atlas-graph/src/kretzmann_adapter.rs` `normalize` | one `Source`, one `CommentaryItem` per unit, one `CommentsOn` per unit | gains the Kretzmann containers, chapter-level `comments-on`, each unit's place (Task 3) |
| `server/atlas-graph/src/citations.rs` `cite_scripture` (the Concord's reading spine only) | compiled Scripture citations | every prose corpus: the Concord and Kretzmann (Task 3) |
| `server/atlas-core/src/data.rs` `default_explanation_heading` (a domain string in code, rule 26) | default heading | moves into `data/curated/catechism.toml` (Task 1) |

## File overlap with FOCUS-2, FOCUS-3 and FOCUS-4, and the order per file

FOCUS-7 runs after FOCUS-2 and FOCUS-3 have landed, so on their files it always goes last. FOCUS-4 (Codex) may land before or after FOCUS-7; on shared row files whoever lands second rebases. "Rows" means each batch deletes or adds only its own kind's members.

| File | FOCUS-2 | FOCUS-3 | FOCUS-4 | FOCUS-7 | Order |
|---|---|---|---|---|---|
| `client/Legacy/PopoverSectionProviders.cs`, `client/Legacy/PopoverSections.cs` | verse/Concord sections; `CatechismSeamSection` verse arm; passage arms renamed | `ChapterCardSection`, `Passage*Section` | four person sections | six catechism/commentary sections, `CatechismSectionRendering`, `CatechismLinks` | F2 → F3 → {F4, F7} |
| `client/Legacy/LegacyNodes.cs`, `client/Exploring/PopoverChromeRegistry.cs` | TextUnit arm | Container arm | Person arm/row | CatechismItem, CommentaryItem arms/rows | F2 → F3 → {F4, F7} |
| `client.Tests/Explore/{DeletionLawTests,LegacyNodesTests,LegacyViews,IdentityTests,PushViaConformanceTests}.cs`, `client.Tests/PopoverSectionRegistryTests.cs`, `client.Tests/Components/ExplorerPopoverTests.cs` | `LegacyNames`; its rows | Container rows | Person rows | Catechism/Commentary rows; `ExplorerPopoverTests`' three `CommentaryItemNode` fixtures move to `PolityDeltaNode` | F2 → F3 → {F4, F7} |
| `client.Tests/{ReferenceParsingLawTests,WholeReadLawTests}.cs` (created by F2 T5) | create | shrink | shrink | remove FOCUS-7's entries | F2 → F3 → {F4, F7} |
| `client/Exploring/{Presentation,Presenter}.cs`, `client/Views/FocusView.razor`, `client.Tests/Explore/{PresentationTests,GraphPresenterTests,ServedGraph}.cs`, `client.Tests/Views/FocusViewTests.cs` | `Text` form, `TextOf`, `UnitTextView` arm, reader hatch | `Sequence` form, `ReadingArrows` | person fields, `EntryText`, `Disclosure`, year buttons | catechism fields; `CommentaryItem` rows → `Text`; `.focus-field` keeps served line breaks; hatch for commentary | F2 → F3 → {F4, F7}; F7's edits are new arms |
| `client/Exploring/Affordances.cs` | — | maybe | `SectionList` gains two members | none (OPEN 1 by reference); under OPEN 1 "print the words", one member | F4 → F7 |
| `client/Pages/Kretzmann.razor` | T4 openings by position | T6 arrows, address, verse rows from `ChapterText` | — | T6 rewrite | F2 → F3 → F7 |
| `client/Views/SequenceView.razor`, `client/Reading/{Reading,ReadingAddress}.cs` (new in F3) | — | create | — | T6 adds `OnChildOpened`/`OnChildClosed`; T5 adds the Commentary arm of `ReadingAddress` | F3 → F7 |
| `client/Components/{CatechismList,ConcordUnitList}.razor` | T4 edits `ConcordUnitList:11`; deletes a `CatechismList` caller | T8 deletes the other `CatechismList` caller | — | T2 deletes both files | F2 → F3 → F7 |
| `client/AtlasClient.cs` | `Verse` | `Books`, `Chapter`, `Xrefs`, `Catechism` | — | `CatechismItem`, `KretzmannChapter` | F2 → F3 → F7 |
| `client/Exploring/IExplorableClient.cs`, `client/GraphExplorableClient.cs` | — | `Text` added, `Reading` deleted | removes two `Card` callers | removes the last two `Card` callers; deletes `Card` if FOCUS-4 has landed | F3 → {F4, F7}; the second to land deletes `Card` |
| `server/atlas-contract/src/wire/graph.rs`, `server/atlas-contract/src/graph.rs` | T1B `UnitText`, `NodeRecord.text`, `unit_text` | T2 `TextPage`, `node_text` | T1B `PersonLife` | T1 `CatechismDetail`; T4 `UnitHeading` sum, commentary in `unit_text` and `node_text` | F2 → F3 → {F4, F7} |
| `server/atlas-contract/src/wire/locus.rs` (`TextRef`) | — | — | — | T4 `Commentary` variant | F7 only |
| `server/atlas-contract/src/{catechism,reading}.rs`, `wire/{catechism,reading}.rs` | T6 `verse` | T9 `books`, `chapter`, `xrefs`, `catechism_for_span`, `CatechismRef` | — | T8 `catechism_item`, `kretzmann_chapter` and their wire | F2 → F3 → F7 |
| `server/atlas-graph/src/{labels,references}.rs` | T1A, T2 (`references.rs` new) | T3 Concord citation label | T2 parentage wording | T3 commentary label; T4 commentary reference arm | F2 → F3 → {F4, F7} |
| `server/atlas-graph/src/citations.rs` | — | T3 `ConcordCitations`, `cite` | — | T3 `cite_scripture` over every prose corpus | F3 → F7 |
| `server/atlas-graph/src/{law_check,xref_adapter}.rs`, `graph-types/src/{edge,sections}.rs` | — | T4 passage containers, forest law restated | — | T3 Kretzmann containers and row families | F3 → F7 |
| `graph-types/src/{node,graph}.rs`, `graph-types/src/canon/{node,rows}.rs`, `graph-types/tests/canon_vectors.rs` | — | — | T1 person payload | T3 commentary payload, new row families | F4 ↔ F7 (second rebases and re-pins) |
| `server/atlas-graph/src/event_world.rs` `grounds_of` | — | — | T1 exhaustive over `RowFamily` | T3 adds the new families' arms (a compile error forces it) | F4 → F7 if F4 first; else F4 adds them |
| `data/compiled`, `contracts/*`, pacts | each rebuild/regen | each | each | each | serialized by the `contract` lock |
| `tests/ux/{kretzmann,popover-sections,concord,provenance}.spec.ts`, `tests/ux/lib/api.ts`, `tests/ux/CONTRACT.md` | T7 | T10 | T5 | T9 | F2 → F3 → {F4, F7} |

## Types (for sign-off, PRINCIPLES 12)

### The catechism's headings are data (Task 1; backend: data, ETL, wire)

Data, `data/curated/catechism.toml` (file-level, beside the existing `[[part]]` tables):
```toml
explanation_heading = "What does this mean?"
where_written_heading = "Where is this written?"
```
- Both are Luther's own questions as printed in the 1921 Triglotta; the file's header already cites it. An item that poses its own question keeps its per-item `explanation_heading` (Baptism, Confession, the Sacrament of the Altar), as today.

`server/atlas-core/src/data.rs` (the compiled catechism, written by the ETL):
```rust
pub struct TitledText { pub heading: String, pub body: String }

pub struct CatechismItem {
    pub id: String,
    pub name: String,
    pub text: Option<String>,
    pub explanation: TitledText,
    pub where_written: Option<TitledText>,
    pub verses: Vec<String>,
    pub questions: Vec<CatechismQuestion>,
}
```
- `explanation_heading`/`explanation` and `where_written: Option<String>` become the two `TitledText`s; `default_explanation_heading` (a code literal) is deleted. `server/atlas-etl/src/curated.rs` `parse_catechism(input: &str) -> Result<Vec<CatechismPart>>` keeps its signature and applies the file-level headings; a file without them fails the compile naming the missing key (24b).

Wire (`server/atlas-contract/src/wire/graph.rs`):
```rust
pub struct TitledText { pub heading: String, pub body: String }

pub struct CatechismDetail {
    pub part: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub explanation: TitledText,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub where_written: Option<TitledText>,
}
```
- `part_title` becomes `part`; the heading travels with its words. AQC **major** (ruling 9 by analogy). `catechism_detail(id, data)` keeps its signature and maps the fields; it reads one indexed lookup (`AtlasData::catechism_item_by_id`, a `HashMap`), so 27b holds.

### Kretzmann is a work with its own structure (Task 3; backend: compiler, OPEN 4, 5, 6)

Graph (`graph-types`):
```rust
pub struct KretzmannTag;
impl Corpus for KretzmannTag { type Ref = CommentaryPlace; const ID: &'static str = "kretzmann"; fn cite(r: &CommentaryPlace) -> String; }

pub struct CommentaryPlace { pub book: u8, pub chapter: u16, pub verse: u16, pub unit: u16 }

NodePayload::CommentaryItem { work: SourceId, heading: Option<String>, text: String, place: CommentaryPlace, opens_section: bool }

pub struct ChapterCommentsOn { pub commentary: ContainerNodeId, pub on: ContainerNodeId, pub provenance: ProvenanceId, pub justification: Justification }

pub struct CommentaryCitation { pub item: CommentaryItemId, pub chars: CharSpan, pub cites: TextLocus, pub cites_last: Option<TextLocus>, pub target_display: String, pub provenance: ProvenanceId }
pub struct CharSpan { pub start: u32, pub end: u32 }
```
- `CommentaryPlace` is where a comment stands in Kretzmann's own work: the book and chapter, the first verse he wrote it on, and its order among his comments on that chapter (`unit`, the parser's document order, compiled once; `ordinal_of`'s id parsing dies with `chapter_commentary`). `opens_section` holds exactly for the first comment of each run under one heading (the data-only form of the client's `IsSectionStart`). `CorpusRole` gains `Commentary` (neither the norming Scripture nor the normed confessions; the attestation law does not run over it).
- Containers, minted by `kretzmann_adapter::normalize` through `corpus_root::mint::<KretzmannTag>`: `Container:kretzmann` ("Kretzmann, Popular Commentary of the Bible", the existing `KRETZMANN_SOURCE_LABEL` wording) ⊃ `Container:kretzmann-book-{CODE}` ⊃ `Container:kretzmann-chapter-{CODE}-{n}` ⊃ that chapter's `CommentaryItem`s in `unit` order. Rows: `Contains<KretzmannTag>` (a new `RowFamily::ContainsKretzmann`, Kretzmann section). Container ids are minted by the compiler (the tool), never by served code.
- `ChapterCommentsOn` (`RowFamily::ChapterCommentsOn`, Kretzmann section, relation `CommentsOn`): each Kretzmann chapter `comments-on` the Bible chapter container it explains, so a Bible chapter's `commented-on-by` group names Kretzmann's chapter on it. This is the work's own titling ("Genesis, chapter 1"), a domain fact, and the only way the reader reaches the commentary without composing an id (rule 25).
- `CommentaryCitation` (`RowFamily::CommentaryCitation`, Kretzmann section, relation `Cites`): one row per Scripture citation in a comment's prose, its characters compiled (`chars`). A citation of a span ends at FOCUS-3's passage container for that span [F3 T4], exactly as a Concord citation does. Commentary has no token layer (words-as-base stays deferred), so the span is characters, compiled, never re-scanned.
- Labels (`labels::node_label`, OPEN 5 default): a comment is `Kretzmann on {first verse reference}` or `Kretzmann on {first}-{last}` from its `CommentsOn` row through `references::unit_reference` [F2]; a Kretzmann chapter is `Kretzmann on {Bible chapter label}`; a book `Kretzmann on {book label}`. F-51's `"Commentary"` placeholder and the client's default die.
- Compiler (`server/atlas-graph/src/citations.rs`):
  ```rust
  pub fn cite_scripture(graph: &mut Graph) -> CitationStats;
  fn cite_prose(text: &str) -> Vec<Citation>;
  ```
  `cite_scripture` keeps its signature and now walks every prose corpus: the Concord's reading spine (as today, writing `CrossRef` rows) and every `CommentaryItem` (writing `CommentaryCitation` rows), through the one `scan`. `CitationStats` gains `commentary_cited` and `commentary_unplaced` (OPEN 6: an unplaced citation stays plain text; the close report lists them from the stats).
- Laws (24b): `every_comment_sits_in_exactly_one_kretzmann_chapter_in_document_order`; `every_kretzmann_chapter_comments_on_the_bible_chapter_its_comments_explain`; `container_containment_is_a_forest` holds with the third root [F3's restated law]; `every_commentary_citation_spans_characters_inside_its_comment`; `grounds_of` answers the three new families (no wildcard, [F4]); `SECTION_SCHEMA_VERSION` +1 over the value Task 0 records.

### Commentary is text on the wire (Task 4; backend: wire)

Wire (`server/atlas-contract/src/wire/locus.rs`, `wire/graph.rs`):
```rust
pub enum TextRef {
    Bible { book: BookId, chapter: u16, verse: u16 },
    Concord { part: u8, article: u16, paragraph: u16 },
    Commentary { book: BookId, chapter: u16, verse: u16, unit: u16 },
}

pub enum UnitHeading {
    Pericope { event: NodeRef, kind: EventKind, is_continuation: bool },
    Section { title: String },
}
```
- `TextRef::Commentary` is a comment's `CommentaryPlace` on the wire. `UnitHeading` becomes a closed sum written by `union::tagged_by` (tag `heading`; cases `PericopeHeading`, `SectionHeading`); today's struct is the `Pericope` arm unchanged. A comment whose `opens_section` holds carries `Section { title }`. AQC **major** (ruling 9 by analogy).
- `NodeRecord.text: Option<UnitText>` [F2] is present exactly for a text unit **or a commentary item**; FOCUS-2's law `no_record_but_a_text_units_carries_text` becomes `exactly_text_units_and_comments_carry_text`. `unit_text` [F2] answers a `CommentaryItem` from its payload (`text`, `place`) and its `CommentaryCitation` rows (anchors of kind `cites`, characters read from the row, labels compiled), one index lookup per unit (27b), batched per window (27c).
- `node_text` [F3] (`GET /api/node/{id}/text`): a container's units are its `contains` neighbours that carry text (text units and comments); a Kretzmann chapter answers its comments as `TextUnit` rows (`ref` = the comment's compiled reference [F2 `references`, Commentary arm: its id's local part], `node`, `body`, `heading`, `edge_summary`). Same cursors, same cap, same refusals.
- `node_description`'s `CommentaryItem` arm is deleted (the prose is `text`; D.R.Y.).

### The client (Tasks 2, 5, 6, 7)

`client/Exploring/Presenter.cs` (`GraphPresenter.CardOf`), new constants and the catechism rows, read from `element.Record?.Catechism`:
```csharp
private const string ChiefPartField = "Chief part";
private const string WordsField = "Text";
private static IEnumerable<Presentation.Field?> CatechismOf(CatechismDetail? detail);
```
- `CatechismOf` yields, in order: `Chief part` = `detail.Part`; `Text` = `detail.Text` (only where served); `{detail.Explanation.Heading}` = `detail.Explanation.Body`; `{detail.WhereWritten.Heading}` = `detail.WhereWritten.Body` (only where served). The two variable names are served words, never client literals. `CardOf` places them before `Provenance`.
- FocusView: `.focus-field dd { white-space: pre-line; }` in `client/wwwroot/css/app.css`, so served paragraph breaks show without the client splitting text (rule 25).

`client/Exploring/Presentation.cs` (R16 rows; HomeSurfaces follow from the table):

| Kind | `Surface.World` | `Surface.Reader` | `Surface.Popover` |
|---|---|---|---|
| CatechismItem | `null` | `null` | `Card` (unchanged): Chief part, Text, the explanation, where it is written, Provenance |
| CommentaryItem | `null` | **`Text`** (was `null`) | **`Text`** (was `Card`): its served `UnitText`, Provenance |

`HomeSurfaces.Of(CommentaryItem)` becomes `Reader` by the table; the reader for its locus is the Kretzmann page (`ReadingAddress`, below).

`client/Reading/ReadingAddress.cs` [F3], one arm added:
```csharp
public static ReadingPlace? Kretzmann(Contents bible, TextRef.Commentary place);
```
- It answers the Bible place for the comment's book and chapter by the served locus (FOCUS-3's lookup, no id composed), and `Route` writes `/kretzmann/{book}/{chapter}` for it. The Kretzmann page gains that route (`@page "/kretzmann/{Book}/{Chapter:int}"` beside `/kretzmann`), read only through `ReadingAddress` (`RouteLawTests` [F3] lists it). FocusView's reader hatch [F2 ruling 4b] offers `popover-chip-context` on a comment through it.

`client/Views/SequenceView.razor` [F3], two parameters added:
```csharp
[Parameter] public EventCallback<Link> OnChildOpened { get; set; }
[Parameter] public EventCallback<Link> OnChildClosed { get; set; }
```
- Fired when a child's text window opens (its head enters the viewport margin) and when it stops (`PageWindow.Stop`), the two moments FOCUS-3 already defines. Without a delegate nothing changes for the Reader or the Concord.

`client/Reading/Commentary.cs` (new; the Kretzmann page's column):
```csharp
public sealed class Commentary
{
    public Commentary(IExplorer explorer, Explorable scope);
    public Task Open(Link chapter);
    public void Close(Link chapter);
    public IReadOnlyList<TextUnit> On(TextRef.Bible verse);
    public event Action? Changed;
}
```
- `Open(chapter)` is one walk through `Explore`: follow the Bible chapter, take the first link of its `commented-on-by` group (Kretzmann's chapter on it), and open `Paging.Text(scope, that link)` [F3]. No id is composed; a chapter with no such link has no commentary (the page shows `kretzmann-empty`).
- `On(verse)` answers the resident comments whose served `TextRef.Commentary` has the verse's `(Book, Chapter, Verse)`, in served order. That is equality on served integers, never a parse (rule 25; the FOCUS-6 `Crossing` precedent).
- **Keeping pace (interaction, the client's under rule 27):** a chapter's comment window asks `More()` while its last resident comment's verse precedes the last resident verse row's and it has a next page; `Fewer()` mirrors the verse window. Each window is FOCUS-3's bounded `PageWindow` (20 a page, 40 resident), so the column's resident comments are bounded by the open chapters (27e).
- `Close(chapter)` stops that window.

## Deletion inventory (FOCUS-7 total, with OPEN defaults)

- **Client, files:** `client/Legacy/CatechismNode.cs`, `client/Legacy/CommentaryItemNode.cs`; `client/Components/CatechismList.razor`, `client/Components/ConcordUnitList.razor`; `client/Exploring/KretzmannCitationScan.cs`; `client/wwwroot/js/lazyProse.js`; tests `client.Tests/KretzmannCitationScanTests.cs`.
- **Client, members:** `CatechismTextSection`, `CatechismExplanationSection`, `CatechismWhereWrittenSection`, `CatechismScripturesSection`, `CatechismInConcordSection`, `CommentaryItemProseSection`, `CatechismSectionRendering`, `CatechismLinks` and their six `PopoverSectionRegistry` rows (`PopoverSections.cs:52–56, 72`); `AtlasClient.CatechismItem`, `AtlasClient.KretzmannChapter` and their `AtlasClientTests` cases (`:230`, `:302`); `LegacyNodes.For`'s `CatechismItem` and `CommentaryItem` arms (→ `null`); `PopoverChromeRegistry["Catechism"]`, `["CommentaryItem"]`; in `Kretzmann.razor`: `CommentaryItemRow`, `MergedVerseRow`, `ShowChapter`'s commentary half, `LoadCommentaryAsync`/`ShowItemText`/`UpdateItemText`/`ProseOf`, `_proseRequested`, `_proseFetchGate`, `_lazyJs`, `_selfRef`, `OnExplore(CommentaryItemRow)`, `OnItemKeyDown`, the `Atlas.KretzmannChapter` and `Atlas.ChapterText` fetches, `EmptyToc`, `Slug` (if its last use goes); the `PopoverSectionRegistryTests` (`:149–154, 174`), `LegacyNodesTests` (`:54, 58, 89–90, 107–108`), `LegacyViews` (`:22–23`), `PushViaConformanceTests` (`:23, 34`) rows; `ExplorerPopoverTests`' three `CommentaryItemNode` fixtures (`:83, 141, 236`) re-expressed on `PolityDeltaNode`, the legacy node MAPS keeps; `IExplorableClient.Card`/`GraphExplorableClient.Card` when FOCUS-4 has landed (its last callers die here and there); CSS `.popover-catechism-*`, `.popover-commentary-text`, `.kretzmann-item-loading`, `.catechism-section-heading` where nothing else uses them; the FOCUS-7 entries of `ReferenceParsingLawTests.RetiredBy` and `WholeReadLawTests.RetiredBy` [F2].
- **Server (Task 8):** `catechism.rs` `catechism_item` and its route (the file, its `routes()` and the `lib.rs` merge once FOCUS-3 T9 has removed `catechism_for_span`), the `"catechism"` tag in `document.rs:30` when no route carries it; `wire/catechism.rs` `CatechismItem`, `CatechismProofVerse` (the file and its `wire/mod.rs` lines once `CatechismRef` is gone [F3]); `reading.rs` `kretzmann_chapter` and its route; `wire/reading.rs` `KretzmannChapter`, `KretzmannChapterVerse`, `KretzmannChapterItem`; `kretzmann_adapter::chapter_commentary`, `ChapterCommentaryRow`, `ordinal_of`; `atlas-core` `AtlasData::catechism_item_by_id` and `GraphService::verse_text_of` if no served reader remains (`cargo build` decides; a tool reader is noted); tests `api.rs` `catechism_span_and_item_endpoints` (its item half), `kretzmann_adapter_real_data.rs` `chapter_commentary_*` (two), `graph_api.rs` `commentary_item_record_carries_its_own_real_kretzmann_prose_via_description` (→ the `text` law); `benches/queries.rs` and `server/BENCHMARKS.md` rows; AGC `graph/detail-routes.feature` lines 24–25 and 31–33 and the `kretzmann-chapter`, `catechism-item` projections; fixtures `contracts/atlas-graph-contract/fixtures/{kretzmann-chapter-gen-1,catechism-item-commandment-1}.json`; pact interactions; `tests/ux/lib/api.ts` `catechismItem`, `kretzmannChapter`.
- **Under OPEN 4 "keep the read" only:** `/api/kretzmann/chapter/{cref}` stays; Tasks 3, 4 and 6 shrink to the citations and the prose, and `chapter_commentary` stays a FINDING (27b, 26a).
- **Not deleted** (stated so no one "finishes" it): `LegacySaves`' v1 `"Catechism"`/`"CommentaryItem"` translations (FOCUS-9); `/api/text?ref=` and `AtlasClient.ChapterText` (their FOCUS-5 callers, `MiniReaderExpand` and `VerseTextResolver`, remain; FOCUS-7 removes the Kretzmann and catechism callers); `/api/node/{id}` and `AtlasClient.NodeRecord` (F-34; `AuthorNode`, `EventNode`); `PassageList`, `PassageBlock` (FOCUS-5); `AtlasData.catechism` (the compiler still reads it); the `catechism-link` rows from the brain-fuel question mapping (O-CATECHISM decides them, not this batch).

---

### Task 0: Base, preconditions and the ledger (no code)

**Files:** `.superpowers/sdd/2026-10-0x-focus7/progress.md` (create).

- [ ] **Step 1:** Record the base (the head FOCUS-3 landed at, with FOCUS-2 under it), FOCUS-4's state (landed or not), FOCUS-3's owner answers that Task 6 follows (OPEN 6, 11, 12, 13 there), the OPEN answers here (or "defaults"), and `SECTION_SCHEMA_VERSION` at the base.
- [ ] **Step 2:** Re-verify at the base and record each result; stop for the controller on any difference:
  - `grep -rn "CatechismNode\|CommentaryItemNode\|CatechismLinks\|CatechismList\|ConcordUnitList\|KretzmannCitationScan\|KretzmannChapter\|CatechismItem(" client client.Tests --include=*.cs --include=*.razor` prints only this plan's sites (FOCUS-2 and FOCUS-3 have removed `CatechismSeamSection`, `ConcordSmallCatechismSection`, `PassageCatechismSection`);
  - FOCUS-2's `UnitText`, `NodeRecord.text`, `unit_text`, `references.rs`, `UnitTextView`, `Presentation.Text`, the reader hatch, `LegacyNames` and both ratchets exist; FOCUS-3's `node_text`, `TextPage`, `Paging.Text`, `SequenceView`, `ReadingAddress`, `RouteLawTests` and passage containers exist; `catechism_for_span` and `CatechismRef` are gone;
  - `Kretzmann.razor` reads verse rows from `Atlas.ChapterText` and commentary from `Atlas.KretzmannChapter` (FOCUS-3 T6's stated end state);
  - `ContainerContent<C>` admits a member that is not a text unit (`graph-types/src/edge.rs`). If not, Task 3 uses a sibling family `ContainsCommentary { container, item, provenance, justification }` and says so in the ledger.

### Task 1: The catechism's headings are data, and the record serves each heading with its words (backend)

**Backend change: yes (data, ETL, wire). Why there (rules 26, 27):** a heading is a fact of the source text, so it lives in `data/` with its provenance; the record is the one read of an item's words, so the reshape is the server's; nothing is computed per request.

**Files:**
- Modify: `data/curated/catechism.toml` (the two file-level headings); `server/atlas-etl/src/curated.rs` (`parse_catechism` applies them; fails loud without them); `server/atlas-core/src/data.rs` (`TitledText`, `CatechismItem`; `default_explanation_heading` deleted); `server/atlas-contract/src/wire/graph.rs` (`TitledText`, `CatechismDetail`); `server/atlas-contract/src/graph.rs` (`catechism_detail`); `server/atlas-contract/src/catechism.rs` (`catechism_item` maps the new fields until Task 8 deletes it); `data/compiled` (rebuilt); `contracts/*` regen, `contracts/atlas-query-contract/CHANGELOG.md` (AQC major); `contracts/atlas-query-contract/fixtures/focus-catechismitem.json` (regenerated); pacts; `client.Tests/Explore/ServedGraph.cs` (gains `CatechismDetailOf(...)`).
- Test: `server/atlas-etl/src/curated.rs` unit tests; `server/atlas-contract/tests/graph_api.rs`; `server/atlas-contract/tests/contract_generation.rs`.

- [ ] **Step 1: Failing tests.**
  - `curated.rs`: `an_item_without_its_own_question_takes_the_files_explanation_heading`; `an_item_that_poses_its_own_question_keeps_it`; `an_item_that_says_where_it_is_written_takes_the_files_heading`; `a_catechism_file_without_its_headings_fails_the_compile_naming_the_key`.
  - `graph_api.rs` (expectations read from the artifact's `catechism.json`, F-8): `a_catechism_record_serves_each_heading_with_its_words` (whole `CatechismDetail` for `commandment-1`), `a_baptism_record_serves_its_own_question_and_where_it_is_written` (whole detail for `baptism-1`), replacing `a_catechism_record_carries_its_prose` and `a_catechism_item_that_quotes_scripture_says_where_it_is_written` (`:2111`, `:2147`).
  - `contract_generation.rs`: the document's `CatechismDetail` has `part`, `text`, `explanation`, `where_written` and no `explanation_heading`/`part_title`.
- [ ] **Step 2:** `cargo test -p atlas-etl -p atlas-contract` → red; record the reds.
- [ ] **Step 3: Implement** as in Types.
- [ ] **Step 4: Rebuild and regenerate (critical section `contract`):** rebuild `data/compiled`; `cargo run -p atlas-contract --bin export_contract`, `--bin export_aqc_examples`, `--check` clean; `CHANGELOG.md` AQC **major**; re-bless pacts; `dotnet run --project client.ContractGenerator`; the one client reader (`CatechismNode`'s legacy sections read `CatechismItem` from the legacy route, untouched) compiles. Release the lock.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace`, `(cd graph-types && cargo test --all-features)`, `bash scripts/contract-gate.sh --base <base>`, `bash scripts/contract-semver-gate.sh` (declared major), `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green except `GeneratedUsageTests` for `CatechismDetail.Part`, `.Explanation`, `.WhereWritten`, `TitledText` (read in Task 2). **Commit:** `catechism: Luther's headings are data, and an item's record serves each heading with its words (rules 25, 26; AQC major)`.

### Task 2: A catechism item on FocusView; the legacy catechism path is gone (client)

**Backend change: no.** Rule 27: which fields a card shows and how a link list is offered are the client's interaction derivations over served data.

**Owner gate first:** OPEN 1, 2, 3 (or the defaults).

**Files:**
- Modify: `client/Exploring/Presenter.cs` (`CatechismOf`, two constants); `client/wwwroot/css/app.css` (`.focus-field dd`; deletes `.popover-catechism-*`, `.catechism-section-heading` where unused); `client/Legacy/{PopoverSectionProviders,PopoverSections,LegacyNodes}.cs`; `client/Exploring/PopoverChromeRegistry.cs`; `client/AtlasClient.cs`; `client.Tests/Explore/{GraphPresenterTests,DeletionLawTests,LegacyNodesTests,LegacyViews,PushViaConformanceTests}.cs`, `client.Tests/{PopoverSectionRegistryTests,AtlasClientTests,ReferenceParsingLawTests,WholeReadLawTests,PopoverChromeConformanceTests}.cs`.
- Delete: `client/Legacy/CatechismNode.cs`, `client/Components/CatechismList.razor`, `client/Components/ConcordUnitList.razor`.
- Test: `client.Tests/Explore/GraphPresenterTests.cs`, `client.Tests/Views/FocusViewTests.cs`, `client.Tests/Explore/DeletionLawTests.cs`.

- [ ] **Step 1: Failing tests.**

`client.Tests/Explore/GraphPresenterTests.cs`:
```csharp
[Fact]
public async Task A_catechism_item_on_the_popover_shows_its_chief_part_its_words_and_each_served_heading_with_its_answer()
{
    // Arrange
    var item = await Resolved.Node(NodeKind.CatechismItem, BaptismOne, BaptismOneLabel)
        .With(ServedGraph.CatechismDetailOf(Baptism, null, new TitledText(WhatIsBaptism, BaptismAnswer), new TitledText(WhereIsThisWritten, MatthewQuoted)))
        .Using(new ServedGraph());
    // Act
    var presented = await new GraphPresenter().Present(new PresentationRequest(item, Surface.Popover));
    // Assert
    Assert.Equal(
        new Presentation.Card(BaptismOneLabel,
        [
            new Presentation.Field(ChiefPartField, Baptism),
            new Presentation.Field(WhatIsBaptism, BaptismAnswer),
            new Presentation.Field(WhereIsThisWritten, MatthewQuoted),
            new Presentation.Field(ProvenanceField, CuratedCatechism),
        ]),
        presented);
}
```
Also: `A_commandment_shows_its_text_before_its_explanation` (whole card for a served item with `text`); `An_item_that_says_nowhere_where_it_is_written_shows_no_such_field`.
`FocusViewTests`: `A_field_keeps_its_served_line_breaks` (whole rendered `dd` markup, no split); `A_catechism_item_offers_its_verses_and_its_concord_paragraphs_as_one_list` (a served item whose `catechism-link` group holds two verses and one Concord paragraph; the ordered `popover-link-catechism-link-{id}` ids in served order, one `popover-section-catechism-link`).
`DeletionLawTests`: `MigratedKinds` gains `NodeKind.CatechismItem`; `LegacyNames[CatechismItem] = ["Catechism"]` (the legacy kind string `CatechismNode.Kind` returns, which the kind-name match would miss). Run → red (`CatechismNode`, five `"Catechism"` `AppliesTo` strings).
`ReferenceParsingLawTests`/`WholeReadLawTests` [F2]: remove the FOCUS-7 catechism entries (`CatechismScripturesSection`'s `CanonRef.ParseVerse`, `CatechismInConcordSection`'s `NodeIds.LocalPart`/`NodeIds.Of`, `ConcordUnitList`'s `NodeIds.LocalPart`, `CatechismLinks`' `Paging.Whole`, `LegacyNodes`' catechism arm); their pair laws fail until the sites are deleted.
- [ ] **Step 2:** `dotnet test client.Tests --filter "GraphPresenterTests|FocusViewTests|DeletionLawTests|ReferenceParsingLawTests|WholeReadLawTests"` → red.
- [ ] **Step 3: Implement.** `CatechismOf` as in Types. Delete the files and members in the inventory for catechism; `LegacyNodes.For` answers `null` for `CatechismItem`, so every catechism item, opened or followed, renders on FocusView. Its `catechism-link` list is FocusView's generic `SectionList` (`Affordances.DefaultList` [F4 shape]), paged 20, 40 resident; no client filter, no question headings (OPEN 2), no verse words in the list (OPEN 1). Under OPEN 1 "print the words": `SectionList` gains `Preview { Label, Text }` with `CatechismLink → Text`, and FocusView reads one batched element read per page for entries that are text units (27c), with its own failing test first.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green (`GeneratedUsageTests` now reads `CatechismDetail.*` and `TitledText`). `npx playwright test tests/ux/popover-sections.spec.ts tests/ux/concord.spec.ts` → record the red set by name (expected: CATECH-1 "verse -> catechism item -> proof verse hop", D3 ×2).
- [ ] **Step 5: Commit:** `catechism: an item is FocusView's card and frontier; CatechismNode, its five sections, CatechismList and ConcordUnitList are gone (deletion law: CatechismItem; F-63, F-65)`.

### Task 3: Kretzmann's structure, citations, headings and labels are compiled (backend)

**Backend change: yes (compiler and graph types; no served code). Why there (rule 27, 26a):** the books and chapters of the work, the order of its comments, where each citation in its prose falls, which comment opens a section and what a comment is called are facts over the data alone, so the compiler writes them once. The served system stops walking verses and parsing ids (Task 8).

**Owner gate first:** OPEN 4, 5, 6 (or the defaults).

**Files:**
- Modify: `graph-types/src/{text,node,edge,graph,sections}.rs`, `graph-types/src/canon/{node,rows}.rs` (`KretzmannTag`, `CommentaryPlace`, `CorpusRole::Commentary`, the payload, `ChapterCommentsOn`, `CommentaryCitation`, `CharSpan`, three `RowFamily` members in the Kretzmann section, `SECTION_SCHEMA_VERSION` +1); `server/atlas-etl/src/kretzmann.rs` (each `KretzUnit` carries its `unit` order; no other change to the parser); `server/atlas-graph/src/{kretzmann_adapter,corpus_root,citations,labels,law_check,event_world,provenance}.rs`; `server/atlas-graph/src/sqlite/{ddl,partition,writer,snapshot}.rs` and `sqlite/rows/*` (the three families' tables); `data/compiled` (rebuilt); `graph-types/tests/canon_vectors.rs` and `canon_row_vectors.rs` (re-pinned under `contract`); pacts.
- Test: `server/atlas-graph/src/{kretzmann_adapter,citations,labels}.rs` unit tests; `server/atlas-graph/tests/{kretzmann_adapter_real_data,sqlite_laws,sections_real_data}.rs`; graph-types store laws.

- [ ] **Step 1: Failing tests.**
  - `kretzmann_adapter.rs` (over `tiny_corpus`, whole values): `normalizing_files_each_comment_under_its_chapter_its_book_and_the_work`; `a_kretzmann_chapter_comments_on_the_bible_chapter_it_explains`; `a_comment_records_its_place_in_the_work`; `only_the_first_comment_under_a_heading_opens_a_section`.
  - `citations.rs`: `a_citation_in_a_comment_is_compiled_with_its_characters_and_the_verse_it_cites`; `a_citation_of_a_span_in_a_comment_ends_at_its_passage` [F3 T4]; `the_concord_and_kretzmann_are_cited_through_one_scanner` (the stats of a fixture holding one of each, whole `CitationStats`); `an_unplaced_citation_in_a_comment_is_counted_and_left_as_text`.
  - `labels.rs`: `a_comment_is_labelled_by_the_verses_it_explains`; `a_kretzmann_chapter_is_labelled_by_the_bible_chapter_it_explains`.
  - real data (`kretzmann_adapter_real_data.rs`, expectations from the artifact): `every_comment_sits_in_exactly_one_kretzmann_chapter_in_document_order`; `every_kretzmann_chapter_comments_on_the_bible_chapter_its_comments_explain`; `every_commentary_citation_spans_characters_inside_its_comment`; `psalm_119s_comments_are_its_kretzmann_chapters_contains_page_in_order` (the pages of `Container:kretzmann-chapter-PSA-119`'s `contains` equal the PSA 119 comments in `unit` order, the 24b walk replacing `chapter_commentary_serves_psalm_119_*`).
  - `law_check.rs`: `container_containment_is_a_forest` over three roots; a fixture with a comment in two chapters fails it.
  - store laws, both backends (`assert_answers_match`): the three families round-trip.
  - `event_world.rs` [F4]: the three families' `grounds_of` arms (`ChapterCommentsOn`: its grounds; `ContainsKretzmann`: its grounds; `CommentaryCitation`: none), in the `RowFamily::ALL` walk.
- [ ] **Step 2:** `cargo test -p atlas-etl -p atlas-graph` and `(cd graph-types && cargo test --all-features)` → red; record the reds.
- [ ] **Step 3: Implement** as in Types. `cite_scripture` iterates the Concord spine and every `CommentaryItem` through one `scan`; the abbreviation table stays the compiler's one declaration. Labels through `references::unit_reference` [F2] only. `ordinal_of` is untouched here (Task 8 deletes it with its caller).
- [ ] **Step 4: Rebuild (critical section `contract`):** rebuild `data/compiled`; re-pin the canon vectors; re-bless. Expect: the version root moves; `CitationStats` per corpus and the count of unplaced commentary citations go in the ledger (OPEN 6); no Bible or Concord label changes. If `scene_byte_identity` or a golden map fixture moves, **stop** for the owner (O-GOLDEN rule). Release the lock.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh --base <base>` → green (the vocabulary gate and `no_served_label_composition` included). **Commit:** `kretzmann: the commentary is filed under Kretzmann's own books and chapters in his order, each chapter comments on its Bible chapter, and the citations in his prose are compiled by the one scanner (rule 27; F-51)`.

### Task 4: A comment is text on the wire (backend)

**Backend change: yes (wire and the two existing reads). Why there (rule 27a/27b):** the record and the container text page are the generic reads that already serve text; a comment joins them as one index lookup per unit. No new route, no view-shaped field (a comment's locus and heading are its own, compiled).

**Files:**
- Modify: `server/atlas-contract/src/wire/locus.rs` (`TextRef::Commentary`); `server/atlas-contract/src/wire/graph.rs` (`UnitHeading` sum); `server/atlas-contract/src/graph.rs` (`unit_text` and `node_text` [F2/F3] answer comments; `node_description`'s `CommentaryItem` arm deleted); `server/atlas-graph/src/references.rs` [F2] (the Commentary arm); `contracts/*` regen, `CHANGELOG.md` (AQC major), AQC features and `client.ContractTests/Steps/AqcSteps.cs` where they read `UnitHeading`; pacts; the client sites that read `TextUnit.Heading`'s fields (`PericopeHeading.razor`, `VerseLine.razor`, `SequenceView.razor` [F3]) → one `Match` (mechanical, so the client keeps compiling).
- Test: `server/atlas-contract/tests/graph_api.rs`, `server/atlas-contract/tests/contract_generation.rs`.

- [ ] **Step 1: Failing tests** (`graph_api.rs`, expectations from the artifact):
  - `a_comments_record_serves_its_prose_with_its_compiled_citations` (whole `UnitText` for GEN 1's first comment: locus, text, anchors equal to its `CommentaryCitation` rows).
  - `exactly_text_units_and_comments_carry_text` (walks one node of every `NodeKind`; replaces FOCUS-2's `no_record_but_a_text_units_carries_text`).
  - `a_kretzmann_chapters_text_pages_are_its_comments_in_order` (the union of `GET /api/node/Container:kretzmann-chapter-GEN-1/text` pages equals its `contains` neighbours, whole rows).
  - `a_comment_that_opens_a_section_carries_its_heading_and_no_other_does`.
  - `a_bible_chapter_names_kretzmanns_chapter_on_it` (`commented-on-by` page of `Container:bible-chapter-GEN-1`: one entry, the Kretzmann chapter, with its compiled label).
  - `a_pericope_heading_reads_as_before_under_its_tag` (the same verse's heading before and after, whole).
  - `contract_generation.rs`: the document holds `TextRef`'s three cases and `UnitHeading`'s two.
- [ ] **Step 2:** `cargo test -p atlas-contract --test graph_api` → red.
- [ ] **Step 3: Implement** as in Types.
- [ ] **Step 4: Regenerate (critical section `contract`, after Task 1's):** export, `--check`, AQC **major**, re-bless, client generator, the mechanical `Match` edits. Release the lock.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh --base <base>`, `bash scripts/contract-semver-gate.sh` (declared major), `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green except `GeneratedUsageTests` for `TextRef.Commentary` and `UnitHeading.Section` (read in Tasks 5, 6). `npx playwright test tests/ux --grep "reader|concord"` → the base's set (the reshape changes no behaviour). **Commit:** `kretzmann: a comment's record and its chapter's text pages serve its prose with compiled citations; a heading is a pericope or a section (rule 27a; AQC major)`.

### Task 5: A comment on FocusView, with "read in Kretzmann" (client)

**Backend change: no.** Rule 27: the presentation table and the hatch are interaction derivations.

**Files:**
- Modify: `client/Exploring/Presentation.cs` (CommentaryItem rows); `client/Reading/ReadingAddress.cs` [F3] (`Kretzmann`, the route); `client/Pages/Kretzmann.razor` (the route attribute and its read through `ReadingAddress` only); `client/Views/FocusView.razor` (the hatch for a comment, through the existing reader hatch [F2]).
- Test: `client.Tests/Explore/{PresentationTests,GraphPresenterTests}.cs`, `client.Tests/Views/FocusViewTests.cs`, `client.Tests/Reading/ReadingAddressTests.cs` [F3], `client.Tests/RouteLawTests.cs` [F3].

- [ ] **Step 1: Failing tests.**
  - `PresentationTests`: the table test now expects `(CommentaryItem: World null, Reader Text, Popover Text)`; `HomeSurfaces.Of(CommentaryItem) == Reader`.
  - `GraphPresenterTests`: `A_comment_on_the_popover_presents_its_served_prose_with_its_citations` (whole `Presentation.Text`); `A_comment_served_without_its_text_is_a_contract_breach`.
  - `FocusViewTests`: `A_comment_offers_to_be_read_in_kretzmann` (`popover-chip-context` targets `/kretzmann/GEN/1` from a served `TextRef.Commentary` and a served Bible contents fixture).
  - `ReadingAddressTests`: `A_comments_place_is_the_bible_chapter_it_sits_in`; `A_kretzmann_route_round_trips_every_bible_chapter_of_the_served_tree`.
  - `RouteLawTests`: `/kretzmann/` joins the scanned route literals; only `ReadingAddress.cs` writes or reads it.
- [ ] **Step 2:** red. **Step 3: Implement** as in Types.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green except `UnitHeading.Section` (Task 6).
- [ ] **Step 5: Commit:** `kretzmann: a comment presents as its served text on the popover and the reader, and offers to be read in Kretzmann (R16; ruling 4b by analogy)`.

### Task 6: The Kretzmann page reads the graph (client)

**Backend change: no.** Rule 27: which verse a comment sits under, keeping the two windows in pace, the scroll and the focus are the client's; every read is a generic one (the element read, a neighbour page, a text page).

**Starts after** FOCUS-3's Task 6 and this batch's Tasks 4 and 5 have landed.

**Files:**
- Create: `client/Reading/Commentary.cs`; `client/Components/KretzmannVerseRow.razor` (a verse's `VerseLine` and, under it, its comments: a `Section` heading once where served (`kretzmann-section-heading-{node slug}`), then `UnitTextView` [F2] with `Handle="kretzmann-item-{node slug}"`, the row opening `PopoverOpening.Explore(new NodePosition(unit.Node))` on click or Enter).
- Modify: `client/Pages/Kretzmann.razor` (rewritten over `SequenceView Handle="kretzmann"` with `Row` = `KretzmannVerseRow`, `OnChildOpened`/`OnChildClosed` → `Commentary.Open`/`Close`; keeps its split, follow chip, picker and selection wiring as FOCUS-3 left them); `client/Views/SequenceView.razor` [F3] (the two callbacks).
- Test: `client.Tests/Reading/CommentaryTests.cs`; `client.Tests/Views/SequenceViewTests.cs` [F3]; `tests/ux/kretzmann.spec.ts` (Task 9 re-expresses the rest).

- [ ] **Step 1: Failing tests.**
  - `CommentaryTests` (over `ServedGraph` serving a Bible chapter, its `commented-on-by` link and a Kretzmann chapter's text pages; whole values):
    - `Opening_a_chapter_reads_kretzmanns_chapter_on_it_through_its_commented_on_by_link` (the `ServedGraph` records one element read, one neighbour page and one text page; no id composed);
    - `A_verse_shows_the_comments_that_sit_at_it_in_served_order`;
    - `A_chapter_without_commentary_shows_none`;
    - `The_comment_window_keeps_pace_with_the_verse_window` (verse rows 1–40 resident, comments on verses 1–60 served over three pages: exactly the comments on 1–40 resident and two text pages read);
    - `Closing_a_chapter_stops_its_comment_window`;
    - `Resident_comments_stay_bounded_however_far_the_reader_scrolls` (the 27e law at 300/3,000/30,000 comments, as FOCUS-3's growth law).
  - `SequenceViewTests`: `A_child_reports_when_its_text_window_opens_and_when_it_stops`.
  - `kretzmann.spec.ts` KRETZ-GRAPH-1 (new, live): on `/kretzmann/PSA/119` the network log holds no request to `/api/kretzmann/`, `/api/catechism/`, `/api/text?ref=` or `/api/node/CommentaryItem:`; commentary requests are text pages of `Container:kretzmann-chapter-PSA-119` only, at most `ceil(resident comments / 20)` of them.
- [ ] **Step 2:** red.
- [ ] **Step 3: Implement** as in Types. The page's `ShowChapter` commentary half, the lazy prose observer, the per-item `Graph.Card`, the heading comparison and `KretzmannCitationScan` leave `Kretzmann.razor`; `Atlas.ChapterText` leaves it (its verse rows come from `Paging.Text` through `SequenceView`); the page's entries leave FOCUS-3's `/api/text?ref=` ratchet.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green (`GeneratedUsageTests` reads `UnitHeading.Section`). `npx playwright test tests/ux/kretzmann.spec.ts tests/ux/reader.spec.ts tests/ux/split-view.spec.ts tests/ux/composition.spec.ts` → record the red set by name (expected: KRETZMANN-2b, -4, -10, -13, -14, -15, the `kretzmann-ref-*`/`data-loaded` steps).
- [ ] **Step 5: Commit:** `kretzmann: the page reads a Bible book as one scroll with each verse's comments under it, read as text pages of Kretzmann's chapter (R12; FOCUS-3 OPEN 2, 11; 27c, 27e)`.

### Task 7: The legacy commentary path is gone (client)

**Backend change: no.**

**Files:**
- Delete: `client/Legacy/CommentaryItemNode.cs`, `client/Exploring/KretzmannCitationScan.cs`, `client/wwwroot/js/lazyProse.js`, `client.Tests/KretzmannCitationScanTests.cs`.
- Modify: `client/Legacy/{PopoverSectionProviders,PopoverSections,LegacyNodes}.cs`, `client/Exploring/PopoverChromeRegistry.cs`, `client/AtlasClient.cs`, `client/wwwroot/css/app.css`, `client/Exploring/IExplorableClient.cs` and `client/GraphExplorableClient.cs` (`Card` deleted if FOCUS-4 has landed), `client.Tests/Explore/{DeletionLawTests,LegacyNodesTests,LegacyViews}.cs`, `client.Tests/Components/ExplorerPopoverTests.cs`, `client.Tests/{AtlasClientTests,ReferenceParsingLawTests,PopoverChromeConformanceTests}.cs`, `client/BibleAtlas.Client.Exploring.csproj` only if it lists the scanner.

- [ ] **Step 1: Failing law:** `DeletionLawTests.MigratedKinds` gains `NodeKind.CommentaryItem` (`LegacyNames[CommentaryItem] = ["CommentaryItem"]`); `ReferenceParsingLawTests` loses `KretzmannCitationScan.cs` and the Kretzmann page's `NodeIds.LocalPart`. Run → red.
- [ ] **Step 2: Delete** the inventory's commentary half; re-express `ExplorerPopoverTests`' three legacy fixtures on `PolityDeltaNode` (behaviour unchanged: they test the legacy opening, which MAPS keeps).
- [ ] **Step 3:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green; `grep -rn "Paging.Whole" client` prints only sites listed for later batches (none at all if FOCUS-5 has landed: F-63 closed).
- [ ] **Step 4: Commit:** `kretzmann: CommentaryItemNode, its prose section and the client citation scanner are gone (deletion law: CommentaryItem; rule 25, 14b; F-65, F-66)`.

### Task 8: `/api/catechism/item/{id}` and `/api/kretzmann/chapter/{cref}` are gone (backend)

**Backend change: yes (deletions only). Why (rule 27a, 27b, 26a):** both are view-shaped reads whose content is now the generic record, neighbour page and text page; the second walked every verse with an unbounded page and parsed ids for order.

**Files:** the server half of the deletion inventory; `tests/ux/lib/api.ts`; `tests/ux/CONTRACT.md`.

- [ ] **Step 1: Failing test:** `contract_coverage.rs` `no_route_serves_a_catechism_item_or_a_kretzmann_chapter_view` (the published path set holds neither). Run → red.
- [ ] **Step 2: Delete** the handlers, wire types, `chapter_commentary`, `ChapterCommentaryRow`, `ordinal_of`, their tests, bench and BENCHMARKS rows, AGC feature lines, projections and fixtures, and pact interactions. `cargo build` decides whether `AtlasData::catechism_item_by_id` and `GraphService::verse_text_of` keep a served reader (`catechism_detail` reads the first, so it stays; record the decision for the second) and whether `catechism.rs`/`wire/catechism.rs` are empty (delete the files and their `lib.rs`/`wire/mod.rs`/`document.rs` lines if so).
- [ ] **Step 3: Regenerate (critical section `contract`, after Task 4's):** export, `--check`, AQC/AGC per `scripts/contract-semver-gate.sh` (removed routes), re-bless, client generator. Release the lock.
- [ ] **Step 4 (lock `heavy`):** `cargo test --workspace && (cd graph-types && cargo test --all-features) && dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh --base <base>` → green, the vocabulary gate and `no_served_label_composition` included. **Commit:** `catechism, kretzmann: the item and chapter views are gone; both are read as the graph's records, neighbours and text pages (rule 27a, 27b)`.

### Task 9: Re-express the specs, gates, close

- [ ] **Step 1: Re-express** each expected-red Playwright spec under the new behaviour, by name, keeping every surviving test id, reading every expectation from the served API (never a literal composed in the test). Test ids move:

  | Legacy id | FocusView / page id |
  |---|---|
  | `popover-section-catechism-text` | `popover-field-Text` |
  | `popover-section-catechism-explanation` + its `catechism-section-heading` | `popover-field-{served explanation heading}` |
  | `popover-section-catechism-where-written` + its heading | `popover-field-{served where-written heading}` |
  | `popover-section-catechism-scriptures`, `catechism-verse-{ref}`, `popover-section-catechism-in-concord`, `catechism-in-concord-heading`, `concord-link-{slug}` | `popover-section-catechism-link`, `popover-link-catechism-link-{id}` |
  | `popover-section-commentary-text` | `popover-text` |
  | `kretzmann-item-{slug}` with `data-loaded` | `kretzmann-item-{node slug}` (always loaded) |
  | `kretzmann-ref-*` (prose citations) | `kretzmann-item-{node slug}-anchor-cites-{id}-{n}` |
  | `kretzmann-section-heading-{slug}` | `kretzmann-section-heading-{node slug}` |

  Specs: `kretzmann.spec.ts` (KRETZMANN-2, -2b, -4, -10, -11, -12, -13, -14, -15, -19, -22; the rest must stay green unchanged), `popover-sections.spec.ts` CATECH-1 "verse -> catechism item -> proof verse hop", `concord.spec.ts` D3 ×2, `reader.spec.ts` NAV-STUTTER-2 (Kretzmann arm, `kretzmann-next`), `lib/api.ts`, `tests/ux/CONTRACT.md`. A spec whose behaviour an OPEN default removed is rewritten to what FocusView offers, and the removal is listed in the close report (question headings, OPEN 2; the verse words in the list, OPEN 1; the "IN THE BOOK OF CONCORD (n)" heading, ruling 3).

  **Acceptance (the batch is not done while any is red):**
  - **CAT-FV-1:** from MAT 28:19's popover, its `catechism-link` entry for `baptism-1` opens the item: `popover-title` is its served label; `popover-field-Chief part`, the served explanation heading's field and the served where-written heading's field show the served words, line breaks kept; no `popover-field-Text` (Baptism serves none); `popover-section-catechism-link` lists `text-unit:MAT.28.19`; following it shows MAT 28:19's `popover-text`, and Back returns to the item.
  - **CAT-FV-2 (D3):** from `/concord` the Small Catechism's First Commandment paragraph's popover reaches `commandment-1` through `catechism-link`, and the item's list holds that paragraph's served id; from GEN 1:1 a catechism item's list reaches a Concord paragraph.
  - **CAT-FV-3:** an item whose `catechism-link` group is longer than 20 shows 20, More shows 40, Less returns to 20 (the paging rulings).
  - **KRETZ-1:** `/kretzmann/GEN/1` shows every verse group of GEN 1 (`kretzmann-verse-group-{n}` for every served verse), each comment's served prose inline at first paint, each section heading once, where served.
  - **KRETZ-2:** a citation inside a comment is a link (`…-anchor-cites-…`); clicking it opens that verse on FocusView (`popover-text`).
  - **KRETZ-3:** clicking a comment opens its popover: `popover-title` is its compiled label ("Kretzmann on GEN.1.1", OPEN 5), `popover-text` its served prose; `popover-chip-context` from a comment opened elsewhere (a verse's `commented-on-by` entry) navigates to `/kretzmann/{book}/{chapter}`.
  - **KRETZ-4 (R12):** scrolling past GEN 1's last verse continues into GEN 2 on the same page with its comments; `kretzmann-next`/`kretzmann-prev` step chapters from `follows-in`; EXO 1 is reached only by the explicit `kretzmann-scope-next`.
  - **KRETZ-5 (27c):** KRETZ-GRAPH-1's network bounds hold for PSA 119 (replaces KRETZMANN-13's "one chapter-scoped request").
  - **KRETZ-6:** split, follow, release and the picker behave as before (KRETZMANN-5, -6, -6b, -6c, -7, -9, -9b green unchanged); Ctrl-click selects (KRETZMANN-21); a shift-click range selects its verses (KRETZMANN-19, per FOCUS-3 OPEN 6).
  - **NO-LEGACY-1:** across the whole suite, no request reaches `/api/catechism/` or `/api/kretzmann/`.
- [ ] **Step 2: Gates.**
  - `client.Tests/stryker-config.json` `mutate` gains `**/Reading/Commentary.cs`; `client.Tests/stryker-config.exploring.json` covers `**/Exploring/{Presentation,Presenter}.cs` (verify).
  - Mutation, inside the owner's window only (`heavy` with "mutation", `free -g` ≥ 18 GB): `bash scripts/mutants-parallel.sh -n 3 -b <base>` (covers the Kretzmann adapter, `cite_scripture`, labels, `unit_text`'s comment arm, `parse_catechism`), then `dotnet stryker`. Outside the window, `.superpowers/MUTATION-GATE-DEBT.md` names `<base>` as this batch's base.
  - Then `cargo test --workspace && (cd graph-types && cargo test --all-features) && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh --base <base> && bash scripts/timing-gates.sh run && npx playwright test tests/ux` → green, except the carried reds named by spec. The timing gates gain the Kretzmann chapter's text page at a full page, against FOCUS-3's 10× fixture if it exists (27f; else at 1×, said so, the FINDING FOCUS-3 raises).
- [ ] **Step 3:** Close report `docs/superpowers/reports/2026-10-0x-focus-7-close.md`: every rule-24 category below with its guarantee; the FINDINGS; the spec amendments (§5's FOCUS-7 row gains `/api/kretzmann/chapter/{cref}`, `KretzmannCitationScan`, `CatechismList`, `ConcordUnitList`, `CatechismLinks`; the "five `Catechism*Section`s" are four here and `CatechismSeamSection` in FOCUS-2; §3.3's home surfaces gain the Kretzmann route); the unplaced commentary citations (OPEN 6); the behaviours removed by OPEN defaults.
- [ ] **Step 4:** Commit; push the batch branch; hand to the other agent for review (14b + 24a). Land under `land` after review.

**Rule-24 categories and their closures:**

| Category | Closure | Guarantee |
|---|---|---|
| CatechismItem and CommentaryItem served by both mechanisms | the deletion law with `LegacyNames` (`"Catechism"`, `"CommentaryItem"`) | compiler + law |
| Citations scanned outside the compiler (`KretzmannCitationScan`, its alias table restating `citations.rs`) | one compiled scanner over every prose corpus; the client file deleted | `the_concord_and_kretzmann_are_cited_through_one_scanner`; `ReferenceParsingLawTests` |
| A served read that walks a range with an unbounded page and parses ids for order (`chapter_commentary`) | Kretzmann's order is containment, compiled; the read is the bounded text page | the route deleted; `psalm_119s_comments_are_its_kretzmann_chapters_contains_page_in_order` |
| A request per item on the client (`Graph.Card` per comment; one chapter text per proof-verse chapter) | text pages carry the prose; the item's verses are one neighbour page | KRETZ-GRAPH-1's network bound |
| Domain words in code ("Where is this written?" on the client, "What does this mean?" in `atlas-core`, "Commentary" as a title) | data (`catechism.toml`), compiled labels | the file-level heading law; the label laws |
| The client splitting served text or deciding a corpus from an id prefix (`"\n\n"`, `"BoC "`) | served line breaks kept by CSS; one served group | `ReferenceParsingLawTests` (empty for FOCUS-7) |
| A whole collection read on the client (`CatechismLinks`, F-63) | the generic paged list | `WholeReadLawTests` |
| A text row the client cannot place (a comment with no locus) | `TextRef.Commentary`, compiled | `exactly_text_units_and_comments_carry_text` |

---

## Wave schedule

Primary is the lane's critical-path work; the companion is unlike work paired beside it (rule 23: Rust beside C#). A task starts only when the tasks it names have landed on `lane/<agent>/F7-int`.

| Wave | Primary | Companion | Critical sections | Expected red at wave close (the ledger records names) |
|---|---|---|---|---|
| 0 | owner: OPEN 1–6; Task 0 | — | — | — |
| 1 | Task 1 (Rust/data: headings; rebuild, regen #1) | Task 2's tests written against `ServedGraph` | `contract` rebuild + regen #1, re-bless; `heavy` | `GeneratedUsageTests`: `CatechismDetail.*`, `TitledText` |
| 2 | Task 3 (Rust: Kretzmann compiled; rebuild #2) | Task 2 (C#: catechism on FocusView, deletions) | `contract` rebuild #2, re-pin, re-bless; `heavy` | Playwright: Task 2 Step 4's set |
| 3 | Task 4 (Rust: commentary on the wire; regen #2) | Task 8's catechism half written, not regenerated | `contract` regen #2, re-bless; `heavy` | + `GeneratedUsageTests`: `TextRef.Commentary`, `UnitHeading.Section` |
| 4 | Task 5 (C#: comment on FocusView, route) | — | — | as wave 3 |
| 5 | Task 6 (C#: the Kretzmann page) | Task 8's Kretzmann half written | — | + Task 6 Step 4's set |
| 6 | Task 7 (C#: legacy commentary gone) | then Task 8 (Rust: route deletions; regen #3) | `contract` regen #3, re-bless; `heavy` | as wave 5 |
| 7 | Task 9 (re-express, gates, close) | — | `heavy`; mutation in the owner's window; `land` after review | carried reds and named F-55 flakes only |

**Critical path:** OPEN answers → Task 3 → Task 4 → Task 5 → Task 6 → Task 7 → Task 8 → Task 9. Tasks 1 and 2 ride beside it and land before Task 8.

## FINDINGS this plan expects to raise (for the queue; the owner decides)

- **A `comments-on` row is indexed only at its first verse** (`kretzmann_adapter.rs`: "a multi-verse unit shows ONLY at its FIRST verse"); the same shape as FOCUS-2's `cites` with `to_last`. A verse inside a commented span does not list the comment.
  - Closure: a commented span ends at FOCUS-3's passage container, as a cited span does.
- **A catechism item's words live beside the graph** (`AtlasData`'s catechism, read by `catechism_detail`), not on its node; the node carries only its label.
  - Closure: the words become the node's payload, read through the store, with F-34's record migration.
- **The catechism-link rows from the brain-fuel question mapping have no license** (LICENSES.md "No license file"; O-CATECHISM). FOCUS-7 drops the question headings (OPEN 2) but keeps their verse links.
  - Closure: O-CATECHISM's answer (a granted license, or our own mapping from the public-domain 1921 Triglot).
- **`AtlasData::catechism_items_for_span` and the verse-to-catechism index** lose their last served reader with FOCUS-3's `/api/catechism/{sref}` and FOCUS-2's `/api/verse`; Task 8 reports what `cargo build` keeps.
- **The commentary has no token layer**; its citation spans are compiled characters. Closure: words-as-base (deferred by the owner) tokenizes it like the KJV and the Concord.
- **The Kretzmann column places a comment by comparing served verse numbers** (`Commentary.On`). Rule 25 holds (equality on served integers, as `Crossing`), but a stricter closure is a served `comments-on` anchor per verse row; noted, not proposed.
- **F-51's remainder:** if the parser's heading join (`"….: …"`) survives into `Section { title }`, it stays F-51 for the ETL.
- **Pre-existing comments in touched files** (`CommentaryItemNode.cs`'s unreachable-fallback comment dies with the file; `Kretzmann.razor`'s `@*…*@` blocks die where the rewrite deletes their lines; `kretzmann_adapter.rs` and `catechism.rs` doc comments go with the deleted functions). No other comment is touched (F-12, A-STRIP).

## Self-review against rule 27, the spec and the brief

- **Data-only derivations are compiled:** the catechism's headings (data), Kretzmann's books, chapters and order, each chapter's Bible chapter, each comment's place, section openings, labels and citations (Task 3). Nothing in the served path scans text, parses an id or orders by an id.
- **Per-request derivations are bounded index reads:** the record (one lookup), a comment's `UnitText` (one lookup per unit, anchors batched per window), the container text page (keyset, capped), the neighbour page. `chapter_commentary`'s unbounded walk dies.
- **Interaction derivations are the client's:** the card's fields, the presentation table rows, the hatch, the Kretzmann page's scope and focus (R12), keeping the comment window in pace with the verse window, and placing a comment under its verse.
- **The graph models the domain, never a view:** no relation appended; Kretzmann's containers are the work's own structure; a Kretzmann chapter commenting on a Bible chapter is how the work is titled; no field exists for the client (a comment's locus and heading are its own).
- **27c/27e:** a chapter arrival on the Kretzmann page is FOCUS-3's reads plus one `commented-on-by` page and one text page per open chapter; no per-item read; resident comments bounded per open chapter (the growth law).
- **Coverage of §5's FOCUS-7 row:** the four `Catechism*Section`s here (the fifth is FOCUS-2's), `CatechismInConcordSection`, `CommentaryItemProseSection`, `CatechismNode`, `CommentaryItemNode` and `/api/catechism/item/{id}` are deleted; FOCUS-3's hand-offs (the Kretzmann column, its continuous scroll, its `/api/text?ref=` caller) are taken.
- **Type consistency:** `TitledText`/`CatechismDetail` (Task 1) are what `CatechismOf` (Task 2) reads; `CommentaryPlace` (Task 3) is what `TextRef.Commentary` (Task 4) serves and `ReadingAddress.Kretzmann` (Task 5) and `Commentary.On` (Task 6) read; `UnitHeading.Section` (Task 4) is what `KretzmannVerseRow` (Task 6) renders; `SequenceView`'s callbacks (Task 6) are what `Commentary` subscribes.

## Assumptions

**Verified against `b3d7cfa`:**
- `CatechismNode` is constructed at `CatechismList.razor:8`, `LegacyNodes.cs:27` and in tests (`PopoverSectionRegistryTests:152, 154, 174`, `LegacyViews:22`, `LegacyNodesTests:89`); `CommentaryItemNode` at `LegacyNodes.cs:28`, `Kretzmann.razor:507` and in tests (`LegacyViews:23`, `ExplorerPopoverTests:83, 141, 236`, `LegacyNodesTests:90`).
- The six providers are registered at `PopoverSections.cs:52–56, 72` and match the strings `"Catechism"` and `"CommentaryItem"`; `CatechismLinks` has two callers (`:1340`, `:1379`, the second FOCUS-2's); `CatechismList` is opened by `CatechismSeamSection` (`:408`) and `ConcordSmallCatechismSection` (`:1398`); `ConcordUnitList` only by `CatechismInConcordSection` (`:1358`).
- `AtlasClient.CatechismItem` is read only by `CatechismNode`; `AtlasClient.KretzmannChapter` only by `Kretzmann.razor:419`; `KretzmannCitationScan` only by `Kretzmann.razor:505`; `lazyProse.js` only by `Kretzmann.razor`; `IExplorableClient.Card` by `Kretzmann.razor:478`, `CommentaryItemProseSection` (`:1310`) and two person sections (FOCUS-4).
- `CatechismScripturesSection` reads `api.ChapterText` per distinct chapter (`:541`), a caller FOCUS-3's OPEN 2 list does not name.
- `NodeRecord.catechism: CatechismDetail { part_title, text, explanation_heading, explanation, where_written }` is read from `AtlasData` (`graph.rs:248–259`); a commentary item's prose is served as `description` (`graph.rs:281`); `NodePayload::CommentaryItem { work, heading, text }` and `NodePayload::CatechismItem { label }`.
- `kretzmann_adapter::chapter_commentary` pages `comments-on` at each verse with `limit: usize::MAX` and orders by `ordinal_of(&item_id.raw)`; its only served reader is `reading.rs:100`.
- `citations::cite_scripture` scans only the Concord reading spine; `citations.rs` holds `ABBREVIATIONS` (69 entries), which `KretzmannCitationScan.CitationAliases` restates.
- `catechism-link` is symmetric, served in compiled `ord` order (`snapshot.rs` `edges_inner`); `TextRef` (wire) has two cases; `UnitHeading` is one struct; the Kretzmann section holds `CommentsOn` only; `SECTION_SCHEMA_VERSION` is 22 at `b3d7cfa`.
- `HomeSurfaces.Of` is derived from `Presentation.Of` (`Surface.cs:14`), so a Reader row makes the Reader the home surface.
- Playwright: `kretzmann.spec.ts` (22 tests), `popover-sections.spec.ts` (the item hop at `:986`; the verse-side CATECH-1 tests are FOCUS-2's), `concord.spec.ts` D3 (`:438`, `:469`), `reader.spec.ts` NAV-STUTTER-2 (`:648`), `provenance.spec.ts` PROV-1 (`:262`, the passage path, FOCUS-3's).

**To verify at execution (each with its fallback):**
- **`ContainerContent<C>` admits a comment** (Task 0). Fallback: a `ContainsCommentary` family with the same laws.
- **Every Kretzmann unit's first verse is in the KJV** (the parser checks conservation). Fallback: a unit outside it fails the compile naming it, and the owner decides (never a silent drop).
- **`catechism-link`'s compiled order is the curated order** (item verses, question verses, Concord paragraphs). Fallback: the compiler writes rows in curated order, with a law.
- **FOCUS-3's `SequenceView` opens and stops child windows at single, observable moments.** Fallback: the two callbacks are raised from `PageWindow`'s own open/stop, and `SequenceViewTests` pins them.
- **The client generator emits the two new tagged unions** (`TextRef.Commentary`, `UnitHeading`) as it does `PositionRef`.
- **FOCUS-4's `grounds_of` has landed** (else FOCUS-4 adds the three families' arms when it lands; the compile error makes it unmissable).
