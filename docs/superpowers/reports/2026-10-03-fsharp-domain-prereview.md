# F# domain proposal: pre-review for the owner's sign-off

- **Reviewed:** `lane/codex/CX-FSHARP` at `6118be2`: the one-page spec `docs/superpowers/specs/2026-10-03-fsharp-domain.md`, the four signature files under `client-fsharp/Domain/` (`Model.fsi`, `Structures.fsi`, `Algebras.fsi`, `BoundaryModel.fsi`), and the notes `docs/superpowers/reports/2026-10-02-fsharp-client-domain.md`.
- **Compared against:** the owner's rulings ("OWNER ANSWERS, 2026-10-03" in ops `QUEUE.md`); the signed-off wire identities (`lane/claude/WIREID-spec`, `2026-10-03-wire-identities-design.md`); the Year design (`lane/claude/YEAR-spec`, `2026-10-03-year-design.md`); FOCUS-3 v2 (`lane/claude/F3-v2`, `2026-10-03-focus3-server-data.md`); the costume audit (`a78559f`).
- **Read-only.** Nothing was built or compiled. Package versions were checked against nuget.org on 2026-10-03.
- **Verdict: not ready for sign-off. One revision should get it there.** The direction is right, and most of the hard ideas are already in it. But the owner cannot yet see the data. Every domain type is opaque, so whether something is a costume can't be decided. And in five places the model disagrees with the server designs it must match: the Year, an event's date, passages and containers, the cursors, and the trail's history.
- **Already ruled (owner, 2026-10-03), so not asked again:** "I don't like it all crammed into one file. I want things to be in the files where they'll actually live to eliminate potential for drift." The single proposal layout is a defect. Part 2 §0 maps where each type should live.

---

# Part 1: for the owner

## 1. A plain-language tour of the proposed domain

The proposal names about 119 types. Read in domain words, it says this.

**Time.**
- A **Year** is one year, BC or AD, with no year zero: 1446 BC, AD 30. The year after 1 BC is AD 1.
- A **YearSpan** is an unbroken run of years with its ends in order, for example 1450–1400 BC. A single year is a span of one.
- A **YearCoverage** is several spans with gaps kept, for example "1446 BC and 1406–1400 BC". It can also be empty.
- *Server difference:* the server's Year design has no coverage. An event has one dated span, marked exact or "c.", or it is undated. See question 2.

**Text and references.**
- `GEN.1.1` is a **BibleVerse**, the proposal's name for what the server calls a **VerseReference**.
- `GEN.1` is a **BibleChapter** (server: **ChapterReference**).
- `GEN.29.32-30.24` is a **BiblePassage**: two verses in order, and the range may cross a chapter. The server splits this into a **PassageReference** (`GEN.1.1-5`, within one chapter) and a **VerseRangeReference** (across chapters).
- `BoC 4.4.48` is a **ConcordParagraph**, and a run of them is a **ConcordPassage**.
- A **UnitReference** is either a verse or a Concord paragraph. That matches the server exactly.
- Your ruling "Bible references anywhere resolve to the same verse units" is built into the type. A BibleVerse carries no translation, so `GEN.1.1` cited from the Large Catechism and `GEN.1.1` in the KJV reader are the same value.
- Your ruling "the Small Catechism paragraphs ARE the catechism" is honoured: there is no catechism-item type at all.

**What a reader opens.**
- A **TextUnit** is one verse or one paragraph: its id, its reference, its served label (`Ap IV 48`) and its text.
- An **Entity** is a person, place, era and so on.
- An **Edge** is a served link between two things ("Mary · Mother of · Jesus").
- An **Event** is a named happening that **groups one or more historical accounts**. Each account is a passage that records history, so the Exodus event groups its accounts in Exodus 12–14. "One or more" is enforced by the type (`NonEmpty`), which matches your F3 Q5 ruling.

**Chronology and stories, kept apart (your F5 Q3).**
- An event's **Chronology** is one earlier event, one later event, and a set of **concurrent events** ("At the same time").
- Separately, the event has a list of **StorySteps**: for each story it belongs to, the step before and after it in that story.
- The two can't be confused, because they are different types.

