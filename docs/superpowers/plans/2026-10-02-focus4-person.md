# FOCUS-4 (Person) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Executor: Codex** (roadmap; queue item `CX-F4`). This plan is written so it can be run with no context beyond it. Codex touches only the files a task lists. Anything else found on the way goes to FINDINGS in the queue, never into the diff (rules 24a, AGENTS.md).

**Goal:** Move Person off the legacy popover onto FocusView and the generic presentation, and delete what spec §5 lists for it: `PersonLifeSection`, `PersonEventsSection`, `PersonFamilySection`, `PersonCardAndMentionsSection` and `PersonNode`, with everything only they used (`PersonMentionsList`, `Kinship`, `PersonSectionRendering`, the `"Person"` chrome row). Fold in what the move closes: F-63's person sites (five `Paging.Whole` reads and the per-parent sibling walk), F-31 and F-7's `PersonNode` sites, F-66's `Kinship`, the client literal `"Jesus"` (rule 25/26), the curated eternity the server still serves as a lifespan (`god_1324` serves first/last 4004 BC – AD 96), and the justification grounds the compiler wires for only four of the row families that carry them.

**Architecture:** Rule 27 splits the work by who knows the inputs.
- The **compiler** applies the eternity curation (an eternal person carries no year), writes each curated ground as a `justified-by` edge (the eternity grounds from the person, and the grounds of every row family that records them, kinship included), and labels a parent-of edge by its curated parentage wording, which moves from client code into `data/curated/parentage.toml`.
- The **server** reads. It gains no route and no read; one wire field leaves (`PersonLife.eternal_grounds`, now edges) and, under OPEN 4(b), one boolean arrives.
- The **client** derives the interaction. Person's popover row is the generic `Card` (life fields) plus its frontier on FocusView: every group the graph serves, paged 20 at a time through the one root-aware door, each entry a `Link` followed through `Explore`. Three generic presentation choices are added to `Affordances` and FocusView (an entry may read its edge's compiled label; a list may open collapsed; a year field may open the World), each keyed by edge kind or field, never by person.
- Nothing is appended to `relations!`. No node, field or element exists for a client construct.

**Tech Stack:** Rust (axum, utoipa, `atlas-etl`, `atlas-graph`, `atlas-contract`, `graph-types`); .NET 10 Blazor WebAssembly (`BibleAtlas.Client`, `BibleAtlas.Client.Exploring`); xUnit + bUnit; Playwright; Stryker.NET; cargo-mutants via `scripts/mutants-parallel.sh`.

