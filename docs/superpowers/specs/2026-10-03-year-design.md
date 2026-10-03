# The Year: design

**Status:** draft for the owner's sign-off (PRINCIPLES 12). Planning only; no code changes with this document.
**Queue:** item TIME (owner, 2026-10-03: "Year: spec it now, build after FOCUS-5").
**Base read:** `origin/worktree-bible-atlas-m1` at `25203b8`; the FOCUS-4 plan (`lane/claude/F4-plan` `da92793`) and the FOCUS-5 plan (`lane/claude/F5-plan` `564f4ca`), as rewritten by `docs/superpowers/plans/2026-10-03-focus45-server-data.md` (this branch).
**Binding rulings:** the "OWNER ANSWERS, 2026-10-03" block in `QUEUE.md` (ops), in particular F4 Q5, F5 Q2, F5 Q3, F5 Q4, and the C# client freeze.

## 0. What the owner asked for

> "Years are explorable and you can get to the map from there but the primary thing you should get is the set of events in chronological order that happened, active prophets, etc. all explorable."
>
> "the unit we care about fundamentally is an individual year. Year range is a composition over year."
>
> "Stories are different from objective chronology. Chronologically every event has only forward or back or is in parallel with another set of events." ... "Events need to get a concurrent events section (user-friendly name) that just lists the events."
>
> "if we can agree on the domain, data structure, and algebrae then we should be okay."

So:
- a **Year** is a node of the graph and the atomic unit of time;
- a **span** (a life, a reign, an event's date, an era's window, a range the reader picks) is a composition of years, never a node of its own;
- reading a year (or a span) gives, first, its events in chronological order; then the people alive, the kings reigning and the prophets active; then the eras, maps and polities covering it; the World is one step away;
- chronology (one previous, one next, a concurrent set) is separate from stories.

This document has three parts, as the owner asked: §1 the domain types, §2 the data structures with their invariants and bounds, §3 the algebras with their laws stated as the properties the tests check. §4 decides what is compiled and what is read per request, with sizes. §5 is the contract. §6 is the F# client backlog. §7 holds the owner's questions.

## What exists today (read-only survey at `25203b8`)

| Thing | Where | What it is |
|---|---|---|
| `Year(i32)` | `graph-types/src/chrono.rs` | signed year, constructor refuses 0. No successor, no ordinal. |
| `TimePoint`, `ResolvedDate`, `ResolvedPlacement`, `temporal_order` | same | an event's resolved date (year, optional month and day) and its total order key (date, then the curated sequence key `SeqKey`). |
| `DatedBy`, `DatePlacement`, `PlacementBasis` | same | how an event is dated: an anchor, a reign year, after a prior event, or an era; basis Textual or Traditional. |
| `atlas_core::time::TimeRange` (`YearSpan` in the schema) | `server/atlas-core/src/time.rs` | `{from_year, to_year}` with no zero and no inversion; `next_year` skips 0; `undated()` hard-codes -4004..100 (a rule-26 offender, FINDING below). |
| `atlas_core::label::{Year, TimeRange}` | `server/atlas-core/src/label.rs` | the wire forms with their labels, formatted **per request** (F-36). `DateClaim` adds `c. ` when a note qualifies the claim. |
| `NodePayload::{Era, Map}` | `graph-types/src/node.rs` | `from_year`, `to_year` on the payload. `Polity` carries per-era `from_year`, `to_year`. |
| `PersonLife` | `server/atlas-contract/src/wire/graph.rs` | `birth`, `death`, `first`, `last` (`first`/`last` are the span the person is **mentioned** across, not a life). |
| `EraDetail`, `MapDetail`, `PolityDetail` | same | `window` / `reign` as a labelled `TimeRange` (F-47: two names for one shape; F-48: "reign" means two things). |
| `temporal_adjacency` rows | Core section | 911 rows `earlier_id → later_id` lowered to a direction-less symmetric kind; the server scans `chrono.order` per request to tell back from forward (`temporal_neighbors_of`). |
| `succession_step` | Core section | 13 narratives, 255 steps `(narrative, ord, event)`; lowered to event→event `follows-in` edges carrying `EdgeMeta::Narrative`. |
| Client `YearNode` | `client/Legacy/YearNode.cs` | the "year" popover: one `/api/scene` read, then the client de-duplicates and sorts the events (rule-25 offender). Its identity is an **event** (F-57). |