**Reading (your F3 Q12, Q13).**
- A **ReadingPage** holds at most **20** units.
- A **ReadingWindow** keeps at most **40** units and drops whole pages from the far end as you read on.
- **WholeChapter** is the separate "Whole chapter" mode: exactly one chapter, every verse, not limited to 40.
- *Gap:* the proposal does not yet say that scrolling into the next chapter is the same change as the next-chapter arrow (Part 2, I2).

**Getting around.**
- A **Position** is something you can stand on: a graph element, a passage, a year, or a span of years.
- **Rooted** stamps a value with the data version it came from (the "artifact root").
- A **Trail** is your path of steps. Each step must be backed by a real served link, so you can't step along a connection the server never gave.
- If the data version changes under you, the trail asks to be **renewed** on the new version rather than mixing old and new.
- The **Focus** is what is open now, plus its neighbour lists. Each list has a size cap, and so does the total.
- The **PageCache** remembers recent pages, keyed by data version, query and cursor, and forgets the least recently used.

**Failures.** Each failure is a closed list of named cases with no free text. For example: "years reversed", "passage crosses corpora", "the data version changed". A network hiccup (transient, may retry) is a different type from a bad answer (terminal, never retried).

## 2. What's good

- **Your rulings are mostly captured.**
  - An event is a nonempty group of accounts.
  - Chronology and stories are separate types.
  - There is no year zero.
  - There is no catechism-item hop.
  - A verse identity carries no translation.
  - Reading is 20/40 plus a distinct whole-chapter mode.
  - There is a concurrent set.
- **It states real algebra, not decoration.**
  - It says plainly that `cover` (the hull: "1450–1440 BC and 1420 BC together span 1450–1420 BC") is **not** the union of years, and has no empty identity. Coverage union is the one that keeps gaps.
  - It says that after a page is dropped, append and prepend don't simply cancel. That is the kind of precision you asked for.
- **Trust is threaded through the types.**
  - A trail step needs a served edge.
  - Every resolved value carries its data version.
  - A cross-version step can't silently succeed.
- **The costume audit's worst offenders have proper shapes.**
  - The JSON path is now a list of structured steps (`Path<JsonStep>`), not a string.
  - Positions in a document are one-based lines and columns.
  - Scalar offsets and UTF-16 offsets are different types.
  - URLs must be http(s), coordinates are bounded, and failures carry no English.
- **It is honest about its state.** It says the signatures parse but don't compile, and that the laws are tests still to be written.

## 3. Decisions for you (six)

Each question has options, and my recommendation comes first.

**Q1. Is a Year something the server hands the client, or something the client builds?**
- (a) **Served.** A Year is a node of the graph (`Year:-1446`) with its label. The client steps to the next year by following the served link and never computes "the year after". The client may still compare two years it was given.
- (b) **Built by the client.** The client can make "1446 BC" from a number and work out the next year itself. This is what the proposal does (`Years.year`).
- **Recommend (a).** It is what the signed Year design says, and it makes a year outside the atlas (99999 BC) impossible to hold.

**Q2. How is an event dated?**
- (a) **One span plus a "c." flag, or undated.** This is the server design. An undated event has no previous, next or "At the same time".
- (b) **A coverage.** Several spans with gaps, possibly empty. This is the proposal.
- **Recommend (a).** No data source dates an event in several pieces, and (b) lets an undated event carry a chronology, which can't happen.

**Q3. What happens to the trail's oldest steps when it gets long?**
- (a) **Keep a fixed number (say 50) and forget older ones.** Saved journeys are how you keep a path for good.
- (b) **The server stores old steps** and hands back "receipts" for paging back through them. This is the proposal. No server design provides it, so it would be a new contract item.
- (c) **The browser stores them** locally.
- **Recommend (a).** It is the simplest choice that is bounded and honest. Choose (b) only if Back must reach arbitrarily far.

