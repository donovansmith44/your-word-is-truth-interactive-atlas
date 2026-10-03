# The Year: design

**Status:** draft for the owner's sign-off (PRINCIPLES 12). Planning only; no code changes with this document.
**Queue:** item TIME (owner, 2026-10-03: "Year: spec it now, build after FOCUS-5").
**Base read:** `origin/worktree-bible-atlas-m1` at `25203b8`; the FOCUS-4 plan (`lane/claude/F4-plan` `da92793`) and the FOCUS-5 plan (`lane/claude/F5-plan` `564f4ca`), as rewritten by `docs/superpowers/plans/2026-10-03-focus45-server-data.md` (this branch).
**Binding rulings:** the "OWNER ANSWERS, 2026-10-03" block in `QUEUE.md` (ops), in particular F4 Q5, F5 Q2, F5 Q3, F5 Q4, and the C# client freeze.
**Amendment 2 (2026-10-03, `lane/claude/YEAR-spec-2`):** applies the last rulings of that block: the Year spec's questions (Q1–Q3), the F# domain answers (Q1 years are served nodes stepped by served links, Q2 one span per event or undated, Q6 "At the same time" is not same-year), "an account is ONE unbroken run of verses", and **time granularity** ("a year is not the finest granularity"). What changed: §1.3 (one dating per event), §1.5 (roles), new §1.7 (finer positions) and §1.8 (festivals), §1.6 (`concurrent-with`, `at-festival`, `instituted-in`), §2.5 (chronology is a linear extension of a partial order; concurrency is certain overlap or a curated parallel, compiled, loaded whole), §2.8 (accounts are single runs, FOCUS-3 v2's passages), new §2.9–§2.12 (curated formats, the measured size, Theographic date normalisation, the library survey), §3.4, §3.6 and new §3.8 (laws), §4, §5, §6, §7 (new questions). Removed: "the concurrent set is the event's own window".

## 0. What the owner asked for

> "Years are explorable and you can get to the map from there but the primary thing you should get is the set of events in chronological order that happened, active prophets, etc. all explorable."
>
> "the unit we care about fundamentally is an individual year. Year range is a composition over year."
>
> "Stories are different from objective chronology. Chronologically every event has only forward or back or is in parallel with another set of events." ... "Events need to get a concurrent events section (user-friendly name) that just lists the events."
>
> "if we can agree on the domain, data structure, and algebrae then we should be okay."
>
> "a year is not the finest granularity. If there's a festival or date or whatever that we can associate with an event then we should."
>
> On "At the same time": "It actually shouldn't be too long… It's long if the finest granularity you have is years, but even then, I doubt it's 20 concurrent events all attested by the Bible literally anywhere."

So:
- a **Year** is a node of the graph and the atomic unit of time;
- a **span** (a life, a reign, an event's date, an era's window, a range the reader picks) is a composition of years, never a node of its own;
- reading a year (or a span) gives, first, its events in chronological order; then the people alive, the kings reigning and the prophets active; then the eras, maps and polities covering it; the World is one step away;
- chronology (one previous, one next, a concurrent set) is separate from stories;
- an event's date may be finer than its year: a day of a month of the text's calendar, a festival, a day of the week, or a number of days after another event, each resting on a verse, none guessed. The **year stays the explorable time node**; a finer position places an event inside its year and orders it among that year's events. **Festivals are explorable nodes** with their institution verses;
- **"At the same time" means genuinely simultaneous**: two events whose spans certainly overlap at the finest granularity known for both, or a curated parallel with its verses. Sharing a year is not enough. The list is short and is served whole;
- an event has **one span**, marked "c." when approximate, **or is undated**, and an undated event has no chronology;
- **an account is one unbroken run of verses**; a story told in pieces is several accounts of one event;
- **years are served nodes stepped by served links**; the client does no date arithmetic.

This document has three parts, as the owner asked: §1 the domain types, §2 the data structures with their invariants and bounds, §3 the algebras with their laws stated as the properties the tests check. §4 decides what is compiled and what is read per request, with sizes. §5 is the contract. §6 is the F# client backlog. §7 holds the owner's questions.

## What exists today (read-only survey at `25203b8`)

| Thing | Where | What it is |
|---|---|---|
| `Year(i32)` | `graph-types/src/chrono.rs` | signed year, constructor refuses 0. No successor, no ordinal. |
| `TimePoint`, `ResolvedDate`, `ResolvedPlacement`, `temporal_order` | same | an event's resolved date (year, optional month and day) and its total order key (date, then the curated sequence key `SeqKey`). `month: Option<u8>` names no calendar, and the derived order sorts an unknown month **before** every known one ("`None` sorts before `Some`"): a guess (FINDING below). No row today carries a month: `event_date.from_month` is null on all 912 rows. |
| Theographic `startDate` | `server/atlas-etl/src/theographic.rs` `parse_theo_year` | 8 spellings (§2.11), among them `30` and `0030`, `29` and `0029`, and 50 Gregorian `YYYY-MM-DD` dates with no grounds; a hand-rolled parser keeps the leading integer and **drops an unparseable event silently**. |
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
| Events per dated year | 294 years; median **2**; four years hold more than 20 (AD 33: 98, AD 32: 57, AD 30: 27, AD 31: 24), all Jesus's ministry, which is sequential. (Theographic alone: 450 events over **191** start years once `30`/`0030` and `29`/`0029` are one year, not 221; median 1; AD 30 holds 46.) So "same year" is not "at the same time". |
| Attested events / accounts as unbroken runs | 1,710 events with attestation; **2,082** maximal runs: 1,897 of two or more verses (passages), **185** of one verse; 223 events have more than one account; the most on one event is 8 |
| Events whose own verses give a finer date | **≈ 95** after review (§2.10) |

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