Measured on the artifact (core section, read-only):

| Quantity | Count |
|---|---|
| Atlas span (the eras tile it exactly: Primeval 4004 BC … Early Church AD 100) | **4,104 years** |
| Events | 1,711, of which **912 dated**; every dated event is one year today; 294 distinct years hold an event; the fullest year holds 98 (AD 33) |
| Persons | 3,058: **46** with a recorded birth and death (14,075 person-years), 2,707 with only a mention span (245,330 person-years), 2 eternal |
| Eras / maps | 10 / one map per era window (4,104 year-links each) |
| Polity eras | 25 (14,867 polity-years) |
| Kings, judges, prophets with terms | **none in any source we hold** (Theographic has no office or reign field) |

---

## 1. Domain types

All in `graph-types` unless noted. Rust, for sign-off.

### 1.1 Year

```rust
pub struct Year(i32);

impl Year {
    pub fn new(y: i32) -> Result<Year, YearError>;
    pub fn get(self) -> i32;
    pub fn era(self) -> Era;
    pub fn succ(self) -> Year;
    pub fn pred(self) -> Year;
    pub fn ord(self, scale: &AtlasSpan) -> Option<YearOrd>;
}

pub enum Era { BeforeChrist, AnnoDomini }

pub struct YearOrd(u16);

pub struct AtlasSpan { first: Year, last: Year }
```

- `Year` exists; it gains `era`, `succ`, `pred` and `ord`. `succ(Year(-1)) == Year(1)` and `pred(Year(1)) == Year(-1)`: there is no year 0 (BC 1 is followed by AD 1).
- `YearOrd` is the year's position on the atlas's scale, counted from 0 at its first year. It turns "no year 0" into plain integer arithmetic, so every span computation is done on ordinals and never special-cases zero twice.
- `AtlasSpan` is **data**: the compiler reads it as the hull of the eras in `data/curated/eras.toml` (4004 BC – AD 100 today). No year bound is written in code (rule 26). `TimeRange::undated()`'s literal is a FINDING.

### 1.2 Precision ("c.")

```rust
pub enum Precision { Exact, Circa }
```