**Q4. Should the F# model have one "container" idea shared by the Bible and the Book of Concord?**
- (a) **Yes, one container type.** A Concord document is a book, an article is a chapter, and a passage is a container too. So "Whole chapter", next and previous, and the contents all work the same for both. This is FOCUS-3 v2 and your "1 should be a shared interface".
- (b) **No, keep separate Bible and Concord types.** This is the proposal. It has no Concord document or article, and "Whole chapter" is Bible-only.
- **Recommend (a).**

**Q5. Whose words for the names?**
- (a) **One vocabulary across the stack:**
  - the wire names you signed off: VerseReference, ChapterReference, PassageReference, VerseRangeReference, ConcordReference, EdgePageCursor;
  - your words: previous and next, "At the same time", Bible and Concord.
- (b) **The proposal's own names:** BibleVerse, BibleChapter, Scripture or Confession, Earlier or Later, Outward or Inward.
- **Recommend (a).** Two names for one thing are drift waiting to happen. A story step is "previous" and "next", not "earlier" and "later", because a story can look back in time.

**Q6. Should "At the same time" be a whole list or a paged list?**
- (a) **Paged, like every other list.** The server reads it as a window over the event's years. AD 33 alone holds 98 events.
- (b) **Complete, held whole on the event.** This is the proposal.
- **Recommend (a).** It reuses the one paging door and the 20-at-a-time rule.

---

# Part 2: for Codex

Defects are ranked. The `file:line` references are at `6118be2`. **C** means it blocks sign-off, **I** means important, **M** means minor.

## 0. Where each type should live (owner ruling: no single proposal file)

The four `client-fsharp/Domain/*.fsi` proposal files are deleted. Each type moves, as compiling code, into the module it will live in. Each module has an `.fsi` where a private representation needs one, and its laws sit as property tests beside it in `client-fsharp.Tests/Domain/<Module>Laws.fs`. The spec then links to these files and restates no type.

The table gives the compile order inside `client-fsharp/Core/BibleAtlas.FSharp.Core.fsproj`, after the generated contract (`Wire.g.fs`, `Vocabulary.g.fs`).

| # | File | Holds |
|---|---|---|
| 0 | *(generated, not hand-written)* `obj/Contract/Wire.g.fs` | `ArtifactRoot`, `NodeId`, `EdgeId`, `ElementId`, `EdgePageCursor`, `ElementPageCursor`, `WindowCursor`, `VerseReference`, `ChapterReference`, `PassageReference`, `VerseRangeReference`, `ConcordReference`, the reference unions, `BookId`, `Corpus`, `RelationKind`/`EdgeKind`, `Facet`, `Office`, `PassageMark`, `ContainerLevel`, `TextPartRole`. **Never re-declared in the domain**: that is the drift the owner names. |
| 1 | `Core/Domain/NonEmpty.fs(i)` | `NonEmpty<'a>`, `Positive` (or library types, M1) |
| 2 | `Core/Domain/Rooted.fs(i)` | `Rooted<'a>` |
| 3 | `Core/Domain/Time.fs(i)` | `Year` (admitted from the served `Year`), `YearSpan`, `Precision`, `Dating`, span algebra |
| 4 | `Core/Domain/Text.fs(i)` | `UnitReference` ordering, `TextPart`, `TextBody`/`Rendering`, `TextUnit` |
| 5 | `Core/Domain/Containers.fs(i)` | `Container` (shared by both corpora), `Passage` (span plus mark), `WholeChapter` |
| 6 | `Core/Domain/Graph.fs(i)` | `Entity`, `Edge`, `Node`, `Element`, `RelationDirection` |
| 7 | `Core/Domain/History.fs(i)` | `Event`, `HistoricalAccount`, `Chronology`, `StoryStep`, `EventContext` |
| 8 | `Core/Domain/YearReading.fs(i)` | `YearContext`, `SpanContext`, `OfficeTerm`, facet counts |
| 9 | `Core/Domain/Position.fs(i)` | `Position`, `ResolvedValue`, `Resolved` |
| 10 | `Core/Paging/ReadingWindow.fs(i)` | `ReadingKey`, `ReadingPage`, `ReadingWindow`, `ReadingExtent` |
| 11 | `Core/Paging/NeighbourWindow.fs(i)` | `NeighbourKey`, `NeighbourPage`, `NeighbourWindow`, `NeighbourCapacity` |
| 12 | `Core/Paging/PageCache.fs(i)` | page keys, `CachedPage`, `CacheCapacity`, `PageCache` |
| 13 | `Core/Exploration/Trail.fs(i)` | `Transition`, `TrailCapacity`, `Trail`, `Renewal`, `StepOutcome`, `BackOutcome` |
| 14 | `Core/Exploration/Focus.fs(i)` | `FrontierCapacity`, `Frontier`, `Focus`, and the one focus-change function (I2) |
| 15 | `Core/Admission/Paths.fs(i)`, `WireFailure.fs`, `ReadFailure.fs`, `Scalars.fs(i)`, `TextSpans.fs(i)` | `Path<'step>`, `JsonStep`, `JsonKind`, `WireFailure`, `Transient`/`Terminal`/`ReadFailure`, `HttpUrl`, `Latitude`/`Longitude`, `ColorToken`, statuses, offsets and spans, each door with its own failure (I1) |
| — | `client-fsharp.ContractGenerator/` | `DocumentStep`, `DocumentPath`, `ContractError`, `Keyword` (build-time only; not in the client) |

