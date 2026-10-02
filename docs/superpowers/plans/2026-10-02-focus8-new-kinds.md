# FOCUS-8 (the kinds no batch owned: Narrative, Anchor, Source, Translation, PeopleGroup, LexiconEntry) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every node kind opens, presents and explores on the one new path. The six kinds that no earlier batch owned (spec §5 "the seven new kinds", §10 R3) are proven, kind by kind, by a Playwright spec that reaches each one through the app; the wire stops losing two of them; their cards carry the facts their records hold; and the popover's legacy fallback path (a legacy node's `ExploreAsync`/`BodyAsync` rendered when no section provider applies) is deleted, with every legacy piece that FOCUS-2…7 left without a caller.

**What is true after FOCUS-8:**
- Every member of `NodeKind` round-trips through the wire id (`encode_node_id` / `decode_node_id`), proven by a law that walks `NodeKind::ALL`. Today `PeopleGroup` and `Source` encode but do not decode, so no link to one can be followed and no saved step at one can be resumed.
- For every kind, the count a node's `edge_summary` gives for a kind equals what its neighbour pages return, proven on the real artifact for one node per kind.
- Each of the six kinds has a Playwright test that opens it from the app, checks its served title and fields, follows one of its links and comes Back. A coverage law fails when a `NodeKind` has no spec that opens it.
- A lexicon entry's card shows its Strong's number, transliteration and glosses; an anchor's card shows its date (OPEN 2).
- A narrative's card lists its events in order (OPEN 3).
- The popover has two body paths: `FocusView`, and the section registry, now serving `PolityDeltaNode` alone (held for MAPS; FOCUS-9 retires the registry). `IExplorable.BodyAsync`, the `_body` branch, and `LegacyNodes.For` are gone.

**Architecture (rule 27, by who knows the inputs):**
- **Compiler:** a narrative's legs become `contains` rows (OPEN 3): a data-only fact from `data/curated/narratives/*.toml`.
- **Server:** reads. The wire id codec becomes total over `NodeKind` (one table, both directions). `NodeRecord` gains two per-kind details read from the node's own row (`lexicon`, `anchor`; OPEN 2), the precedent `place`/`book`/`map`/`era`/`polity` set. No route is added or removed.
- **Client:** derives nothing new. `GraphPresenter.CardOf` reads the new details as fields. The legacy fallback is deleted.

**Tech Stack:** Rust (axum, utoipa, `atlas-contract`, `atlas-graph`, `atlas-etl`, `graph-types`); .NET 10 Blazor WebAssembly (`BibleAtlas.Client`, `BibleAtlas.Client.Exploring`); xUnit + bUnit; Playwright; Stryker.NET; cargo-mutants via `scripts/mutants-parallel.sh`.

**Spec:** `docs/superpowers/specs/2026-09-26-focus-exploration-design.md` §1, §4 (the "Narrative, Anchor, …" row), §5 (FOCUS-8 row), §9, §10 R3, §12 R15–R20. **Principles:** 4, 9, 12, 14b, 21–23, 24–24b, 25, 26, 26a, 27–27g. **Queue:** A-FPLANS (7+8); F-34 (per-kind details, noted), F-36 (year labels, noted), F-50 (anchor citations carry curation notes).

**Agent:** Codex (roadmap Lane F: "FOCUS-2 and FOCUS-4/5/7/8"), or Claude; nothing below depends on which.

**Base:** the head of `worktree-bible-atlas-m1` after FOCUS-2, 3, 4, 5 and 7 have landed (FOCUS-6 landed at `b3d7cfa`). Write it into the ledger (`.superpowers/sdd/2026-10-0x-focus8/progress.md`) at Task 0 and pass it to every gate as `--base <base>` (PRINCIPLES 22). This plan was written against `b3d7cfa` with the FOCUS-2, 3 and 4 plans read; the FOCUS-5 and FOCUS-7 plans were not yet pushed, so what this plan assumes they delete is stated under "Assumptions about FOCUS-2…7" and checked by Task 0.

## OPEN: for the owner, before the task named starts (each blocks only that task)

Each is answerable in one line. The plan builds the recommendation if unanswered.

1. **People groups and sources cannot be opened today (Task 1):** a link to a people group (the Moabites) or to a source fails, because the server can write their ids but not read them back. Fix it here, for every kind at once, with a test that checks every kind? **Recommend yes.**
2. **Cards for lexicon entries and anchors (Task 2):** a lexicon entry's card shows only its Hebrew/Greek word; an anchor's card shows its citation but not its date. Add the Strong's number, transliteration, part of speech and glosses to the lexicon card, and the date to the anchor card? **Recommend yes.**
3. **A narrative lists its events (Task 3):** a narrative ("Abraham's Migration") today links only to the maps it is drawn on, not to its events. (a) Add "contains" links from each narrative to its events, in order, from the curated narrative files; (b) leave it, and queue it. **Recommend (a).**
4. **Translations have no links (Task 6):** the six translation nodes (Vulgate, Douay-Rheims, …) are linked to nothing, so the app can only reach one through a saved exploration. Prove them that way and queue "what should link to a translation" as a finding? **Recommend yes.**
5. **Anchor citations show curation notes (Task 4, F-50):** an anchor's served citation still contains developer notes ("HOTFIX-7…"). Show the citation on the card now as served and leave F-50 to its own fix, or hide the citation until F-50 is fixed? **Recommend show as served; F-50 fixes the data.**