pub enum EventWhen { Undated, Dated(Dating) }
```

- **One span per event** (F# domain Q2): an event is `Undated` or has exactly one `Dating`; `Circa` renders "c.". An `Undated` event has no chronology, no position, no concurrency and no window entry (I-CH2). Today every dated event is a one-year span (912 rows, `from_year == to_year`).

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
- **Roles (owner, Year Q2):** "a column denoting whether someone is prophet judge or whatever (or multiple roles)". The curated file `data/curated/offices.toml` has one `[[holder]]` row per person with a `roles` column (`["judge", "prophet"]` for Samuel and Deborah) and one term per role, each with Ussher's years and its verses; the compiler lowers each (person, role) into one `OfficeTerm`. A person's served `offices` lists every role.

### 1.6 Chronology and stories (shared with FOCUS-5)

```rust
pub struct ChronologicalSuccession { pub earlier: EventId, pub later: EventId, pub provenance: ProvenanceId }
pub struct YearSuccession { pub earlier: YearId, pub later: YearId }
pub struct StoryStep { pub story: NarrativeId, pub ord: u16, pub event: EventId, pub provenance: ProvenanceId }
```

- Both successions lower into **one new directed relation**, `Precedes => "precedes" / "follows"`: chronology over time positions (years and events). Story order is **not** this relation (F5 Q3).
- `StoryStep` is today's `succession_step` row, now lowered into the story membership relation `InStory => "in-story" / "story-steps"` (event → story), whose edge carries the step's place: its `ord`, and the story's previous and next event. The event-to-event `follows-in` edges carrying `EdgeMeta::Narrative` retire (the plan, Task F5-2 adds `in-story`; Task R removes the old edges).
- `temporal-adjacency` retires (F5 Q2). Both removals happen at the C# client's retirement (the plan's freeze rule, Task R), because the frozen client reads them.

```rust
pub struct Concurrency { pub a: EventId, pub b: EventId, pub ground: ConcurrencyGround, pub provenance: ProvenanceId }
pub enum ConcurrencyGround { CertainOverlap, Curated(Justification) }
pub struct FestivalKeeping { pub event: EventId, pub festival: FestivalId, pub provenance: ProvenanceId }
pub struct Institution { pub festival: FestivalId, pub passage: AccountTarget, pub provenance: ProvenanceId }
```

- `Concurrency` lowers into **one new symmetric relation**, `ConcurrentWith => "concurrent-with"` (display "At the same time"). The pair is stored once, `a < b` by id. The existing symmetric `parallel` is **not** reused: it means parallel accounts (Container ↔ Container), a different fact.
- `FestivalKeeping` lowers into `AtFestival => "at-festival" / "festival-events"` (event → festival); `Institution` into `InstitutedIn => "instituted-in" / "institutes"` (festival → passage or verse, §1.8).

### 1.7 Finer positions (`DatePosition`)

An event's span ends may be known more finely than the year. The text gives five kinds of claim, and each claim rests on a verse:

```rust
pub struct MonthOrdinal(u8);
pub struct DayOfMonth(u8);
pub enum MonthRef { Ordinal(MonthOrdinal), Named(MonthNameId) }
pub enum Weekday { First, Second, Third, Fourth, Fifth, Preparation, Sabbath }
pub enum FestivalRelation { On, During, Before(Option<DayCount>), After(Option<DayCount>) }
pub enum DayReckoning { OnTheNth(DayCount), After(DayCount), About(DayCount), Next, Same }
pub struct DayCount(u16);

pub enum PositionClaim {
    Calendar { month: MonthRef, day: Option<DayOfMonth> },
    Festival { festival: FestivalId, relation: FestivalRelation },
    Weekday(Weekday),
    Relative { prior: EventId, reckoning: DayReckoning },
}

pub enum SpanEnd { Start, End, Both }

pub struct PositionRow {
    pub event: EventId,
    pub end: SpanEnd,
    pub claim: PositionClaim,
    pub justification: Justification,
    pub provenance: ProvenanceId,
}
```

- **The calendar is the text's.** `MonthOrdinal` is 1..=12 counted from Abib, "the first month" (Ex 12:2, Ex 13:4, Num 33:3); its only constructor refuses anything else. `DayOfMonth` is 1..=30. A month **name** is a `MonthNameId` whose ordinal is data (`months.toml`, §2.9), present only where a verse equates the name with a number ("the month Zif, which is the second month", 1 Kgs 6:1). Elul (Neh 6:15) has no such verse, so "the 25th of Elul" is served as written and its month is **unknown** for ordering (the partial order below).
- **Nothing is converted.** No claim becomes a Julian or Gregorian date: the text's months were set by observation, so every conversion rests on assumptions the verses do not make. Ordering needs none: within a year, positions compare as `(month, day)` in the text's own count (§3.8). The library survey (§2.12) records the calendars considered and why none is used.
- **`Weekday`:** the text names "the first day of the week" (John 20:1) and "the sabbath"; "the preparation" is "the day before the sabbath" (Mark 15:42). A weekday alone gives no position in the year (that needs arithmetic the text does not supply); it is served as said, and checked against relative claims (law `weekdays_agree_with_counted_days`).
- **`Relative`:** "on the third day" is `OnTheNth(3)`, counted inclusively as the text counts it (Friday's crucifixion to the first day of the week is "the third day", Luke 24:21 with Mark 15:42 and 16:2), so it is a difference of 2 days; "after six days" is `After(6)`, "about eight days after" is `About(8)` (Matt 17:1 and Luke 9:28 tell one event both ways), "the next day" is `Next` (1), "the same day" is `Same` (0). `OnTheNth`, `Next` and `Same` fix a day difference; `After` and `About` fix only "later".
- **`Festival`:** "at the passover" (`On`), "the days of unleavened bread" (`During`), "six days before the passover" (`Before(6)`, John 12:1), "the passover was nigh" (`Before(None)`, John 11:55), "after the days of unleavened bread" (`After(None)`, Acts 20:6). The festival's own calendar (§1.8) turns the claim into a window.
- **`SpanEnd`:** a claim places the start, the end or both (one-day events) of the event's one span. The Flood starts "the second month, the seventeenth day" (Gen 7:11) and ends "in the second month, on the seven and twentieth day" of the next year (Gen 8:14).
- **A claim dates the event, not a moment inside it.** "Prophecies of Ezekiel" holds 31 dated verses; each dates an oracle, not the whole. The curator writes a claim only where the verse dates the event's own start or end (§2.10's review applies this rule).

**The resolved position.** The compiler resolves an event end's claims into an interval of keys:

```rust
pub enum Bound<T> { Bottom, At(T), Top }
pub struct DayKey { pub year: YearOrd, pub month: Bound<MonthOrdinal>, pub day: Bound<DayOfMonth> }
pub struct EndPosition { pub earliest: DayKey, pub latest: DayKey }
pub struct ResolvedSpan { pub start: EndPosition, pub end: EndPosition }
```

- `DayKey` is ordered lexicographically with `Bottom < At(_) < Top`. A year-only end is `[(y, Bottom, Bottom), (y, Top, Top)]`: sometime in that year. A month-only end is `[(y, m, Bottom), (y, m, Top)]`. A known day is a single key. Each claim narrows the interval; the resolved interval is the **meet** of every claim's interval, and an empty meet fails the compile (contradictory verses, or a typo).
- A festival claim with `Before(n)`/`After(n)` resolves to a day only when the result stays inside the festival's month (no month length is assumed: a Hebrew month had 29 or 30 days by observation); otherwise it contributes only "before"/"after".
- `Relative` claims do not resolve into keys; they are edges of the order (§2.5).

### 1.8 Festivals

```rust
pub struct Festival {
    pub id: FestivalId,
    pub label: String,
    pub calendar: FestivalCalendar,
    pub calendar_grounds: Justification,
    pub provenance: ProvenanceId,
}