Each failure type sits in the file of the door that returns it. The cross-module `DomainFailure` only wraps them, and only if an update genuinely needs one sum.

## Critical

**C1. The data can't be seen, so the domain can't be signed off, and no costume can be ruled out.**
- *Where:* `Model.fsi:3-30`, `Structures.fsi:3-12, 22-30, 40-43`, `BoundaryModel.fsi:5-24`. These are abstract types with no representation, and no explanation beyond a one-line table row.
- *Problem:*
  - "Is `DisplayText`, `TextBody`, `Vocabulary`, `WireField` or `RefusalCode` a costume?" is undecidable.
  - The notes say so themselves: "Abstract representations are proposals" (report:44).
- *Fix:*
  - Show every representation as `private`, for example `type Year = private { Era: YearEra; Number: Positive; Node: NodeId }`.
  - Put one in-context example per type in the spec's tour, linked to the type's file ("`GEN.1.1` is a VerseReference …"). Don't put it in `///` comments: PRINCIPLES 9 forbids comments in application code.
  - State, in the spec and in the door's property-test names, what each type's one door checks. A wrapper whose door checks nothing is either deleted or justified in one line (`DisplayText`: served prose, never parsed).

**C2. The Year diverges from the server's Year.**
- *Where:* `Algebras.fsi:14` `Years.year: YearEra -> Positive -> Year` is a total, client-side factory. `Model.fsi:59` makes `Position.Year` a bare value, not an element.
- *Problem:* the Year design has three rules that this breaks:
  - a Year is a node (`Year:-1446`) inside `AtlasSpan` (4004 BC–AD 100);
  - "the client passes what it was given, never a computed year" (§5);
  - next and previous are served `precedes`/`follows` links (§2.2).
- *Consequences:*
  - A year outside the atlas is representable.
  - The client could compute the next year.
  - A year can't be opened as an element.
- *Fix:*
  - Admit `Year` only from the served `Year { value; label; node }`.
  - Keep `compare` and add the span operations the server has: `overlaps`, `meet`, `within`, `adjacent`, `years`.
  - `Years.year` becomes a test generator only.
  - A year opens as an element (`NodeId`); `Position.Years` stays as the client range explorable (server §6.3).

**C3. An event's date diverges, and an undated event can carry a chronology.**
- *Where:* `Model.fsi:48` `Years: YearCoverage`; `Model.fsi:63-65` `Chronology` is always present.
- *Problem:* the server has `Dating { span; precision: Exact | Circa }` or undated (§1.2, §1.3). Its invariant I-CH2 says an undated event has no chronology edge and no concurrent set.
- *Fix:*
  - Use `type When = Dated of Dating * Chronology | Undated` (or `Dating option` plus a chronology only on `Dated`).
  - Add `Precision`.
  - Delete `YearCoverage`, `empty`, `includeSpan`, `union` and `spans` (`Algebras.fsi:21-24`), unless a served consumer is named.