**Rulings applied by analogy, not re-asked:**
- **R3 (the server kinds get the generic Card and their frontier in FOCUS-8)** settles that no kind gets a bespoke presentation here: each is a `Card` of served fields plus its frontier.
- **FOCUS-6's rule-4 pull-forward** ("`YearNode` is constructed only by `PlaceCard`… so they die here"): a legacy piece that loses its last caller dies in the batch that removed the caller or, if that batch left it, in the first batch after. Task 5 applies it to whatever FOCUS-2…7 left (see the inventory).
- **FOCUS-2 ruling 7 (reach every site of the category in the batch):** Task 1 closes the wire-id codec for every kind, not only the two found broken.
- **FOCUS-2 ruling 9 (accept the AQC major when the semver gate classes a change so):** Tasks 1–3 take whatever class `scripts/contract-semver-gate.sh` assigns.
- **The place/book precedent for per-kind details** (FOCUS-6 Task 4 added `map`/`era`/`polity`): OPEN 2 follows it; F-34's sum-type migration of those details is not started here.

## Global Constraints

- `docs/PRINCIPLES.md` binds, especially:
  - rule 4: zero dead code;
  - rule 9: no comments in application code (the review greps the diff for added comment lines; a comment on a deleted line goes with it; no other comment is touched);
  - rule 12: every signature below is for sign-off;
  - rules 24, 24a, 24b: every fix names its category and closes it; an offender found on the side goes to FINDINGS;
  - rule 25: the client composes over the contract (no id composition, no year formatting, no kind decided from a string);
  - rules 26, 26a: narrative legs, lexicon facts and anchor dates are read from the artifact, entered only through the ETL;
  - **rule 27**: data-only derivations compiled (narrative containment); per-request reads indexed and bounded (the node record's details); interaction derivations the client's. No client exploration word (explore, explorable, frontier, card, popover, presentation) in `server/` or `graph-types/` (the vocabulary gate, `server/atlas-contract/tests/backend_vocabulary.rs`).
- Tests: whole-body assertions; one behaviour per test, named as a sentence; `// Arrange` `// Act` `// Assert` only; no magic numbers; newspaper order. Real-data expectations are read from the artifact or the API, never written as literals (F-8).
- **Total matches.** A closed sum is matched exhaustively; `Presentation.Of` and `Affordances.Of` stay total.
- **The one walk door and the one paging door** (R19, R20, F-59, F-70): no new caller of `IExplorer.Resolve`; every list pages through `Paging`.
- **No relation is appended.** OPEN 3 adds a row family under the existing `Contains` relation; `DECLARED_DIRECTED_RELATIONS` and `DECLARED_SYMMETRIC_RELATIONS` do not change.
- Build no interaction that works only by hovering.
- Every task in its own worktree (`git -c core.autocrlf=false worktree add -b lane/<agent>/F8-t<n> ~/w/F8-t<n> <base>`), `CARGO_TARGET_DIR=~/mut/<agent>-F8-t<n>`, `nice -n 10 cargo -j 4`; `data/raw` and `data/cache` copied, never linked (AGENTS.md). Integrate on `lane/<agent>/F8-int`; the other agent reviews; Claude lands one squashed commit per task under `land`. Never force.

## Critical sections (PRINCIPLES 21): one holder at a time

| Section | Held by | Why |
|---|---|---|
| `contract`: rebuilding `data/compiled` | Task 3 | one artifact; the version root moves |
| `contract`: `export_contract`, `export_aqc_examples`, `client.ContractGenerator` | Task 1, then Task 2, then Task 3, strictly in that order | one generated document |
| `contract`: re-blessing pacts and fixtures | right after each rebuild or regen | moves the version root |
| `heavy`: `cargo test --workspace`, full Playwright | Tasks 1, 2, 3, 7 | memory |
| `heavy` with "mutation" in the message | Task 7 only, inside the owner's window | once per batch (3a) |
| appending to `relations!` | **nobody** | rule 27 |

## The six kinds, grounded at `b3d7cfa`

Probed against the source at `b3d7cfa` and a running `atlas-server` on :8000 (an older build; Task 0 re-probes on the base). "Way in" is the click path Task 6 uses; every id in it is read from the API at test time, never written as a literal.

| Kind | Nodes | Frontier today | Record today | Way in (Task 6) |
|---|---|---|---|---|
| Narrative | 13 (`data/curated/narratives/*.toml`) | `shown-on` only (e.g. `Narrative:exodus` → 2 maps); legs are only a `narrative` attribute on events' `follows-in` entries | label, provenance | focus a Map on /world, follow its `shows` link to a narrative |
| Anchor | curated (`chronology-anchors.toml`) | `dates` → event, `justifies` → an edge (e.g. `Anchor:david-jerusalem`) | label, provenance, `description` = citation (F-50) | an event's `dated-by` link (the first event whose `dated-by` neighbour is an Anchor) |
| Source | `kretzmann_adapter` (`Source:` the commentary work) and justification grounds (`Ground::Source`) | `justifies` where a ground names it | **cannot be read**: `decode_node_id` has no `Source` arm | a `justified-by` link to a source if the artifact has one; else the saved-exploration door |
| Translation | 6 (`brainfuel_adapter::EDITION_LABELS`) | **empty** | label, provenance | the saved-exploration door only (OPEN 4) |
| PeopleGroup | `people-groups.toml` (6 groups, 18 `named_after`) + Theographic + reclassified | `mentioned-in`, `namesake-of` inverse | **cannot be read**: `decode_node_id` has no `PeopleGroup` arm; on :8000 `Person:moab_2103` summarises `namesake-of: 1` but its page is empty | a person's `namesake-of` link, or a verse's `mentions` link to a group |
| LexiconEntry | lexicon section | `occurs-in` (e.g. `LexiconEntry:H1980` → 1,322) | label (the lemma), provenance; Strong's, transliteration, glosses not served | a verse's `words` link |

Era, Polity and Map were the World view's (FOCUS-6, `tests/ux/world-geography.spec.ts`); Task 6's coverage law counts them as covered there (Era only if a spec there opens one; else Task 6 adds an Era test). Era's empty frontier (an era does not link its own Map) is raised as a finding, not fixed.

## What the popover does today, and what FOCUS-8 changes

`client/Components/ExplorerPopover.razor` at `b3d7cfa` renders the body three ways:
1. `_legacy is null` → `FocusView` (every kind `LegacyNodes.For` answers `null`, and every `PopoverOpening.Explore`);
2. `_usingSectionRegistry` → the `PopoverSectionRegistry` providers whose `AppliesTo(legacy)` holds;
3. otherwise → **the legacy fallback**: `_chips` from `legacy.ExploreAsync(Atlas)` in the head and `_body` from `legacy.BodyAsync(Atlas)` (`LoadLegacy`, the `else { @_body }` arm).

At `b3d7cfa` path 3 serves the legacy nodes that no provider applies to: `AuthorNode` ("Author") and `BookNode` ("Book"). FOCUS-3 deletes `BookNode`. `AuthorNode` is constructed only by the "About this book" / authored-by chips of `VerseNode`, `PassageNode`, `BookNode` and `ChapterNode` (FOCUS-2, FOCUS-3), so after FOCUS-3 nothing constructs it. FOCUS-8 deletes path 3 (Task 5). Path 2 stays for `PolityDeltaNode` until FOCUS-9 folds it into one border-change view.

## Assumptions about FOCUS-2…7 (Task 0 checks each; a miss is recorded, and the piece joins Task 5's inventory)

| Piece at `b3d7cfa` | Deleted by (plan) | If still present at the base |
|---|---|---|
| `VerseNode`, `ConcordUnitNode`, nine verse/paragraph sections, `LegacyNodes.For`'s TextUnit arm | FOCUS-2 | stop: the base is wrong |
| `ChapterNode`, `BookNode`, `PassageNode`, `ChapterCardSection`, the Container arm | FOCUS-3 | stop |
| `PersonNode`, four person sections, the Person arm, `PopoverChromeRegistry["Person"]` | FOCUS-4 | stop |
| `EventNode`, `INarrativeAware`, six `Event*Section`s, `MapFocusHatch` (F-54), `ArrowNav`'s and `PericopeHeading`'s legacy openings, `ExplorerPopover.SyncNarrativeFocusAsync`, the Event arm | FOCUS-5 (assumed; plan not pushed when this was written) | stop |
| `CatechismNode`, `CommentaryItemNode`, five `Catechism*Section`s, `CatechismInConcordSection`, `CommentaryItemProseSection`, `CatechismList`'s and `Kretzmann.razor`'s legacy openings, the two arms | FOCUS-7 (assumed; plan not pushed) | stop |
| `YearNode`, `YearFrontierSection` (`YearNode`'s only constructor is `EventDateAndPlacesSection`, FOCUS-5's) | FOCUS-5 by the rule-4 pull-forward, or FOCUS-9 by spec §5 | **Task 5 deletes them** (no caller once `EventDateAndPlacesSection` is gone) |
| `AuthorNode` (no constructor after FOCUS-3) | FOCUS-3 by rule 4, or FOCUS-9 by spec §5 (FOCUS-3's plan keeps it for FOCUS-9) | **Task 5 deletes it**; FOCUS-9 keeps only the Author *replacement* (the book's author field as a link) |
| `LegacyNodes.For` answers `null` for every kind | the last arm leaves with FOCUS-7 | **Task 5 deletes `For`** (a constant function, rule 4); `BookContainerId`/`ChapterContainerId` stay for `LegacySaves` until FOCUS-9 |

"Stop" means: the batch that owned the kind has not landed, so FOCUS-8's base is not ready. Record it in the ledger and wait; do not delete another batch's kind here.

## Types (for sign-off, PRINCIPLES 12)

### Task 1: the wire id codec is total over `NodeKind` (backend)

`graph-types/src/node.rs` (or wherever `NodeKind` is declared by `kind_tags!`):
```rust
impl NodeKind {
    pub fn wire_prefix(self) -> &'static str;
    pub fn of_wire_prefix(prefix: &str) -> Option<NodeKind>;
}
```
- `wire_prefix` is the one table: `"text-unit"` for `TextUnit`, the variant's name for every other kind (today's `format!("{other:?}:…")` output, so no served id changes). `of_wire_prefix` is its inverse, written as a search over `NodeKind::ALL`, so a new kind needs no second edit.
- `server/atlas-graph/src/node_ref.rs::encode_node_id` writes `wire_prefix` (no `Debug` formatting on the wire; the Haskell bar).
- `server/atlas-contract/src/graph_wire.rs::decode_node_id` reads `of_wire_prefix`; the TextUnit arm keeps its Bible/Concord parse; every other kind is `AnyNodeId { kind, raw }`. The fifteen hand-written arms are deleted.