pub enum FestivalCalendar {
    Fixed(NonEmpty<DayWindow>),
    Counted { from: FestivalId, days: DayCount },
    Unfixed,
}

pub struct DayWindow { pub month: MonthOrdinal, pub first: DayOfMonth, pub last: DayOfMonth }
```

- `NodeKind::Festival`, id `Festival:{slug}`. Exactly the eight the owner named (rule 4): Passover, Unleavened Bread, Weeks (Pentecost), Tabernacles, the Day of Atonement, Trumpets, Purim, Dedication. The sabbath and the new moon are not festivals here: the sabbath is a `Weekday`.
- **The calendar is data with its verse:** Passover `Fixed[1/14 (Lev 23:5), 2/14 (Num 9:11, the second passover)]`; Unleavened Bread `Fixed[1/15–21]` (Lev 23:6); Weeks `Counted { from: Unleavened Bread, days: 50 }` (Lev 23:15–16; "Pentecost", Acts 2:1, is the Greek "fiftieth" of the same feast); Trumpets `Fixed[7/1]` (Lev 23:24); Atonement `Fixed[7/10]` (Lev 23:27); Tabernacles `Fixed[7/15–22]` (Lev 23:34–36); Purim `Fixed[12/14–15]` (Esth 9:21, with Adar the twelfth month, Esth 3:7); Dedication `Unfixed` (John 10:22 says only "it was winter"; its date and institution are in 1 Maccabees, outside the canon we serve).
- A festival claim resolves to the window that the event's own `Calendar` claim names (Hezekiah's passover in the second month, 2 Chr 30:15, is the second window), else the first window; a `Calendar` claim that matches no window of a claimed festival fails the compile. `Counted` gives "after the `from` festival's first day" and no day (no month length is assumed). `Unfixed` gives nothing but the festival link.
- **Explorable:** a festival's record shows its calendar line (compiled label, e.g. "the 14th day of the first month"), its `instituted-in` passages (Passover: EXO.12.1-14; Unleavened Bread: EXO.12.15-20; the Lev 23 sections; Atonement: LEV.16.1-34; Purim: EST.9.20-32; Dedication: none), and its `festival-events`, ordered by the chronology.

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

- **What is known: a partial order.** Two dated events `a`, `b` stand in exactly one of four relations (§3.8):
  - **Before** (`a ≺ b`): `a` certainly ends before `b` starts: `a.end.latest < b.start.earliest` on `DayKey`s, or a path of `Relative` claims with a positive day difference or `After`/`About` leads from `a` to `b`, or a festival relation does (`Before` a festival `b` is `On`);
  - **After**: `b ≺ a`;
  - **Concurrent** (`a ∥ b`): their spans certainly overlap (§ below), or a `Same` claim joins them, or a curated parallel does;
  - **Indeterminate**: none of these. Two events known only to the same year are indeterminate: neither is known to come first, and they are not known to be simultaneous.
  `≺` is a strict partial order (irreflexive, transitive). It is **total where the positions are known**: on events whose ends resolve to single, distinct days, every pair is `≺` one way. It is **partial where they are not**, and stays partial: unknown is never read as "earlier" (the FINDING on `temporal_order`).
- **The chain: a linear extension.** The reader still needs one previous and one next (F5 Q3). The compiler builds the chain as the **linear extension** of `≺` that breaks every remaining tie by `(start.earliest, the curated SeqKey, id)`: Kahn's topological order over the `≺` graph with that key on its ready queue. So the chain respects every verse-grounded order, and only indeterminate pairs are ordered by the curated sequence, as today. A curated `SeqKey` or `SequenceAfter` that **contradicts** `≺` fails the compile (I-CH4), as does a cycle in `≺` (contradictory verses).
- **Previous and next.** One `ChronologicalSuccession` row per consecutive pair of the chain (911 today), lowered into `precedes`/`follows`. An event's `follows` neighbour is its previous; its `precedes` neighbour is its next.
- **Concurrent set ("At the same time").** Compiled `Concurrency` rows, lowered into the symmetric `concurrent-with` relation, from two grounds only:
  1. **Certain overlap** (`CertainOverlap`): `max(a.start.latest, b.start.latest) ≤ min(a.end.earliest, b.end.earliest)`: there is a day that lies in both spans however the unknown parts resolve. The day is the finest unit the atlas has, so two events on the same known day overlap; two events in the same year or the same month, with no day, do not; two multi-year spans overlap when one's interior years meet the other's.
  2. **Curated parallel** (`Curated`): a row of `data/curated/parallels.toml` (§2.9) with the verses that say so ("while he was yet speaking, there came also another", Job 1:16–18; Elijah's ministry while Jehoshaphat reigns), and a `Same` relative claim.
  It is **not** the window read and **not** "the event's own window" (removed by this amendment). It is not transitive: `a ∥ b` and `b ∥ c` do not make `a ∥ c`, and no closure is computed.
- **Bounds.** Today 0 rows (no event has a finer position, and every dated event is one year, so nothing certainly overlaps); after the §2.10 curation, the busiest day (the day of the crucifixion) holds about 6 events, so about 15 pairs. A set is served **whole** in one neighbour read, never paged; the compile fails if any event's set exceeds `CONCURRENT_WHOLE` (a named constant, 20: the owner's figure), so the "loads whole" promise is a gate, not a hope.
- Invariants: I-CH1 the `precedes` edges form one chain through every dated event; I-CH2 an undated event has no chronology edge, no position and no `concurrent-with` edge; I-CH3 the chain is a linear extension of `≺` (it never puts `b` before `a` when `a ≺ b`); I-CH4 no curated sequence contradicts `≺`, and `≺` is acyclic; I-CH5 `≺` and `∥` are disjoint (no pair is both, §3.8's proof); I-CH6 every concurrent set has at most `CONCURRENT_WHOLE` members.

### 2.6 The window read (per request, bounded)

A window is a `YearSpan` (a single year, or a range the reader picks: "Year range is a composition over year"). Its ends are always **served Year ids**: the reader picks two years it was given, never a computed one (F# domain Q1). The window answers what a Year or a range **holds**; it does not answer "At the same time" (§2.5). For one facet it answers the occupants whose span overlaps the window, in the facet's order, a page at a time:

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
**An account is one unbroken run of verses** (owner, 2026-10-03). This is FOCUS-3 v2's passage exactly (`lane/claude/F3-v2` 3a4526b, §D2 and laws P1–P9): a `Passage { span: PassageSpan::Bible { first, last }, mark }`, one contiguous run, minted by the one door `PassageMint`, id `passage_container_id(&span)`. FOCUS-5 adds nothing to the passage type; it mints with `PassageMark::RecordsHistory` and adds the `narrates` edge. The earlier draft's multi-run passage (`MRK.14.54, 66-72`) and `passage_container_id(&[runs])` are withdrawn.

```rust
pub enum AccountTarget { Verse(VerseNodeId), Passage(ContainerNodeId) }
pub struct Narration { pub account: AccountTarget, pub event: EventId, pub provenance: ProvenanceId, pub justification: Justification }
```

| | |
|---|---|
| Target | FOCUS-3 v2's `target(span)`: a run of two or more verses is a `Passage` (minted or reused, mark raised to `RecordsHistory` by P5); a run of one verse is the **verse itself** (P6: one unit is not a passage, nothing is minted) |
| Runs | the event's attested verses, coalesced into maximal runs in reading order; a run may cross a chapter and never crosses a book (FOCUS-3's spine). A curated witness told in pieces is **several accounts**: Peter's denial in Mark is `MRK.14.54` (a verse) and `MRK.14.66-72` (a passage) |
| Edge | `Narration` row, lowered into `Narrates => "narrates" / "narrated-in"` (account → event) |
| Label | the target's own label: FOCUS-3's `PassageCode` (`MAT.5.1-7.29`, `GEN.29.32-30.24`) or the verse code (`MRK.14.54`); no account-specific label function |
| Opening | the first words of the account's first verse in the canonical layer, compiled per account target, at most `OPENING_WORDS` (a named constant, 8) words, then `…` |
| Size | 2,082 accounts today: 1,897 passages, 185 single verses; 223 events have more than one; the most is 8 (measured on the artifact, §2.10's script) |
| Invariants | I-A1 an event's accounts partition its attested verses; I-A2 each account is one maximal run: no two accounts of one event are reading-adjacent; I-A3 one verse set is one node (a cited range and an account over the same verses are one passage, FOCUS-3 P5); I-A4 a passage has mark `RecordsHistory` exactly when it has a `narrates` edge |

### 2.9 The curated sources (every claim carries its verse)

Three new files and one extended, all flat `[[row]]` tables (the `event-witnesses.toml` discipline: each row names its own owner, no nested tables). Every row has a `verses` field in the curator's existing reference syntax (`curated::expand_verse_ref`), compiled to a `Justification`; a row without verses fails the parse.

`data/curated/months.toml`: the month names the text equates with a number.
```toml
[[month]]
name = "Zif"
ordinal = 2
verses = ["1KI.6.1"]