**C4. Passages and containers don't match FOCUS-3 v2, and the shared Bible/Concord interface is missing.**
- *Where:*
  - `Model.fsi:41` `Passage = Scripture | Confession` is a client-built value, not the served passage node.
  - `Model.fsi:51` `Node` has no container case, so a passage is representable twice: as `Position.Passage` (`:59`) and as an `Entity`.
  - There is no Concord document or article, and `WholeChapters.chapter` returns `BibleChapter` (`Algebras.fsi:90`).
  - `References.between` (`Algebras.fsi:29`) admits a one-unit passage; the server refuses `OneUnit` (F3 §3.1).
  - `BiblePassage` merges `PassageReference` and `VerseRangeReference`.
  - There is no `PassageMark`, so "an account is a passage marked as recording history" lives only in prose (`HistoricalAccount` is opaque, `Model.fsi:26`).
- *Fix:*
  - Add `Container = private { Id: NodeId; Level: ContainerLevel; Title: DisplayText; Passage: PassageDetail option }` over both corpora.
  - Add `Passage = private { Span; Mark }`, with spans of at least two units.
  - Make `HistoricalAccount` a passage whose mark is `RecordsHistory`.
  - Let the whole-chapter extent cover any Chapter-level or Passage-level container of either corpus.
  - Delete `References.between`. The client doesn't mint passages; the server's `PassageMint` does.
- *Note for Claude, not Codex:* the server specs disagree with each other. Year §2.8 has multi-run account passages (`MRK.14.54, 66-72`); F3 `Passage` is one span. Claude reconciles that on the server side.

**C5. The cursor types diverge from the signed wire identities.**
- *Where:* `Structures.fsi:22-23` `TextCursor`, `EdgeCursor`.
- *Problem:*
  - WIREID O2 has two cursors, `EdgePageCursor` and `ElementPageCursor`. The Year design adds `WindowCursor`.
  - F3 D3 says outright that the text page's cursor **is** `EdgePageCursor`, "not a third cursor type" (law W7).
  - `ElementPageCursor` and `WindowCursor` are missing.
- *Fix:* use the generated cursor types. Bind each to its query in the page key (`ReadingPageKey`), not in a new cursor type.

**C6. The trail's history invents a server mechanism.**
- *Where:* `Structures.fsi:7-9, 16` (`TrailCursor`, `TrailPage`, `EarlierSteps`, `NeedsEarlierSteps`); `Algebras.fsi:62-63`.
- *Problem:* "server receipts" for earlier steps exist in no server spec. The notes list them as a gap (report:48-50).
- *Fix:* follow owner Q3. The recommended answer is a bounded trail that drops its oldest steps, with `BackOutcome = Backed of Trail | AtStart`.

## Important

**I1. Each door returns a failure union far wider than it can fail.**
- *Where:*
  - `Positive.create` returns `DomainFailure`, which has 10 cases (`Algebras.fsi:10`, `Model.fsi:72-82`), though it can only be `NonPositive`. `Years.between` (`:17`) and `References.between` (`:29`) are the same.
  - `ArrayIndices.create`, `Coordinates.latitude` and the other admission doors share `BoundaryFailure` (`BoundaryModel.fsi:69-80, 88-114`). So `latitude` can "fail" with `NegativeIndex`.
- *Problem:* the type admits failures the door can never produce.
- *Fix:* give each door its own failure type, in the door's file (Part 2 §0).

**I2. "Scrolling equals stepping" is not modelled.**
- *Where:* `ReadingWindow` (`Structures.fsi:26, 32`), `Focus` (`:43`) and `Trail` (`:10`) are three structures with no operation or law joining them (`Algebras.fsi:79-88, 114-119`).
- *Problem:* the server guarantees S1 (spine and chain agree), and FOCUS-3's B3 requires one focus change. A text page never crosses its container, so moving into the next chapter is a focus change.
- *Fix:* add one `Focus.change` function that the arrows and scrolling both call, and the law `scrollInto (next c) = step Next c`. State it as values, with a source law that the focus changes in one place.