### Task 2: lexicon and anchor details (backend; OPEN 2)

`server/atlas-contract/src/wire/graph.rs`:
```rust
pub struct LexiconDetail {
    pub strong: String,
    pub lang: String,
    pub translit: Option<String>,
    pub part_of_speech: Option<String>,
    pub glosses: Vec<String>,
}

pub struct AnchorDetail {
    pub at: super::Year,
}

pub struct NodeRecord {
    pub lexicon: Option<LexiconDetail>,
    pub anchor: Option<AnchorDetail>,
}
```
- Both read the node's own payload (`NodePayload::LexiconEntry`, `NodePayload::Anchor { at, .. }`) in the one `read_node_record`, beside `map`/`era`/`polity`. `at` is the anchor's served `Year` (`wire::Year::of`, as every year is served today; F-36 is untouched). `senses` and `root` are not served: nothing reads them (rule 4); a later batch adds them with a reader.
- Each is `#[serde(skip_serializing_if = "Option::is_none")]`, a type-level `#[schema(description = "…")]` only (A-STRIP ruling), present exactly when the kind is `LexiconEntry` / `Anchor`.

### Task 3: a narrative contains its legs (backend; OPEN 3)

`graph-types` (row family, Core section):
```rust
pub struct NarrativeLeg {
    pub narrative: NarrativeId,
    pub event: EventId,
    pub order: u16,
    pub provenance: ProvenanceId,
}
```
- `RowFamily::NarrativeLeg => Directed(RelationId::Contains)`, appended to `RowFamily` (the family list is not positional; `relations!` is untouched). `contains` from a narrative lists its legs in `order`; `member-of` from an event lists its narratives.
- The ETL reads `legs` from `data/curated/narratives/*.toml` (provenance `curated-narratives`, as the narrative nodes carry). A leg that does not resolve to an `Event` fails the compile (26: a curated id that does not resolve fails the compile).
- `law_check`: `every_narrative_leg_is_an_event_and_in_order` (orders are `0..n` per narrative, no duplicate event within a narrative). The container forest law is not widened: it governs `ContainerNodeId` containment only.
- `section_schema_version` moves by one; AGC minor; AQC scenario for a narrative's `contains` page.