[[month]]
name = "Elul"
verses = ["NEH.6.15"]
```
A row without `ordinal` is a name the text never numbers (Elul): served, never ordered by month.

`data/curated/festivals.toml`: the eight festivals, their calendars and institution passages.
```toml
[[festival]]
id = "passover"
label = "Passover"
windows = [ { month = 1, first = 14, last = 14, verses = ["LEV.23.5"] },
            { month = 2, first = 14, last = 14, verses = ["NUM.9.11"] } ]
instituted_in = ["EXO.12.1-14"]

[[festival]]
id = "weeks"
label = "Weeks (Pentecost)"
counted = { from = "unleavened-bread", days = 50, verses = ["LEV.23.15-16"] }
instituted_in = ["LEV.23.15-21"]
also_named = { name = "Pentecost", verses = ["ACT.2.1"] }

[[festival]]
id = "dedication"
label = "Dedication"
unfixed = { verses = ["JHN.10.22"] }
```

`data/curated/event-positions.toml`: one row per claim, flat.
```toml
[[position]]
event = "num_passover_sinai"
end = "both"
calendar = { month = 1, day = 14 }
verses = ["NUM.9.5"]

[[position]]
event = "theo-38"
end = "start"
calendar = { month = 2, day = 17 }
verses = ["GEN.7.11"]

[[position]]
event = "mary_anoints_jesus_bethany"
end = "both"
festival = { id = "passover", relation = "before", days = 6 }
verses = ["JHN.12.1"]

[[position]]
event = "the_risen_jesus_appears_to_mary_magdalene"
end = "both"
weekday = "first"
verses = ["MRK.16.9"]