**I3. The trail can't be fed from what the focus shows.**
- *Where:* `Neighbours.positions: NeighbourWindow -> Position list` (`Algebras.fsi:99`), but `Trails.transition` needs a `Rooted<Edge>` (`:60`).
- *Problem:* the edges the trail needs are not exposed by the neighbour window.
- *Fix:* `Neighbours.entries: NeighbourWindow -> NonEmpty<Rooted<Edge> * Position>` (or a list).

**I4. "At the same time" is held whole.**
- *Where:* `ConcurrentEvents` (`Model.fsi:30, 63`); `History.concurrent` (`Algebras.fsi:46`).
- *Problem:* the server serves it as the paged window read (Year §2.5–2.6), with no stored set.
- *Fix:* it is a `NeighbourWindow`-like window over the event's dating, keyed by `WindowCursor` (owner Q6).

**I5. A Year's contents are opaque.**
- *Where:* `YearContext`, `SpanContext` (`Model.fsi:28-29`).
- *Problem:* the server defines:
  - the facets `Event | Alive | Office | Era | Map | Polity | Written` with counts;
  - offices `King | Judge | Prophet`, with a person able to hold several (owner YEAR Q2).
- *Fix:* model `YearContext = { Year; Counts: (Facet * Count) list }` and `OfficeTerm` from the generated vocabulary.

**I6. Some names are not the domain's words.**
- *Where:*
  - `StoryStep.Earlier/Later` (`Model.fsi:64`): a story can look back in time, so these should be `Previous`/`Next`.
  - `Passage = Scripture | Confession` (`:41`), against Bible/Concord everywhere else.
  - `BibleVerse`, `BibleChapter`, `BiblePassage` and `ConcordParagraph` (`:16-21`), against the signed wire names.
  - `Direction = Outward | Inward` (`:36`): the relations already have served names ("precedes"/"follows").
- *Fix:* owner Q5. The recommended answer is one vocabulary.

**I7. Some laws are false of the signature, or can't be written against it.**
- *"Resident step/back inverse"* (spec:23). It fails when the trail is full, because a step evicts the oldest step. Restate it: `current (back (step x t)) = current t`, and `back (step x t) = t` when `|steps t| < capacity`.
- *"Renew twice on the same complete root is idempotent"* (report:34). It can't be written: `renew` returns a `Trail`, and no function takes a `Trail` back to a `Renewal` (`Algebras.fsi:64`). Either add that function or drop the law.
- *"Append/prepend are not inverses after eviction"* (spec:24). This is weaker than the server's B2 (`back ∘ slide = id` on held units).
  - The stronger law holds under W6 (pure pages): for a full window `w`, let `p` be the page at `after w` and `q` the page at `before (append p w)`. Then `prepend q (append p w) = w`.
  - State that law; the "not inverses" sentence only covers being handed some other page.