### Task 4: the card reads the new details (client)

`client/Exploring/Presenter.cs`, `GraphPresenter`:
```csharp
private const string DescriptionField = "Description";
private const string StrongField = "Strong's";
private const string LanguageField = "Language";
private const string TransliterationField = "Transliteration";
private const string PartOfSpeechField = "Part of speech";
private const string GlossesField = "Glosses";
private const string DatedField = "Dated";
```
- `CardOf` gains, in this order after the existing geography fields: `Description` (from `NodeRecord.Description`; skipped if FOCUS-4/7 already added it, the Task 0 check), `Dated` (`Anchor.At.Label`), then `Strong's`, `Language`, `Transliteration`, `Part of speech`, `Glosses` (served glosses joined by `"; "`, the one composition a list of served strings needs; no other formatting). `Provenance` stays last.
- If `Presentation.Field` has gained a target member in FOCUS-4 (`Year? At`), `Dated` sets it from the served `Year`, so an anchor's date opens the World like a person's years (FOCUS-4 OPEN 5b, by analogy).
- No new `Presentation` arm, no per-kind presenter.

### Task 5: the popover loses its legacy fallback (client)

`client/Legacy/IExplorable.cs`:
```csharp
public interface IExplorable
{
    string Title { get; }
    string Kind { get; }
    NodeRef Identity { get; }
    Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api);
}
```
- `BodyAsync` is deleted from the interface and every implementation left (`PolityDeltaNode`'s returns an empty fragment today).
- `ExplorerPopover.LoadLegacy` keeps only the registry arm; `_body`, the `else { @_body }` branch and the `request.Fetch(() => legacy.ExploreAsync(Atlas), () => legacy.BodyAsync(Atlas))` call go. A legacy opening whose node no provider applies to is a `ContractBreach` (`UnreachableException` is wrong here: it is reachable by a new legacy class), named in the message by the node's `Kind`.
- `LegacyPresentations.Retain` stops calling `LegacyNodes.For`: a trail entry keeps the view it arrived with, or none.
- `client.Tests/Explore/DeletionLawTests.cs` becomes one law over the whole enum:
```csharp
[Fact] public void Every_node_kind_opens_on_the_new_path();
[Fact] public void The_only_legacy_view_is_the_one_held_for_the_maps_migration();
```
  The first walks `Enum.GetValues<NodeKind>()` and asserts `Presentation.Of(new ElementKind.Node(kind), Surface.Popover)` is `Form.Card` for every kind, and that no type in the app assembly implements `IExplorable` for a kind's name (the old `MigratedKinds` list is gone; there is nothing left to list). The second asserts the set of concrete `IExplorable` types is exactly `{ PolityDeltaNode }`.

### Task 6: the coverage law (Playwright)

`tests/ux/node-kinds.spec.ts`:
```ts
const OPENED_ELSEWHERE: Record<string, string>;
test('every node kind is opened by some spec', …);
```
- `OPENED_ELSEWHERE` maps each kind another spec opens to that spec's file (`TextUnit`, `Container` → FOCUS-2/3's reader specs; `Person` → `person-card.spec.ts`; `Event` → FOCUS-5's; `Place`, `Polity`, `Map` → `world-geography.spec.ts`; `Era` → `world-geography.spec.ts` only if it opens an era's popover at the base, else this spec; `CatechismItem`, `CommentaryItem` → FOCUS-7's), the six kinds of this spec are its own tests' titles. The law reads `NodeKind`'s enum from `contracts/openapi.yaml` and fails on any kind in neither list, and on any listed file that does not exist (the `spec-hygiene.spec.ts` precedent: a test named in a script must exist).

## Deletion inventory (FOCUS-8 total)

- **Client files:** `client/Legacy/AuthorNode.cs`, `client/Legacy/YearNode.cs` (if present at the base); tests `client.Tests/AuthorNodeTests.cs`, `client.Tests/YearNodeEventTimeTests.cs`.
- **Client members:** `IExplorable.BodyAsync` and every implementation; `ExplorerPopover`'s `_body`, the fallback branch of `LoadLegacy`, and its `BodyAsync` fetch; `YearFrontierSection` and its `PopoverSectionRegistry` row (if present); `LegacyNodes.For` (both overloads) and its private arms; `LegacyPresentations`' use of it; `PopoverChromeRegistry.ByKind["Author"]` and `["Year"]`; the `LegacyNodesTests`, `LegacyViews`, `IdentityTests`, `PushViaConformanceTests`, `ChipTests`, `PopoverChromeConformanceTests` rows for Author and Year; `DeletionLawTests.MigratedKinds` and its three tests (replaced, above); `AtlasClient.NodeRecord` (its readers at `b3d7cfa` are `PersonNode`, `EventNode` and `AuthorNode`, all gone once Task 5 lands; the route `/api/node/{id}` stays, `GraphExplorableClient` reads it).
- **Server:** the fifteen hand-written arms of `decode_node_id`; `encode_node_id`'s `Debug` formatting; the hand-listed round-trip test `event_narrative_anchor_place_ids_round_trip_through_the_wire_form` (replaced by the `NodeKind::ALL` law).
- **Not deleted** (stated so no one "finishes" it): `IExplorable` itself, `ExploreAsync`, `Chip`/`ChipTarget`, `PopoverSectionRegistry`, `IPopoverSectionProvider`, `PolityDeltaNode` and its three sections, `PopoverOpening.Legacy`, `LegacyPresentations`, `LegacySaves`, `LegacyNodes.BookContainerId`/`ChapterContainerId`, `client/Contracts/Frontier.cs` (all FOCUS-9); `/api/node/{id}` and `GraphExplorableClient`'s read of it (F-34).

---

### Task 0: Ledger, base, and the facts the plan assumes (no code)

**Backend change: no.**

- [ ] **Step 1:** Write the ledger: the base commit; which of FOCUS-2…7 landed at which commits; this plan's answers to OPEN 1–5.
- [ ] **Step 2:** Check every row of "Assumptions about FOCUS-2…7" with a grep at the base (`grep -rn "class VerseNode\|class EventNode\|class CatechismNode\|class CommentaryItemNode\|class PersonNode\|class ChapterNode" client` → nothing; `grep -rn "new AuthorNode\|new YearNode" client` → nothing; `grep -rn "PopoverOpening.Legacy(" client` → only `client/Pages/World.razor`'s `PolityDeltaNode`). Record each result. A "stop" row stops the batch.
- [ ] **Step 3:** Probe the base's server (`atlas-server --port 8100`, Codex's port; 8000 for Claude): for each of the six kinds, one real id and its `edge_summary`, and whether each "way in" in the grounding table exists in the artifact (an event whose `dated-by` neighbour is an Anchor; a person with a non-empty `namesake-of` page; a verse with a `words` page; a Map whose `shows` includes a Narrative; any `justified-by` neighbour that is a Source). Record which kinds fall back to the saved-exploration door.
- [ ] **Step 4:** Check whether any spec at the base opens an Era's popover (`grep -rn "Era:" tests/ux`); record it for Task 6.
- [ ] **Step 5:** Check whether `GraphPresenter.CardOf` already reads `NodeRecord.Description` (FOCUS-4 or FOCUS-7) and whether `Presentation.Field` has a year target (FOCUS-4's `At`). Record both; Task 4 follows them.

### Task 1: Every node kind round-trips through the wire id (backend)

**Backend change: yes (graph-types + `atlas-contract`'s id codec; no route, no wire field). Why there (rule 27):** decoding an id is the generic element read's own job (27a: "elements by id"); the client cannot open a position the server cannot read. The defect is a hand-written table beside an enum: a kind added to `NodeKind` compiled, encoded, and silently failed to decode. Category: *a closed vocabulary restated by hand on the wire*. Closure (24b): one table on the enum, the inverse derived from it, a law over `NodeKind::ALL`.

**Owner gate first:** OPEN 1.

**Files:**
- Modify: `graph-types/src/node.rs` (or the file `kind_tags!` expands in), `server/atlas-graph/src/node_ref.rs`, `server/atlas-contract/src/graph_wire.rs`, `server/atlas-contract/src/reference.rs` (only if it restates a prefix).
- Test: `graph-types/src/tests.rs` (or the crate's id tests), `server/atlas-contract/src/graph_wire.rs` tests, `server/atlas-contract/tests/graph_api.rs`.

- [ ] **Step 1: Failing tests.**
  - graph-types: `every_node_kind_has_one_wire_prefix_and_reads_back_from_it` (walks `NodeKind::ALL`; asserts the whole `Vec<(NodeKind, &str, Option<NodeKind>)>`); `no_two_node_kinds_share_a_wire_prefix`.
  - `graph_wire`: `every_node_kind_round_trips_through_the_wire_form` (walks `NodeKind::ALL` with one sample raw id per kind from a `const` table whose keys are checked to equal `NodeKind::ALL`; text units use the existing verse and Concord samples).
  - `graph_api.rs` (real artifact): `a_people_group_and_a_source_open_by_their_served_ids` — takes ids from a served neighbour page (a person's `namesake-of`, a `justified-by` page; or, if the artifact has no source reached by an edge, from `NodeKind::Source` nodes listed by the store) and asserts `/api/elements` answers a `NodeElement` for each, whole.
  - `graph_api.rs`: `every_kinds_summary_counts_what_its_pages_return` — one node per `NodeKind` present in the artifact; for each of its summary entries, the sum of entries over all pages equals the count. Red today on `Person:moab_2103`'s `namesake-of` if the :8000 probe reproduces on the base; if it does not, the test is still the law.
- [ ] **Step 2:** `cargo test -p atlas-contract graph_wire && cargo test -p atlas-contract --test graph_api && (cd graph-types && cargo test --all-features)` → red.
- [ ] **Step 3: Implement** the Types. If the summary/page law stays red after the codec fix, name the second cause in the ledger (the neighbour page's description of a far end) and fix it in the same category (a far end the page cannot describe), not around it.
- [ ] **Step 4: Regenerate (critical section `contract`):** `cargo run -p atlas-contract --bin export_contract -- --check` (expect no diff: ids are unchanged); AQC: add one scenario that opens a people group by id; `export_aqc_examples`; re-bless pacts if the scenario adds an interaction; `CHANGELOG.md` with the class the semver gate assigns.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace && (cd graph-types && cargo test --all-features) && bash scripts/contract-gate.sh --base <base>` → green.
- [ ] **Step 6: Commit:** `graph: every node kind round-trips through its wire id; the codec is one table on NodeKind (people groups and sources open)`.

### Task 2: A lexicon entry and an anchor serve their facts (backend)

**Backend change: yes (two optional details on `NodeRecord`, read from the node's own row). Why there (rule 27):** the facts are in the artifact (the node's payload); reading one row per requested node is a bounded indexed read (27b). The client cannot derive them. No new endpoint (27a).

**Owner gate first:** OPEN 2. Under "no", this task is skipped and Task 4 adds only `Description`.

**Files:**
- Modify: `server/atlas-contract/src/wire/graph.rs`, `server/atlas-contract/src/graph.rs` (`read_node_record`), `contracts/openapi.yaml`, `contracts/atlas-query-contract/{aqc.schema.json,features/*,CHANGELOG.md}`, pacts, the generated `Wire.g.cs` (built by `client.ContractGenerator` into untracked output, rule 13).
- Test: `server/atlas-contract/tests/graph_api.rs`, `client.ContractTests` (`GeneratedUsageTests` goes red until Task 4 reads the members).

- [ ] **Step 1: Failing tests** (`graph_api.rs`, real artifact, whole records):
  - `a_lexicon_entrys_record_carries_its_strongs_number_transliteration_and_glosses` (expected built from the lexicon section's own row for one entry read by the test from the store, never a literal);
  - `an_anchors_record_carries_its_date`;
  - `no_other_kind_carries_a_lexicon_or_anchor_detail` (one node per other kind present).
- [ ] **Step 2:** red. **Step 3:** implement. **Step 4: Regenerate (critical section `contract`, after Task 1's):** export, `--check`, AQC (additive: minor unless the gate says otherwise), re-bless, `dotnet run --project client.ContractGenerator`.
- [ ] **Step 5 (lock `heavy`):** workspace + graph-types + contract gate green; `dotnet test client.ContractTests` red only on `GeneratedUsageTests` for `LexiconDetail`/`AnchorDetail` members (recorded; Task 4 turns it green).
- [ ] **Step 6: Commit:** `graph: a lexicon entry's record serves its Strong's number, transliteration and glosses; an anchor's serves its date`.

### Task 3: A narrative contains its legs (backend: ETL and compiler)

**Backend change: yes (a row family in the Core section, compiled from curated data; no relation appended, no served code). Why there (rule 27):** which events a narrative strings together, and in what order, is a fact of the curated data alone; the compiler writes it once. The graph models the domain: a narrative containing its legs is that, not a view (today the legs ride only as an attribute on `follows-in` entries, so a narrative's own frontier cannot reach them).

**Owner gate first:** OPEN 3. Under (b), skip this task and add the FINDING "a narrative does not reach its events" to the queue.

**Files:**
- Modify: `graph-types/src/{edge,graph,canon/rows}.rs` (the `NarrativeLeg` family, its `RowFamily` arm and canonical row form), `graph-types/src/sections.rs` (Core), `server/atlas-etl/src/{curated,compile}.rs`, `server/atlas-graph/src/{law_check,build}.rs`, `server/atlas-graph/src/sqlite/{partition,writer,snapshot,columns}.rs` as the family requires, AGC feature lines and `VERSION`, `data/compiled` (rebuilt).
- Test: `server/atlas-etl/tests/etl.rs`, `server/atlas-graph/tests/` (a `narrative_legs_real_data.rs`), `server/atlas-contract/tests/graph_api.rs`.

- [ ] **Step 1: Failing tests:**
  - ETL: `a_narratives_legs_become_ordered_contains_rows` (a two-narrative fixture TOML → the whole row list); `a_leg_that_names_no_event_fails_the_compile` (whole error);
  - law: `every_narrative_leg_is_an_event_and_in_order`;
  - real data: `every_curated_narrative_contains_exactly_its_legs_in_order` (expected read from `data/curated/narratives/*.toml` by the test);
  - API: `a_narratives_contains_page_lists_its_legs_in_order` and `an_events_member_of_page_names_its_narratives`.
- [ ] **Step 2:** red. **Step 3:** implement; `section_schema_version` +1.
- [ ] **Step 4: Rebuild (critical section `contract`, after Task 2's regen):** rebuild `data/compiled`; AGC minor; AQC scenario for the narrative's `contains` page; export, `--check`, re-bless.
- [ ] **Step 5 (lock `heavy`):** `cargo test --workspace && (cd graph-types && cargo test --all-features) && bash scripts/contract-gate.sh --base <base> && bash scripts/timing-gates.sh run` → green.
- [ ] **Step 6: Commit:** `graph: a narrative contains its legs in order, compiled from the curated narratives`.

### Task 4: The card shows what the record holds (client)

**Backend change: no.** Presentation over served data is the client's (rule 27).

**Files:**
- Modify: `client/Exploring/Presenter.cs`.
- Test: `client.Tests/Explore/GraphPresenterTests.cs`, `client.Tests/Explore/ServedGraph.cs` (fixtures for the six kinds).

- [ ] **Step 1: Failing tests** (`GraphPresenterTests`, whole `Card` each, fixtures in `ServedGraph` shaped like the Task 0 probes):
  - `A_lexicon_entry_on_the_popover_presents_its_strongs_number_language_transliteration_part_of_speech_and_glosses`;
  - `An_anchor_on_the_popover_presents_its_date_and_its_citation`;
  - `A_people_group_on_the_popover_presents_its_description`;
  - `A_narrative_on_the_popover_presents_its_title_and_provenance`;
  - `A_source_on_the_popover_presents_its_title_and_provenance`;
  - `A_translation_on_the_popover_presents_its_title_and_provenance`;
  - `Every_node_kind_presents_a_card_on_the_popover` (walks `Enum.GetValues<NodeKind>()` over one fixture per kind; asserts each `Present` result is a `Presentation.Card`, not null).
- [ ] **Step 2:** `dotnet test client.Tests --filter GraphPresenterTests` → red. **Step 3:** implement. **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green (`GeneratedUsageTests` now reads every Task-2 member).
- [ ] **Step 5: Commit:** `focus: a lexicon entry, an anchor and a people group show their served facts on the card`.

### Task 5: The popover's legacy fallback is gone (client)

**Backend change: no.**

**Files:**
- Modify: `client/Legacy/{IExplorable,LegacyNodes,LegacyPresentations,PolityDeltaNode,PopoverSections,PopoverSectionProviders}.cs`, `client/Components/ExplorerPopover.razor`, `client/AtlasClient.cs`, `client/Exploring/PopoverChromeRegistry.cs`, `client.Tests/Explore/{DeletionLawTests,LegacyNodesTests,LegacyViews,IdentityTests,PushViaConformanceTests,ChipTests,LegacyPresentationsTests}.cs`, `client.Tests/{PopoverChromeConformanceTests,PopoverSectionRegistryTests,FrontierMatrixConformanceTests}.cs`, `client.Tests/Components/ExplorerPopoverTests.cs`, `client.Tests/stryker-config.json` (drop `**/Legacy/LegacyNodes.cs` only if the file is gone; it is not: `BookContainerId` stays).
- Delete: the inventory's client files and members.

- [ ] **Step 1: Failing tests:**
  - `DeletionLawTests.Every_node_kind_opens_on_the_new_path` and `The_only_legacy_view_is_the_one_held_for_the_maps_migration` (red while `AuthorNode`/`YearNode` exist);
  - `ExplorerPopoverTests.A_legacy_opening_no_section_applies_to_is_a_contract_breach` (a fake `IExplorable` with an unknown `Kind`; the whole failure message);
  - `LegacyPresentationsTests.A_trail_entry_keeps_only_the_view_it_arrived_with` (replaces the tests that relied on `LegacyNodes.For` rebuilding a view).
- [ ] **Step 2:** red. **Step 3:** delete per the inventory; then make the new laws pass. `PushViaConformanceTests`, `IdentityTests`, `LegacyViews` lose their Author/Year rows. If `grep -rn "PopoverOpening.Legacy(" client` shows any site but `World.razor`'s `PolityDeltaNode`, it is a same-category site (24): migrate it to `PopoverOpening.Explore(new NodePosition(<served ref>))` in this task and name it in the close report and FINDINGS (the batch that owned it missed it).
- [ ] **Step 4:** `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` → green; `npx playwright test tests/ux/world-border-morph.spec.ts tests/ux/saved-explorations.spec.ts tests/ux/state-focus.spec.ts` → record the red set by name (expected: the saved-explorations and state-focus tests that push an `AuthorNode` through "About this book", if FOCUS-3 did not already rewrite them; they are FOCUS-9's to re-express, and this task rewrites only the step that clicked the deleted chip to open the book's Container through its served link).
- [ ] **Step 5: Commit:** `focus: the popover has no legacy body path; AuthorNode, YearNode and LegacyNodes.For are gone (every node kind opens on FocusView)`.

### Task 6: Each kind is proven in the app (Playwright)

**Backend change: no.**

**Files:** `tests/ux/node-kinds.spec.ts` (new), `tests/ux/lib/api.ts` (only if a needed read is missing), `tests/ux/CONTRACT.md` (a "node kinds" section listing the ids each test uses).

- [ ] **Step 1: Write the specs**, one `test` per kind, each reading every expected value from the API (`api.node`, `api.nodeEdges`, `api.elements`), asserting `popover-title` / `popover-card-title`, every `popover-field-*` the record serves, one `popover-section-{kind}` heading per served group (the heading text from the served count), then following one `popover-link-*` and pressing `popover-breadcrumb-back`:
  - "a narrative opens from the map that shows it, lists its events in order, and an event opens from it" (OPEN 3a; under 3b: "… shows the maps it is drawn on");
  - "an anchor opens from an event's dated-by link, shows its date and citation, and the event it dates opens from it";
  - "a people group opens from a person's namesake-of link (or a verse's mention), shows its description, and a verse that mentions it opens from it";
  - "a lexicon entry opens from a verse's words link, shows its Strong's number and glosses, and a verse it occurs in opens from it";
  - "a source opens from a justified-by link and shows its title" (the saved-exploration door if Task 0 found no edge to a source);
  - "a translation opens from a saved exploration and shows its title and provenance" (OPEN 4; asserts no section, because its frontier is empty);
  - only if Task 0 found no spec that opens an Era: "an era opens from a saved exploration and shows its window" (its frontier is empty; see FINDINGS);
  - "every node kind is opened by some spec" (the coverage law).
  The saved-exploration door is the one `explore-edges.spec.ts` uses: seed `explorations-v3` with `{ start: { position: 'node', node }, steps: [] }`, open the hamburger, continue it.
- [ ] **Step 2:** `npx playwright test tests/ux/node-kinds.spec.ts` → green.
- [ ] **Step 3: Commit:** `tests: every node kind is opened, presented and explored in the app; a law fails on a kind no spec opens`.

### Task 7: Gates, mutation, close

- [ ] **Step 1: Gates.** `client.Tests/stryker-config.exploring.json` covers `**/Presenter.cs` (verify). Mutation inside the owner's window only (critical section `heavy` with "mutation" in the message, `free -g` ≥ 18 GB): `bash scripts/mutants-parallel.sh -n 3 -b <base>` (covers `wire_prefix`/`of_wire_prefix`, `decode_node_id`, the two details, the narrative-leg compile and law) → 100% or equivalents recorded; then `dotnet stryker` → 100% or equivalents recorded. Outside the window, `.superpowers/MUTATION-GATE-DEBT.md` names `<base>`. Then (lock `heavy`): `cargo test --workspace && (cd graph-types && cargo test --all-features) && dotnet test client.Tests && dotnet test client.ContractTests && bash scripts/contract-gate.sh --base <base> && bash scripts/timing-gates.sh run && npx playwright test tests/ux` → green except the carried reds named in the queue at the base. The diff adds no comment line (`git diff <base>.. -- server graph-types client | grep -E '^\+.*(//|/\*|<!--)'` prints nothing).
- [ ] **Step 2:** Close report `docs/superpowers/reports/2026-10-0x-focus-8-close.md`: the rule-24 table below, the FINDINGS, the behaviour changes (narrative legs on the card), and the spec amendment for §5 (FOCUS-8 row: "the six kinds no batch owned; Era, Polity and Map went with FOCUS-6"; the fallback path named).
- [ ] **Step 3:** Push `lane/<agent>/F8-int`; set the queue item to `review` with the range. The other agent reviews (14b + 24a); Claude lands under `land`.

**Rule-24 categories and their closures:**

| Category | Closure | Guarantee |
|---|---|---|
| A closed vocabulary restated by hand on the wire (the id codec) | `NodeKind::wire_prefix` / `of_wire_prefix`, the fifteen arms deleted | a law over `NodeKind::ALL`; a new kind cannot compile without a prefix |
| A summary that promises neighbours its pages do not return | the cause named in Task 1, fixed at its one site | `every_kinds_summary_counts_what_its_pages_return` on the real artifact |
| A kind no test reaches through the app | `node-kinds.spec.ts` | the coverage law over the contract's `NodeKind` enum |
| A legacy view served beside the new path | the fallback path and `LegacyNodes.For` deleted | `DeletionLawTests` over the whole enum; the legacy implementations are one named set |

---

## Wave schedule

Primary is the critical path; the companion is unlike work beside it (rule 23: Rust beside C#). A task starts when the tasks it names as inputs have landed on `lane/<agent>/F8-int`.

| Wave | Primary | Companion | Critical sections | Expected red at wave close |
|---|---|---|---|---|
| 0 | owner: OPEN 1–5 (defaults stand if unanswered); Task 0 | — | — | — |
| 1 | Task 1 (Rust) | Task 5 (C#; disjoint files) | `contract` regen #1; `heavy` | Playwright: the AuthorNode steps of `saved-explorations`/`state-focus` if still present (named in the ledger) |
| 2 | Task 2 (Rust) | Task 4 tests written against `ServedGraph` (C#) | `contract` regen #2, re-bless; `heavy` | `GeneratedUsageTests` (the two details) |
| 3 | Task 3 (Rust; rebuild) | Task 4 implemented | `contract` rebuild + regen #3, re-bless; `heavy` | — |
| 4 | Task 6 (TypeScript) | — | — | — |
| 5 | Task 7 | — | `heavy` (mutation only in the owner's window); `land` | only the carried reds |

**Critical path:** OPEN answers → Task 1 → Task 2 → Task 3 → Task 6 → Task 7. Task 5 rides beside Task 1; Task 4 beside Tasks 2–3 and lands before Task 6.

## FINDINGS this plan expects to raise (for the queue; the owner decides)

- **An era does not link its own map** (`Era:united-kingdom` has an empty `edge_summary`; `Map:era-united-kingdom` is its picture). Closure: a compiled edge from each Map to the Era it draws (an existing relation), or the Era retired in favour of its Map. FOCUS-6's kind, not fixed here.
- **A translation is linked to nothing** (OPEN 4). Closure: decide what a translation is reached from (a verse's parallel renderings, the sources page) and compile those edges.
- **`client/Contracts/Frontier.cs` is read only by its own tests** (`FocusKind`, `FocusKinds.Parse`, nine `IHas*` markers, `FrontierMatrix`; its Rust authority `graph-types/src/frontier.rs` died in FOCUS-6). Dead under rule 4; FOCUS-9 deletes it with `FrontierMatrixConformanceTests` and `FrontierMatrixRustParityTests`.
- **`client.Tests/stryker-config.json` names a file that does not exist** (`**/Exploring/RenderedLegacyNodes.cs`). Closure: a law that every `mutate` glob matches at least one file. FOCUS-9 removes the entry.
- **A comment in application code survives in `client/Legacy/PopoverSections.cs`** (the registry's ordering note and `IPopoverSectionContext`'s render-time notes; F-12/A-STRIP). FOCUS-9 deletes the file.
- **F-50 reaches the card:** an anchor's served citation carries curation notes (OPEN 5).
- **The rule-4 pull-forward was missed twice:** FOCUS-6's plan said `YearNode` died with `PlaceCard`, but `EventDateAndPlacesSection` also constructs it; FOCUS-3's plan keeps `AuthorNode` with no constructor. Closure: a law that every concrete `IExplorable` has a constructor call in the app (reflection over types, a source scan for `new <Type>(`), until FOCUS-9 removes the interface.

## Self-review against rule 27, the spec and the brief

- **Rule 27:** narrative legs are compiled (Task 3); the two details are one-row reads (Task 2); the id codec is the element read's own (Task 1); the client adds fields from served values only (Task 4). No endpoint is added; no relation is appended; nothing in `server/` names a client construct (`LexiconDetail`, `AnchorDetail`, `NarrativeLeg`, `wire_prefix` pass the vocabulary gate).
- **Spec §5 FOCUS-8:** each kind verified by a Playwright spec (Task 6); the legacy fallback path deleted (Task 5). §10 R3: the generic `Card` plus frontier, nothing bespoke.
- **The end state the brief asks for:** after FOCUS-8 the popover's legacy machinery is `IExplorable` with one implementation (`PolityDeltaNode`), its chips, the registry with three providers, `PopoverOpening.Legacy`, `LegacyPresentations`, the save readers and `Frontier.cs`: exactly FOCUS-9's inventory.
- **Assumptions:** see the FOCUS-2…7 table (checked by Task 0); `NodeKind::ALL` exists at the base (it does at `b3d7cfa`, `graph-types/src/id.rs:236`); the summary/page mismatch seen on :8000 may not reproduce on the base (the law stands either way); the saved-exploration store key is `explorations-v3` (as at `b3d7cfa`).