[[position]]
event = "the_empty_tomb"
end = "both"
relative = { prior = "crucifixion_and_burial", reckoning = "on-the-nth", days = 3 }
verses = ["LUK.24.21"]
```
(Event ids illustrative; Task 3b's ledger pins the real ones.) A `festival` claim also writes the `at-festival` edge; nothing else does.

`data/curated/parallels.toml`: curated "at the same time".
```toml
[[parallel]]
events = ["job_messenger_sabeans", "job_messenger_fire"]
verses = ["JOB.1.16"]
```

Parse-time invariants (each a test): every `event`/`prior`/`festival`/`from` id resolves; every claimed month is 1..=12 and day 1..=30; a month name used in a claim exists in `months.toml`; an event with any position is dated (I-CH2); no row duplicates another; a festival has exactly one of `windows`, `counted`, `unfixed`. Compile-time invariants: every end's claims meet non-empty (§1.7); I-CH4; I-CH6; `weekdays_agree_with_counted_days`.

### 2.10 The size: finer dates recoverable from the events' own verses

Measured on the artifact (`core` section at `25203b8`'s build, read-only): every event's attested verses (43,067 attestation rows over 1,710 events), KJV text, scanned for phrases (scratchpad script `finer_dates.py`, not committed), then reviewed by hand with §1.7's rule (the claim must date the event itself, not a moment inside it, and not a law about the calendar).

| Kind | Phrase hits (events) | After review: dates the event itself | Examples |
|---|---|---|---|
| Month and day ("the fourteenth day of the first month", "the month Zif") | 74 (66 with an explicit day) | **46** | the Passover at Sinai (NUM.9.5, 1/14); Israel camps at Gilgal (JOS.4.19, 1/10); the Flood begins (GEN.7.11, 2/17); Israel departs Sinai (NUM.10.11, 2/20); Jerusalem besieged (2KI.25.1, 10/10); the temple completed (EZR.6.15, 3 Adar); Ezra reads the law (NEH.8.2, 7/1); Ezekiel's call (EZK.1.1, 4/5); Haggai's first oracle (HAG.1.1, 6/1) |
| Festival | 73 (58 naming one of the eight) | **31** | Jesus anointed at Bethany, "six days before the passover" (JHN.12.1); the Holy Spirit comes (ACT.2.1, Pentecost); at the Feast of Dedication (JHN.10.22); Hezekiah's passover (2CH.30.15, second month); the boy Jesus at the Passover (LUK.2.41); Herod imprisons Peter in the days of unleavened bread (ACT.12.3); Purim instituted (EST.9.21) |
| Day of the week | 83 | **24** | the risen Jesus appears to Mary Magdalene "early the first day of the week" (MRK.16.9); Eutychus revived (ACT.20.7); the burial on "the preparation" (MRK.15.42); a withered hand healed on the sabbath (MRK.3.2) |
| Any of the three (union) | 194 | **≈ 95** (5.5% of 1,711 events; ≈ 10% of the 912 dated) | |
| Relative day ("on the third day", "after six days", "the next day") | 142 (not reviewed) | an order between two events, not a place in the year | the transfiguration "after six days" (MAT.17.1) |

What the review excluded: **15 broad events** that contain dated moments ("Prophecies of Ezekiel", 31 dated verses; "Reign of Zedekiah"; "Wilderness Wanderings"): a dated verse dates a part of them, and the part is the finer event that already exists (Jerusalem besieged; Ezekiel's call). **Law passages** that state the calendar ("The appointed feasts of the LORD", LEV.23; NUM.28–29; DEU.16; EXO.23.15): these are the festivals' institution verses (§1.8), not dated events. **False hits**: "unleavened bread" as food (GEN.19.3, Lot's meal), "Adar" as a place (JOS.15.3), "a sabbath day's journey" as a distance (ACT.1.12), habitual sabbaths ("every sabbath", ACT.18.4).

Bounds: positions ≤ 4 claims per event end (a day, a festival, a weekday, a relative), so ≤ 8 rows per event and ≈ 13,700 at most today; ≈ 150 rows for the ≈ 95 events. Festivals: 8 nodes, ≈ 15 `instituted-in` edges, ≈ 31 `at-festival` edges. `months.toml`: 11 names.

### 2.11 Theographic date normalisation (ETL law)

Measured on `events.json`: 450 events, 221 distinct `startDate` strings, **191** distinct years. The strings take 8 shapes: `-YYYY` (162), `-YYY` (85), `YY` (82), `YYYY` (53), `YYYY-MM-DD` (49), `-Y` (16), `Y` (2), `YYYY-MM-D` (1). The same year is spelled two ways: `30` and `0030` (AD 30), `29` and `0029` (AD 29). Values are astronomical years (`-4003` is 4004 BC).

The law (ETL, a tool; PRINCIPLES 26a):
- **One door:** `theographic::start_year(raw) -> Result<Year, TheoDateError>` is the only reader of `startDate`. It accepts exactly the shapes above (a closed `TheoDateShape` enum), converts astronomical to historical once (`-4003` → 4004 BC, `0` refused), and is **padding-blind**: `start_year("30") == start_year("0030")`.
- **Refuse, never drop:** a string of any other shape fails the ETL with the event's id (today's parser drops the event silently; FINDING).
- **The Gregorian month and day are discarded:** the 50 `YYYY-MM-DD` strings are Theographic's own estimates with no verse; they never become a `PositionRow` (§1.7: nothing guessed). The ledger lists them so a curator can look for the verse.
- Laws (proptest over generated strings + real data): `theographic_years_are_padding_blind`; `every_theographic_start_date_has_a_known_shape`; `an_unknown_shape_fails_the_etl_naming_its_event`; `no_theographic_month_or_day_reaches_a_position`.

### 2.12 Library survey (standing rule #1)

| Need | Candidates (licence, maintenance) | Fit | Choice |
|---|---|---|---|
| Calendar conversion (Hebrew ↔ Julian/Gregorian) | `icu_calendar` 2.3.0 (Unicode-3.0, ICU4X, active); `heca-lib` 1.3.2 (MIT); `heca` 1.5.0 (MIT, CLI); `kosher-rust` 0.1.0 (LGPL-2.1: licence out); Python `convertdate` 2.5.1 (MIT, tools only); `chrono`/`jiff` (Gregorian only) | All Hebrew calendars here compute the **fixed arithmetic calendar** of the 4th century AD; the text's months were set by observation, so a conversion would assert dates the verses do not give | **None.** The design converts nothing (§1.7); ordering is lexicographic on the text's own `(month, day)`. If the owner later wants Julian dates shown, `icu_calendar` is the candidate (permissive, maintained, the widest calendar set), and its output would be marked Traditional with "c." |
| Parsing Theographic `startDate` | `jiff` (MIT/Unlicense), `chrono` (MIT/Apache-2.0), `time` (MIT/Apache-2.0) ISO-8601 parsers; `regex` 1.13 (already a dependency) | ISO parsers reject `-4003`, `-445`, `0029-10-9` (astronomical, unpadded); only the year is kept | `regex` for the closed shape set + `str::parse::<i32>`; no date parser (the month/day is discarded, so nothing is validated as a date) |
| Linear extension with a tie key | `petgraph` (MIT/Apache-2.0) `toposort` | gives *a* topological order, not the one keyed by `(start, SeqKey, id)`; cycle detection fits | `petgraph` for the `≺` graph and `is_cyclic_directed`/cycle report; the keyed order is Kahn's loop over `std::collections::BinaryHeap` (a dozen lines; written reason: no surveyed crate takes a tie key) |
| Interval meet over `DayKey` | `intervallum`, `gcollections` (MIT/Apache-2.0) | integer intervals; our keys are lexicographic triples with `Bottom`/`Top` | hand-written `meet` on `EndPosition` (two `max`/`min` calls); reason: a three-line function over our own ordered type |

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
| extension | `the_chain_is_a_linear_extension_of_what_is_known`: if `a ≺ b` then `a` comes before `b` in the chain (subsumes "respects dates": a span ending in an earlier year is `≺`). `the_chain_breaks_only_unknown_ties_by_the_curated_sequence`: for consecutive `a, b` with `a ⊀ b`, `(start.earliest, SeqKey, id)` of `a` is the smaller. `the_chain_is_deterministic`: the same rows in any input order give the same chain. |
| contradiction | `a_curated_sequence_against_a_verse_fails_the_compile`; `contradictory_positions_fail_the_compile` (a `≺` cycle, or an empty meet). |
| years | `the_year_after_1_bc_is_ad_1_in_the_graph` (real data): `Year:-1`'s `precedes` neighbour is `Year:1`. |

Concurrency's laws are §3.8's.

### 3.5 Stories

| Operation | Law (test) |
|---|---|
| steps | `a_storys_steps_are_numbered_without_gaps`; `a_steps_previous_and_next_are_its_neighbours_in_the_story`; within one story, `previous` and `next` are inverses. |
| order | `a_story_lists_its_events_in_its_own_order` (real data): a story's `story-steps` page is in `ord` order. |
| independence | `stories_are_independent_of_chronology`: compiling with every narrative file removed, or with the steps of any story permuted, leaves every `precedes` edge unchanged; changing any dated-by placement leaves every `in-story` edge unchanged. No law ties a story's order to time (a story may look back). |

### 3.6 Accounts

| Operation | Law (test) |
|---|---|
| partition | `an_events_accounts_partition_its_attestation`: the union of the verses of an event's `narrated-in` accounts equals its `attested-in` verses, and no verse is in two of its accounts. |
| runs | `an_account_is_one_unbroken_run`: every account is a single verse or a FOCUS-3 passage (`first..=last`, contiguous); `no_two_accounts_of_an_event_touch`: no two of one event's accounts are reading-adjacent (runs are maximal); `a_run_may_cross_a_chapter`; `a_run_never_crosses_a_book`. |
| pieces | `a_story_told_in_pieces_is_several_accounts` (real data): Peter's denial in Mark is `MRK.14.54` and `MRK.14.66-72`, two `narrated-in` entries of one event. |
| one verse | `a_one_verse_account_is_the_verse` (FOCUS-3 P6): a one-verse run narrates from the verse node and mints no passage. |
| identity | `one_verse_set_is_one_passage`: an account and a citation over the same verses are one node (FOCUS-3 P5). |
| history | `a_passage_records_history_iff_it_narrates`: a passage has mark `RecordsHistory` exactly when it has a `narrates` edge. |
| opening | `an_opening_is_the_first_words_of_the_first_verse`: a prefix of the canonical text, at most `OPENING_WORDS` words, with `…` exactly when cut. |

### 3.7 Parentage (shared with FOCUS-4)

| Operation | Law (test) |
|---|---|
| `wording: Parentage → String` | `parentage_wording_is_total`: `data/curated/parentage.toml` holds exactly one wording for each member of `Parentage::ALL` (`Natural`, `Eternal`, `Virgin`, `Legal`, `Created`); a missing, extra or duplicate entry fails the compile. |
| edge label | `a_parent_of_edge_reads_its_wording`: the edge's compiled label is `{parent} · wording(p) · {child}`, the same edge whichever end it is read from; the wording appears nowhere but the edge. |

### 3.8 Positions, order and concurrency

Generators: random datings with random claim sets (including contradictory ones), random curated sequences, random parallels.

| Operation | Law (test) |
|---|---|
| `DayKey` order | `day_keys_are_totally_ordered_with_bottom_and_top`: lexicographic on `(year, month, day)`, `Bottom < At(_) < Top`. `a_year_only_end_is_the_whole_year`: its interval contains every key of that year and no other. |
| meet | `claims_meet_as_intervals`: an end's resolved interval is the intersection of its claims' intervals; commutative, associative, idempotent; adding a claim never widens it; `a_claim_set_resolves_in_any_order`. |
| festival | `a_festival_claim_resolves_to_its_window`; `a_calendar_claim_selects_the_festival_window` (2/14 picks Passover's second window); `a_calendar_claim_matching_no_window_fails`; `a_festival_offset_stays_in_its_month_or_gives_order_only`. |
| month names | `a_month_name_orders_only_if_the_text_numbers_it` (Elul's events are month-unknown). |
| `≺` | `before_is_a_strict_partial_order`: irreflexive, transitive, asymmetric. `before_is_total_where_days_are_known`: over events whose ends are single distinct days, every pair is comparable. `unknown_is_never_earlier`: an end with an unknown month is not `≺` a known month of its year, nor after it (contrast `temporal_order` today). `relative_claims_order`: `Relative { prior: a, OnTheNth(n ≥ 2) | After | About | Next }` gives `a ≺ b`; `Same` gives `a ∥ b`. |
| four relations | `two_dated_events_stand_in_exactly_one_relation`: exactly one of `a ≺ b`, `b ≺ a`, `a ∥ b`, indeterminate. `concurrency_and_order_are_disjoint` (I-CH5; proof: if `a.end.latest < b.start.earliest` then `b.start.latest ≥ b.start.earliest > a.end.latest ≥ a.end.earliest`, so `max(start.latest) > min(end.earliest)`: no certain overlap; a curated parallel or `Same` that meets a `≺` fails the compile). |
| concurrency | `concurrency_is_symmetric`; `concurrency_is_irreflexive`; `concurrency_is_certain_overlap_or_curated`: a `concurrent-with` edge exists ⟺ certain overlap, a `Same` claim or a curated parallel; `same_year_is_not_concurrency`: two one-year events in one year with no finer claim are indeterminate, not concurrent; `same_day_is_concurrency`: two events on one known day are concurrent; `concurrency_is_not_closed_under_transitivity` (a generated `a ∥ b ∥ c` with `a ≺ c` stands). `an_undated_event_has_no_position_and_no_concurrency`. |
| bound | `every_concurrent_set_loads_whole` (real data): every event's `concurrent-with` group has at most `CONCURRENT_WHOLE` members and is served in one read with no cursor. |
| weekdays | `weekdays_agree_with_counted_days`: if `b` is `OnTheNth(n)` of `a` (difference `n − 1`), `Next` (1) or `Same` (0), and both carry a weekday, the weekdays differ by that difference mod 7 (real data: the crucifixion on the preparation, the empty tomb on the first day, "the third day"). |
| grounds | `every_position_rests_on_a_verse`: every `PositionRow`, festival window, month name and parallel has a non-empty justification; `every_position_is_justified_by_an_edge`: its verses are `justified-by` edges from the event (FOCUS-4's `grounds_of` gains the `PositionRow` and `Concurrency::Curated` arms). |
| festivals | `the_festivals_are_exactly_the_eight`; `a_festival_lists_its_events_in_chronological_order`; `every_fixed_festival_has_its_institution` (Dedication, `Unfixed`, is the one without). |

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
- **Compiler:** Year nodes and labels; year succession; resolved positions (§1.7) and their labels; the partial order `≺` and its keyed linear extension, the event chronology (`precedes`); **concurrent sets, compiled** as `concurrent-with` edges (certain overlap and curated parallels; ≈ 15 pairs after curation, bounded by `CONCURRENT_WHOLE`, so edges are cheap here, unlike the year's occupancy); festivals, `at-festival`, `instituted-in`; the cover index, span starts and counts; every year, span and position label with its `c.` (F-36); story membership with each step's previous and next; accounts (single runs), their labels and openings.
- **Server:** the element read (a Year's record), the neighbour read (`precedes`/`follows`, `concurrent-with`, `in-story`, `narrated-in`, `at-festival`, `instituted-in`), the window read. Nothing else.
- **Client (F#):** the Year presentation, the range presentation (a window between two served years), "At the same time" (one neighbour group, whole), the Stories section, the festival presentation, the map one step away. **No date arithmetic:** it never adds to a year, compares two dates or composes a Year id; every step is a served link (F# domain Q1).

---

## 5. The contract

All additive (AQC minor) while the C# client is frozen; nothing the C# client reads changes shape (the plan's freeze rule).

```rust
pub struct Year { pub value: i32, pub label: String, pub node: NodeRef }