**I8. Important laws are missing.**
- `Nodes.dual` (`Algebras.fsi:40`) has no involution law (`dual ∘ dual = id`).
- Symmetric relations (`Spouses`, `Parallel`, `Analogue` and others in the contract's `symmetric` list) make `{ Spouses; Outward }` and `{ Spouses; Inward }` two values for one thing.
  - Fix: `RelationDirection = Directed of Relation * Direction | Symmetric of Relation`, generated from the vocabulary.
- `References.compareBible` (`:27`) depends on "admitted compiler ordinals" that no served field carries. The wire `VerseReference` is a string, and the order of `BookId` in its enum is not a stated contract guarantee.
  - Fix: either name the served ordinal or state the enum-order guarantee as a contract law.
- The accessors have no laws: `Trails.requestedPositions` (it should equal the trail's positions, in order), `PageCaches.entries` (count ≤ capacity), `Focuses.frontier` (total ≤ the frontier capacity), and `Nodes.entityKinds` and `relations` (each equals its generated vocabulary).

**I9. TextUnit is missing what the rulings need.**
- *Where:* `Model.fsi:47`.
- *Problem:* `TextUnit` has no parts (`Heading | Text | Bracket`), so "brackets hidden by default with a toggle" can't be typed. It also lacks the event heading above a verse, and the `edge_summary` that decides "only paragraphs with somewhere to go are clickable" (F3 B7, B8).
- `Id: NodeId` and `Reference: UnitReference` can disagree, and checking them would mean parsing the id.
- *Fix:* add the parts, the heading and the summary. Document that the id/reference pair is accepted as served, or keep only one of them.

**I10. The reading key doesn't match the server's text read.**
- *Where:* `Reading = Bible of BibleSelection * TranslationId | Concord of ...` (`Structures.fsi:19`, `Model.fsi:43`).
- *Problem:*
  - The server reads `/api/node/{container}/text` with only cursor, limit and extent. There is no translation, and a book container answers an empty page (F3 D3).
  - So `Reading.Bible (Book …)` and a verse-keyed reading are states the server doesn't serve.
  - Putting a translation in the key splits the cache for the same units.
- *Fix:* `ReadingKey = { Root; Container: NodeId }`. Translation is a presentation choice of layer.

## Minor

**M1. The library survey is partly real.** (report:86-98)
- *Real:* the packages exist (nuget.org, 2026-10-03):
  - `Microsoft.OpenApi` and `.YamlReader` 3.10.2 (MIT);
  - `Thoth.Json.Core` 0.9.1 and `Thoth.Json.System.Text.Json` 0.4.0 (MIT, **both pre-1.0**);
  - `FsToolkit.ErrorHandling` 5.2.0 stable (6.0 is in beta);
  - `Bolero` 0.25.65 (Apache-2.0).
- *Missing:*
  - versions for Thoth, FsToolkit and Bolero, and Bolero's licence;
  - `FsCheck` 3.4.0, which all the property laws will run on;
  - the WASM/AOT fit, which is stated as an inference with no spike.
- *Hand-rolled without a written rejection* (standing rule #1): `NonEmpty` (`FSharpPlus.NonEmptyList` 1.9.1 exists), `Validation` (`Model.fsi:85`, `Algebras.fsi:121-123`; FsToolkit has `Validation`, though it isn't nonempty), and the immutable LRU cache.
  - The reasons are probably sound (FSharpPlus's size and trimming; the nonempty guarantee). Write them down.

**M2.** `Edge` (`Model.fsi:55`) drops what an edge records: parentage, votes, the story step, provenance.
- Only `justifies` may end at an edge (contract `EdgeRecord`, `PositionRef`), but `Subject`/`Object: ElementId` lets any relation do so.
- Fix: carry the served metadata, and type edge-to-edge ends as their own case.

**M3.** `Event.Accounts: NonEmpty` (`Model.fsi:48`) will refuse any served event that has no attestation.
- Check the real artifact before adoption. Otherwise the admission door turns a data gap into an unreadable event.

**M4.** `Trails.steps` and `Trails.breadcrumb` (`Algebras.fsi:66-67`) have the same type, and the difference between them is unstated. Merge them, or say what each one is.

**M5.** `TransientFailure.Cancelled` (`BoundaryModel.fsi:62`) is retryable by the "only transient retries" rule, but a user cancel should not retry. Move it to its own case outside both.

**M6.** The cache's root is held twice: `PageCache` is single-root (`Algebras.fsi:108`) and every key also carries a root (`Structures.fsi:20-21`), so "keys differ by root" is vacuous. The unit of `CacheCapacity` (pages or units) is also unstated.

**M7.** `EventId` and `StoryId` (`Model.fsi:9-10`) narrow the wire's `NodeId`. Admit them from the served `kind`, never from the `Event:` prefix (WIREID §1: no narrowing by parsing).

**M8.** The spec page (`specs/2026-10-03-fsharp-domain.md`) is one dense table of 119 names with words like "admitted", "witness", "receipts", "rebase" and "frontier".
- Once the types live in their files (§0), the spec should be a short tour like Part 1 §1, with links. It should restate no type.