**Spec:** `docs/superpowers/specs/2026-09-26-focus-exploration-design.md` §3.3 (`Card`, `Field`), §3.4 (`Affordances`), §5 (FOCUS-4 row), §6 (test ids), §9 (deletions), §12 R15–R20. **Owner directive D5** (2026-09-15, quoted in `tests/ux/person-card.spec.ts`): "when we click on a person's name there's no point to just see every verse that name is mentioned … i want to see the years that person is alive -- years are explorable positions; the events (explorable) in which they are mentioned; optionally a family tree whose names are all explorable; the exception is God because he is eternal." **Principles:** 4, 9, 12, 15–18, 21–22, 24–24b, 25, 26, 26a, 27–27g (rule 27 is on `worktree-bible-atlas-m1`'s `docs/PRINCIPLES.md`, not yet on `F1-int`; Task 0 notes it). **Queue:** FOCUS-4; closes F-63's person sites, F-31's and F-7's `PersonNode` sites, F-66's `Kinship`; touches F-53, F-56 (below).

**Base:** verified against `origin/lane/claude/F1-int` at `eea9023` (FOCUS-1 + FOCUS-6 + fix waves, about to land), with F-74's semantics assumed (`previous` is null only on the first page). **FOCUS-4 starts on the head FOCUS-2 lands at** (Codex runs FOCUS-2 first; the files they share are listed under "Overlap"). Task 0 writes that head into the ledger as this batch's `--base` (PRINCIPLES 22), and every gate takes it.

## OPEN: for the owner, before the task named starts (each blocks only that task)

Each is one line to answer. The plan builds the recommendation if unanswered.

1. **God's eternity grounds (Task 1):** (a) compile them as `justified-by` edges from the person to each ground verse, so they are a clickable group on the card and `PersonLife.eternal_grounds` leaves the wire; (b) show them as a plain "Grounds" field, not clickable. **Recommend (a).**
2. **Parentage on the card (Task 3):** (a) shown only on the edge (its compiled label and Scripture grounds, one ⋮ step away); (b) also inline: `parent-of`/`child-of` entries read their edge's compiled label ("Joseph · Father, as was supposed, of · Jesus"). **Recommend (b).**
3. **Parentage wording (Task 2), data you can edit later:** eternal "Eternal Father of", virgin "Virgin mother of", legal "Father, as was supposed, of", created "Creator of"; natural keeps "Parent of". **Recommend approve as written.**
4. **"EARTHLY LIFE" for Jesus (Tasks 1, 3; today a `"Jesus"` literal in client code):** (a) drop it, His card shows Born and Died like anyone's; (b) mark the incarnation in `data/curated/people-eternal.toml` (`[[incarnate]]`), serve `PersonLife.incarnate`, and His date fields read "Born (earthly life)" / "Died (earthly life)". **Recommend (b)** (the curated file already records the intent: "the card labels them 'Earthly life'").
5. **Years as explorable positions (Task 3, D5; PERSON-2):** (a) dates are plain fields and the year chips die; (b) each served year field on a person's card is a button that opens the World at that year. **Recommend (b).**
6. **Where the verse list sits (Task 3, D5):** (a) served order, every list open (Mentioned in usually comes first); (b) `mentioned-in` lists open collapsed and render after the open ones, on every kind's card. **Recommend (b).**
7. **F-36, year labels formatted per request (Task 1):** (a) leave F-36 to its own item; (b) close it here by compiling every year and time-range label. **Recommend (a)** (it spans every kind; FOCUS-4 adds no new per-request formatting).

**Rulings applied by analogy, not re-asked** (FOCUS-2's owner answers, 2026-10-02, recorded in `docs/superpowers/plans/2026-10-02-focus2-textunit.md` on `lane/claude/F2-plan`):
- **Ruling 2 (a client-derived view one step away is dropped):** the **Siblings** group goes. It was worked out on the client from each parent's whole `parent-of` read (F-63, 27c: a request per parent). A sibling is one step away: a parent's `parent-of` group. D5 calls the family tree optional.
- **Ruling 3 (one group per edge kind, in served order, no client split):** the family is the served `parent-of`, `child-of`, `spouse-of` and `brethren-of` groups, not a client "FAMILY" heading with parentage sub-groups.
- **Ruling 7 (reach every site of the category in the batch):** Task 1 closes the justification category for every row family, not only kinship.
- **Ruling 9 (accept the AQC major):** removing `PersonLife.eternal_grounds` is an AQC major.

## Global Constraints

- `docs/PRINCIPLES.md` binds, especially:
  - rule 4: zero dead code;
  - rule 9: no comments in application code (the review greps the diff for added comment lines; a comment on a deleted line goes with it; no other comment is touched);
  - rule 12: every signature below is for sign-off;
  - rules 24, 24a, 24b: every fix names its category and closes it; an offender found on the side goes to FINDINGS;
  - rule 25: the client composes over the contract; it never parses a reference, composes an id, formats a year or decides a kind from a string;
  - rules 26, 26a: domain facts (wording, ids, names) live in `data/`; source parsing stays in the tools;
  - **rule 27**: data-only derivations are compiled; the server reads; interaction derivations are the client's; the graph models the domain, never a view. No client exploration vocabulary (explore, explorable, frontier, card, popover, presentation) in `server/` or `graph-types/`.
- Tests: whole-body assertions; one behaviour per test, named as a sentence; `// Arrange` `// Act` `// Assert` only; no magic numbers; newspaper order. Real-data expectations are read from the artifact, never written as literals (F-8).
- **Total matches.** A closed sum is matched exhaustively (build error on a missing arm). `Affordances.Of` stays total over `EdgeKind`.
- **The one walk door.** Every follow from FocusView goes `OnFollow(Link)` → `ExplorerPopover.FollowAsync` → `Explore.Follow`. No new caller of `IExplorer.Resolve` (it is `internal`, R20).
- **The one paging door.** Every list on FocusView opens through `Paging.Window(PresentationRequest, EdgeKind)` (F-70); `RootConsistencyLawTests` enumerates the doors and must stay green with one door.
- **No relation is appended.** `DECLARED_DIRECTED_RELATIONS` and `DECLARED_SYMMETRIC_RELATIONS` do not change.
- Build no interaction that works only by hovering.
- Commit per task on `lane/codex/F4-t<n>`; integrate on `lane/codex/F4-int`. Codex never pushes `worktree-bible-atlas-m1`; Claude lands by cherry-pick under `land` after review. Never force.
- Every task runs in its own worktree (`git -c core.autocrlf=false worktree add -b lane/codex/F4-t<n> ~/w/F4-t<n> <base>`), `CARGO_TARGET_DIR=~/mut/codex-F4-t<n>`, `nice -n 10 cargo -j 4`, data copied never linked (AGENTS.md).

## Critical sections (PRINCIPLES 21): one holder at a time

| Section | Held by | Why |
|---|---|---|
| `contract`: rebuilding `data/compiled` | Task 1 Part A, then Task 2 | one artifact; the version root moves |
| `contract`: `export_contract`, `export_aqc_examples`, `client.ContractGenerator` | Task 1 Part B | one generated document |
| `contract`: re-blessing pacts and fixtures | right after each rebuild or regen | moves the version root |
| `heavy`: `cargo test --workspace`, full Playwright | Tasks 1, 2, 6 | memory |
| `heavy` with "mutation" in the message | Task 6 only, inside the owner's window (`.superpowers/MUTATION-GATE-DEBT.md`) | once per batch (3a) |
| appending to `relations!` | **nobody** | rule 27 |

## What a person's popover shows, before and after (acceptance)

Opened from a reader mention (`verse-mention-person-{n}-{personId}`), the popover title stays `popover-title` = the served label. The body is FocusView (`popover-*` ids):

| Legacy section (test ids today) | After FOCUS-4 (generic presentation) |
|---|---|
| `PersonLifeSection`: heading `LIFE`/`EARTHLY LIFE` (`person-life-heading`), line "Born c. 1575 BC · Died c. 1452 BC" or "Eternal" or "Mentioned across c. X – Y" (`person-life`); eternal grounds as chips (`person-eternal-ground-*`) | `popover-section-card`: `popover-card-title` = label; fields in this order, each present only when served: `Life` = "Eternal" (eternal persons only), `Born`, `Died`, `First mentioned` and `Last mentioned` (only when neither Born nor Died is served, as today), `Provenance`. Under OPEN 4(b) an incarnate person's date fields are named `Born (earthly life)` / `Died (earthly life)`. Under OPEN 5(b) each year field's value is a button `popover-year-{field name}` that opens `/world?from={y}&to={y}`. God's card shows `Life: Eternal`, no year at all. The grounds are the `justified-by` group: `popover-section-justified-by`, entries `popover-link-justified-by-text-unit:PSA.90.2` (OPEN 1a). |
| `PersonNode` chips `popover-chip-year-born/-died/-span` (NavigateWorld) | OPEN 5(b): the year buttons above; (a): gone. |
| `PersonEventsSection`: `EVENTS (n)` (`person-events-heading`), every event at once (`Paging.Whole`), chips `person-event-*` | `popover-section-participates-in`, heading `Participates in (n)` (the compiled count), 20 per page, More/Less (`-more`, `-collapse`, `-position`), entries `popover-link-participates-in-{Event id}`. Following one opens the event (legacy `EventNode` until FOCUS-5). |
| `PersonFamilySection`: `FAMILY`, `Parents (2)` / `Father (eternal Son of God)` / `Mother (born of the Virgin)` / `Legal father (as was supposed)` / `Spouses` / `Children` / `Siblings` / `Brethren`, chips `person-{group}-{id}`; siblings from one whole read per parent | One section per served group, in served order: `popover-section-parent-of` (`Parent of (n)`), `-child-of`, `-spouse-of`, `-brethren-of`, entries `popover-link-{kind}-{Person id}`. OPEN 2(b): `parent-of`/`child-of` entries read their edge's compiled label (Jesus's `child-of`: "God · Eternal Father of · Jesus", "Mary · Virgin mother of · Jesus", "Joseph · Father, as was supposed, of · Jesus"). Each entry's ⋮ step (`popover-entry-edge-{kind}-{edge id}`) opens the edge, whose `justified-by` group lists its curated Scripture grounds (Task 1). **Siblings: gone** (ruling 2). |
| `PersonCardAndMentionsSection`: `<details>` `MENTIONED IN SCRIPTURE (n)` collapsed, `Source: …` (`popover-person-provenance`), rows `person-mention-{vref}`, `person-mentions-more/-less/-position` | `popover-section-mentioned-in`, heading `Mentioned in (n)`, entries `popover-link-mentioned-in-text-unit:{ref}`, More/Less `popover-section-mentioned-in-more/-collapse/-position`; following an entry opens the verse on FocusView (FOCUS-2). OPEN 6(b): the section is a `<details>` closed by default and renders after the open sections. Provenance is the card's `Provenance` field. |
| — | Every other group a person serves appears too, in served order (e.g. Jesus's `speech-of`, `named-after`), as FocusView offers on every kind. |

## Re-anchor table: what the person popover touches at `eea9023`, and where each goes

| Today (`eea9023`) | Role | FOCUS-4 |
|---|---|---|
| `client/Legacy/PersonNode.cs` (61 lines) | identity, year chips, `Born`/`Died`/`MentionedAcross` strings | **deleted** (Task 4) |
| `PopoverSectionProviders.cs:1257–1296` `PersonCardAndMentionsSection` | provenance + mentions over `Paging.Window(presenting, MentionedIn)` (F-70's door) | → FocusView `mentioned-in` group, same door; **deleted** |
| `:1446–1476` `PersonSectionRendering` (`file static class`) | heading + chips helpers | **deleted** |
| `:1478–1536` `PersonLifeSection` (incl. `LineOf`, `person.Title == "Jesus"`, `new VerseNode(g)` for grounds) | life line, eternal grounds | → card fields + `justified-by` group; **deleted** |
| `:1538–1563` `PersonEventsSection` (`Paging.Whole` participates-in) | events | → `participates-in` group; **deleted** |
| `:1565–1616` `PersonFamilySection` (four `Paging.Whole` + one per parent) | family | → four served groups; **deleted** |
| `client/Exploring/Kinship.cs` (`KinGroup`, `Kinship`, the parentage wording) | family grouping and wording in client code | wording → `data/curated/parentage.toml` (Task 2); **deleted** (Task 4) |
| `client/Components/PersonMentionsList.razor` | the mentions list (opens `new VerseNode(vref)`) | **deleted** |
| `client/Components/MentionScan.razor:16–22` person arm (`PopoverOpening.Legacy(new PersonNode(...))`) | open a person from verse text | `PopoverOpening.Explore(new NodePosition(person))`, the place arm's door (Task 4) |
| `client/Legacy/LegacyNodes.cs:25` `NodeKind.Person => new PersonNode(...)` | bridge | → `null` |
| `client/Legacy/PopoverSections.cs:68–71` | four registrations | removed |
| `client/Exploring/PopoverChromeRegistry.cs:29` `["Person"]` | chip ids | removed; `PopoverChromeConformanceTests.ConcreteExplorableNodeClasses` − 1 |
| `client/wwwroot/css/app.css` `.person-life-line`, `.person-family-label`, `.person-mentions-disclosure > summary`, `.popover-person-mentions` | legacy styling | removed (Task 4) |
| `server/atlas-etl/src/compile.rs:61–68` (eternity applied) | sets `eternal`, `eternal_grounds` only | also clears the four years (Task 1) |
| `server/atlas-graph/src/event_world.rs:510` `add_justified_by` (`_ => continue` over four families) | grounds → `justified-by` | every family that records a justification, plus eternity grounds (Task 1) |
| `server/atlas-graph/src/labels.rs:55` `edge_label` | `{subject} · {kind} · {object}` | a parent-of edge with a declared parentage reads its curated wording (Task 2) |
| `server/atlas-contract/src/wire/graph.rs:68` `PersonLife` | person facts | loses `eternal_grounds`; gains `incarnate` under OPEN 4(b) (Task 1) |

**Call sites counted at `eea9023`:**
- `PersonNode`: constructed at `MentionScan.razor:20,21`, `LegacyNodes.cs:25`, `PopoverSectionProviders.cs:1228` (in `VersePersonsSection`, which FOCUS-2 Task 5 deletes, F-64) and `:1611`; type-tested at `:1263, 1484, 1544, 1571`; its statics read at `:1527, 1528, 1533`. Tests: `LegacyNodesTests.cs:50, 87, 105`, `LegacyViews.cs:20`, `PopoverSectionRegistryTests.cs:172`.
- The four sections: registered at `PopoverSections.cs:68–71`; named by `PopoverSectionRegistryTests.cs:165–174` and `PushViaConformanceTests.cs:33, 36–39` (and that test's `PersonSectionRendering`/`Chips` special case in `PushesIn`).
- `PersonMentionsList`: `PopoverSectionProviders.cs:1285–1291`, `PersonMentionsListTests.cs`, `RootConsistencyLawTests.cs:38` (as the app assembly's marker type).
- `Kinship`: `PopoverSectionProviders.cs:1583, 1585, 1595`, `KinshipTests.cs`.
- No server route serves Person alone; FOCUS-4 retires no route and no `AtlasClient` method (`AtlasClient.NodeRecord` stays for `AuthorNode`, `EventNode`).

**Playwright specs that cover them (`tests/ux/`):**
- `person-card.spec.ts`: PERSON-1 (life, events, family, siblings, mentions last and collapsed), PERSON-2 (born chip → world), PERSON-3 (God eternal, grounds clickable), PERSON-4 (Jesus's parentage groups, brethren). All four re-expressed (Task 5).
- `reader-persons.spec.ts`: PERSONS-1 loop (verse → person → mention → verse) and PERSONS-2 (mentions page 20, More/Less) re-expressed; PERSONS-1 "no PERSONS section" and PERSONS-3 (server-only) unchanged.
- `popover-sections.spec.ts` MENTION-2 (Aaron), MENTION-3 (Canaan); `reader-recursion.spec.ts` RECURSE-1 (God, `popover-title` + `popover-breadcrumb-back`); `kretzmann.spec.ts` (God mention class only): assert only the title and chrome, so they must stay green unchanged.
- `tests/ux/CONTRACT.md:2808` (`PersonNode`'s id note) and the person-card section: updated (Task 5).

## Overlap with FOCUS-2 (Codex, before this) and FOCUS-3 (Claude, in parallel)

| File | FOCUS-2 | FOCUS-3 (expected) | FOCUS-4 |
|---|---|---|---|
| `client/Legacy/PopoverSectionProviders.cs` | deletes verse/concord sections and `VersePersonsSection` (`PersonNode` site `:1228`); `:1508` eternal grounds → `LegacyTextUnits.Opening` | deletes `ChapterCardSection` | deletes the four person classes and `PersonSectionRendering` |
| `client/Components/PersonMentionsList.razor:27` | → `LegacyTextUnits.Opening` | — | **deleted** (one caller of `LegacyTextUnits` leaves) |
| `client/Legacy/{LegacyNodes,PopoverSections}.cs`, `client.Tests/Explore/{DeletionLawTests,LegacyNodesTests,LegacyViews,IdentityTests,PushViaConformanceTests}.cs`, `client.Tests/PopoverSectionRegistryTests.cs` | its rows | its rows | Person rows |
| `client/Exploring/{Presentation,Presenter}.cs`, `client/Views/FocusView.razor` | `Text` form and arm | `Sequence` form and arm | person fields; `EntryText`, `Disclosure`, year buttons |
| `client/Exploring/Affordances.cs`, `client.Tests/Explore/AffordancesTests.cs` | — | maybe | `SectionList` gains two members |
| `server/atlas-graph/src/labels.rs` | Task 2 `object_label` (cites) | — | Task 2 parent-of wording |
| `server/atlas-contract/src/wire/graph.rs`, `contracts/*`, `data/compiled` | `NodeRecord.text`, `UnitText` | likely | `PersonLife` |
| `client.Tests/{WholeReadLawTests,ReferenceParsingLawTests}.cs` (created by FOCUS-2 Task 5) | creates the ratchets | shrinks them | re-run; remove any entry whose site this batch deletes |
| `client/Components/MentionScan.razor` | not listed | may move with the reader body | person arm only (lines 16–22) |
| `client/Exploring/PopoverChromeRegistry.cs`, `client.Tests/PopoverChromeConformanceTests.cs` | not listed in its plan (Verse/ConcordUnit rows; a FINDING for FOCUS-2's review) | Chapter/Book/Passage rows | Person row; the class count |

The shared files are row files: each batch deletes its own rows. Whoever lands second rebases. FOCUS-4's tasks touching `labels.rs`, `Presenter.cs`/`FocusView.razor` and `wire/graph.rs` start only after FOCUS-2's Tasks 2, 3 and 1 respectively have landed (Task 0 checks).

## Types (for sign-off, PRINCIPLES 12)

### The compiler (Tasks 1, 2)

`server/atlas-etl/src/curated.rs`:
```rust
pub fn parse_people_eternal(input: &str) -> Result<CuratedEternity>;
pub struct CuratedEternity { pub eternal: Vec<(String, Justification)>, pub incarnate: Vec<String> }
pub fn parse_parentage(input: &str) -> Result<CuratedParentage>;
pub struct CuratedParentage { pub declared: Vec<ParentageSeed>, pub excluded: Vec<ParentageExclusion>, pub wording: BTreeMap<Parentage, String> }
```
- Grounds are read once, with the existing `scripture_justification` (as `parentage.toml`'s are), so nothing downstream parses a reference string (26a).
- `incarnate` exists only under OPEN 4(b). Each id must name a Theographic person and must not also be eternal; the compile fails loud otherwise.
- `wording` holds exactly one entry for every `Parentage` except `Natural`; the compile fails loud on a missing, extra or `natural` entry (24b: enumerated through the vocabulary's `ALL`).

`server/atlas-core/src/data.rs` (`Person`): `eternal_grounds: Vec<String>` becomes `eternal_grounds: Justification`; gains `incarnate: bool` (OPEN 4b). `AtlasData` gains `parentage_wording: BTreeMap<Parentage, String>`.

`graph-types/src/node.rs` (`NodePayload::Person`) and `graph-types/src/canon/node.rs`: `eternal_grounds: Vec<String>` becomes `eternal_grounds: BTreeSet<Ground>`; gains `incarnate: bool` (OPEN 4b). The canonical encoding changes, so `graph-types/tests/canon_vectors.rs` is re-pinned under `contract` (not a map fixture; no owner approval needed beyond this plan's sign-off).

`server/atlas-graph/src/event_world.rs`:
```rust
pub fn add_justified_by(graph: &mut Graph) -> usize;
fn grounds_of(family: RowFamily, graph: &Graph, row: usize) -> Option<&BTreeSet<Ground>>;
```
- The signature is unchanged (its 36 call sites keep compiling). `grounds_of` is an exhaustive `match` over `RowFamily` with **no wildcard arm**: a family whose rows record a `Justification` answers `Some(&row.justification.grounds)`; one whose rows record none answers `None`, written out per family. Adding a family fails to compile until it says which (24b).
- Then one pass over person nodes: an eternal person's `eternal_grounds` each become a `justified-by` pair from `Position::Node(person)`.
- `RowFamily` members carrying a `Justification` at `eea9023`: `ContainsBible`, `ContainsConcord`, `Attests`, `Succession`, `CanonSuccession`, `DatedBy`, `LocatedAt`, `Fulfills`, `Typology`, `NamedAfter`, `Catechism`, `CommentsOn`, `SpokenBy`, `SpokenAt`, `ParentOf`, `Brethren`, `Authored`, `Confesses` (Task 1 Step 0 re-derives this from `graph-types/src/edge.rs`). Today only `DatedBy`, `Fulfills`, `Typology`, `NamedAfter` are wired.

`server/atlas-graph/src/labels.rs`:
```rust
pub struct ReaderNames<'a> { geography: &'a Geography, anchors: BTreeMap<AnyNodeId, &'a str>, parentages: &'a BTreeMap<Parentage, String> }
pub fn edge_label(kind: EdgeKind, subject: &str, object: &str) -> String;
fn relation_label(record: &EdgeRecord, names: &ReaderNames) -> String;
```
- `relation_label` is `kind.display_label()`, except a `ParentOf` record whose `meta` is `EdgeMeta::Parentage(p)` with `p != Natural`, which reads `names.parentages[p]`. `edge_label`'s callers pass it. A label is a data-only derivation, so it is compiled (rule 27); served code is untouched. If FOCUS-2 Task 2's `object_label` has landed, the two compose: `edge_label(relation_label(...), subject, object_label(...))`.

### The wire (Task 1 Part B)

`server/atlas-contract/src/wire/graph.rs`:
```rust
pub struct PersonLife {
    pub gender: Option<String>,
    pub birth: Option<super::Year>,
    pub death: Option<super::Year>,
    pub first: Option<super::Year>,
    pub last: Option<super::Year>,
    pub eternal: bool,
    pub incarnate: bool,
    pub also_called: Vec<String>,
}
```
- `eternal_grounds` leaves: the grounds are now the person's `justified-by` group (D.R.Y.; OPEN 1a). `incarnate` arrives only under OPEN 4(b). AQC **major** (ruling 9 by analogy).
- `read_node_record`'s person arm (`server/atlas-contract/src/graph.rs:61`) maps the two fields; no new read.

### The client (Tasks 3, 4)

`client/Exploring/Affordances.cs`:
```csharp
public sealed record SectionList(SectionStyle Style, SectionOrder Order, EntryText Text, Disclosure Disclosure) : Affordance(Affordances.PageSize);

public enum EntryText
{
    Neighbour,
    Edge,
}

public enum Disclosure
{
    Open,
    Collapsed,
}

public static class Affordances
{
    public static readonly Affordance.SectionList Cites = new(SectionStyle.Quiet, SectionOrder.VotesRanked, EntryText.Neighbour, Disclosure.Open);
    public static readonly Affordance.SectionList DefaultList = new(SectionStyle.Standard, SectionOrder.Canonical, EntryText.Neighbour, Disclosure.Open);
    public static readonly Affordance.SectionList Kinship = new(SectionStyle.Standard, SectionOrder.Canonical, EntryText.Edge, Disclosure.Open);
    public static readonly Affordance.SectionList Mentions = new(SectionStyle.Standard, SectionOrder.Canonical, EntryText.Neighbour, Disclosure.Collapsed);
}
```
- `Of`: `ParentOf or ChildOf => Kinship` (OPEN 2b; under 2a they stay `DefaultList`); `MentionedIn => Mentions` (OPEN 6b; under 6a it stays `DefaultList`). Every other arm is unchanged. These are kind-keyed presentation choices, the client's under rule 27; no person is named.

`client/Exploring/Presentation.cs`:
```csharp
public sealed record Field(string Name, string Value, Year? At = null);
```
- `At` is set only on a field whose value is one served `Year` (OPEN 5b). §3.3 signed `Field(Name, Value, Link? Link)`; a year is not a graph position (R2 made Year an event, and a person's birth has no event), so a `Link` cannot carry it. **This amends §3.3 and is for sign-off.**

`client/Exploring/Presenter.cs` (`GraphPresenter.CardOf`): new field-name constants and the person rows, read from `element.Record?.Person`:
```csharp
private const string LifeField = "Life";
private const string EternalValue = "Eternal";
private const string BornField = "Born";
private const string DiedField = "Died";
private const string EarthlyBornField = "Born (earthly life)";
private const string EarthlyDiedField = "Died (earthly life)";
private const string FirstMentionedField = "First mentioned";
private const string LastMentionedField = "Last mentioned";
private static IEnumerable<Presentation.Field?> LifeOf(PersonLife? life);
```
- `LifeOf` yields, in order: `Life = Eternal` when `life.Eternal`; `Born`/`Died` (or the earthly names when `life.Incarnate`) from `Birth`/`Death` with `At` set; `First mentioned`/`Last mentioned` with `At` only when neither `Birth` nor `Death` is served. Values are the served `Year.Label`; no year is formatted, compared or computed on the client (rule 25). `CardOf` places `LifeOf` before `Provenance`.

`client/Views/FocusView.razor`:
```csharp
[Parameter] public EventCallback<Year> OnShowYear { get; set; }
```
- A field with `At` renders its value as `<button data-testid="{handle}-year-{field.Name}">` invoking `OnShowYear(field.At)`; without a delegate, or without `At`, the value is plain text, as today.
- A `SectionList` entry's text is `list.Text switch { EntryText.Neighbour => Label(entry.Neighbour), EntryText.Edge => Label(entry.Edge) }`; the test id stays `{handle}-link-{kind}-{neighbour id}`.
- A `Disclosure.Collapsed` list renders as `<details data-testid="{section}">` with `<summary data-testid="{section}-heading">`; collapsed lists render after every open list. Its window is opened through the same door at load (bounded, one page).

`client/Components/ExplorerPopover.razor`: passes `OnShowYear="ShowYear"`, where `ShowYear(Year year) => NavigateWorld(WorldQuery.At(year))`.

`client/Exploring/WorldQuery.cs` (new, the one composer of a World query from a served year):
```csharp
public static class WorldQuery
{
    public static string At(Year year);
}
```
- It returns `from={year.Value}&to={year.Value}`, the query `PersonNode` builds today. FINDING (below): `MapFocusHatch.Query` and the legacy chips' inline `from=…&to=…` strings are the same category.

`client/Components/MentionScan.razor`: one opening for every anchored piece, `new PopoverOpening.Explore(new NodePosition(piece.Anchor.Node))`; the place arm keeps only its blink and its test id. No mention can open a legacy node.

## Deletion inventory (FOCUS-4 total)

- **Client, files:** `client/Legacy/PersonNode.cs`; `client/Components/PersonMentionsList.razor`; `client/Exploring/Kinship.cs`; tests `client.Tests/Components/PersonMentionsListTests.cs`, `client.Tests/KinshipTests.cs`.
- **Client, members:** `PersonCardAndMentionsSection`, `PersonSectionRendering`, `PersonLifeSection` (with `LineOf`), `PersonEventsSection`, `PersonFamilySection` and their four `PopoverSectionRegistry` rows; `LegacyNodes.For`'s `Person` arm (→ `null`); `PopoverChromeRegistry.ByKind["Person"]`; the `"PersonSectionRendering"`/`"Chips"` special case in `PushViaConformanceTests.PushesIn` and its regex alternative; the Person rows in `PopoverSectionRegistryTests`, `LegacyNodesTests`, `LegacyViews`, `IdentityTests`, `PushViaConformanceTests`; the four CSS rules above; `client.Tests/Explore/RootedMentions.cs` only if `RootConsistencyLawTests` no longer reads it (it does at `eea9023`, so it stays).
- **Server/data:** `PersonLife.eternal_grounds` (wire); `Person.eternal_grounds: Vec<String>` (replaced by `Justification`); the four-family wildcard in `add_justified_by`.
- **Not deleted** (stated so no one "finishes" it): `AtlasClient.NodeRecord` (`AuthorNode`, `EventNode`); `Paging.Whole` (`EventAccounts`, `CatechismLinks`: FOCUS-5, FOCUS-7); `RootedMentions` (the root law's fake); `MapFocusHatch` (event place buttons, FOCUS-5; F-54); `LegacySaves`'s v1 `"Person"` translation (FOCUS-9); `persons_at_verse` on `/api/chapter` (FOCUS-3).

---

### Task 0: Base and preconditions (no code)

**Files:** `.superpowers/sdd/2026-10-0x-focus4/progress.md` (the ledger; create).

- [ ] **Step 1:** Record the base (the head FOCUS-2 landed at) and that rule 27 is read from `worktree-bible-atlas-m1`'s `docs/PRINCIPLES.md`. Record the OPEN answers (or "defaults").
- [ ] **Step 2:** Re-verify at the base, recording each result: `grep -rn "PersonNode\|PersonLifeSection\|PersonEventsSection\|PersonFamilySection\|PersonCardAndMentionsSection\|Kinship\b\|PersonMentionsList" client client.Tests --include=*.cs --include=*.razor` (expect this plan's call-site list minus FOCUS-2's `VersePersonsSection` and its `:1508` change); `VersePersonsSection` is gone; FOCUS-2's Tasks 1–3 have landed (`NodeRecord.Text` exists, `Presentation.Text` exists, `labels.rs` has `object_label`). If any differs, list the difference in the ledger and stop for the controller.

### Task 1: Eternity and every ground are compiled; the wire carries no ground strings (backend)

**Backend change: yes (ETL, compiler, one wire field). Why there (rule 27):** whether an eternal person has years, and which Scripture grounds a curated fact, are facts over the data alone, so the compiler applies them once. The server only stops serving a string list that the graph now holds as edges. No new read; no relation appended.

**Owner gate first:** OPEN 1, 4, 7 (or the defaults).

Two commits: Part A is the data and the compile; Part B is the wire and its regen.

**Files (Part A):**
- Modify:
  - `data/curated/people-eternal.toml` (header only, unless OPEN 4b: add `[[incarnate]] id = "jesus_905"`);
  - `server/atlas-etl/src/{curated,compile,people}.rs` (`CuratedEternity`; clear `birth_year`, `death_year`, `first_year`, `last_year` for an eternal person; typed grounds);
  - `server/atlas-core/src/data.rs` (`Person.eternal_grounds: Justification`, `Person.incarnate`);
  - `graph-types/src/{node,graph}.rs`, `graph-types/src/canon/node.rs` (payload fields);
  - `server/atlas-graph/src/{person_adapter,event_world,description_adapter,peoples_adapter,red_letter_adapter,law_check,service}.rs` (payload construction sites; `add_justified_by`);
  - `data/compiled` (rebuilt), and the pacts the rebuild re-blesses.
- Test:
  - `server/atlas-etl/src/{curated,people}.rs` unit tests;
  - `server/atlas-graph/src/event_world.rs` unit tests;
  - `server/atlas-graph/tests/sqlite_laws.rs`;
  - `graph-types/tests/canon_vectors.rs` (re-pinned);
  - `server/atlas-contract/tests/graph_api.rs`.

**Files (Part B):**
- Modify:
  - `server/atlas-contract/src/wire/graph.rs` (`PersonLife`);
  - `server/atlas-contract/src/graph.rs` (person arm);
  - `contracts/*` regen, `contracts/atlas-query-contract/CHANGELOG.md` (AQC major);
  - pacts; `client.ContractTests/Steps/AqcSteps.cs` if it names `eternal_grounds`;
  - `client/Legacy/PopoverSectionProviders.cs` (only the grounds-chip block, Step 10);
  - `client.Tests/Explore/ServedGraph.cs` (gains `PersonLifeOf(...)`).
- Test:
  - `server/atlas-contract/tests/graph_api.rs`;
  - `server/atlas-contract/tests/contract_generation.rs`.

- [ ] **Step 0:** Re-derive the `RowFamily` members whose row struct records a `justification: Justification` (`graph-types/src/edge.rs`, `graph-types/src/canon/rows.rs`) and write the list in the ledger.
- [ ] **Step 1 (A): Failing tests.**
  - `curated.rs`: `an_eternity_ground_is_read_as_a_scripture_justification`; `an_incarnate_person_who_is_also_eternal_fails_the_compile` (OPEN 4b only).
  - `people.rs` (or `compile.rs`'s test module): `an_eternal_person_carries_no_year` (whole `Person` for `god_1324` from the curated file over a fixture).
  - `event_world.rs`: `every_family_that_records_grounds_justifies_its_edges_by_them` walks `RowFamily::ALL` with one fixture row per family, asserting the whole `justified-by` page of each edge (24b: a family added later is walked); `an_eternal_persons_grounds_justify_the_person`.
  - `graph_api.rs`, expectations read from the artifact (F-8): `gods_record_serves_no_year` (whole `PersonLife`); `gods_justified_by_group_is_his_curated_grounds` (the page equals `people-eternal.toml`'s grounds as text-unit refs with compiled labels); `the_parent_of_edge_from_joseph_to_jesus_is_justified_by_its_curated_grounds` (the edge read from `/api/node/Person:jesus_905/edges?kind=child-of`, its `justified-by` page equal to `parentage.toml`'s grounds).
- [ ] **Step 2 (A):** `cargo test -p atlas-etl -p atlas-graph -p atlas-contract` and `(cd graph-types && cargo test --all-features)` → red; record the reds.
- [ ] **Step 3 (A): Implement** as in Types. `grounds_of` has no wildcard arm.
- [ ] **Step 4 (A): Rebuild (critical section `contract`):** rebuild `data/compiled`, re-pin `canon_vectors`, re-bless. Expect: the version root moves; `justified-by` counts grow (record per family in the ledger); no label changes. If `scene_byte_identity` or a golden map fixture moves, **stop** for the owner (O-GOLDEN rule). Release the lock.
- [ ] **Step 5 (A):** `cargo test -p atlas-etl -p atlas-graph -p atlas-contract`, graph-types, `bash scripts/contract-gate.sh --base <base>` → green. **Commit:** `persons: eternity is applied at compile (no years) and every curated ground is a justified-by edge, kinship and eternity included (rule 27; 24b over RowFamily)`.
- [ ] **Step 6 (B): Failing tests:** `graph_api.rs` `a_persons_record_serves_its_life_without_ground_strings` (whole `PersonLife` for `aaron_1`, read against the artifact's payload); OPEN 4b: `jesus_record_is_incarnate`. `contract_generation.rs`: the document's `PersonLife` has no `eternal_grounds`.
- [ ] **Step 7 (B):** red.
- [ ] **Step 8 (B): Implement** the wire change.
- [ ] **Step 9 (B): Regenerate (critical section `contract`):** `export_contract`, `export_aqc_examples`, `--check` clean; `CHANGELOG.md` AQC **major**; re-bless pacts; `dotnet run --project client.ContractGenerator`. Release the lock.
- [ ] **Step 10 (B, lock `heavy`):** `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh --base <base>`, `bash scripts/contract-semver-gate.sh` (declared major), `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green. The regenerated client has no `PersonLife.EternalGrounds`, so Part B deletes its one reader, the grounds-chip statement in `PersonLifeSection` (`PopoverSectionProviders.cs`, the `if (life.Eternal && life.EternalGrounds.Count > 0)` block); the class itself dies in Task 4. PERSON-3's grounds step is red until Task 5 (ledger). **Commit:** `persons: the record serves a person's life without ground strings; grounds are edges (AQC major)`.

### Task 2: Parentage wording is data, and a parent-of edge is labelled by it (backend)

**Backend change: yes (data + compiler labels only; no served code). Why there (rule 27, 26):** the wording is a domain fact, so it lives in `data/` with provenance; an edge's label is a data-only derivation, so the compiler writes it. The client then shows labels as served.

**Owner gate first:** OPEN 3 (or the default). Starts after FOCUS-2 Task 2 has landed (`labels.rs`).

**Files:**
- Modify:
  - `data/curated/parentage.toml` (a `[[wording]]` table: `kind`, `as_parent`);
  - `server/atlas-etl/src/{curated,compile}.rs`;
  - `server/atlas-core/src/data.rs` (`AtlasData.parentage_wording`);
  - `server/atlas-graph/src/labels.rs`;
  - `data/compiled` (rebuilt), and pacts.
- Test:
  - `curated.rs` unit tests;
  - `labels.rs` unit tests;
  - `server/atlas-contract/tests/graph_api.rs`.

- [ ] **Step 1: Failing tests.**
  - `curated.rs`: `every_declared_parentage_has_wording_and_natural_has_none` (walks `Parentage::ALL`); `a_missing_wording_fails_the_compile`.
  - `labels.rs`: `a_parent_of_edge_with_a_declared_parentage_reads_its_curated_wording` (whole string); `a_natural_parent_of_edge_reads_its_kind`.
  - `graph_api.rs`: `jesus_child_of_page_names_each_parentage_on_its_edge`: the whole `child-of` page's edge labels equal `{parent label} · {wording} · {child label}`, wording read from `parentage.toml`.
- [ ] **Step 2:** `cargo test -p atlas-etl -p atlas-graph labels` → red.
- [ ] **Step 3: Implement** as in Types.
- [ ] **Step 4: Rebuild (critical section `contract`, after Task 1 Part A's):** rebuild, re-bless. Only parent-of edge labels with a declared parentage change (the ledger records the count: the declared rows in `parentage.toml`). A golden map fixture that moves stops the task for the owner.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh --base <base>` → green, the vocabulary gate (rule 27 closure) included. **Commit:** `labels: a parent-of edge is labelled by its curated parentage wording, moved out of client code into data (rules 26, 27)`.

### Task 3: A person on FocusView: life fields, kinship entries, collapsed mentions, years that open the World (client)

**Backend change: no.** Everything here is presentation over served data: the client's under rule 27.

**Owner gate first:** OPEN 2, 4, 5, 6 (or the defaults). Starts after FOCUS-2 Task 3 has landed (`Presenter.cs`, `FocusView.razor`).

**Files:**
- Create:
  - `client/Exploring/WorldQuery.cs`;
  - `client.Tests/Explore/WorldQueryTests.cs`.
- Modify:
  - `client/Exploring/{Affordances,Presentation,Presenter}.cs`;
  - `client/Views/FocusView.razor`;
  - `client/Components/ExplorerPopover.razor` (`OnShowYear`);
  - `client/wwwroot/css/app.css` (`.focus-field-year`, the `<details>` summary cursor);
  - `client.Tests/stryker-config.exploring.json` (adds `**/WorldQuery.cs`).
- Test:
  - `client.Tests/Explore/{AffordancesTests,GraphPresenterTests,PresentationTests}.cs`;
  - `client.Tests/Views/FocusViewTests.cs`;
  - `client.Tests/Components/ExplorerPopoverTests.cs`;
  - `client.Tests/Explore/ServedGraph.cs`.

- [ ] **Step 1: Failing tests.**
  - `GraphPresenterTests`:
    - `A_person_on_the_popover_presents_their_birth_and_death_with_their_provenance` (whole `Card`; `Born`/`Died` carry `At`);
    - `An_eternal_person_presents_as_eternal_with_no_year`;
    - `A_person_with_no_birth_or_death_presents_the_span_they_are_mentioned_across`;
    - `An_incarnate_person_presents_their_dates_as_their_earthly_life` (OPEN 4b);
    - `A_person_served_without_their_life_is_a_contract_breach` (message names the person).
  - `AffordancesTests`: the table test now expects `ParentOf`, `ChildOf → Kinship` and `MentionedIn → Mentions`; `Affordances_are_total` stays green.
  - `WorldQueryTests.A_year_opens_the_world_at_that_year_alone` (whole string).
  - `FocusViewTests`:
    - `A_kinship_entry_reads_its_edges_compiled_label` (whole markup of `popover-section-child-of`);
    - `A_collapsed_list_is_a_closed_disclosure_after_every_open_list` (ordered section ids and the `<details>` without `open`);
    - `A_year_field_is_a_button_that_shows_that_year` (the `OnShowYear` argument is the whole served `Year`);
    - `A_field_without_a_year_is_plain_text`;
    - `A_person_offers_every_served_group_in_served_order` (a served Aaron-like person: `mentioned-in`, `parent-of`, `child-of`, `participates-in`, `spouse-of`; asserts the ordered section ids).
  - `ExplorerPopoverTests.Showing_a_year_navigates_to_the_world_at_that_year`.
- [ ] **Step 2:** `dotnet test client.Tests --filter "GraphPresenterTests|AffordancesTests|PresentationTests|FocusViewTests|WorldQueryTests|ExplorerPopoverTests"` → red or compile errors.
- [ ] **Step 3: Implement** the types exactly as above. `CardOf` stays one expression; `LifeOf` holds no arithmetic. Under OPEN 2(a)/5(a)/6(a) the matching member is not built (rule 4: no member without a reader).
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green. A person still opens on the legacy popover (Task 4 switches it), so no Playwright change yet; run `npx playwright test tests/ux --grep "world|place|polity|edge"` → the same set as at the base (the `Disclosure` and `EntryText` defaults leave other kinds unchanged except `mentioned-in` under 6b, which the ledger lists by spec).
- [ ] **Step 5: Commit:** `focus: a person's card presents their life, kinship entries read their compiled edge label, mentions open collapsed and last, and a served year opens the World (R16, §3.3 amended, D5)`.

### Task 4: Persons open by position, and the legacy person path is gone (client)

**Backend change: no.**

**Files:**
- Modify:
  - `client/Components/MentionScan.razor`;
  - `client/Legacy/{LegacyNodes,PopoverSections,PopoverSectionProviders}.cs`;
  - `client/Exploring/PopoverChromeRegistry.cs`;
  - `client/wwwroot/css/app.css` (the four person rules);
  - `client.Tests/Explore/{DeletionLawTests,LegacyNodesTests,LegacyViews,IdentityTests,PushViaConformanceTests,RootConsistencyLawTests}.cs`;
  - `client.Tests/{PopoverSectionRegistryTests,PopoverChromeConformanceTests}.cs`;
  - `client.Tests/Components/PlaceOpeningTests.cs` → renamed `MentionOpeningTests.cs`;
  - `client.Tests/{WholeReadLawTests,ReferenceParsingLawTests}.cs` (FOCUS-2's ratchets: remove any entry this task's deletions retire, nothing else).
- Delete:
  - `client/Legacy/PersonNode.cs`, `client/Components/PersonMentionsList.razor`, `client/Exploring/Kinship.cs`;
  - `client.Tests/Components/PersonMentionsListTests.cs`, `client.Tests/KinshipTests.cs`.
- Test:
  - `client.Tests/Explore/DeletionLawTests.cs`;
  - `client.Tests/Components/MentionOpeningTests.cs`.

- [ ] **Step 1: Failing tests.**
  - `DeletionLawTests`: add `NodeKind.Person` to `MigratedKinds` (and, if FOCUS-2's `LegacyNames` exists, `Person → ["Person"]`). Run → red: `PersonNode`, the four `AppliesTo(... == "Person")` strings.
  - `MentionOpeningTests.Every_anchored_mention_opens_on_its_served_node`: walks one anchored piece per `NodeKind` the reader anchors today (Place, Person) and asserts each whole `PopoverOpening.Explore`. Red on Person. This is the 24b closure: MentionScan has one opening for every anchor.
  - `MentionOpeningTests.A_person_mentioned_in_the_text_toggles_its_served_node_into_the_selection` (the place twin, unchanged behaviour).
- [ ] **Step 2:** red.
- [ ] **Step 3: Implement.**
  - `MentionScan`: one opening as in Types.
  - `LegacyNodes.For` answers `null` for `Person`, so every person, opened or followed (from an event's participants, a verse's mentions, a saved exploration), renders on FocusView.
  - Delete the files and members in the inventory.
  - `RootConsistencyLawTests` names the app assembly by `typeof(ExplorerPopover)`; its `Doors` stays the one door.
  - `PushViaConformanceTests` loses the four person rows and its `PersonSectionRendering`/`Chips` special case.
  - `PopoverChromeConformanceTests.ConcreteExplorableNodeClasses` decreases by one.
  - Re-run the two FOCUS-2 ratchets: their pair laws fail for any listed site this task deleted; remove exactly those entries.
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green. `npx playwright test tests/ux/person-card.spec.ts tests/ux/reader-persons.spec.ts tests/ux/popover-sections.spec.ts tests/ux/reader-recursion.spec.ts tests/ux/kretzmann.spec.ts` → record the red set by name in the ledger (expected: PERSON-1…4, PERSONS-1 loop, PERSONS-2).
- [ ] **Step 5: Commit:** `person: a person opens and renders on FocusView from its served position; PersonNode, its four sections, PersonMentionsList and Kinship are gone (deletion law: Person; F-63, F-31, F-66)`.

### Task 5: Re-express the person specs (TypeScript)

**Backend change: no.**

**Files:** `tests/ux/person-card.spec.ts`, `tests/ux/reader-persons.spec.ts`, `tests/ux/world-hover-text.spec.ts` (OPEN 6b only: its place popover's `mentioned-in` list, line 175, is opened before its link is clicked), `tests/ux/CONTRACT.md` (the person-card section and line 2808's `PersonNode` note).

- [ ] **Step 1:** Re-express by name, every expected value read from the API (F-8), keeping D5's intent:
  - **PERSON-1** (Aaron, `EXO.4.14`): `popover-field-Born` / `-Died` hold the served labels; `popover-section-participates-in-heading` = `Participates in ({count from api.node})`; `popover-section-parent-of`, `-child-of` (`popover-link-child-of-Person:amram_242` …), `-spouse-of`; `popover-section-mentioned-in` is a closed `<details>` and the last section (OPEN 6b). Siblings: replaced by "a sibling is one step away": follow `popover-link-child-of-Person:amram_242`, then assert `popover-link-parent-of-Person:moses_2108` is offered.
  - **PERSON-2:** `popover-year-Born` opens `/world?from=-1575&to=-1575` (OPEN 5b; under 5a the test asserts no year button and is renamed).
  - **PERSON-3** (God, `GEN.1.1`): `popover-field-Life` = `Eternal`; no `popover-field-Born`, no `popover-year-*`; `popover-section-justified-by` lists `PSA.90.2` and `REV.1.8`; following `popover-link-justified-by-text-unit:PSA.90.2` lands on `PSA.90.2`.
  - **PERSON-4** (Jesus, `/read/LUK/3` verse 22's mention, as today): the `child-of` entries read the three compiled edge labels from `api.nodeEdges('Person:jesus_905', 'child-of')`; `popover-section-brethren-of-heading` = `Brethren of ({count})`; no siblings section; OPEN 4b: `popover-field-Born (earthly life)`.
  - **PERSONS-1 loop:** `popover-section-mentioned-in-heading` = `Mentioned in ({mentionedInCount})`; open it; follow `popover-link-mentioned-in-{first verse id}`; title = the verse.
  - **PERSONS-2 paging:** the ids become `popover-section-mentioned-in-more`, `-collapse`, `-position`; rows `[data-testid^="popover-link-mentioned-in-"]`; the 20/40 window and the position text are unchanged.
  - **world-hover-text** (OPEN 6b): open `popover-section-mentioned-in-heading` before clicking `popover-link-mentioned-in-{verse id}`; nothing else changes.
  - Comments in the touched spec lines that name `PersonNode` or the deleted sections are removed with the lines they describe.
- [ ] **Step 2:** `npx playwright test tests/ux/person-card.spec.ts tests/ux/reader-persons.spec.ts tests/ux/popover-sections.spec.ts tests/ux/reader-recursion.spec.ts tests/ux/kretzmann.spec.ts` → green.
- [ ] **Step 3: Commit:** `tests: the person specs read FocusView's generic presentation; a sibling is one step through a parent (ruling 2)`.

### Task 6: Gates, mutation, close

- [ ] **Step 1: Gates.**
  - `client.Tests/stryker-config.exploring.json` covers `**/Affordances.cs`, `**/Presenter.cs`, `**/Presentation.cs`, `**/WorldQuery.cs` (verify).
  - Mutation, inside the owner's window only (critical section `heavy` with "mutation" in the message, `free -g` ≥ 18 GB): `bash scripts/mutants-parallel.sh -n 3 -b <base>` → 100% or equivalents recorded (covers `grounds_of`, `add_justified_by`, `relation_label`, the eternity clear, `parse_people_eternal`, the wording parse); then `dotnet stryker` → 100% or equivalents recorded. Outside the window, `.superpowers/MUTATION-GATE-DEBT.md` names `<base>`.
  - Then (lock `heavy`): `cargo test --workspace && (cd graph-types && cargo test --all-features) && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh --base <base> && bash scripts/timing-gates.sh run && npx playwright test tests/ux` → green except the carried reds (`world-quiet-places` density smoke, `world-cluster-chooser` C3-M1) and any F-55 flake named by spec.
  - The diff adds no comment line (`git diff <base>.. | grep -E '^\+.*(//|/\*|<!--|#\[doc)'` over application files prints nothing).
- [ ] **Step 2:** Write the close report, `docs/superpowers/reports/2026-10-0x-focus-4-close.md` (Codex's lane): every rule-24 category with its closure and guarantee (below), the FINDINGS raised, the §3.3 amendment (`Field.At`), the §3.4 amendment (`SectionList` gains `EntryText`, `Disclosure`), and the behaviours removed by rulings (siblings; the `FAMILY` and `LIFE` headings; the `c.` before a year).
- [ ] **Step 3:** Push `lane/codex/F4-int`; set `CX-F4` to `review` with the commit range. Claude reviews (14b + 24a) and lands under `land`.

**Rule-24 categories and their closures:**

| Category | Closure | Guarantee |
|---|---|---|
| Person served by both mechanisms | deletion law (Person) | compiler + law |
| A mention opens a legacy node | `MentionScan`'s one opening; `Every_anchored_mention_opens_on_its_served_node` | law over every anchored kind |
| A curated ground the graph does not hold as an edge | `grounds_of` exhaustive over `RowFamily`; eternity grounds as edges | compiler (no wildcard) + `RowFamily::ALL` walk |
| A curated eternity served as a lifespan | the compile clears an eternal person's years; `an_eternal_person_carries_no_year`, `gods_record_serves_no_year` | ETL law + real-data law |
| Domain wording in client code (`Kinship`, `"Jesus"`) | wording in `data/curated/parentage.toml`, compiled into labels; the literal deleted with its class | compile law over `Parentage::ALL`; source scan in review |
| A whole collection read on the client (F-63, person sites) | the five reads and the per-parent walk deleted; lists page through the one door | `WholeReadLawTests` ratchet (FOCUS-2) + `RootConsistencyLawTests` |
| A World query composed from a year at each site | `WorldQuery.At` for FocusView | partial: legacy sites listed as a FINDING |

---

## Wave schedule

Primary is the critical path; the companion is unlike work beside it (rule 23: Rust beside C#). A task starts only when the tasks it names as inputs have landed on `lane/codex/F4-int`.

| Wave | Primary | Companion | Critical sections | Expected red at wave close |
|---|---|---|---|---|
| 0 | owner: OPEN 1–7 (defaults stand if unanswered); Task 0 | — | — | — |
| 1 | Task 1 Part A (Rust; rebuild #1) | Task 3 tests written against `ServedGraph` (C#) | `contract` rebuild #1, re-bless; `heavy` | — |
| 2 | Task 1 Part B (regen; AQC major) | Task 3 implemented | `contract` regen, re-bless; `heavy` | Playwright PERSON-3 (grounds chips gone until Task 5); under OPEN 6b, `world-hover-text:175` |
| 3 | Task 4 (C#) | Task 2 (Rust; rebuild #2) | `contract` rebuild #2; `heavy` | Playwright: PERSON-1…4, PERSONS-1 loop, PERSONS-2, and wave 2's |
| 4 | Task 5 (TypeScript) | — | — | none of the above |
| 5 | Task 6 | — | `heavy` (mutation only in the owner's window) | only the carried reds and named F-55 flakes |

**Critical path:** OPEN answers → Task 1 → Task 3 → Task 4 → Task 5 → Task 6. Task 2 rides beside Task 4 and lands before Task 5 (PERSON-4 reads its labels).

## FINDINGS this plan expects to raise (for the queue; the owner decides)

- **World queries are composed at several client sites** (`MapFocusHatch.Query`, the legacy chips' `from=…&to=…` in `EventNode`, `YearNode`, `AuthorNode`, `PolityDeltaNode`, `BookNode`, `ChapterNode`). Closure: one `WorldQuery` composer (FOCUS-4 creates it for years); each site migrates with its batch.
- **Other kinds' date fields are not explorable** (Place established/destroyed, Polity reign, Map/Era window) under D5's "years are explorable positions". Closure: `GraphPresenter` sets `At` (or a window) on every served date field.
- **`PopoverChromeRegistry` is not in FOCUS-2's file list** though its Verse/ConcordUnit rows die there (review gap for FOCUS-2).
- **`also_called` serves search names, not names** (`god_1324`: "last", "hosts", "father"). Not shown on the card; A-NAMES' worklist.
- **A person's `description` is served (Easton text with site-relative markdown links) and shown nowhere**; it is also under A-THEO's licence question. Not shown by this plan.
- **F-53 now reaches persons:** a person mention opens `PopoverOpening.Explore`, which is value-equal across clicks, so a second click on the same mention keeps the trail, as places do (owner's open F-53 question).
- **F-56 now reaches persons:** Escape after a step inside a person's FocusView behaves as on places.
- **Pre-existing comments in touched files** (F-12): only lines this batch deletes take theirs.

## Self-review against rule 27, the spec and the brief

- **Data-only derivations are compiled:** an eternal person's absent years (Task 1); every curated ground as a `justified-by` edge (Task 1); a parent-of edge's label from curated wording (Task 2). The edge counts the card shows are the compiled `edge_count` (schema 22).
- **Per-request derivations are bounded index reads:** the element read for a person; one neighbour page per group (20, server-capped). No new read; no read changes beyond one field leaving.
- **Interaction derivations are the client's:** the Person popover row (`Card`), the life fields, `EntryText`, `Disclosure`, the year buttons, every follow through `Explore`.
- **The graph models the domain, never a view:** no relation appended; a ground is the curated fact's own justification; the wording is the curated relation's own name. The Siblings view and the FAMILY grouping are not built into the graph (ruling 2, 3).
- **27b/27c/27e:** the person popover reads one element and one first page per group; the five whole reads and the per-parent walk are gone.
- **Coverage of §5's FOCUS-4 row:** every provider named is deleted; `PersonNode` is deleted; no route is Person-only.
- **Type consistency:** `PersonLife` (Task 1) is what `LifeOf` (Task 3) reads; `Field.At` is what FocusView renders as a year button and `WorldQuery.At` composes; `Affordances.Kinship` reads the labels Task 2 compiles; `MentionScan`'s opening is `PopoverOpening.Explore`, which FOCUS-6 built.

## Assumptions

**Verified against `eea9023`:**
- The four sections and `PersonNode` sit where the re-anchor table says; `PersonCardAndMentionsSection` opens its window through `Paging.Window(PresentationRequest, EdgeKind)` (F-70), the door FocusView uses for every group (`FocusView.razor` `Load`).
- `PersonLife` is `{gender, birth, death, first, last, eternal, eternal_grounds, also_called}`; `eternal_grounds` are curated dot-ref strings (`data/curated/people-eternal.toml`), read into `Person.eternal_grounds: Vec<String>` (`atlas-etl/src/compile.rs:61–68`).
- `/api/node/Person:god_1324` serves `first` 4004 BC and `last` AD 96 with `eternal: true`; the legacy client hides them by checking `Eternal`.
- `/api/node/Person:aaron_1`'s groups are `mentioned-in`, `parent-of`, `child-of`, `participates-in`, `spouse-of`; Jesus's add `speech-of` and `brethren-of`; God's are `mentioned-in`, `parent-of`, `participates-in`.
- `add_justified_by` wires only `DatedBy`, `Fulfills`, `Typology`, `NamedAfter` (`_ => continue`), though `ParentOf` and `Brethren` rows carry grounds (`parentage.toml`, `brethren.toml`).
- `EdgeRecord.meta` carries `EdgeMeta::Parentage`; `labels::edge_label` is `{subject} · {kind display} · {object}`.
- `Kinship.Labels` holds the parentage wording in client code; `PersonLifeSection` tests `person.Title == "Jesus"`.
- FocusView renders a `SectionList` heading as `{DisplayLabel} ({Count})` on the popover, entries by neighbour label, an edge step per entry, and `PageControls` per group.
- `ContractGeneration.Unread` is empty; `PersonLife` and `Parentage` stay reachable (`EdgeEntry.parentage`).

**To verify at execution (each with its fallback):**
- **The compiled `edge_count` counts `justified-by` from a person node.** `IndexPass` runs `add_justified_by` before labels and the writer; if the count misses node-subject pairs, Task 1 extends the count with a law and reports it.
- **No golden map fixture moves** with the justified-by growth (Task 1 Step 4 stops otherwise).
- **Jesus's popover is reachable from a reader mention** in PERSON-4's current verse; if the page moved, the spec reads the mention from the artifact.
- **`canon_vectors.rs` re-pins only the Person payload's vectors.** Any other vector moving stops Task 1 for the controller.