pub struct TimeRange { pub from: Year, pub to: Year, pub label: String, pub circa: bool, pub position: Option<PositionDetail> }

pub struct PositionDetail { pub label: String, pub festival: Option<NodeRef> }

pub struct FestivalDetail { pub calendar: Option<String> }

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

- `Year.node` and `TimeRange.circa` are new fields on existing wire types (labels compiled, F-36). `NodeRecord` gains `year: Option<YearDetail>` and `festival: Option<FestivalDetail>`; `PersonLife` gains `offices: Vec<OfficeTermDetail>`.
- **`Year.value` has no F# reader.** The F# client reads only `label` and `node` (F# domain Q1: no date arithmetic on the client, so no integer to do it with). `value` stays for the frozen C# client and is on the retirement list (the plan's Task R). This closes the category: with no year integer on the F# wire, client date arithmetic cannot be written.
- **`TimeRange.position`** is served on an event's `when` only when it has a finer position: its compiled label ("the 14th day of the first month", "six days before the Passover", "the first day of the week", "on the third day after the crucifixion") and, for a festival claim, the festival's `NodeRef`. The verses behind it are the event's `justified-by` group (§3.8 grounds law), not a field.
- **Route:** `GET /api/window?from={Year id}&to={Year id}&facet={facet}&cursor=&limit=`. The ends are served Year ids (the client passes what it was given, never a computed year). This is 27a's generic "range read by time window", not a view route: it serves every facet, for any window, in one shape. `WindowCursor` is the third named cursor type (O-WIRE-IDENTITIES O2 precedent).
- **Vocabulary:** `NodeKind::Year`, `NodeKind::Festival`; `Precedes` ("precedes" / "follows"); `InStory` ("in-story" / "story-steps"); `Narrates` ("narrates" / "narrated-in", FOCUS-5); `AtFestival` ("at-festival" / "festival-events"); `InstitutedIn` ("instituted-in" / "institutes"); symmetric `ConcurrentWith` ("concurrent-with"). Display labels are derived from the wire names until O-LABELS is ruled.
- `/api/scene`'s time read stays for the map (MAPS, F-34); the World's year view uses `/api/scene` and the window read side by side until MAPS.

