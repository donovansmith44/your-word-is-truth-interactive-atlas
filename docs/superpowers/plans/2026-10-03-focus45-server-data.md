# FOCUS-4 (Person) and FOCUS-5 (Event): server, data and contract plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Executor:** Claude or Codex, as the controller names (queue items `A-F45` / `CX-F45`). Server, data and contract only. **The C# client is frozen** (owner, 2026-10-03: "there's no point in writing a bunch of code that's going to be rewritten"): every client task of the earlier FOCUS-4 (`lane/claude/F4-plan` `da92793`) and FOCUS-5 (`lane/claude/F5-plan` `564f4ca`) plans is moved to **the F# client backlog** at the end. The executor touches only the files a task lists; anything else goes to FINDINGS in the queue (rule 24a).

**Goal:** Put into the graph, the artifact and the contract everything a person's and an event's presentation needs, as the owner ruled on 2026-10-03, so the F# client composes it with no domain logic:
- a person: eternity applied at compile, God's grounds of eternity as links, every curated ground as a `justified-by` edge, parentage wording as data shown only on the link, a curated incarnate record for Jesus's earthly life;
- an event: chronology with direction (`precedes`/`follows`), separate from stories; each story's previous and next on the event; a concurrent set ("At the same time"); accounts as passages marked as recording history, built from contiguous attestation verses, listed by reference and opening words; the event's map view's data;
- F-36: every year and span label compiled.

**Supersedes:** the server/data/contract halves of the two plans above. Their client halves, specs re-expressions and C# deletions are the F# client backlog. Their OPEN lists are answered (below).

**Spec:** `docs/superpowers/specs/2026-10-03-year-design.md` (this branch): §1 domain types, §2 data structures, §3 algebras. This plan uses its `YearSpan`, chronology, story, account and parentage parts; the Year itself builds after this plan (owner: "Year: spec it now, build after FOCUS-5").

**Principles:** 4, 9, 12, 14b, 15–18, 21–23, 24–24b, 25, 26, 26a, 27–27g.

**Sequencing:** after **FOCUS-3** (it builds passages: `passage_container_id`, the container text read, `references.rs` from FOCUS-2). Then this plan. Then the Year (TIME). Task 0 records the base (PRINCIPLES 22).

## Rulings applied (OWNER ANSWERS, 2026-10-03)

| Ruling | Where |
|---|---|
| F4 Q1: God's eternity grounds are clickable links | Task 1: `justified-by` edges from the person |
| F4 Q2: parentage wording only on the link, not inline | Task 2: the wording is the edge's compiled label; no entry-text field is built |
| F4 Q3: God→Jesus "Eternal Father of", God→Adam "Creator of", Mary "Mother of", Joseph "Father of" | Task 2's data |
| F4 Q4: keep Jesus's "Earthly life" from a curated incarnate record | Task 1: `[[incarnate]]`, `PersonLife.incarnate` |
| F4 Q5: years are explorable and lead to Year | Year spec; Task 3 compiles the labels; backlog item |
| F4 Q6 / F5 Q6: Mentioned in / Attested in collapsed, last | client only: backlog |
| F4 Q7: F-36 is its own item | Task 3, its own section |
| F3 Q5 / F5 Q6 / F5 Q7: an account is a passage marked as recording history; the event groups its accounts; contiguous attestation verses become passages; accounts listed as reference + opening words | Task 6 |
| F5 Q2: store direction (follows/precedes); temporal adjacency retires | Task 4; retirement in Task R |
| F5 Q3: card arrows = chronology only; a "Stories" section lists each story's previous/next; a concurrent section ("At the same time") | Tasks 4, 5, 7 |
| F5 Q4: the event's map view frames its years and highlights its stories | Task 8 (the map's data) |
| F5 Q5: the card shows When, superscription, Source; no bookkeeping | client: backlog; wire unchanged until Task R |
| F5 Q8: labels stay as codes (`GEN.1.1`, `GEN.29.32-30.24`) | Task 6: `runs_reference` |
| C# client frozen; client halves become the F# backlog | whole plan; the freeze rule below |

## The freeze rule (how the server changes while the C# client is frozen)

The frozen C# client is still the owner's demo build. It decodes the contract with generated types and refuses an unknown edge kind or a missing required field (F-81 aside). So:

1. **Additive now.** Every change in Tasks 1–8 adds: a relation, a row family, an optional field, a route. Nothing the C# client reads is removed or reshaped.
2. **New vocabulary reaches the C# build mechanically** (OPEN 1 below): each new edge kind regenerates the C# contract types and adds one `DefaultList` arm to `Affordances.Of`, nothing else, so the frozen client keeps decoding every node. This is the only C# change in this plan.
3. **Removals wait for the C# retirement (Task R).** `PersonLife.eternal_grounds`, `temporal-adjacency`, the event-to-event narrative `follows-in` edges, the `loci`/`note` on attestation entries, `/api/event`, `/api/narrative/event`, the per-request `EventAccounts` and `temporal_neighbors_of`, and `EventDetail`'s bookkeeping fields all keep serving until the F# client has replaced the C# person and event views. Task R removes them in one AQC major. Until then each is listed in `.superpowers/sdd/F45/retirement.md` with its reader, so none is forgotten (a shrinking list, O-WIRE-IDENTITIES O3's precedent).

