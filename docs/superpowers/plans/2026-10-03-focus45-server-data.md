# FOCUS-4 (Person) and FOCUS-5 (Event): server, data and contract plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Executor:** Claude or Codex, as the controller names (queue items `A-F45` / `CX-F45`). Server, data and contract only. **The C# client is frozen** (owner, 2026-10-03: "there's no point in writing a bunch of code that's going to be rewritten"): every client task of the earlier FOCUS-4 (`lane/claude/F4-plan` `da92793`) and FOCUS-5 (`lane/claude/F5-plan` `564f4ca`) plans is moved to **the F# client backlog** at the end. The executor touches only the files a task lists; anything else goes to FINDINGS in the queue (rule 24a).

**Goal:** Put into the graph, the artifact and the contract everything a person's and an event's presentation needs, as the owner ruled on 2026-10-03, so the F# client composes it with no domain logic:
- a person: eternity applied at compile, God's grounds of eternity as links, every curated ground as a `justified-by` edge, parentage wording as data shown only on the link, a curated incarnate record for Jesus's earthly life;
- an event: positions finer than a year (a day of the text's month, a festival, a day of the week, a counted day after another event), each on its verse; festivals as explorable nodes; chronology with direction (`precedes`/`follows`), a linear extension of what the verses fix, separate from stories; each story's previous and next on the event; a concurrent set ("At the same time": certain overlap or a curated parallel, compiled, served whole); accounts as single unbroken runs (FOCUS-3 v2's passages, or one verse), marked as recording history, listed by reference and opening words; the event's map view's data;
- F-36: every year, span and position label compiled;
- Theographic's dates read through one padding-blind door that refuses unknown shapes.

**Supersedes:** the server/data/contract halves of the two plans above. Their client halves, specs re-expressions and C# deletions are the F# client backlog. Their OPEN lists are answered (below).

**Spec:** `docs/superpowers/specs/2026-10-03-year-design.md` (this branch): §1 domain types, §2 data structures, §3 algebras. This plan uses its `YearSpan`, finer positions and festivals (§1.7, §1.8, §2.9), chronology and concurrency (§2.5, §3.4, §3.8), story, account (§2.8, reconciled with FOCUS-3 v2) and parentage parts, and the Theographic normalisation law (§2.11); the Year itself (Year nodes, the cover index, the window read) builds after this plan (owner: "Year: spec it now, build after FOCUS-5").

**Principles:** 4, 9, 12, 14b, 15–18, 21–23, 24–24b, 25, 26, 26a, 27–27g.

**Sequencing:** after **FOCUS-3 v2** (`lane/claude/F3-v2` 3a4526b: it builds the single-run `Passage`, `PassageMint`, `target(span)`, `passage_container_id(&PassageSpan)` and the container text read; `references.rs` from FOCUS-2). Then this plan. Then the Year (TIME). Task 0 records the base (PRINCIPLES 22).

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
| F5 Q8: labels stay as codes (`GEN.1.1`, `GEN.29.32-30.24`) | Task 6: the account target's own code (FOCUS-3 P9) |
| C# client frozen; client halves become the F# backlog | whole plan; the freeze rule below |
| Year Q1–Q3: alive = recorded lives + office holders; curate offices with a roles column; arrows step every event in curated order | TIME (the Year spec §1.5, §7); Task 4 keeps the curated sequence as the tie-break |
| Year Q4 = OPEN 1 (a): the frozen C# app gets regenerated types and one line per new link kind | the freeze rule, item 2 |
| F# domain Q1: years are served nodes stepped by served links; the client does no date arithmetic | Year spec §5: `Year.value` has no F# reader, retires in Task R; `TimeRange.position` is a compiled label |
| F# domain Q2: one span per event with "c.", or undated; undated has no chronology | Tasks 3, 3b, 4, 7: `EventWhen`; laws `an_undated_event_has_no_chronology`, `an_undated_event_has_no_position_and_no_concurrency` |
| F# domain Q6 + the data check: "At the same time" is not same-year | Task 7 rewritten: compiled `concurrent-with` from certain overlap and curated parallels, loaded whole; the window read leaves this plan for TIME |
| An account is one unbroken run; a story told in pieces is several accounts | Task 6 rewritten on FOCUS-3 v2's `Passage`/`target(span)` |
| Time granularity: a festival, a day or a weekday attaches to an event, on its verse; festivals are explorable | Task 3b (new) |
| Theographic spells AD 30 two ways | Task 3a (new) |

## The freeze rule (how the server changes while the C# client is frozen)

The frozen C# client is still the owner's demo build. It decodes the contract with generated types and refuses an unknown edge kind or a missing required field (F-81 aside). So:

1. **Additive now.** Every change in Tasks 1–8 adds: a relation, a row family, an optional field, a route. Nothing the C# client reads is removed or reshaped.
2. **New vocabulary reaches the C# build mechanically** (OPEN 1 below): each new edge kind regenerates the C# contract types and adds one `DefaultList` arm to `Affordances.Of`, nothing else, so the frozen client keeps decoding every node. This is the only C# change in this plan.
3. **Removals wait for the C# retirement (Task R).** `PersonLife.eternal_grounds`, `temporal-adjacency`, the event-to-event narrative `follows-in` edges, the `loci`/`note` on attestation entries, `/api/event`, `/api/narrative/event`, the per-request `EventAccounts` and `temporal_neighbors_of`, and `EventDetail`'s bookkeeping fields all keep serving until the F# client has replaced the C# person and event views. Task R removes them in one AQC major. Until then each is listed in `.superpowers/sdd/F45/retirement.md` with its reader, so none is forgotten (a shrinking list, O-WIRE-IDENTITIES O3's precedent).

## OPEN: for the owner

1. ~~The frozen client and new link kinds.~~ **Answered (a)** (Year Q4, 2026-10-03): regenerated types plus one line per new link kind.
2. The Year spec §7 questions 4–6 (what counts as "at the same time"; the Bible's own calendar with no conversion; what is curated first). Tasks 3b and 7 follow the recommendations if unanswered.

## Global constraints

- `docs/PRINCIPLES.md` binds: rule 4 (no member without a reader: every new wire field names its F# backlog reader), 9 (no comments), 12 (every type below is for sign-off), 24–24b (each category closed; offenders to FINDINGS), 26/26a (facts in `data/`), **27** (data-only derivations compiled; the server reads; the graph models the domain; no exploration word in `server/` or `graph-types/`).
- Tests: whole-body assertions, one behaviour per test named as a sentence, `// Arrange` `// Act` `// Assert` only, no magic numbers, newspaper order; real-data expectations read from the artifact (F-8); the algebra laws below are proptest properties.
- Every closed vocabulary is matched without a wildcard arm (`Parentage`, `RowFamily`, `Facet`).
- Worktree per task (`git -c core.autocrlf=false worktree add -b lane/<agent>/F45-t<n> ~/w/F45-t<n> <base>`), `CARGO_TARGET_DIR=~/mut/<agent>-F45-t<n>`, `nice -n 10 cargo -j 4`, data copied never linked; `find <wt> -type l | wc -l` prints 0 before removal.

## Critical sections (PRINCIPLES 21)

| Section | Held by |
|---|---|
| `contract`: rebuilding `data/compiled` | Tasks 1, 2, 3, 3a, 3b, 4, 5, 6, 7 (one rebuild each, in order; tasks may pair rebuilds when they land together) |
| `contract`: `export_contract`, `export_aqc_examples`, `client.ContractGenerator`, re-blessing | Tasks 1, 3, 3b, 4, 5, 6, 7, 8 |
| appending to `relations!` | Task 3b (`AtFestival`, `InstitutedIn`), Task 4 (`Precedes`), Task 5 (`InStory`), Task 6 (`Narrates`), Task 7 (`ConcurrentWith`, symmetric), in that order, each last |
| `heavy` | every task's closing gate; mutation only in Task Z, in the owner's window |

---

## Part 1. Domain types (for sign-off)

The shared types (`Year`, `YearSpan`, `Precision`, `Dating`, `EventWhen`, the finer positions `PositionClaim`/`PositionRow`/`DayKey`/`EndPosition`, `Festival`/`FestivalCalendar`, `ChronologicalSuccession`, `Concurrency`, `StoryStep`, `AccountTarget`/`Narration`) are the Year spec's §1. The passage is FOCUS-3 v2's `Passage` unchanged. This plan adds or changes:

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
    directed { …, AtFestival => "at-festival" / "festival-events", InstitutedIn => "instituted-in" / "institutes", Precedes => "precedes" / "follows", InStory => "in-story" / "story-steps", Narrates => "narrates" / "narrated-in" }
    symmetric { …, ConcurrentWith => "concurrent-with" }
}

pub struct FestivalKeeping { pub event: EventId, pub festival: FestivalId, pub provenance: ProvenanceId }
pub struct Institution { pub festival: FestivalId, pub passage: AccountTarget, pub provenance: ProvenanceId }
pub struct ChronologicalSuccession { pub earlier: EventId, pub later: EventId, pub provenance: ProvenanceId }
pub struct Concurrency { pub a: EventId, pub b: EventId, pub ground: ConcurrencyGround, pub provenance: ProvenanceId }
pub enum ConcurrencyGround { CertainOverlap, Curated(Justification) }
pub struct StoryStep { pub story: NarrativeId, pub ord: u16, pub event: EventId, pub provenance: ProvenanceId }
pub enum AccountTarget { Verse(VerseNodeId), Passage(ContainerNodeId) }
pub struct Narration { pub account: AccountTarget, pub event: EventId, pub provenance: ProvenanceId, pub justification: Justification }

pub enum EdgeMeta { …, Step { ord: u16, previous: Option<EventId>, next: Option<EventId> } }
```
- `passage_container_id(&PassageSpan)` and `target(span)` are FOCUS-3 v2's, used as they are: an account is one run, so no multi-run form is added (the earlier `passage_container_id(&[BibleLocusRange])` and `runs_reference` are withdrawn).

`graph-types/src/chrono.rs` (Task 3b): the Year spec §1.7 types; `TimePoint.month: Option<u8>` is replaced by the resolved `EndPosition`; `temporal_order` is replaced by `relation(a, b) -> Known { Before, After, Concurrent, Indeterminate }` and the keyed linear extension (Task 4).

`server/atlas-graph/src/references.rs`:
```rust
pub fn opening_words(text: &str) -> String;
pub const OPENING_WORDS: usize = 8;
```

`server/atlas-graph/src/event_world.rs`:
```rust
pub const CONCURRENT_WHOLE: usize = 20;
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

`server/atlas-contract/src/wire/graph.rs` (Task 3b): `TimeRange.position: Option<PositionDetail>`, `PositionDetail { label, festival: Option<NodeRef> }`, `NodeRecord.festival: Option<FestivalDetail>`, `NodeKind::Festival`.

The window read is **not** in this plan any more: "At the same time" is the compiled `concurrent-with` group (Task 7), and the window read (with Year ids as its ends) builds with the Year (TIME).

## Part 2. Data structures, invariants and bounds

| Structure | Rows today | Invariants (compile fails otherwise) | Bound |
|---|---|---|---|
| Eternity | 2 eternal, 1 incarnate | an eternal person has no year; an incarnate person is not eternal; every id resolves | data-sized |
| `justified-by` from every row family that records grounds, and from eternal persons | grows by the grounds of `ParentOf`, `Brethren`, `Attests`, … (recorded per family in the ledger) | `grounds_of` exhaustive over `RowFamily` | one edge per ground |
| Parentage wording | 5 (`Natural` "Parent of", `Eternal` "Eternal Father of", `Virgin` "Mother of", `Legal` "Father of", `Created` "Creator of") | exactly one per `Parentage::ALL` | closed enum |
| Theographic `startDate` (Task 3a) | 450 events, 221 strings, 191 years, 8 shapes | one door, padding-blind, refuses unknown shapes; month/day discarded | closed shape set |
| Festivals → `NodeKind::Festival`, `instituted-in` (Task 3b) | 8 nodes, ≈ 15 edges | exactly the eight; every calendar claim on its verse | closed |
| `months.toml` (Task 3b) | 11 names (10 numbered, Elul not) | a numbered name rests on a verse equating it | closed |
| `event-positions.toml` → resolved `EndPosition`, `at-festival`, `justified-by` (Task 3b) | ≈ 150 claims on ≈ 95 events (Year spec §2.10) | every claim on a verse; every end's claims meet non-empty; a positioned event is dated | ≤ 8 claims per event |
| Chronology `ChronologicalSuccession` → `precedes` | 911 | one chain over the 912 dated events; a linear extension of `≺`; no curated sequence contradicts a verse; `≺` acyclic | dated events − 1 |
| Concurrency → `concurrent-with` (Task 7) | 0 today; ≈ 15 pairs after Task 3b's curation, plus curated parallels | symmetric, irreflexive; certain overlap or curated; disjoint from `≺` | ≤ `CONCURRENT_WHOLE` (20) per event: a compile gate |
| Story steps → `in-story` | 255 in 13 stories | `ord` without gaps; `previous`/`next` are the neighbours in the story | steps |
| Accounts → `narrates` | 2,082: 1,897 passages, 185 single verses (measured) | each account one maximal unbroken run; accounts partition each event's attested verses; one node per verse set (FOCUS-3 P5) | ≤ attested verses |
| Compiled year/span/position labels (Tasks 3, 3b) | every `Year`, `TimeRange` and `PositionDetail` the wire serves | one label rule, with `c. ` for a circa dating | one per served dating |

## Part 3. Algebras and their laws (the tests)

| Algebra | Laws (test names; properties unless marked real-data) |
|---|---|
| Parentage | `parentage_wording_is_total` (walks `Parentage::ALL`; a missing, extra or duplicate entry fails the parse); `a_parent_of_edge_reads_its_wording` (`{parent} · wording(p) · {child}`, the same edge read from either end); `the_wording_shows_only_on_the_edge` (real data: no person's label or entry text carries a wording). |
| Eternity | `an_eternal_person_carries_no_year`; `an_incarnate_person_is_not_eternal`; `gods_justified_by_group_is_his_curated_grounds` (real data). |
| Grounds | `every_family_that_records_grounds_justifies_its_edges_by_them` (walks `RowFamily::ALL`). |
| Span (from the Year spec §3.2) | `no_year_is_zero`, `the_year_after_1_bc_is_ad_1`, `a_span_is_never_inverted`, `a_span_is_the_union_of_its_years`, `contains_is_membership`, `within_is_subset`, `overlap_is_shared_years`, `overlap_is_symmetric`, `hull_is_a_semilattice`, `spans_meet_across_the_zero_gap`. |
| Labels (F-36) | `a_years_label_names_its_era_once`; `a_span_names_a_shared_era_once` (`1450 – 1400 BC`, `5 BC – AD 30`, `AD 33`); `a_circa_dating_reads_c`; `no_served_code_formats_a_year` (source law over `server/` outside the compiler: no `" BC"`/`"AD "` literal). |
| Theographic dates (Task 3a) | `theographic_years_are_padding_blind`; `every_theographic_start_date_has_a_known_shape`; `an_unknown_shape_fails_the_etl_naming_its_event`; `no_theographic_month_or_day_reaches_a_position`. |
| Positions (Task 3b; Year spec §3.8) | `day_keys_are_totally_ordered_with_bottom_and_top`; `a_year_only_end_is_the_whole_year`; `claims_meet_as_intervals`; `a_claim_set_resolves_in_any_order`; `a_festival_claim_resolves_to_its_window`; `a_calendar_claim_selects_the_festival_window`; `a_calendar_claim_matching_no_window_fails`; `a_festival_offset_stays_in_its_month_or_gives_order_only`; `a_month_name_orders_only_if_the_text_numbers_it`; `weekdays_agree_with_counted_days`; `every_position_rests_on_a_verse`; `every_position_is_justified_by_an_edge`; `the_festivals_are_exactly_the_eight`; `every_fixed_festival_has_its_institution`. |
| Order (Tasks 3b, 4) | `before_is_a_strict_partial_order`; `before_is_total_where_days_are_known`; `unknown_is_never_earlier`; `relative_claims_order`; `two_dated_events_stand_in_exactly_one_relation`. |
| Chronology (Task 4) | `previous_and_next_are_inverses`; `the_chronology_is_one_chain`; `an_undated_event_has_no_chronology`; `the_chain_is_a_linear_extension_of_what_is_known`; `the_chain_breaks_only_unknown_ties_by_the_curated_sequence`; `the_chain_is_deterministic`; `a_curated_sequence_against_a_verse_fails_the_compile`; `contradictory_positions_fail_the_compile`. |
| Concurrency (Task 7) | `concurrency_is_symmetric`; `concurrency_is_irreflexive`; `concurrency_is_certain_overlap_or_curated`; `same_year_is_not_concurrency`; `same_day_is_concurrency`; `concurrency_and_order_are_disjoint`; `concurrency_is_not_closed_under_transitivity`; `an_undated_event_has_no_position_and_no_concurrency`; `every_concurrent_set_loads_whole` (real data). |
| Stories | `a_storys_steps_are_numbered_without_gaps`; `a_steps_previous_and_next_are_its_neighbours_in_the_story`; `a_story_lists_its_events_in_its_own_order` (real data); `stories_are_independent_of_chronology` (compile with the narrative files removed or a story permuted: every `precedes` edge unchanged; change a dated-by placement: every `in-story` edge unchanged). |
| Accounts | `an_events_accounts_partition_its_attestation`; `an_account_is_one_unbroken_run`; `no_two_accounts_of_an_event_touch`; `a_run_may_cross_a_chapter`; `a_run_never_crosses_a_book`; `a_one_verse_account_is_the_verse`; `one_verse_set_is_one_passage`; `a_passage_records_history_iff_it_narrates`; `an_opening_is_the_first_words_of_the_first_verse`; `a_story_told_in_pieces_is_several_accounts` (real data: `MRK.14.54` and `MRK.14.66-72`); `an_accounts_label_is_its_targets_code` (real data: `MAT.5.1-7.29`). |

---

## Tasks

### Task 0: Base and preconditions (no code)

**Files:** `.superpowers/sdd/F45/progress.md`, `.superpowers/sdd/F45/retirement.md` (create).

- [ ] **Step 1:** Record the base: the head FOCUS-3 landed at. Record OPEN 1's answer (or the default).
- [ ] **Step 2:** Verify at the base, recording each: FOCUS-3 v2's `Passage`, `PassageMark::RecordsHistory`, `PassageMint::mint`, `target(span)` and `passage_container_id(&PassageSpan)` exist and `GET /api/node/{id}/text` exists; `theographic::parse_theo_year` is still the only `startDate` reader; no `event_date` row has a month; FOCUS-2's `references.rs` exists; `relations!` still lists `TemporalAdjacency`; `add_justified_by` still wires only four families. A difference stops the task for the controller.
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

### Task 3a: Theographic dates through one padding-blind door (ETL)

**Category (24):** a raw date read by a hand-rolled parser over an unclosed set of spellings, which drops what it cannot read (`30`/`0030`, `29`/`0029`; 8 shapes; Year spec §2.11). **Side:** the ETL (a tool, 26a). **Closure (24b):** one door `theographic::start_year(raw) -> Result<Year, TheoDateError>` over a closed `TheoDateShape`; the ETL fails on an unknown shape naming the event; a source law that nothing else in `atlas-etl` reads `startDate`. Library survey: Year spec §2.12 (`regex` + `str::parse`; no ISO parser fits astronomical, unpadded years).

**Files:** `server/atlas-etl/src/theographic.rs` (`start_year`, `TheoDateShape`, `TheoDateError`; `parse_theo_year` deleted); `server/atlas-etl/tests/theographic_dates.rs` (new); `.superpowers/sdd/F45/theographic-gregorian.md` (the 50 `YYYY-MM-DD` strings, for curators: never positions). `data/compiled` only if a year moves (expected: none; any move stops the task for the controller).

- [ ] **Step 1 (red):** the Theographic dates laws of Part 3, over generated spellings and over the real `events.json` (every one of the 450 has a known shape; the 221 strings give 191 years).
- [ ] **Step 2:** Implement. **Step 3 (`heavy`):** gates; the artifact's logical hash is unchanged (recorded). **Commit:** `etl: Theographic start dates are read through one padding-blind door over a closed set of shapes; an unknown shape fails the build instead of dropping the event (30 and 0030 are one year)`.

### Task 3b: Finer positions and festivals (FOCUS-5, from the Year spec §1.7, §1.8, §2.9)

**Why the compiler (rule 27):** which day, festival or weekday a verse gives an event, and the order and overlap that follow, are facts over curated data alone. **Why curated data (rule 26):** every claim is a domain fact with its verse; the ETL never scans verse text for dates (the §2.10 scan was a one-off sizing in the scratchpad, never part of the build: 26a).

**Files:**
- Data (create): `data/curated/months.toml` (11 names), `data/curated/festivals.toml` (the eight, windows, institution passages, verses), `data/curated/event-positions.toml` (scope per the Year spec §7 Q6; default (a): the ≈ 95 events of §2.10, plus counted days in the Flood, the Exodus and Passion week), `data/curated/parallels.toml` (curated parallels, each with verses; may start empty). `LICENSES.md` unchanged (KJV verses, our own curation).
- Code: `graph-types/src/chrono.rs` (`MonthOrdinal`, `DayOfMonth`, `MonthRef`, `Weekday`, `FestivalRelation`, `DayReckoning`, `PositionClaim`, `SpanEnd`, `PositionRow`, `Bound`, `DayKey`, `EndPosition`, `ResolvedSpan`, `EventWhen`, `relation`; `TimePoint.month: Option<u8>` and the derived `Ord` that sorts unknown first are removed); `graph-types/src/{node,edge,graph,sections,id}.rs` (`NodeKind::Festival`, `FestivalId`, `Festival`, `FestivalCalendar`, `DayWindow`, `FestivalKeeping`, `Institution`, `AtFestival`, `InstitutedIn`; directed 22 → 24), canon, common; `server/atlas-etl/src/{curated,compile}.rs` (parsers for the four files, `expand_verse_ref` for every `verses`); `server/atlas-graph/src/{event_world,labels,law_check,provenance}.rs` (resolution, `grounds_of` arms for `PositionRow` and `Concurrency::Curated`, position and calendar labels); `server/atlas-graph/src/sqlite/{ddl,writer,partition}.rs` (`event_position`, `festival`, `festival_window`, `month_name` tables; `SECTION_SCHEMA_VERSION` +1); `server/atlas-contract/src/{graph,wire/graph,wire/time}.rs` (`TimeRange.position`, `PositionDetail`, `FestivalDetail`); contracts, CHANGELOG (AQC minor), AGC (minor); pacts; OPEN 1(a) C# regen and one `Affordances.Of` arm per new kind.
- Test: `graph-types/tests/positions.rs` (proptest), `curated.rs` unit tests, `server/atlas-contract/tests/graph_api.rs`.

- [ ] **Step 1:** Curate. For each event of §2.10's reviewed list, write its claims with verses; record in the ledger every event the review excluded and why (broad event, law passage, false hit), so the next curator sees the decision. Hand-check every claim against the KJV text; nothing is inferred from Theographic's Gregorian dates (Task 3a's list).
- [ ] **Step 2 (red):** the Positions and Order laws of Part 3; parse-time invariants of the Year spec §2.9 (a row without verses, an unknown id, month 13, a festival with two calendar kinds, a positioned undated event: each fails); real data: `the_passover_at_sinai_is_the_fourteenth_of_the_first_month`, `hezekiahs_passover_is_in_the_second_window`, `the_flood_starts_and_ends_on_its_days`, `elul_is_served_but_not_numbered`, `pentecost_is_the_feast_of_weeks`, `the_crucifixion_and_the_empty_tomb_agree_with_the_third_day`, `the_passovers_events_are_its_festival_events`.
- [ ] **Step 3:** Implement. The festival record's calendar line and every position label are compiled (F-36's rule); no served code formats a month or a day (the Task 3 source law gains `" month"`/`"day of the"` literals).
- [ ] **Step 4 (`contract`, `relations!` last):** rebuild, regen, re-bless. **Step 5 (`heavy`):** gates. **Commit:** `time: an event's date may be finer than its year (a day of the text's month, a festival, a weekday, a counted day), each on its verse and never converted; festivals are explorable with their calendar and institution; unknown is never read as earlier (AQC/AGC minor)`.

### Task 4: Chronology with direction (FOCUS-5)

**Why the compiler (rule 27):** which event comes before which is `chrono.order`, data alone; the server stops guessing direction per request (the scan dies in Task R, when its C# reader goes).

**Files:** `graph-types/src/{edge,graph,sections}.rs`, `graph-types/src/canon/*`, `graph-types/tests/common/mod.rs` (directed 24 → 25); `server/atlas-graph/src/{event_world,labels,provenance,law_check}.rs` (`populate_chronological_succession`, lowering into `Precedes`); `server/atlas-graph/src/sqlite/{ddl,partition,writer}.rs` (`chronological_succession` table; `SECTION_SCHEMA_VERSION` +1); `data/compiled`; `contracts/*` regen, CHANGELOG (AQC minor), AGC VERSION (minor); pacts; under OPEN 1(a) `client/Contract` regen and one `Affordances.Of` arm (`Precedes`/`Follows` → `DefaultList`), `client.Tests/Explore/AffordancesTests.cs`.

- [ ] **Step 1 (red):** the Order and Chronology laws of Part 3 over generated datings, positions and curated sequences; real data: `every_dated_events_next_in_time_is_its_precedes_neighbour` (walks the compiled chain); `the_passion_week_reads_in_the_order_its_verses_fix`; `an_undated_event_serves_no_chronology`; the relation-count law (25 directed, 7 symmetric).
- [ ] **Step 2:** Implement the chain as the Year spec §2.5's keyed linear extension: `petgraph` for the `≺` graph and its cycle report, Kahn's loop over `BinaryHeap` keyed by `(start.earliest, SeqKey, id)` (survey: Year spec §2.12). A curated sequence that contradicts `≺` fails the compile with both events and their verses. Record in the ledger how many consecutive pairs are fixed by verses and how many by the curated sequence alone. The `TemporalAdjacency` rows keep lowering to the symmetric kind until Task R (one row family read twice; recorded in `retirement.md` as the D.R.Y. debt it is).
- [ ] **Step 3 (`contract`, `relations!` last):** rebuild, regen, re-bless. A moved golden map fixture stops the task.
- [ ] **Step 4 (`heavy`):** gates. **Commit:** `chronology: the timeline is precedes/follows, one chain that extends every order the verses fix, with only unknown ties broken by the curated sequence (F5 Q2; AQC/AGC minor)`.

### Task 5: Stories, separate from chronology (FOCUS-5)

**Why the compiler:** a story's order and each step's neighbours are the narrative files alone. **Why a membership relation:** "this event is a step of that story" is the domain fact; an event-to-event edge cannot say which story it belongs to when two stories share a step (the FOCUS-5 plan's FINDING) and merges with chronology. It also closes "narrative membership is not modelled as edges".

**Files:** `graph-types/src/{edge,graph,sections}.rs`, canon, common (directed 25 → 26); `server/atlas-graph/src/{event_world,labels,law_check}.rs` and `server/atlas-graph/src/sqlite/writer.rs` (where `Succession` rows get `EdgeMeta::Narrative` today; Task 0 confirms); `server/atlas-contract/src/{graph,wire/graph}.rs` (`EdgeEntry.step: Option<StoryPlace>`); sqlite; `data/compiled`; contracts, CHANGELOG (AQC minor), AGC (minor); pacts; OPEN 1(a) C# regen and one arm.

- [ ] **Step 1 (red):** the Stories laws of Part 3; real data: `every_story_step_is_an_in_story_edge_with_its_neighbours` (walks every narrative); `a_storys_steps_page_is_in_its_own_order`; `an_event_in_two_stories_has_two_in_story_entries`; the relation-count law (26 directed).
- [ ] **Step 2:** Implement. The `in-story` edge's label is `{event} · in story · {story}`; the narrative's `story-steps` page is ordered by `ord` (the compiled `edge_index` order). The event-to-event narrative `follows-in` edges keep serving until Task R (the C# `ArrowNav` and the map read them).
- [ ] **Step 3 (`contract`):** rebuild, regen, re-bless. **Step 4 (`heavy`):** gates. **Commit:** `stories: each story step is an in-story edge carrying the step's previous and next; stories are compiled independently of chronology (F5 Q3; AQC/AGC minor)`.

### Task 6: Accounts are passages that record history (FOCUS-5)

**Why the compiler:** which verses make an account, its runs, its code label and its opening words are facts over the curated witnesses, the attestation rows and the canonical text alone.

**Files:** `graph-types/src/edge.rs` (`Narrates`, `Narration`, `AccountTarget`; FOCUS-3 v2's `passage_container_id(&PassageSpan)` used unchanged), sections, canon, common (directed 26 → 27); `server/atlas-graph/src/{event_world,references,labels,law_check}.rs` (accounts through FOCUS-3's `PassageMint`; no second minting path); sqlite (`narration` table; a compiled `opening` per account target); `server/atlas-contract/src/{graph,wire/graph}.rs` (`NodeRef.opening`, on account targets: passages and the 185 single verses); `data/compiled`; contracts, CHANGELOG (AQC minor), AGC (minor); pacts; OPEN 1(a) C# regen and one arm.

**An account is one unbroken run** (owner, 2026-10-03; Year spec §2.8). The compiler, for each event:
1. its attested verses (witness verses included: today every attestation row is a witness's) are coalesced into maximal runs in reading order, breaking at a book; **each run is one account**. A curated witness told in pieces gives one account per piece;
2. a run of two or more verses is `PassageMint::mint(span, RecordsHistory)` (reusing a cited span's node and raising its mark, FOCUS-3 P5); a run of one verse is `target(span)` = the verse (P6), and no passage is minted;
3. each account gets its compiled `opening` (`opening_words` of its first verse's canonical text) and one `Narration` row to the event; its label is the target's own code (FOCUS-3 P9), so no account label function exists;
4. the per-verse `Attests` rows stay (a verse's `attests` group is unchanged, FOCUS-2 answer 3).

Measured at `25203b8` (Year spec §0): 2,082 accounts, 1,897 passages, 185 single verses, 223 events with more than one, at most 8 on one event.

- [ ] **Step 1 (red):** the Accounts laws of Part 3; `references.rs`: `an_opening_is_cut_at_its_word_limit`; real data: `the_sermon_on_the_mount_is_narrated_in_its_curated_accounts`, `peters_denial_in_mark_is_two_accounts` (`MRK.14.54`, `MRK.14.66-72`), `every_attestation_run_is_one_account`, `a_one_verse_account_mints_no_passage`, `a_cited_span_and_an_account_over_one_verse_set_are_one_passage`; `grounds_of` gains the `Narration` arm (FOCUS-4's law walks it).
- [ ] **Step 2:** Record in the ledger: accounts (passages / single verses), witnesses split into several accounts (with their pieces), passages shared with FOCUS-3's cited spans (their mark raised).
- [ ] **Step 3:** Implement. **Step 4 (`contract`):** rebuild, regen, re-bless. **Step 5 (`heavy`):** gates. **Commit:** `accounts: each account of an event is one unbroken run of verses, a FOCUS-3 passage marked as recording history or a single verse, that narrates it, labelled by its code with its opening words; a story told in pieces is several accounts (F3 Q5, F5 Q6-Q8; AQC/AGC minor)`.

### Task 7: "At the same time": compiled concurrency, loaded whole (FOCUS-5)

**Why edges now (Year spec §2.5, §4):** concurrency is genuinely simultaneous events, not same-year: certain overlap at the finest known granularity (a day at most) or a curated parallel. The data check found a median of 2 events per dated year and only Jesus's ministry years above 20, so same-year was never the right set; certain overlap gives 0 pairs today and ≈ 15 after Task 3b. So the set is compiled as edges and read whole, with a gate that keeps it short. The earlier window-read design for this section ("the event's own window") is withdrawn; the window read builds with the Year (TIME).

**Files:** `graph-types/src/{edge,graph,sections}.rs` (`Concurrency`, `ConcurrencyGround`, symmetric `ConcurrentWith`; symmetric 7 → 8), canon, common; `server/atlas-graph/src/{event_world,labels,law_check,provenance}.rs` (certain-overlap pairs from the resolved spans, curated pairs from `parallels.toml`, `CONCURRENT_WHOLE` gate, disjointness from `≺`); sqlite (`concurrency` table); `server/atlas-contract/tests/graph_api.rs`; `data/compiled`; contracts, CHANGELOG (AQC minor), AGC (minor); pacts; OPEN 1(a) C# regen and one arm.

- [ ] **Step 1 (red):** the Concurrency laws of Part 3 over generated positions and parallels; real data: `the_burial_and_pilates_second_hearing_are_at_the_same_time` (both on "the preparation", if Task 3b curated them); `two_events_known_only_to_one_year_are_not_at_the_same_time`; `every_concurrent_set_loads_whole`; `a_curated_parallel_against_a_verse_fails_the_compile`.
- [ ] **Step 2:** Implement: pairs from a sweep over events sorted by `start.latest` (each event compared only with those whose `end.earliest` is not yet passed: `O(n log n + pairs)`), plus curated rows; stored once per unordered pair.
- [ ] **Step 3 (`contract`, `relations!` last):** rebuild, regen, re-bless. **Step 4 (`heavy`):** gates. **Commit:** `concurrency: "At the same time" is the compiled concurrent-with group, from spans that certainly overlap to the day or a curated parallel with its verses; same-year is not concurrency; every set is served whole and gated at 20 (F5 Q3, F# Q6; AQC/AGC minor)`.

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
- `event_from_node`'s `located-at` drain from the record (`event_detail` reads the payload and `chrono.resolved` only);
- `Year.value` on the wire (no F# reader; F# domain Q1: the client does no date arithmetic), with a contract law that no wire type carries a year integer.

Gate: `retirement.md` is empty; `no_legacy_event_reads.rs` and `contract_coverage.rs` assert each removal.

### Task Z: Gates, mutation, close

- [ ] Mutation, in the owner's window only (`heavy` with "mutation", `free -g` ≥ 18): `bash scripts/mutants-parallel.sh -n 3 -b <base>` → 100% or equivalents recorded (covers `grounds_of`, `ParentageWording`, the eternity clear, the label functions, `populate_chronological_succession`, the story lowering, the account minting, `opening_words`, `start_year`, position resolution and meet, `relation`, the keyed linear extension, the concurrency sweep). Outside the window, `.superpowers/MUTATION-GATE-DEBT.md` names `<base>`.
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
| Concurrency derived on the client, or read as same-year | compiled `concurrent-with` from certain overlap and curated parallels | concurrency laws (`same_year_is_not_concurrency`); `CONCURRENT_WHOLE` gate |
| An unknown time read as earliest (`temporal_order`) | `≺` partial order; chain as its keyed linear extension | `unknown_is_never_earlier`; `the_chain_is_a_linear_extension_of_what_is_known` |
| A finer date asserted without a verse | `PositionRow` carries a `Justification`; Theographic's Gregorian dates discarded | `every_position_rests_on_a_verse`; `no_theographic_month_or_day_reaches_a_position` |
| A raw date spelled two ways read as two | one padding-blind door over a closed shape set | Task 3a's laws; source law on `startDate` |
| An account spanning a gap | FOCUS-3 v2's single-run `Passage` and `target(span)` | `an_account_is_one_unbroken_run` |
| A map row compared by a composed id | `SceneArrow.from_node`/`to_node` | real-data law |

## Wave schedule

| Wave | Primary (Rust) | Beside it | Critical sections |
|---|---|---|---|
| 0 | Task 0; OPEN 1 | — | — |
| 1 | Task 1 | Task 3 tests written | `contract` rebuild #1, regen |
| 2 | Task 2 | Task 3; Task 3a (ETL only); Task 3b Step 1 curation (data, no build) | rebuilds #2, #3 |
| 3 | Task 3b | Task 4 tests written | rebuild #3b, `relations!` |
| 4 | Task 4 | Task 6 tests written | rebuild #4, `relations!` |
| 5 | Task 5 | Task 8 | rebuild #5, `relations!`; regen |
| 6 | Task 6 | Task 7 tests written | rebuild #6, `relations!` |
| 7 | Task 7 | — | rebuild #7, `relations!`, regen |
| 8 | Task Z | — | `heavy` |
| later | Task R (after the F# client) | — | one major |

Tasks 1–3 are FOCUS-4's and F-36; 3a is ETL hygiene; 3b–8 are FOCUS-5's. Rust is the only build in every wave; the frozen-client gate's C# run is the unlike work beside it (rule 23). Build nothing beyond what a task's gate needs.

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
7. Event card: `When` (linked as item 2) with the served `position` label under it (its festival a link), `Superscription`, `Source`; no bookkeeping fields.
8. Arrows: `follows` (‹ previous) and `precedes` (next ›) only, each with its edge step.
9. **Stories** section: one row per `in-story` entry, `‹ previous · story · next ›`, each a link (from `EdgeEntry.step`).
10. **At the same time** section: the `concurrent-with` group, read whole in one neighbour read (no paging); absent when empty and for an undated event.
11. Accounts: the `narrated-in` group, each entry `reference — opening words…` from the served label and `opening`; following one opens the passage (or the verse, for a one-verse account).
11a. Festival: label, calendar line, `instituted-in`, `festival-events` in served order.
12. `attested-in` collapsed and last.
13. Event map view: the World framed on the event's `when`, its `in-story` stories highlighted by `SceneArrow.from_node`/`to_node`; leaving the event clears the highlight.
14. A pericope heading opens its served event.

**Year (TIME, after this plan):** the Year spec §6.

**Re-expressed specs:** `person-card`, `reader-persons`, `event-timeline`, `event-timeplace`, `popover-sections`, `accounts-and-mentions`, `provenance`, `reader-headings`, `world-pin`, `world-narrative-focus`, `w1`–`w5`, rewritten against the F# client's ids when its views exist (the old plans' Task 5/Task 6 lists are the inventory).

## FINDINGS this plan expects to raise

- `TemporalAdjacency` rows are read twice (symmetric and `Precedes`) between Task 4 and Task R: D.R.Y. debt, held on `retirement.md`.
- `parallel` is a declared symmetric relation with no row family (rule 4). (Not reused for concurrency: it means parallel accounts.)
- `TimePoint.month: Option<u8>` names no calendar and sorts unknown first (Year spec FINDINGS); closed by Task 3b.
- `parse_theo_year` drops unreadable events silently (Year spec FINDINGS); closed by Task 3a.
- Event part-of is not modelled (Theographic `partOf`, 201 events): a broad event and its parts become concurrent once both carry positions (Year spec FINDINGS).
- `TimeRange::undated()` writes the atlas span in code (rule 26; the Year spec).
- `also_called` serves search names, not names (A-NAMES).
- A person's `description` (Easton, with site-relative links) is served and shown nowhere; also under A-THEO's licence question.
- Account notes are curator text in `Narration.justification` (F-50's category; not served).