---

## 6. F# client backlog (the C# client is frozen)

The C# client gets nothing from this design. In the F# client, after the F# style sign-off and after the server work lands:

1. **Year presentation** (`Presentation.Of(Year, Popover)`): the year's label; arrows `‹ 1447 BC` / `1445 BC ›` from its `follows`/`precedes`; then the facets in this order, each a list paged 20 at a time through the one paging door, heading `{facet label} ({count})`: **Events** (chronological), **Alive**, **Kings and judges**, **Prophets** (the office facet, split by the served `office`), **Eras**, **Maps**, **Polities**, **Written**. Facet headings are client presentation words over a closed served enum.
2. **Map one step away:** `Presentation.Of(Year, World)` = `Geography(Frame.Bounded(year))`; the year's "Show on the map" chip.
3. **Range presentation:** a span is a client explorable with identity `(from Year id, to Year id)`, made from any served `TimeRange` (a life, a reign, an event's date) or by the reader picking two years; it reads the same facets through the window read, with no counts (More/Less only). Saved explorations store the two ids.
4. **Years are explorable everywhere:** every served `Year` and `TimeRange` field on a card (Born, Died, When, a window, a reign, a term) is a link: a single year opens its Year; a span opens the range.
5. **Event "At the same time":** the event's `concurrent-with` group, read whole in one neighbour read (no paging, no window); absent when empty, and always absent for an undated event.
6. **Event chronology arrows:** `follows` (previous) and `precedes` (next) only; **Stories** section: one row per `in-story` entry, `‹ previous · story · next ›`, each a link.
7. **Event card:** When (linked, item 4) with its served `position` label under it (the festival in it is a link), Superscription, Source; no bookkeeping.
8. **Accounts:** `narrated-in` entries as `reference — opening words…`.
9. **Collapsed and last:** `mentioned-in` and `attested-in` render collapsed, after every open list (F4 Q6, F5 Q6).
10. **Kinship:** entries show the person; the parentage wording shows only on the edge (its ⋮ step).
11. **Incarnate:** an incarnate person's Born/Died read "Born (earthly life)" / "Died (earthly life)".
12. **Event map view:** the World framed on the event's years, its stories highlighted.
13. **Festival presentation:** label, the served calendar line, `instituted-in` (reference — opening words), `festival-events` in served order.

## FINDINGS this design raises (for the queue)

- `atlas_core::time::TimeRange::undated()` writes the atlas span in code (-4004..100): rule 26. Closure: `AtlasSpan` read from the eras.
- `next_year` (server) and `Year::succ` (graph-types) will be two declarations of the zero gap until `atlas_core::time` goes (14b). Closure: the server uses graph-types' `Year`.
- The client's `YearNode` sorts and de-duplicates events itself (rule 25) and is identified by an event (F-57). Closes with the C# client.
- `PersonLife.first`/`last` are a mention span, served under names that read like a life. Closure: renamed `mentioned_from`/`mentioned_to` at the C# retirement.
- **`TimePoint.month: Option<u8>` names no calendar, and `temporal_order` sorts an unknown month before every known one** (graph-types `chrono.rs`, the derived `Ord`): an order the data does not know, presented as known. Harmless today (no month rows), wrong the day the first position lands. Category: unknown read as earliest. Closure: `MonthOrdinal` (the text's count) replaces `u8`; `≺` is the only order with a law that unknown is never earlier; `temporal_order` becomes the keyed linear extension (§2.5).
- **`parse_theo_year` drops an event whose date it cannot read** (`theographic.rs`: "an unparseable or year-zero date drops the event rather than failing") and is a hand-rolled parser of an unclosed shape set. Closure: §2.11's one door with a closed `TheoDateShape` and refusal.
- **Event part-of is not modelled.** Theographic's `partOf` (201 events) is not ingested, so a broad event ("Holy Week") and its parts ("The Last Supper") will be concurrent by certain overlap once both carry positions. Proposed: a `part-of` relation, after which `concurrent-with` excludes an event's own parts (not built here; rule 4 until the owner asks).

## 7. Questions for the owner

**Answered (2026-10-03):** Q1 recorded lives and office holders only; Q2 curate kings of Israel and Judah, judges and prophets with Ussher's dates and verses, with a roles column (§1.5); Q3 the arrows step through every event in curated order. Q3's second half ("At the same time" lists the year's other events) is superseded by the F# domain Q6 ruling (§2.5). The earlier wording is kept below for the record.

<details><summary>The answered questions, as asked</summary>

1. **Who is "alive" in a year?** Only people with a recorded birth and death (46 people today), plus the kings, judges and prophets of question 2; or also the 2,707 people the source places only by the years they are mentioned (about 245,000 more links, mostly people who are only named)? **Recommend: recorded lives and office holders only.**
2. **Kings reigning and prophets active:** none of our sources records reigns or ministries. Shall we curate one file of the kings of Israel and Judah, the judges and the prophets, with Ussher's dates (1658, public domain, the same scale the atlas's anchors already use) and the verses for each? **Recommend yes, kings and judges first, then prophets.**
3. **Events in the same year:** the arrows step through every event in the order the timeline already sets (the curated sequence, so the Passion week reads in order), and "At the same time" lists every other event of that year, so the next event can appear in both; or should the arrows jump straight to the next year that has an event? **Recommend: step through every event.**

</details>

### Amendment 2's questions

Each is one line to answer; the build follows the recommendation if unanswered.

4. **Does "the same day" count as "at the same time", but "the same month" not?** For example: Jesus's second appearance before Pilate and his burial both fall on "the preparation" (John 19:14, Mark 15:42), so they would be listed together. Two events that the text puts only "in the seventh month" of one year, with no day, would not be listed (we do not know they met). A reign and a prophet's ministry that share whole years would be listed. **Recommend: yes. The day is the smallest unit, and only overlap that is certain counts.**
5. **Keep the Bible's own calendar, with no conversion to our calendar?** For example, Jerusalem's siege began "in the tenth month, in the tenth day of the month" (2 Kings 25:1). We would show and order it as the 10th day of the 10th month of its year, and never as a January date. Every conversion assumes how the months were set, and the verses do not say. A consequence is that a year's events are ordered by the Bible's months, counted from Abib (spring). **Recommend: yes, no conversion.**
6. **What is curated first?** About 95 events have a month and day, a festival, or a day of the week in their own verses. For example, the Passover at Sinai is on the 14th day of the first month (Numbers 9:5), and the risen Jesus appears on "the first day of the week" (Mark 16:9). Another 142 events have only counted days ("after six days"). (a) Curate the 95 now with FOCUS-5, and add counted days only where they order events of one year (the Flood, the Exodus, Passion week). (b) Curate all 237. (c) Curate only Passion week now. **Recommend (a).**