- The **year** is never approximate: a Year node is a unit. **A dating** is approximate. `Circa` marks a dating whose basis is `Traditional`, a `DateClaim` with a qualifying note, or a Theographic life year (Theographic gives no grounds for them). `Exact` marks a dating Scripture fixes (`PlacementBasis::Textual`).
- The compiled label of a circa dating is `c. 1446 BC` / `c. 1450 – 1400 BC` (one rule, `DateClaim`'s today, now compiled: F-36).

### 1.3 YearSpan: the composition of years

```rust
pub struct YearSpan { from: Year, to: Year }

impl YearSpan {
    pub fn new(from: Year, to: Year) -> Result<YearSpan, SpanError>;
    pub fn point(y: Year) -> YearSpan;
    pub fn from(self) -> Year;
    pub fn to(self) -> Year;
    pub fn years(self) -> impl Iterator<Item = Year>;
    pub fn len(self) -> u32;
    pub fn contains(self, y: Year) -> bool;
    pub fn within(self, outer: YearSpan) -> bool;
    pub fn overlaps(self, other: YearSpan) -> bool;
    pub fn meet(self, other: YearSpan) -> Option<YearSpan>;
    pub fn hull(self, other: YearSpan) -> YearSpan;
    pub fn adjacent(self, next: YearSpan) -> bool;
}

pub enum SpanError { Inverted }

pub struct Dating { pub span: YearSpan, pub precision: Precision }
```

- Fields private; the only constructor refuses `from > to`. Year 0 is unrepresentable because `Year` refuses it.
- One type for every span in the system: an event's date (`ResolvedDate` projected to years), a life (`birth..death`), an office term, an era's or map's window, a polity era, a book's writing, and a range the reader picks. `atlas_core::time::TimeRange` becomes this type's server alias and then goes (it duplicates it, 14b), and F-47/F-48 close with it: an era, a map and a polity era all have a `window: YearSpan`.

### 1.4 The year's occupants: facets

```rust
pub enum Facet { Event, Alive, Office, Era, Map, Polity, Written }
```

What a year links to, as a closed set. A `match` over `Facet` has no wildcard arm anywhere (24b).

| Facet | Occupant | Its span | Source |
|---|---|---|---|
| `Event` | an event | its resolved date in years | dated-by (chronology) |
| `Alive` | a person | `birth..death` | Theographic, people-eternal (eternal persons have none) |
| `Office` | a person, in an office | the term | **new** `data/curated/offices.toml` (§7 Q2) |
| `Era` | an era | its window | eras.toml |
| `Map` | a map | its window | map adapter |
| `Polity` | a polity, in one of its eras | the polity era's years | polities/*.toml |
| `Written` | a Bible book | when it was written | books.toml (`BookDetail.written`) |

A person's **mention span** (`first..last`) is not a facet: it is the span of verses that mention them, not a time they lived (§7 Q1). People who **took part** in an event are not a facet either: they are one step away, through the event's `participants` (FOCUS-2 ruling 2: a view one step away is not built).

### 1.5 Office

```rust
pub enum Office { King, Judge, Prophet }

pub struct OfficeTerm {
    pub person: PersonId,
    pub office: Office,
    pub realm: Option<PolityId>,
    pub term: YearSpan,
    pub basis: PlacementBasis,
    pub justification: Justification,
    pub provenance: ProvenanceId,
}
```

- A row family (`RowFamily::OfficeTerm`), not an edge: it is a fact about a person, like a life. `realm` is "King **of Judah**"; a prophet has none.
- `grounds_of` (FOCUS-4) gains its arm: it records a `Justification`, so its grounds are `justified-by` edges from the person (FOCUS-4's eternity precedent).
- Only the three offices the owner named exist (rule 4). A fourth (high priest) is one enum member and one data file section later.

### 1.6 Chronology and stories (shared with FOCUS-5)

```rust
pub struct ChronologicalSuccession { pub earlier: EventId, pub later: EventId, pub provenance: ProvenanceId }
pub struct YearSuccession { pub earlier: YearId, pub later: YearId }
pub struct StoryStep { pub story: NarrativeId, pub ord: u16, pub event: EventId, pub provenance: ProvenanceId }
```

- Both successions lower into **one new directed relation**, `Precedes => "precedes" / "follows"`: chronology over time positions (years and events). Story order is **not** this relation (F5 Q3).
- `StoryStep` is today's `succession_step` row, now lowered into the story membership relation `InStory => "in-story" / "story-steps"` (event → story), whose edge carries the step's place: its `ord`, and the story's previous and next event. The event-to-event `follows-in` edges carrying `EdgeMeta::Narrative` retire (the plan, Task F5-2 adds `in-story`; Task R removes the old edges).
- `temporal-adjacency` retires (F5 Q2). Both removals happen at the C# client's retirement (the plan's freeze rule, Task R), because the frozen client reads them.

---

## 2. Data structures, invariants and bounds

### 2.1 Year nodes

| | |
|---|---|
| Node | `NodePayload::Year { year: Year }`, id `Year:{signed value}` (`Year:-1446`, `Year:30`), kind `NodeKind::Year`, provenance the eras file |
| Label | compiled: `1446 BC`, `AD 30` (the rule `label.rs` holds today, moved to the compiler) |
| Count | one per year of `AtlasSpan`: **4,104**; at 10× size (27f), a longer span, still under 10^5 |
| Invariants | I-Y1: the set of Year nodes is exactly `AtlasSpan.years()`; I-Y2: no `Year:0`; I-Y3: every id resolves (a dated element outside the span fails the compile) |

### 2.2 Year chronology

| | |
|---|---|
| Rows | `YearSuccession`, one per consecutive pair: 4,103 |
| Edges | `Year:-1 —precedes→ Year:1` and so on; neighbours `follows` / `precedes` |
| Invariants | I-YC1: exactly one `precedes` neighbour for every year but the last, one `follows` for every year but the first; I-YC2: `Year:-1`'s next is `Year:1` |
| Why edges | the reader's arrows read the graph (rule 25: the client never computes `y + 1`, which is wrong at the zero gap anyway) |

### 2.3 YearSpan on the wire and in the artifact

- Stored on its owner (an event's `event_date` row, a person payload, an era payload, an office term row) as two years; never exploded into per-year **edges**.
- Served as `TimeRange { from: Year, to: Year, label, circa }`, where each `Year` carries the Year node's `NodeRef` (§5), so a reader can step from a span's end into a year without composing an id (F-31's category).

### 2.4 The year cover index (compiled)

Two narrow tables in the Core section, written by the compiler from the spans of §1.4:

```rust
pub struct YearCover { pub facet: Facet, pub year: YearOrd, pub key: OrderKey, pub element: AnyNodeId }
pub struct SpanStart { pub facet: Facet, pub from: YearOrd, pub key: OrderKey, pub element: AnyNodeId }
pub struct OrderKey(u32);
pub struct FacetCount { pub year: YearOrd, pub facet: Facet, pub count: u32 }
```

- `YearCover`: one row for every year of every occupant's span (the stabbing index). Primary key `(facet, year, key, element)`.
- `SpanStart`: one row per occupant, at its first year. Primary key `(facet, from, key, element)`.
- `OrderKey`: the facet's reading order, compiled. `Event`: the event's position in the chronology (§2.5), so a year lists its events in chronological order. Every other facet: the span's `(from, to)` ordinals, then the occupant's compiled label.
- `FacetCount`: the count per (year, facet), what a year's record shows next to each list.
- Invariants: I-C1 every dated element's span lies within `AtlasSpan`; I-C2 `YearCover` holds exactly `Σ len(span)` rows per facet; I-C3 `SpanStart` exactly one row per occupant; I-C4 `FacetCount` equals the `YearCover` row count per (year, facet).
- Bounds (today → 10×): see §4.

### 2.5 The chronology (previous, next, concurrent)

- **Order.** The dated events in `temporal_order` (the resolved date, then the curated `SeqKey`): a strict total order today (912 distinct keys). It is the existing `chrono.order`, computed once by the compiler.
- **Previous and next.** One `ChronologicalSuccession` row per consecutive pair (911 today), lowered into `precedes`/`follows`. An event's `follows` neighbour is its previous; its `precedes` neighbour is its next.
- **Concurrent set ("At the same time").** The dated events whose span overlaps this event's span, this event excluded. It is not stored as edges (16,836 ordered pairs today, growing with the square of a busy year's count); it is the event facet of the window read (§2.6) over the event's own span. One read, keyset paged, chronological.
- Invariants: I-CH1 the `precedes` edges form one chain through every dated event; I-CH2 an undated event has no chronology edge and no concurrent set; I-CH3 the chain is consistent with dates: an event whose span ends before another's begins comes before it.

### 2.6 The window read (per request, bounded)

A window is a `YearSpan` (a single year, or a range the reader picks: "Year range is a composition over year"). For one facet it answers the occupants whose span overlaps the window, in the facet's order, a page at a time:

1. **Phase Covers:** the `YearCover` rows at `(facet, ord(window.from))`, in key order: everything already under way in the window's first year.
2. **Phase Starts:** the `SpanStart` rows with `from` in `(ord(window.from), ord(window.to)]`, in `(from, key)` order: everything that begins later inside the window.

Both phases are index range scans with no filter and no duplicate (an occupant covering the first year started at or before it, so it is never in phase 2), so a page costs one seek plus the rows it returns (27b). The cursor is `(phase, from, key, element)`, keyset (never offset). Events come out in chronological order across the phase boundary, because a phase-1 event starts no later than the window's first year and every phase-2 event starts after it.

### 2.7 A story's sequence

| | |
|---|---|
| Rows | `StoryStep (story, ord, event)`: 255 today in 13 stories |
| Edges | one `in-story` edge per step, event → story; `EdgeMeta::Step { ord, previous: Option<EventId>, next: Option<EventId> }` |
| Invariants | I-S1 per story, `ord` is `0..n` with no gap; I-S2 a step's `previous`/`next` are the steps at `ord - 1` / `ord + 1`; I-S3 an event may be in several stories and several times in one story (a revisit) |
| Order | a story's `story-steps` page is in `ord` order ("narratives contain their events in order", F8 Q3) |
| Independence | stories are compiled from `data/curated/narratives` only; chronology from dated-by only (law T2) |

### 2.8 An account passage (shared with FOCUS-5)

| | |
|---|---|
| Node | a passage `Container` (FOCUS-3's), identity = its verse set (`passage_container_id(&runs)`) |
| Runs | maximal contiguous verse runs in canon order (a run may cross a chapter); an account recorded by one book in two places is one passage of two runs (`MRK.14.54, 66-72`) |
| Edge | `Narration` row, lowered into `Narrates => "narrates" / "narrated-in"` (passage → event): the passage is marked as recording history by this edge |
| Label | the run reference as a code (`MAT.5.1-7.29`, `MRK.14.54, 66-72`, `GEN.29.32-30.24`) |
| Opening | the first words of its first verse in the canonical layer, compiled, at most `OPENING_WORDS` (a named constant, 8) words, then `…` |
| Invariants | I-A1 an event's accounts partition its attested verses; I-A2 runs are maximal (no two of one passage are canon-adjacent) and sorted; I-A3 a passage that is both cited and an account is one node |

---

## 3. Algebras and their laws

Every law below is a property test (proptest for the pure types in `graph-types`; a real-data law over the artifact where named). The names are the test names.

### 3.1 Year

| Operation | Law (test) |
|---|---|
| `Year::new` | `no_year_is_zero`: `new(0)` is `Err`; `new(y)` is `Ok` for every `y ≠ 0`. |
| `succ`, `pred` | `succ_and_pred_are_inverse`: `pred(succ(y)) == y` and `succ(pred(y)) == y`. `the_year_after_1_bc_is_ad_1`: `succ(Year(-1)) == Year(1)`. `succ_is_strictly_increasing`: `succ(y) > y`. |
| `ord` | `ord_is_an_order_isomorphism`: for `y, z` in the span, `y < z ⟺ ord(y) < ord(z)`, and `ord(succ(y)) == ord(y) + 1`; `ord` of the first year is 0 and of the last is `len(AtlasSpan) - 1`. |
| label | `a_years_label_names_its_era_once`: `label(y) == "{-y} BC"` for `y < 0`, `"AD {y}"` for `y > 0`; `labels_are_injective`. |

### 3.2 YearSpan (composition, containment, overlap)

| Operation | Law (test) |
|---|---|
| `new` | `a_span_is_never_inverted`: `new(a, b)` is `Ok ⟺ a ≤ b`. |
| `years` | `a_span_is_the_union_of_its_years`: `years(s)` is `from, succ(from), …, to`, strictly increasing, contains no 0, and `len(s) == ord(to) - ord(from) + 1`. `a_span_is_the_hull_of_its_points`: folding `hull` over `point(y)` for `y ∈ years(s)` gives `s`. |
| `contains` | `contains_is_membership`: `contains(s, y) ⟺ y ∈ years(s) ⟺ from ≤ y ≤ to`. |
| `within` | `within_is_subset`: `within(s, t) ⟺ years(s) ⊆ years(t)`; reflexive, antisymmetric, transitive. |
| `overlaps` | `overlap_is_shared_years`: `overlaps(s, t) ⟺ years(s) ∩ years(t) ≠ ∅`; `overlap_is_symmetric`; `overlap_is_reflexive`. |
| `meet` | `meet_is_intersection`: `meet(s, t)` is `Some(m) ⟺ overlaps(s, t)`, and then `years(m) == years(s) ∩ years(t)`; commutative, associative. |
| `hull` | `hull_is_a_semilattice`: commutative, associative, idempotent; `years(s) ∪ years(t) ⊆ years(hull(s, t))`, with equality `⟺ overlaps(s, t) ∨ adjacent(s, t) ∨ adjacent(t, s)`. |
| `adjacent` | `adjacent_spans_have_no_gap`: `adjacent(s, t) ⟺ succ(s.to) == t.from`; `spans_meet_across_the_zero_gap`: `adjacent(new(-5,-1), new(1,3))`. |
| eras | `the_eras_tile_the_atlas_span` (real data): the era windows, sorted, are pairwise adjacent and their hull is `AtlasSpan`. |

### 3.3 The window read

| Operation | Law (test) |
|---|---|
| `window(w, facet)` | `a_window_is_everything_its_span_overlaps`: over generated occupants, the set answered equals the brute-force `{o | overlaps(span(o), w)}`. |
| phases | `covers_and_starts_are_disjoint_and_complete`: `window(w) == covers(w.from) ⊎ starts_in(succ(w.from)..=w.to)`, with no element in both. |
| composition | `a_range_is_the_union_of_its_years`: as sets, `window(w) == ⋃_{y ∈ years(w)} window(point(y))`. `a_point_window_is_the_years_cover`: `window(point(y)) == covers(y)`. |
| order | `a_window_reads_in_facet_order`: the concatenation of all pages is sorted by `OrderKey` within each phase and, for `Event`, chronological across both. |
| paging | `paging_loses_and_repeats_nothing`: concatenating every page (any `limit` up to the server cap) equals the whole answer, with no duplicate; `previous` of page `n+1` reopens page `n` (F-74's rule: null only on the first page). |
| cost | `a_window_page_scans_only_what_it_returns` (VM-step law, F-69's method): SQLite steps per page grow with the page size, not with the facet's row count, at 1× and 10×. |
| counts | `a_years_counts_are_its_window_sizes` (real data): `FacetCount(y, f) == |window(point(y), f)|`. |

### 3.4 Chronology

| Operation | Law (test) |
|---|---|
| `next`, `previous` | `previous_and_next_are_inverses`: `previous(next(e)) == e` wherever `next(e)` exists, and `next(previous(e)) == e` wherever `previous(e)` exists. Same law over years. |
| chain | `the_chronology_is_one_chain`: from the one event with no previous, `next` visits every dated event exactly once and ends at the one with no next. `an_undated_event_has_no_chronology`. |
| dates | `the_chronology_respects_dates`: if `span(a).to < span(b).from` then `a` comes before `b` in the chain. |
| concurrency | `concurrency_is_overlap`: `concurrent(a, b) ⟺ a ≠ b ∧ overlaps(span(a), span(b))`. `concurrency_is_symmetric`; `concurrency_is_irreflexive`. `the_concurrent_set_is_the_window_less_itself`: `concurrent(e) == window(span(e), Event) \ {e}`. |
| trichotomy | `two_events_are_before_after_or_at_the_same_time`: for dated `a ≠ b` with disjoint spans, exactly one of "a before b", "b before a" holds and they are not concurrent; with overlapping spans they are concurrent (the chain still orders them by the curated sequence, §7 Q3). |
| years | `the_year_after_1_bc_is_ad_1_in_the_graph` (real data): `Year:-1`'s `precedes` neighbour is `Year:1`. |

### 3.5 Stories

| Operation | Law (test) |
|---|---|
| steps | `a_storys_steps_are_numbered_without_gaps`; `a_steps_previous_and_next_are_its_neighbours_in_the_story`; within one story, `previous` and `next` are inverses. |
| order | `a_story_lists_its_events_in_its_own_order` (real data): a story's `story-steps` page is in `ord` order. |
| independence | `stories_are_independent_of_chronology`: compiling with every narrative file removed, or with the steps of any story permuted, leaves every `precedes` edge unchanged; changing any dated-by placement leaves every `in-story` edge unchanged. No law ties a story's order to time (a story may look back). |

### 3.6 Accounts

| Operation | Law (test) |
|---|---|
| partition | `an_events_accounts_partition_its_attestation`: the union of the verses of an event's `narrated-in` passages equals its `attested-in` verses, and no verse is in two of its accounts. |
| runs | `contiguous_verses_are_one_run`: runs are sorted and no two runs of one passage are canon-adjacent; `a_run_may_cross_a_chapter`. |
| identity | `one_verse_set_is_one_passage`: an account and a citation over the same verses are one node. |
| history | `a_passage_records_history_iff_it_narrates`: a passage has the mark exactly when it has a `narrates` edge. |
| opening | `an_opening_is_the_first_words_of_the_first_verse`: a prefix of the canonical text, at most `OPENING_WORDS` words, with `…` exactly when cut. |

### 3.7 Parentage (shared with FOCUS-4)

| Operation | Law (test) |
|---|---|
| `wording: Parentage → String` | `parentage_wording_is_total`: `data/curated/parentage.toml` holds exactly one wording for each member of `Parentage::ALL` (`Natural`, `Eternal`, `Virgin`, `Legal`, `Created`); a missing, extra or duplicate entry fails the compile. |
| edge label | `a_parent_of_edge_reads_its_wording`: the edge's compiled label is `{parent} · wording(p) · {child}`, the same edge whichever end it is read from; the wording appears nowhere but the edge. |

---

## 4. Compiled versus read per request (rule 27)

**Decision: compile the Year nodes, the year chronology, the year and span labels, the cover index and the counts; answer what a year (or a span) holds with one bounded indexed read, the window read. Do not compile a year's occupants as edges.**

| Option | What it adds to the artifact | Problem |
|---|---|---|
| (a) Every occupancy an edge (`Year —holds→ X`) | ≈ 42,000 edges today, each in `edge_index` twice and with a compiled edge label (`{X} · in · 1446 BC`): ≈ 84,000 index rows (+50% on today's 163,048) and ≈ 1.7 MB of labels, uncompressed; 245,000 more if mention spans counted | F-40: labels per position are what is pushing the blobs over GitHub's limits. And a range ("1450 – 1400 BC") would need one neighbour read per year (51 requests), breaking 27c. |
| (b) Nothing compiled: filter every span per request | none | A stabbing query over unindexed spans scans; breaks 27b; the server derives (27). |
| **(c) Compiled cover index + window read** | Year nodes 4,104 (labels ≈ 50 KB); year chronology 4,103 edges (≈ 125 KB of labels); `YearCover` ≈ 42,000 rows (event 912, alive 14,075, era 4,104, map 4,104, polity 14,867, office ≤ 3,000 est., written ≤ 1,000 est.) at ≈ 12 bytes plus its key index: **≈ 1 MB uncompressed, ≈ 0.25 MB compressed**; `SpanStart` ≈ 1,200 rows; `FacetCount` ≈ 29,000 small rows | none: index-bounded, a range is one read per facet, no edge labels for occupancy |

At ten times today's size (27f) option (c) is ≈ 10 MB uncompressed in the Core section (12.7 MB compressed today), far from the 50 MB advisory that `kjv` and `lexicon` already exceed; option (a) would add ≈ 17 MB of edge labels alone. Mention spans, if the owner wanted them (§7 Q1), would add 245,330 cover rows (≈ 3 MB uncompressed) under (c), and are not recommended.

Who derives what:
- **Compiler:** Year nodes and labels; year succession; event chronology (`precedes`); concurrent sets are *not* compiled; the cover index, span starts and counts; every year and span label with its `c.` (F-36); story membership with each step's previous and next; account passages, their runs, labels and openings.
- **Server:** the element read (a Year's record), the neighbour read (`precedes`/`follows`, `in-story`, `narrated-in`), the window read. Nothing else.
- **Client (F#):** the Year presentation, the range presentation (a window the reader picks between two served years), "At the same time", the Stories section, the map one step away.

---

## 5. The contract

All additive (AQC minor) while the C# client is frozen; nothing the C# client reads changes shape (the plan's freeze rule).

```rust
pub struct Year { pub value: i32, pub label: String, pub node: NodeRef }

pub struct TimeRange { pub from: Year, pub to: Year, pub label: String, pub circa: bool }

pub struct YearDetail { pub year: Year, pub counts: Vec<FacetCount> }

pub struct FacetCount { pub facet: Facet, pub count: u32 }

pub enum Facet { Event, Alive, Office, Era, Map, Polity, Written }

pub struct OfficeTermDetail { pub office: Office, pub realm: Option<NodeRef>, pub term: TimeRange }

pub struct WindowPage {
    pub window: TimeRange,
    pub facet: Facet,
    pub entries: Vec<WindowEntry>,
    pub next: Option<WindowCursor>,
    pub previous: Option<WindowCursor>,
    pub version: String,
}

pub struct WindowEntry { pub node: NodeRef, pub when: TimeRange, pub office: Option<OfficeTermDetail> }

pub struct WindowCursor(String);
```

- `Year.node` and `TimeRange.circa` are new fields on existing wire types (labels compiled, F-36). `NodeRecord` gains `year: Option<YearDetail>`; `PersonLife` gains `offices: Vec<OfficeTermDetail>`.
- **Route:** `GET /api/window?from={Year id}&to={Year id}&facet={facet}&cursor=&limit=`. The ends are served Year ids (the client passes what it was given, never a computed year). This is 27a's generic "range read by time window", not a view route: it serves every facet, for any window, in one shape. `WindowCursor` is the third named cursor type (O-WIRE-IDENTITIES O2 precedent).
- **Vocabulary:** `NodeKind::Year`; `Precedes` ("precedes" / "follows"); `InStory` ("in-story" / "story-steps"); `Narrates` ("narrates" / "narrated-in", FOCUS-5). Display labels are derived from the wire names until O-LABELS is ruled.
- `/api/scene`'s time read stays for the map (MAPS, F-34); the World's year view uses `/api/scene` and the window read side by side until MAPS.

---

## 6. F# client backlog (the C# client is frozen)

The C# client gets nothing from this design. In the F# client, after the F# style sign-off and after the server work lands:

1. **Year presentation** (`Presentation.Of(Year, Popover)`): the year's label; arrows `‹ 1447 BC` / `1445 BC ›` from its `follows`/`precedes`; then the facets in this order, each a list paged 20 at a time through the one paging door, heading `{facet label} ({count})`: **Events** (chronological), **Alive**, **Kings and judges**, **Prophets** (the office facet, split by the served `office`), **Eras**, **Maps**, **Polities**, **Written**. Facet headings are client presentation words over a closed served enum.
2. **Map one step away:** `Presentation.Of(Year, World)` = `Geography(Frame.Bounded(year))`; the year's "Show on the map" chip.
3. **Range presentation:** a span is a client explorable with identity `(from Year id, to Year id)`, made from any served `TimeRange` (a life, a reign, an event's date) or by the reader picking two years; it reads the same facets through the window read, with no counts (More/Less only). Saved explorations store the two ids.
4. **Years are explorable everywhere:** every served `Year` and `TimeRange` field on a card (Born, Died, When, a window, a reign, a term) is a link: a single year opens its Year; a span opens the range.
5. **Event "At the same time":** the window read over the event's own `when`, facet Event, omitting the current event; hidden for an undated event.
6. **Event chronology arrows:** `follows` (previous) and `precedes` (next) only; **Stories** section: one row per `in-story` entry, `‹ previous · story · next ›`, each a link.
7. **Event card:** When (linked, item 4), Superscription, Source; no bookkeeping.
8. **Accounts:** `narrated-in` entries as `reference — opening words…`.
9. **Collapsed and last:** `mentioned-in` and `attested-in` render collapsed, after every open list (F4 Q6, F5 Q6).
10. **Kinship:** entries show the person; the parentage wording shows only on the edge (its ⋮ step).
11. **Incarnate:** an incarnate person's Born/Died read "Born (earthly life)" / "Died (earthly life)".
12. **Event map view:** the World framed on the event's years, its stories highlighted.

## FINDINGS this design raises (for the queue)

- `atlas_core::time::TimeRange::undated()` writes the atlas span in code (-4004..100): rule 26. Closure: `AtlasSpan` read from the eras.
- `next_year` (server) and `Year::succ` (graph-types) will be two declarations of the zero gap until `atlas_core::time` goes (14b). Closure: the server uses graph-types' `Year`.
- The client's `YearNode` sorts and de-duplicates events itself (rule 25) and is identified by an event (F-57). Closes with the C# client.
- `PersonLife.first`/`last` are a mention span, served under names that read like a life. Closure: renamed `mentioned_from`/`mentioned_to` at the C# retirement.

## 7. Questions for the owner

Each is one line to answer; the build follows the recommendation if unanswered.

1. **Who is "alive" in a year?** Only people with a recorded birth and death (46 people today), plus the kings, judges and prophets of question 2; or also the 2,707 people the source places only by the years they are mentioned (about 245,000 more links, mostly people who are only named)? **Recommend: recorded lives and office holders only.**
2. **Kings reigning and prophets active:** none of our sources records reigns or ministries. Shall we curate one file of the kings of Israel and Judah, the judges and the prophets, with Ussher's dates (1658, public domain, the same scale the atlas's anchors already use) and the verses for each? **Recommend yes, kings and judges first, then prophets.**
3. **Events in the same year:** the arrows step through every event in the order the timeline already sets (the curated sequence, so the Passion week reads in order), and "At the same time" lists every other event of that year, so the next event can appear in both; or should the arrows jump straight to the next year that has an event? **Recommend: step through every event.**