## OPEN: for the owner

1. **The frozen client and new link kinds.** Each new kind of link (chronology, stories, accounts) makes the old C# app fail to open an event until its generated types are refreshed. (a) Refresh them and add one line per new kind so the demo build keeps working, and nothing more; (b) leave the C# app alone and accept that people and events stop opening in it until the F# client replaces them. **Recommend (a).**

## Global constraints

- `docs/PRINCIPLES.md` binds: rule 4 (no member without a reader: every new wire field names its F# backlog reader), 9 (no comments), 12 (every type below is for sign-off), 24–24b (each category closed; offenders to FINDINGS), 26/26a (facts in `data/`), **27** (data-only derivations compiled; the server reads; the graph models the domain; no exploration word in `server/` or `graph-types/`).
- Tests: whole-body assertions, one behaviour per test named as a sentence, `// Arrange` `// Act` `// Assert` only, no magic numbers, newspaper order; real-data expectations read from the artifact (F-8); the algebra laws below are proptest properties.
- Every closed vocabulary is matched without a wildcard arm (`Parentage`, `RowFamily`, `Facet`).
- Worktree per task (`git -c core.autocrlf=false worktree add -b lane/<agent>/F45-t<n> ~/w/F45-t<n> <base>`), `CARGO_TARGET_DIR=~/mut/<agent>-F45-t<n>`, `nice -n 10 cargo -j 4`, data copied never linked; `find <wt> -type l | wc -l` prints 0 before removal.

## Critical sections (PRINCIPLES 21)

| Section | Held by |
|---|---|
| `contract`: rebuilding `data/compiled` | Tasks 1, 2, 3, 4, 5, 6 (one rebuild each, in order; tasks may pair rebuilds when they land together) |
| `contract`: `export_contract`, `export_aqc_examples`, `client.ContractGenerator`, re-blessing | Tasks 1, 3, 4, 5, 6, 7, 8 |
| appending to `relations!` | Task 4 (`Precedes`), Task 5 (`InStory`), Task 6 (`Narrates`), in that order, each last |
| `heavy` | every task's closing gate; mutation only in Task Z, in the owner's window |

---

## Part 1. Domain types (for sign-off)

The shared types (`Year`, `YearSpan`, `Precision`, `Dating`, `ChronologicalSuccession`, `StoryStep`, the account passage) are the Year spec's §1. This plan adds or changes:

### Persons (Tasks 1, 2)

`graph-types/src/node.rs`, `NodePayload::Person`:
```rust
Person {
    label: String,
    gender: Option<String>,
    birth_year: Option<i32>,
    death_year: Option<i32>,
    also_called: Vec<String>,
    description: Option<String>,
    first_year: Option<i32>,
    last_year: Option<i32>,
    eternal: bool,
    eternal_grounds: BTreeSet<Ground>,
    incarnate: bool,
}
```

`server/atlas-etl/src/curated.rs`:
```rust
pub struct CuratedEternity { pub eternal: Vec<(PersonId, Justification)>, pub incarnate: Vec<PersonId> }
pub struct CuratedParentage { pub declared: Vec<ParentageSeed>, pub excluded: Vec<ParentageExclusion>, pub wording: ParentageWording }
pub struct ParentageWording(BTreeMap<Parentage, String>);

impl ParentageWording {
    pub fn of(&self, p: Parentage) -> &str;
}
```
- `ParentageWording` is built only by the parser, which refuses a table without exactly one entry per `Parentage::ALL`; so `of` is total and has no failure path.

`server/atlas-graph/src/event_world.rs`:
```rust
pub fn add_justified_by(graph: &mut Graph) -> usize;
fn grounds_of(family: RowFamily, graph: &Graph, row: usize) -> Option<&BTreeSet<Ground>>;
```

`server/atlas-graph/src/labels.rs`:
```rust
fn relation_label(record: &EdgeRecord, names: &ReaderNames) -> String;
```

`server/atlas-contract/src/wire/graph.rs`: `PersonLife` gains `pub incarnate: bool` (additive; `eternal_grounds` stays until Task R).

### Events (Tasks 4–8)

`graph-types/src/edge.rs`:
```rust
relations! {
    directed { …, Precedes => "precedes" / "follows", InStory => "in-story" / "story-steps", Narrates => "narrates" / "narrated-in" }
    symmetric { … }
}

pub struct ChronologicalSuccession { pub earlier: EventId, pub later: EventId, pub provenance: ProvenanceId }
pub struct StoryStep { pub story: NarrativeId, pub ord: u16, pub event: EventId, pub provenance: ProvenanceId }
pub struct Narration { pub account: ContainerNodeId, pub event: EventId, pub provenance: ProvenanceId, pub justification: Justification }

pub enum EdgeMeta { …, Step { ord: u16, previous: Option<EventId>, next: Option<EventId> } }

pub fn passage_container_id(runs: &[BibleLocusRange]) -> ContainerNodeId;
```

`server/atlas-graph/src/references.rs`:
```rust
pub fn runs_reference(runs: &[BibleLocusRange]) -> String;
pub fn opening_words(text: &str) -> String;
pub const OPENING_WORDS: usize = 8;
```

`server/atlas-contract/src/wire/graph.rs`:
```rust
pub struct StoryPlace { pub ord: u16, pub previous: Option<NodeRef>, pub next: Option<NodeRef> }
```
- `EdgeEntry` gains `step: Option<StoryPlace>` (on `in-story` entries only).
- `NodeRef` gains `opening: Option<String>` (on passages only: the compiled opening words), so an account lists as "reference + opening words" from the neighbour page alone (27c).

`server/atlas-core/src/wire.rs`:
```rust
pub struct SceneArrow { …, pub from_node: NodeRef, pub to_node: NodeRef }
```

The window read (Task 7), from the Year spec §5, with only the `Event` facet in this plan:
```rust
pub enum Facet { Event }
pub struct WindowPage { pub window: TimeRange, pub facet: Facet, pub entries: Vec<WindowEntry>, pub next: Option<WindowCursor>, pub previous: Option<WindowCursor>, pub version: String }
pub struct WindowEntry { pub node: NodeRef, pub when: TimeRange }
```
- `GET /api/window?from={y}&to={y}&facet=event&cursor=&limit=`. Until Year nodes exist the ends are the served `Year.value`s of an event's `when` (passed through, never computed); the Year build switches them to Year ids (a declared AQC major in TIME, recorded in the retirement list).

## Part 2. Data structures, invariants and bounds

| Structure | Rows today | Invariants (compile fails otherwise) | Bound |
|---|---|---|---|
| Eternity | 2 eternal, 1 incarnate | an eternal person has no year; an incarnate person is not eternal; every id resolves | data-sized |
| `justified-by` from every row family that records grounds, and from eternal persons | grows by the grounds of `ParentOf`, `Brethren`, `Attests`, … (recorded per family in the ledger) | `grounds_of` exhaustive over `RowFamily` | one edge per ground |
| Parentage wording | 5 (`Natural` "Parent of", `Eternal` "Eternal Father of", `Virgin` "Mother of", `Legal` "Father of", `Created` "Creator of") | exactly one per `Parentage::ALL` | closed enum |
| Chronology `ChronologicalSuccession` → `precedes` | 911 | one chain over the 912 dated events; consistent with dates | dated events − 1 |
| Story steps → `in-story` | 255 in 13 stories | `ord` without gaps; `previous`/`next` are the neighbours in the story | steps |
| Account passages → `narrates` | one per curated witness, plus one per remaining maximal attestation run | accounts partition each event's attested verses; runs maximal and sorted; one node per verse set | ≤ attested verses |
| Event cover index (window, `Event` facet) | 912 `YearCover` + 912 `SpanStart` rows | Year spec I-C1..I-C4 | Σ event span years |
| Compiled year/span labels (Task 3) | every `Year` and `TimeRange` the wire serves | one label rule, with `c. ` for a circa dating | one per served dating |

## Part 3. Algebras and their laws (the tests)

| Algebra | Laws (test names; properties unless marked real-data) |
|---|---|
| Parentage | `parentage_wording_is_total` (walks `Parentage::ALL`; a missing, extra or duplicate entry fails the parse); `a_parent_of_edge_reads_its_wording` (`{parent} · wording(p) · {child}`, the same edge read from either end); `the_wording_shows_only_on_the_edge` (real data: no person's label or entry text carries a wording). |
| Eternity | `an_eternal_person_carries_no_year`; `an_incarnate_person_is_not_eternal`; `gods_justified_by_group_is_his_curated_grounds` (real data). |
| Grounds | `every_family_that_records_grounds_justifies_its_edges_by_them` (walks `RowFamily::ALL`). |
| Span (from the Year spec §3.2) | `no_year_is_zero`, `the_year_after_1_bc_is_ad_1`, `a_span_is_never_inverted`, `a_span_is_the_union_of_its_years`, `contains_is_membership`, `within_is_subset`, `overlap_is_shared_years`, `overlap_is_symmetric`, `hull_is_a_semilattice`, `spans_meet_across_the_zero_gap`. |
| Labels (F-36) | `a_years_label_names_its_era_once`; `a_span_names_a_shared_era_once` (`1450 – 1400 BC`, `5 BC – AD 30`, `AD 33`); `a_circa_dating_reads_c`; `no_served_code_formats_a_year` (source law over `server/` outside the compiler: no `" BC"`/`"AD "` literal). |
| Chronology | `previous_and_next_are_inverses`; `the_chronology_is_one_chain`; `an_undated_event_has_no_chronology`; `the_chronology_respects_dates`. |
| Concurrency | `concurrency_is_overlap`; `concurrency_is_symmetric`; `concurrency_is_irreflexive`; `the_concurrent_set_is_the_window_less_itself`; `two_events_are_before_after_or_at_the_same_time`. |
| Window | `a_window_is_everything_its_span_overlaps`; `covers_and_starts_are_disjoint_and_complete`; `a_range_is_the_union_of_its_years`; `a_window_reads_in_facet_order`; `paging_loses_and_repeats_nothing`; `a_window_page_scans_only_what_it_returns`. |
| Stories | `a_storys_steps_are_numbered_without_gaps`; `a_steps_previous_and_next_are_its_neighbours_in_the_story`; `a_story_lists_its_events_in_its_own_order` (real data); `stories_are_independent_of_chronology` (compile with the narrative files removed or a story permuted: every `precedes` edge unchanged; change a dated-by placement: every `in-story` edge unchanged). |
| Accounts | `an_events_accounts_partition_its_attestation`; `contiguous_verses_are_one_run`; `a_run_may_cross_a_chapter`; `one_verse_set_is_one_passage`; `a_passage_records_history_iff_it_narrates`; `an_opening_is_the_first_words_of_the_first_verse`; `an_accounts_label_is_its_run_code` (real data: `MAT.5.1-7.29`, `MRK.14.54, 66-72`, read against the curated witnesses). |

---

## Tasks

### Task 0: Base and preconditions (no code)

**Files:** `.superpowers/sdd/F45/progress.md`, `.superpowers/sdd/F45/retirement.md` (create).

- [ ] **Step 1:** Record the base: the head FOCUS-3 landed at. Record OPEN 1's answer (or the default).
- [ ] **Step 2:** Verify at the base, recording each: FOCUS-3's `passage_container_id(first, last)` and `GET /api/node/{id}/text` exist; FOCUS-2's `references.rs` exists; `relations!` still lists `TemporalAdjacency`; `add_justified_by` still wires only four families. A difference stops the task for the controller.
- [ ] **Step 3:** Write `retirement.md`: each item of the freeze rule's list with its C# reader (file and line).

### Task 1: Eternity at compile, every ground as an edge, the incarnate record (FOCUS-4)

**Why the compiler (rule 27):** whether an eternal person has years and which verses ground a curated fact depend on the data alone.

**Files:**
- Modify: `data/curated/people-eternal.toml` (add `[[incarnate]] id = "jesus_905"`; the header's "the card labels them 'Earthly life'" stays true); `server/atlas-etl/src/{curated,compile,people}.rs`; `server/atlas-core/src/data.rs` (`Person.eternal_grounds: Justification`, `Person.incarnate`); `graph-types/src/{node,graph}.rs`, `graph-types/src/canon/node.rs`; `server/atlas-graph/src/{person_adapter,event_world,description_adapter,peoples_adapter,red_letter_adapter,law_check,service}.rs`; `server/atlas-contract/src/{wire/graph,graph}.rs` (`PersonLife.incarnate`; `eternal_grounds` still served from the edges' grounds, unchanged on the wire); `contracts/*` regen, CHANGELOG (AQC minor); pacts; `data/compiled`.
- Test: `curated.rs`, `people.rs`, `event_world.rs` unit tests; `graph-types/tests/canon_vectors.rs` (re-pinned under `contract`); `server/atlas-contract/tests/graph_api.rs`.

- [ ] **Step 1:** Re-derive the `RowFamily` members whose rows record a `Justification` into the ledger.
- [ ] **Step 2 (red):** `an_eternity_ground_is_read_as_a_scripture_justification`; `an_incarnate_person_who_is_also_eternal_fails_the_compile`; `an_eternal_person_carries_no_year`; `every_family_that_records_grounds_justifies_its_edges_by_them`; `an_eternal_persons_grounds_justify_the_person`; real data: `gods_record_serves_no_year`, `gods_justified_by_group_is_his_curated_grounds`, `jesus_record_is_incarnate`, `the_parent_of_edge_from_joseph_to_jesus_is_justified_by_its_curated_grounds`.
- [ ] **Step 3:** Implement. `grounds_of` is an exhaustive `match` with no wildcard arm.
- [ ] **Step 4 (`contract`):** rebuild, re-pin `canon_vectors` (only Person vectors may move; any other stops the task), regen, re-bless. A golden map fixture that moves stops the task for the owner.
- [ ] **Step 5 (`heavy`):** `cargo test --workspace`, `(cd graph-types && cargo test --all-features)`, `bash scripts/contract-gate.sh --base <base>`, the frozen-client gate (below) → green. **Commit:** `persons: eternity is applied at compile, every curated ground (eternity and kinship included) is a justified-by edge, and Jesus's earthly life is a curated incarnate record (rule 27; 24b over RowFamily; AQC minor)`.

**The frozen-client gate** (every task): `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests` green, and `npx playwright test tests/ux/person-card.spec.ts tests/ux/event-timeline.spec.ts tests/ux/popover-sections.spec.ts` with the same result as at the base.

### Task 2: Parentage wording is data, total over the parentage kinds, on the edge only (FOCUS-4)

**Why there (rules 26, 27):** the wording is a domain fact (data); an edge's label is a data-only derivation (compiler).

**Files:** `data/curated/parentage.toml` (a `[[wording]]` table, one entry per kind: `natural` "Parent of", `eternal` "Eternal Father of", `virgin` "Mother of", `legal` "Father of", `created` "Creator of"); `server/atlas-etl/src/{curated,compile}.rs`; `server/atlas-core/src/data.rs` (`AtlasData.parentage_wording: ParentageWording`); `server/atlas-graph/src/labels.rs`; `data/compiled`; pacts. Test: `curated.rs`, `labels.rs`, `graph_api.rs`.

- [ ] **Step 1 (red):** `parentage_wording_is_total`; `a_missing_wording_fails_the_compile`; `a_duplicate_wording_fails_the_compile`; `a_parent_of_edge_reads_its_wording`; real data: `jesus_child_of_page_names_each_parentage_on_its_edge` (each edge label `{parent} · {wording} · Jesus`, wording read from the file); `adams_parent_of_edge_from_god_reads_creator_of`; `the_wording_shows_only_on_the_edge`.
- [ ] **Step 2:** Implement: `relation_label` is `names.parentage.of(p)` for every `ParentOf` record (no `Natural` special case: the table is total).
- [ ] **Step 3 (`contract`):** rebuild, re-bless. Only `parent-of` edge labels change (count in the ledger; natural rows change from the derived "Parent of" to the curated "Parent of", so their bytes do not).
- [ ] **Step 4 (`heavy`):** gates as Task 1. **Commit:** `labels: a parent-of edge is labelled by its curated parentage wording, total over the parentage kinds; the wording lives in data, not client code (rules 26, 27)`.

### Task 3: F-36, every year and span label compiled (its own item)

**Category (24):** a year or a span is formatted per request (`atlas_core::label::{Year, TimeRange}::of`, `DateClaim::of`), on every kind with years: Person (birth, death, first, last), Event (`when`), Place (`established`, `destroyed`), Era and Map (`window`), Polity (`reign`), Book (`written`). **Side:** the compiler (rule 27: data-only). **Closure (24b):** the label functions move into the compiler's `labels.rs`; the served `Year`/`TimeRange` are read from compiled rows; `label.rs`'s constructors that format become private to the compile tool; a source law over the served crates forbids `" BC"` / `"AD "` literals and any call of the formatter.

**Files:** `server/atlas-graph/src/labels.rs` (`year_label`, `span_label`, the `c. ` rule from `Precision`); `server/atlas-graph/src/sqlite/{ddl,writer}.rs` (a `dating_label` table keyed by (element, field)); `server/atlas-core/src/label.rs`; `server/atlas-contract/src/{graph,wire/time}.rs`; `server/atlas-contract/tests/no_served_label_composition.rs`; `graph-types/src/chrono.rs` (`Year::{era,succ,pred}`, `YearSpan`, `Precision`, from the Year spec §1); `data/compiled`; pacts. Wire: `TimeRange` gains `circa: bool` (AQC minor); labels unchanged in bytes except where `c. ` now also marks a circa dating (recorded in the ledger by kind).

- [ ] **Step 1 (red):** the Span and Labels laws of Part 3; `every_served_dating_reads_its_compiled_label` (real data, walks every kind with years through `NodeKind`); the source law.
- [ ] **Step 2:** Implement. **Step 3 (`contract`):** rebuild, regen, re-bless. **Step 4 (`heavy`):** gates. **Commit:** `labels: every year and span label is compiled, with c. for a circa dating; served code formats no year (F-36 closed; AQC minor)`.

### Task 4: Chronology with direction (FOCUS-5)

**Why the compiler (rule 27):** which event comes before which is `chrono.order`, data alone; the server stops guessing direction per request (the scan dies in Task R, when its C# reader goes).

**Files:** `graph-types/src/{edge,graph,sections}.rs`, `graph-types/src/canon/*`, `graph-types/tests/common/mod.rs` (directed 22 → 23); `server/atlas-graph/src/{event_world,labels,provenance,law_check}.rs` (`populate_chronological_succession`, lowering into `Precedes`); `server/atlas-graph/src/sqlite/{ddl,partition,writer}.rs` (`chronological_succession` table; `SECTION_SCHEMA_VERSION` +1); `data/compiled`; `contracts/*` regen, CHANGELOG (AQC minor), AGC VERSION (minor); pacts; under OPEN 1(a) `client/Contract` regen and one `Affordances.Of` arm (`Precedes`/`Follows` → `DefaultList`), `client.Tests/Explore/AffordancesTests.cs`.

- [ ] **Step 1 (red):** the Chronology laws of Part 3 over generated datings; real data: `every_dated_events_next_in_time_is_its_precedes_neighbour` (walks `chrono.order`); `an_undated_event_serves_no_chronology`; the relation-count law (23 directed, 7 symmetric).
- [ ] **Step 2:** Implement. The `TemporalAdjacency` rows keep lowering to the symmetric kind until Task R (one row family read twice; recorded in `retirement.md` as the D.R.Y. debt it is).
- [ ] **Step 3 (`contract`, `relations!` last):** rebuild, regen, re-bless. A moved golden map fixture stops the task.
- [ ] **Step 4 (`heavy`):** gates. **Commit:** `chronology: the timeline is precedes/follows, earlier to later, one chain consistent with dates (F5 Q2; AQC/AGC minor)`.

### Task 5: Stories, separate from chronology (FOCUS-5)

**Why the compiler:** a story's order and each step's neighbours are the narrative files alone. **Why a membership relation:** "this event is a step of that story" is the domain fact; an event-to-event edge cannot say which story it belongs to when two stories share a step (the FOCUS-5 plan's FINDING) and merges with chronology. It also closes "narrative membership is not modelled as edges".

**Files:** `graph-types/src/{edge,graph,sections}.rs`, canon, common (directed 23 → 24); `server/atlas-graph/src/{event_world,labels,law_check}.rs` and `server/atlas-graph/src/sqlite/writer.rs` (where `Succession` rows get `EdgeMeta::Narrative` today; Task 0 confirms); `server/atlas-contract/src/{graph,wire/graph}.rs` (`EdgeEntry.step: Option<StoryPlace>`); sqlite; `data/compiled`; contracts, CHANGELOG (AQC minor), AGC (minor); pacts; OPEN 1(a) C# regen and one arm.

- [ ] **Step 1 (red):** the Stories laws of Part 3; real data: `every_story_step_is_an_in_story_edge_with_its_neighbours` (walks every narrative); `a_storys_steps_page_is_in_its_own_order`; `an_event_in_two_stories_has_two_in_story_entries`.
- [ ] **Step 2:** Implement. The `in-story` edge's label is `{event} · in story · {story}`; the narrative's `story-steps` page is ordered by `ord` (the compiled `edge_index` order). The event-to-event narrative `follows-in` edges keep serving until Task R (the C# `ArrowNav` and the map read them).
- [ ] **Step 3 (`contract`):** rebuild, regen, re-bless. **Step 4 (`heavy`):** gates. **Commit:** `stories: each story step is an in-story edge carrying the step's previous and next; stories are compiled independently of chronology (F5 Q3; AQC/AGC minor)`.

### Task 6: Accounts are passages that record history (FOCUS-5)

**Why the compiler:** which verses make an account, its runs, its code label and its opening words are facts over the curated witnesses, the attestation rows and the canonical text alone.

**Files:** `graph-types/src/edge.rs` (`Narrates`, `Narration`, `passage_container_id(&[BibleLocusRange])` generalizing FOCUS-3's two-locus form; its call sites pass `&[range]`), sections, canon, common (directed 24 → 25); `server/atlas-graph/src/{event_world,references,labels,law_check,xref_adapter}.rs`; sqlite (`narration` table; a compiled `opening` per passage); `server/atlas-contract/src/{graph,wire/graph}.rs` (`NodeRef.opening`); `data/compiled`; contracts, CHANGELOG (AQC minor), AGC (minor); pacts; OPEN 1(a) C# regen and one arm.

The compiler, for each dated or undated event:
1. each curated witness (`event-witnesses.toml`) is one account: its verses, coalesced into maximal runs (`runs::coalesce`);
2. the event's remaining attested verses (none of its witnesses' verses) are coalesced into maximal contiguous runs, and **each run is one account** (F5 Q6: "contiguous verses should become passages");
3. each account mints (or reuses, by verse set) one passage Container with its `contains` rows, label `runs_reference(&runs)`, opening `opening_words(first verse's canonical text)`, and one `Narration` row to the event;
4. the per-verse `Attests` rows stay (a verse's `attests` group is unchanged, FOCUS-2 answer 3).

- [ ] **Step 1 (red):** the Accounts laws of Part 3; `references.rs`: `a_run_set_reads_as_one_reference_with_later_runs_shortened` (`MRK.14.54, 66-72`), `a_run_across_chapters_names_both_chapters_once` (`MAT.5.1-7.29`, `GEN.29.32-30.24`), `an_opening_is_cut_at_its_word_limit`; real data: `the_sermon_on_the_mount_is_narrated_in_its_curated_accounts`, `peters_denial_in_mark_is_one_account_of_two_runs`, `an_event_without_witnesses_has_one_account_per_attestation_run`, `a_cited_span_and_an_account_over_one_verse_set_are_one_passage`; `grounds_of` gains the `Narration` arm (FOCUS-4's law walks it).
- [ ] **Step 2:** Record in the ledger: accounts from witnesses, accounts from runs, passages shared with FOCUS-3's cited spans, FOCUS-3 passage labels that change to the run code (F5 Q8).
- [ ] **Step 3:** Implement. **Step 4 (`contract`):** rebuild, regen, re-bless. **Step 5 (`heavy`):** gates. **Commit:** `accounts: each account of an event is a passage that narrates it, built from its witness or from a contiguous attestation run, labelled by its run code with its opening words (F3 Q5, F5 Q6-Q8; AQC/AGC minor)`.

### Task 7: The concurrent set: the window read, event facet (FOCUS-5)

**Why a read, not edges (Year spec §4):** 16,836 ordered pairs today, growing with the square of a busy year; the window read answers the same set with one bounded indexed read, and the Year extends it with its other facets.

**Files:** `graph-types/src/chrono.rs` (`YearSpan` algebra, if Task 3 has not landed it), `graph-types/src/sections.rs` (`YearCover`, `SpanStart` row families, `Event` facet); `server/atlas-graph/src/{event_world,law_check}.rs` (the compiled cover index from `event_date`); `server/atlas-graph/src/sqlite/{ddl,writer,snapshot}.rs` (two index-only reads); `server/atlas-contract/src/{lib,graph,wire/graph}.rs` (`/api/window`, `WindowPage`, `WindowEntry`, `WindowCursor`); `server/atlas-contract/tests/{graph_api,contract_coverage}.rs`; `server/atlas-contract/benches/queries.rs`; contracts, CHANGELOG (AQC minor); pacts.

- [ ] **Step 1 (red):** the Window and Concurrency laws of Part 3; real data: `the_concurrent_set_of_an_event_is_every_other_event_of_its_year`, `a_window_of_the_passion_year_reads_its_events_in_chronological_order`, `the_window_route_is_published`; the cost law at 1× and on the 10× synthetic graph (F-37's generator if it exists, else a generated cover table).
- [ ] **Step 2:** Implement: two keyset range scans, `Covers` then `Starts`; the cursor names its phase. The page cap is the server's one constant (F-62).
- [ ] **Step 3 (`contract`):** rebuild, regen, re-bless. **Step 4 (`heavy`):** gates, `bash scripts/timing-gates.sh run`. **Commit:** `window: a bounded read of what a span of years holds, events in chronological order; an event's concurrent set is its own window (F5 Q3, 27a/27b; AQC minor)`.

### Task 8: The event's map view, server half (FOCUS-5)

**Why there (rule 25):** to highlight an event's stories the map must compare the graph's own references, which only the server holds.

**Files:** `server/atlas-core/src/{wire,scene}.rs` (`SceneArrow.from_node`, `to_node`, read from the compiled labels as FOCUS-6 did for places); `server/atlas-contract/src/map.rs`; contracts, CHANGELOG (AQC minor); pacts. Test: `graph_api.rs` `every_scene_arrow_names_its_two_events_by_their_graph_reference`; `scene_byte_identity.rs` (the recorded scene gains exactly the two fields; any other byte moving **stops for the owner**, O-GOLDEN).

- [ ] Red → implement → regen (`contract`) → gates (`heavy`). **Commit:** `scene: an arrow names its two events by the graph's own reference, so the map can highlight an event's stories (F5 Q4; AQC minor)`.

The frame (the event's years) is the served `when`; the highlighted stories are the event's `in-story` page (Task 5). Nothing else is needed on the server.

### Task R: Retirement (after the F# client replaces the C# person and event views)

**Starts only when** the F# client serves Person and Event (backlog items 1–14 landed) and the owner retires the C# client's person and event views. One AQC major, one AGC major.

Remove, each with the law that keeps it gone:
- `PersonLife.eternal_grounds` (the grounds are the `justified-by` group);
- `SymRelationId::TemporalAdjacency` and its lowering (`DECLARED_SYMMETRIC_RELATIONS` 7 → 6); `GraphService::temporal_neighbors_of`;
- the event-to-event narrative `follows-in`/`precedes-in` edges (stories are `in-story`; canon and map succession stay);
- `loci` and `note` on `attests`/`attested-in` entries; `EventAccounts`, `AccountOf`, `account_verses` and their two `panic!`s in `graph.rs`;
- `/api/event/{id}`, `/api/narrative/event/{id}`, `server/atlas-contract/src/events.rs`, `wire/events.rs`, `atlas-core/src/narrative.rs` (if no reader), `analogue_provenance`, `attests_provenance`, `event_mentions_provenance`;
- `EventDetail`'s bookkeeping (`robertson_section`, `acts_section`, `atlas_section`, `ref_note`): the data keeps them; the wire does not serve what no reader shows (F5 Q5; F-50's curator text);
- `event_from_node`'s `located-at` drain from the record (`event_detail` reads the payload and `chrono.resolved` only).

Gate: `retirement.md` is empty; `no_legacy_event_reads.rs` and `contract_coverage.rs` assert each removal.

### Task Z: Gates, mutation, close

- [ ] Mutation, in the owner's window only (`heavy` with "mutation", `free -g` ≥ 18): `bash scripts/mutants-parallel.sh -n 3 -b <base>` → 100% or equivalents recorded (covers `grounds_of`, `ParentageWording`, the eternity clear, the label functions, `populate_chronological_succession`, the story lowering, the account minting, `runs_reference`, `opening_words`, the window read). Outside the window, `.superpowers/MUTATION-GATE-DEBT.md` names `<base>`.
- [ ] `cargo test --workspace && (cd graph-types && cargo test --all-features) && bash scripts/contract-gate.sh --base <base> && bash scripts/timing-gates.sh run`, the frozen-client gate → green. The diff adds no comment line.
- [ ] Close report `docs/superpowers/reports/2026-10-xx-focus45-server-close.md`: each rule-24 category with its closure and guarantee (below), the FINDINGS, `retirement.md`'s list.

| Category | Closure | Guarantee |
|---|---|---|
| A curated ground the graph does not hold as an edge | `grounds_of` exhaustive over `RowFamily` | compiler (no wildcard) + `RowFamily::ALL` walk |
| A curated eternity served as a lifespan | cleared at compile | ETL law + real-data law |
| Domain wording in client code (`Kinship`, `"Jesus"`) | wording and incarnation in data | `Parentage::ALL` totality; the C# sites die with the C# client |
| A year or span formatted per request (F-36) | compiled labels | source law over served crates |
| Chronology direction guessed per request | `precedes`/`follows` compiled | chain laws; the scan removed in Task R |
| Stories merged with chronology | `in-story` membership | independence law |
| Accounts derived per request and formatted on the client | compiled passages + `narrates` | partition law; one label function |
| Concurrency derived on the client | the window read | concurrency laws; cost law |
| A map row compared by a composed id | `SceneArrow.from_node`/`to_node` | real-data law |

## Wave schedule

| Wave | Primary (Rust) | Beside it | Critical sections |
|---|---|---|---|
| 0 | Task 0; OPEN 1 | — | — |
| 1 | Task 1 | Task 3 tests written | `contract` rebuild #1, regen |
| 2 | Task 2 | Task 3 | rebuilds #2, #3 |
| 3 | Task 4 | Task 6 tests written | rebuild #4, `relations!` |
| 4 | Task 5 | Task 8 | rebuild #5, `relations!`; regen |
| 5 | Task 6 | Task 7 tests written | rebuild #6, `relations!` |
| 6 | Task 7 | — | rebuild #7, regen |
| 7 | Task Z | — | `heavy` |
| later | Task R (after the F# client) | — | one major |

Tasks 1–3 are FOCUS-4's and F-36; 4–8 are FOCUS-5's. Rust is the only build in every wave; the frozen-client gate's C# run is the unlike work beside it (rule 23). Build nothing beyond what a task's gate needs.

---

## F# client backlog (from the FOCUS-4 and FOCUS-5 plans' client tasks, rewritten to the rulings)

Written for the F# client (CX-FSHARP), after the F# style sign-off. Each item composes over the contract above; none parses, formats or computes.

**Person (FOCUS-4 Tasks 3–5):**
1. Person card: fields in order `Life` = "Eternal" (eternal only), `Born`/`Died` (or "Born (earthly life)"/"Died (earthly life)" when `incarnate`), `First mentioned`/`Last mentioned` only when neither Born nor Died is served, `Provenance`.
2. Every served year on the card is a link: to the Year once TIME lands; until then the World at that year.
3. God's grounds: the `justified-by` group, links to the verses.
4. Kinship: `parent-of`, `child-of`, `spouse-of`, `brethren-of` groups in served order, entries by the person's label; the parentage wording only on the edge (its ⋮ step). No siblings group (a parent's `parent-of` is one step away).
5. `mentioned-in` collapsed and last, on every kind's card.
6. A person mention in verse text opens its served node.

**Event (FOCUS-5 Tasks 1, 2, 6):**
7. Event card: `When` (linked as item 2), `Superscription`, `Source`; no bookkeeping fields.
8. Arrows: `follows` (‹ previous) and `precedes` (next ›) only, each with its edge step.
9. **Stories** section: one row per `in-story` entry, `‹ previous · story · next ›`, each a link (from `EdgeEntry.step`).
10. **At the same time** section: `/api/window` over the event's own `when`, facet event, the current event omitted; absent for an undated event.
11. Accounts: the `narrated-in` group, each entry `reference — opening words…` from the served label and `opening`; following one opens the passage.
12. `attested-in` collapsed and last.
13. Event map view: the World framed on the event's `when`, its `in-story` stories highlighted by `SceneArrow.from_node`/`to_node`; leaving the event clears the highlight.
14. A pericope heading opens its served event.

**Year (TIME, after this plan):** the Year spec §6.

**Re-expressed specs:** `person-card`, `reader-persons`, `event-timeline`, `event-timeplace`, `popover-sections`, `accounts-and-mentions`, `provenance`, `reader-headings`, `world-pin`, `world-narrative-focus`, `w1`–`w5`, rewritten against the F# client's ids when its views exist (the old plans' Task 5/Task 6 lists are the inventory).

## FINDINGS this plan expects to raise

- `TemporalAdjacency` rows are read twice (symmetric and `Precedes`) between Task 4 and Task R: D.R.Y. debt, held on `retirement.md`.
- `parallel` is a declared symmetric relation with no row family (rule 4).
- `TimeRange::undated()` writes the atlas span in code (rule 26; the Year spec).
- `also_called` serves search names, not names (A-NAMES).
- A person's `description` (Easton, with site-relative links) is served and shown nowhere; also under A-THEO's licence question.
- Account notes are curator text in `Narration.justification` (F-50's category; not served).
