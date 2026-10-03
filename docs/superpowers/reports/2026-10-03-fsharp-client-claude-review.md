# CX-FSHARP: Claude's review of the F# client, work in progress

> Owner, 2026-10-03: "Our principles have not changed. Readability and clarity and capturing everything under the type system are still our priorities."

That sentence is this review's first lens, and the findings are ranked by it.

- **Reviewed:** `origin/lane/codex/CX-FSHARP` at `9adb89e` (code `86c9232`, Reader slice `dff4ef1`), diffed from base `ace063d`.
- **Scope:** `client-fsharp*/`, `tests/parity-fsharp/`, `scripts/fsharp-client/`, and the spec, plan and foundation report. The A-NOBLURB merge (`e57542c` via `9855596`) was reviewed and approved separately, so it is out of scope.
- **Method:** read-only. Nothing on Codex's branch was changed.

## Verdict: on track with changes

The foundation is sound:

- Closed generated vocabularies.
- A pure `update` that returns typed effects through one Cmd interpreter.
- No mutable state outside Elmish.
- Stale completions are refused.
- F-81 is closed at the one JSON door.
- No comments, no symlinks and no checked-in outputs.

Three things need to change now, before more slices are built on top of them:

1. Domain identities are still bare `string`s and `int`s.
2. The model is a product of per-surface fields, so illegal combinations can be represented.
3. The parity ledger cannot tell whether the two clients behave the same. So the completion gate it feeds is not a parity gate.

All three are cheap to fix at about 1,100 lines of F#, and expensive at full parity.

**Counts: 3 Critical, 10 Important, 12 Minor.**

## Independent gates (Claude, 2026-10-03)

- **F# tests:** `dotnet test client-fsharp.Tests` gives 104 passed, 0 failed, 0 skipped. The build has zero warnings, with TreatWarningsAsErrors on.
- **Comments:** I grepped every added line under `client-fsharp*`, `tests/parity-fsharp` and `scripts/fsharp-client`. There are no comments. The only "comment" is the single header line of each generated file, which is the allowed exemption.
- **Symlinks:** `find . -type l` prints 0.
- **Checked-in outputs:** none are tracked (`git ls-files` shows no `bin/`, `obj/`, `*.g.fs` or publish output). Assets are shared through MSBuild `Content Link` + `ContentRoot` (`client-fsharp/BibleAtlas.FSharp.Client.fsproj:17-21`), not by copies or links.
- **Package licences,** from nuspec metadata in `~/.nuget/packages`:

| Package | Licence | Note |
|---|---|---|
| FSharp.SystemTextJson 1.4.36 | MIT | |
| YamlDotNet 18.1.0 | MIT | |
| FSharp.Core 10.1.401 | MIT | |
| bunit 2.11.3 | MIT | |
| Elmish 4.0.1 | Apache-2.0 | THIRD-PARTY.md wrongly says MIT; see M4 |
| Bolero / Bolero.Build 0.25.65 | none in the nuspec | The upstream repository is Apache-2.0; see M4 |
| @playwright/test 1.62.1 | Apache-2.0 | |

  Every licence is on the permitted list.
- **Bundle size:** I published both clients with Release settings into my scratch directory. The table shows the set referenced by `dotnet.js`. See "Feasibility" below.

## Critical

### C1. Domain identities are bare primitives. The generator turns the contract's named ids into aliases. (Type system)

**Where:**

- The generator turns a schema with no properties into a type abbreviation: `client-fsharp.ContractGenerator/Generator.fs:134` emits `and EraId = string`, and the same for `NarrativeId` and `PolityId`. An abbreviation is not a type: any string is accepted wherever an `EraId` is expected.
- The generated output also has 147 bare `: string` fields. These include every `Id`, every artifact-root `Version`, and the text reference `TextWindow.Next: string option`. Page cursors are `Next: int option` and `Previous: int option`.
- The hand-written core carries the same primitives forward:
  - `Failure.ArtifactMoved of resolved: string * serving: string` (`client-fsharp/Core/Loading.fs:8`).
  - `Resolved` roots typed `string` (`client-fsharp/Core/Exploration.fs:21-22`).
  - `Graph.Reading.Cursor: int option` and `Root: string option` (`client-fsharp/Core/Graph.fs:9-10`).
  - `Route.Concord of string option`, a Concord reference (`client-fsharp/Core/Model.fs:15`).
  - `Positions.id: PositionRef -> string`, which compares node ids and edge ids as one string space (`Exploration.fs:8-11`).

**Defect:**

- A root can be passed where an id is expected.
- A cursor can be passed where a chapter is expected.
- A Concord reference can be passed where a Bible reference is expected.

Each of these compiles. That is exactly the "`String` where the vocabulary is closed" that rule 14b names, and the owner's sentence above asks for the opposite.

**Fix:**

1. *(Codex, now)* Have the generator emit a single-case wrapper with a JSON converter for every named scalar schema, for example `[<Struct>] type EraId = EraId of string`, instead of an abbreviation. Add a generator test that fails if any `type X = <primitive>` survives.
2. *(Owner / Claude, contract side; record as a finding, do not fix on the side)* Name the unnamed identities in `contracts/openapi.yaml`: `ArtifactRoot` (every `version`), `PageCursor` (`next`/`previous`/`cursor`), `NodeId`/`EdgeId`, and `BibleReference`/`ConcordReference` (`ref`, `TextWindow.next`, `ContentsChild.ref`). The generator then wraps them with no hand-written code.
3. Retype `Failure.ArtifactMoved`, `Resolved`, `Graph.Reading` and `Route.Concord` with those types.

### C2. The model is a product of per-surface fields, so illegal states can be represented. (Type system, readability)

**Where:** `client-fsharp/Core/Model.fs:66-72` holds `Route`, `Contents: Map<Corpus, LoadState<Contents>>`, `ReadingSession`, `Sources` and `Focus` side by side.

**Illegal states it can represent:**

- `Route.Sources` together with `ReadingState.Active`.
- `Route.Read` together with a Concord session.
- `ReadingState.Unavailable of RequestId * Failure`, which carries a request id that was never sent. `Model.fs:181` stamps `model.Serial` on a failure that made no request. This contradicts the spec's "carries its failure without an invented request". `Model.Reading` (`:73-77`) then projects it as `Failed(identity, …)`.

**Readability:**

- A reader has to cross-reference three fields to know what the Reader page shows.
- `View.readingState` (`client-fsharp/View.fs:109-112`) merges the contents failure into the reading state inside the *view*.
- `ReadNext` has to enumerate nine Route × LoadState combinations to do nothing (`Model.fs:115-116`).
- `Retry` is one global message. It resets *every* failed load in the model, including off-screen ones, then branches on the reading session (`Model.fs:117-128`).

**At full parity:** World, Kretzmann, time, map, split and selection will each add another independent field, so the illegal combinations multiply with every route.

**Fix:**

1. Make the page a sum type keyed by route:
   ```fsharp
   type Page =
       | ReaderPage of ReaderState
       | ConcordPage of ConcordState
       | SourcesPage of LoadState<SourcesDocument>
       | WorldPage
       | ...
   ```
   Each state is its own closed union. For example, `ReaderState = AwaitingContents of RequestId | NoSuchChapter of ReadingLocation | Reading of ReadSession<TextWindow> * ContentsChild`.
2. Give each page its own message type (`ReaderMessage`, `ConcordMessage`, `SourcesMessage`), and make the top-level `Message` `Page of PageMessage | Focus of FocusMessage | Navigate of Route`.
3. Make each Retry name what it retries.
4. Keep `Contents` as a shared, bounded cache outside `Page`.
5. Delete `Model.Reading` and `View.readingState`.

### C3. The parity ledger and the parity tests cannot detect a behavioural difference. (Parity method)

**Where:** `scripts/fsharp-client/parity-ledger.py:17-21,38,52-55`, `tests/parity-fsharp/ledger.json`, `tests/parity-fsharp/*.spec.ts`.

**Defects:**

- **The unit is a file hash, not a behaviour.** The inventory lists every file under `client/` plus `tests/ux/*.ts`: 235 entries. A file becomes "verified" when its `replacement` and `evidence` lists are non-empty (`:52-55`). Nothing checks that the evidence names an existing passing test. One "verified" `client/Exploring/Presenter.cs` would hide every behaviour inside it.
- **Shared assets count as finished when nothing uses them.** 18 entries are marked `shared-asset` purely by byte equality (`:38`). Five of them are application JavaScript the C# client drives through interop: `js/map.js`, `js/reader.js`, `js/lazyProse.js`, `js/border-morph.js` and `js/geo.js`. The F# `wwwroot/index.html:15-16` loads none of them, and no F# interop exists. They are counted as not pending although nothing in the F# client uses them.
- **Behavioural sources are left out:**
  - `client.Tests/` (110 files of C# behavioural laws).
  - `tests/ux/CONTRACT.md`, the UX scenario contract.
  - `tests/ux/perf-probe-verse-frontier.mjs`.
- **No test runs the C# client.** `boot.spec.ts` and `reading.spec.ts` run the F# client against hand-written mocks and fixtures, and compare it to the test's own expectations. They are good F# acceptance tests, but they do not compare the two clients.

**Fix:**

1. Make the inventory behavioural, with one item for each of the following:
   - each `tests/ux` scenario (each `test(...)`, not each file);
   - each C# behavioural law in `client.Tests`;
   - each route × interaction × failure state in `tests/ux/CONTRACT.md`.
2. Keep the file hashes only as a change detector that reopens the items they cover.
3. Mark the JS modules `pending` until F# interop drives them.
4. Make `verified` require an evidence id that the script resolves to a passing differential test.
5. Add a differential harness that runs both clients against one live server on configurable ports. It should compare normalised DOM, accessibility tree, URL/history and request logs for each scenario. Run it in Chromium and WebKit, which the spec requires.

## Important

### I1. The F# client has no closure laws for rule 25. (Rules 25, 24b; F-75)

**Where:** `client-fsharp.Tests/BibleAtlas.FSharp.Tests.fsproj:3` lists the test files; none of them is a usage or source law.

**Defect:** The C# client has `GeneratedUsageTests`, and F-75 already records that a type-level check is too weak. The F# client has neither a type-level nor a field-level usage law. It also has no source law that would stop domain parsing, formatting or arithmetic from being added.

**Fix:**

1. Add a reflection law that walks every generated record field and union case. It should fail when a field is not read by the client assembly, apart from a declared, reviewed "unread" list. That is the F-75 closure, done right the first time.
2. Add a source law over `client-fsharp/**/*.fs` that refuses `Int32.Parse`/`TryParse`, `Split`, `Regex` and `$"…{…Chapter}"`-style locus formatting outside one named module (`Routes`).

### I2. The view joins, formats and drops served data itself. (Rule 25, readability)

**Where and defect:**

- `View.fs:114-133`, `readerTitle`: joins Contents roots to the window's first unit by `Book`. On a miss it silently falls back to `unit.Node.Label`, so an Option hides a missing case.
- `View.fs:130`: formats the chapter number with `string locus.Chapter`, although the served `ContentsChild.Title` already holds that label.
- `View.fs:102,138,153`: formats the verse number from `locus.Verse`.
- `View.fs:68-76`, `sources`: groups sources client-side with `List.groupBy _.Category`. A source whose category is not listed is dropped without a trace.

**Fix:**

1. When the model opens a reading, keep the served `ContentsRoot`/`ContentsChild` that matched it in the reading state (C2's `Reading of … * ContentsChild`). Render their served titles, and make a miss an explicit state.
2. Ask the contract (as a finding) for a served unit label for verse numbers, or record the verse number as the accepted client presentation.
3. For sources, render all of them, or fail explicitly on an unlisted category.

### I3. The view computes the presentation, and shows a contract failure as a network failure. (Type system, readability)

**Where:** `View.fs:331-333` calls `Presenter.popover` on every render. Its `Error` is shown through `failed (Traverse Traversal.Renew)`, which reads "Couldn't load this — check your connection" (`:378`).

**Defect:**

- A served `TextUnit` without `UnitText` is a contract fault, but it appears as a connection problem.
- Its Retry re-reads the same server data, so it can never succeed.
- The fault is never in `FocusState`, so no test of `update` can see it.

**Fix:** Present once, in `update`, when the trail arrives:

- `FocusState.Opened of Trail * PopoverPresentation`
- `FocusState.CouldNotPresent of Trail * Failure`

Then render contract failures differently from transport failures, and offer no Retry for them.

### I4. `AnchoredText.runs` hides invalid served offsets. (Partial function made total)

**Where:**

- `client-fsharp/Core/AnchoredText.fs:10`, `utf16 scalar = offsets[max 0 (min scalar …)]`, clamps out-of-range offsets.
- `:22`, `List.tryPick`, keeps only the first of two overlapping anchors.

**Defect:** A malformed `UnitText` renders without any error. This hides the same class of contract violation that the JSON door refuses.

**Fix:**

1. Return `Result<TextPiece list list, Failure>`.
2. Refuse `start > end`, out-of-range and overlapping spans as `Contract` failures, or get an owner ruling on whether overlaps are legal and model them.
3. Add a property test over random spans.

### I5. The exploration trail and its renewal grow with the journey. (Rules 27e, 27f; spec)

**Where:**

- `Exploration.fs:60`: `Trail.follow` appends with `Steps @ [step]`, which is O(n) per step.
- `Exploration.fs:105`: `renew` resolves *every* walked position in one `GET /api/elements?ids=…`. That includes the duplicates added by each Back (`:126`) and Resume (`:137`).

**Defect:** Request size and URL length grow without limit as a session goes on. A long journey will produce a URL the server or a proxy rejects (414). There is no law at future size for this, although the spec says "no cursor history grows with the journey".

**Fix:**

1. Store `Steps` in reverse.
2. Make renew resolve the *distinct* identities (`Positions.sameIdentity`), batched under a fixed URL budget.
3. Add a 10,000-step law asserting a bounded number and size of requests, like the 10,000-page Concord law.

### I6. Wire spellings are read back through JSON round trips. (Stringly vocabulary, D.R.Y.)

**Where:**

- `Model.fs:24`, `Json.decode<BookId>(Json.encode book)`.
- `Model.fs:42`, `JsonSerializer.Deserialize<string>(Json.encode location.Book)`.
- `Runtime.fs:14`, `JsonSerializer.Deserialize<string>(Json.encode corpus)`.
- `Reads.contents` takes a plain `string`, because `contracts/openapi.yaml:149-153` declares the `corpus` path parameter as `type: string` and not `$ref: Corpus`.

**Fix:**

1. Have the generator emit `toWire: T -> string` and `ofWire: string -> T option` for every closed union.
2. Record a contract finding: the `/api/contents/{corpus}` parameter should reference `Corpus`, so that `Reads.contents: Corpus -> Request<Contents>` is generated.

### I7. Generated reads are positional, and allow requests the contract refuses. (Readability, type system)

**Where:**

- Callers read like `Reads.textWindow reference (Some 20) (Some WindowDir.Onward) None (Some Corpus.Concord)` (`Model.fs:113,180`, and throughout the tests).
- That signature permits `scope=chapter` with `corpus=concord`, which the contract says is `bad_scope`.
- `Generator.fs:146` comma-joins every array parameter whatever the parameter's `style`/`explode` says. It also ignores `default`, `minimum` and `maximum` (`:150-165`).

**Fix:**

1. Generate one parameter record per operation, with named fields.
2. Honour `style`/`explode`, or refuse an unsupported combination as a generator error. Test both.
3. In the core, wrap text reads in a closed `Reading = ChapterOf of ContentsChild | Paragraphs of ConcordReference * count | Continue of …`, so that a refused combination cannot be built.

### I8. Failures are strings, and every failure reads as "check your connection". (Type system)

**Where:**

- `Loading.fs:5-8`: `Transport of string | Contract of string`.
- `Api.fs:16-20`: flattens the HTTP status and the served `ErrorBody.Error.Code` (a generated closed `ErrorCode`!) into `$"{int status}: {message}"`.
- `View.fs:378`: renders one message for every failure.

**Defect:** A `not_found` or `bad_ref` from the server is shown, and offered a Retry, as if it were a network fault. The typed refusal code is thrown away at the door.

**Fix:**

```fsharp
type Failure =
    | Unreachable of string
    | Refused of status: int * ErrorCode * message: string
    | Malformed of string
    | ArtifactMoved of ArtifactRoot * ArtifactRoot
```

The view then matches on the case to choose its wording and whether to offer Retry.

### I9. The tests are examples, not laws, and `update` is only partly covered. (Tests; rules 15, 16)

**Where and defect:**

- **The "monad laws" are single examples.** `client-fsharp.Tests/ExplorationTests.fs:21-35` checks left identity, right identity and associativity with `Explore.result 7`, `Explore.here` and pure functions only. No effectful action (`follow`, `renew`), no failing action and no generated actions are tested. That is a sample, not a law.
- **There is no property-based testing anywhere.** FsCheck is MIT-licensed, so it would be allowed.
- **Message × state pairs with no test:**
  - `Retry` from `Unavailable`;
  - a failed `ContentsLoaded`;
  - a stale `SourcesLoaded`;
  - `Traverse` from `CouldNotWalk`;
  - `RetryFocus` on `CouldNotWalk`;
  - a successful `FocusLoaded` while `Walking`;
  - `Navigate` while Focus is open, or while a Concord page is pending.
- **No round-trip law for routes.** `Routes.parse ∘ Routes.url` has no law over Concord references (`+`, `%`, `&`).
- **Rule 15 is broken in places.** `StateTests.fs:27-42` (Cache) assert hand-picked tuples, not whole values. `ContractTests.fs:22-29` and `TransportTests.fs:21-27` assert only `Result.isError`, not the whole failure.
- **Rule 17 is broken.** The literal `Some 20` is repeated in the tests (`ModelTests.fs:14,69,130,142,176,190`) where a named constant should be used.

**Fix:**

1. Add FsCheck generators for `Message`, `Route` and `Explore` actions over a fake `Explorer`.
2. Turn the identity and associativity cases, and these model invariants, into real laws:
   - a stale completion is a no-op for every message;
   - every emitted effect carries the current request id;
   - `parse ∘ url = id`.
3. Assert whole values throughout.

### I10. The red-before-green evidence cannot be checked.

**Where:** the foundation report cites `/tmp/codex-fsharp-*-red.log`, which `AGENTS.md` names as where this evidence lives.

**Defect:** On 2026-10-03 none of the cited red logs exists any more. Only an uncited `/tmp/codex-fsharp-newspaper-red.log` remains. So I cannot verify that each test was seen failing first.

**Fix:** For each red, put the failing test name and the first failure line into the report, or into the ledger's `evidence`. A `/tmp` path disappears.

## Minor

- **M1. `Cache` is dead code** (`client-fsharp/Core/Cache.fs:1-22`). It is tested but nothing in the client uses it, which is the "kept for later" that rule 4 forbids. Delete it until the element cache that uses it lands. Its future-size test belongs to that cache.
- **M2. No cancellation.**
  - **Where:** every effect passes `CancellationToken.None` (`Runtime.fs:15-21`, `Graph.fs:20`).
  - **Defect:** A superseded read keeps running after `Navigate`, `CloseFocus` or a newer traversal. The plan lists "cancellation laws" but nothing implements them.
  - **Fix:** Give each request id a `CancellationTokenSource`, held by the interpreter rather than the model, and cancel on supersession.
- **M3. The committed defaults are Codex's personal ports.**
  - **Where:** `client-fsharp/wwwroot/appsettings.Development.json:1` (8100), `tests/parity-fsharp/playwright.config.ts:11-15` (5100), `scripts/fsharp-client/serve-published.py:17` (5100).
  - **Defect:** Any other agent running these gates, or a side-by-side run under `heavy` (which binds 8000/5000), has to edit them.
  - **Fix:** Read ports from the environment, with no agent-owned default.
- **M4. Licence records.**
  - Elmish 4.0.1 is **Apache-2.0** (its nuspec says so), not MIT as `client-fsharp/THIRD-PARTY.md:3` states.
  - The Bolero 0.25.65 nuspec carries no licence metadata at all. Cite the repository LICENSE at tag `v0.25.65` as the evidence.
  - Root `LICENSES.md` still needs reconciling before the client lands.
- **M5. Not-found URLs are rewritten.**
  - **Where:** `Main.fs:25-27` and `Model.fs:49`.
  - **Defect:** An unknown URL is rewritten to `/not-found`. The C# client (`client/App.razor:1`, `NotFoundPage`) renders at the unknown URL and keeps it. The C# `FocusOnNavigate Selector="h1"` behaviour is also missing.
  - **Fix:** Keep the URL the user typed for NotFound; add focus-on-navigate.
- **M6. Kinds tested with `=` instead of an exhaustive match.**
  - **Where:** `View.fs:161,239` (`anchor.Node.Kind = NodeKind.Person`) and `View.fs:249` (`heading.Kind = EventKind.General`).
  - **Defect:** A new kind silently takes the default style.
  - **Fix:** Use `NodeKind -> MentionStyle` and `EventKind -> HeadingStyle` with exhaustive matches.
- **M7. The generator is hard to read.**
  - **Where:** `Generator.fs:43,104-143` uses `ResizeArray`, `let mutable index` with a `while` loop, code assembled from strings, `.Replace("and  ", "and ")` (`:121`), and `scalar >> (=) "true"` (`:94`).
  - **Fix:** Parse the YAML into a closed `Schema` union (`Enum | Record | Tagged | Scalar | …`), then print it with a pure fold. This also makes C1 and I7 easier.
- **M8. The Concord page size is declared twice.** `Model.fs:100` (`concordPageSize = 20`) restates `client/Pages/Concord.razor:152`. It is a client presentation choice, so this is not a rule-26 problem, but the fact now has two homes. Either share one declaration, or have the contract publish it as `default`, and generate it.
- **M9. Package versions differ between the two clients.** The F# client pins `Microsoft.AspNetCore.Components.WebAssembly` 10.0.12 (`client-fsharp/BibleAtlas.FSharp.Client.fsproj:12-13`); the C# client pins 10.0.11. Side-by-side comparisons should use the same runtime.
- **M10. `let rec … and private …` chains** in `View.fs:9`, `Model.fs:102`, `Presentation.fs:16`, `Runtime.fs:11` and `Graph.fs:14`. They exist to satisfy rule 18's newspaper order, but they read as recursion, which they are not. Ask the owner for a ruling on newspaper order in F#: idiomatic F# reads bottom-up.
- **M11. Staleness is detected by comparing whole trees.**
  - **Where:** `Model.fs:132` compares the old and new `LoadState<Contents>` structurally.
  - **Defect:** Each check deep-compares the whole Contents tree.
  - **Fix:** Have `LoadState.complete` return `Accepted of LoadState<'a> | Stale`.
- **M12. Prose is copied from the C# client.** `View.fs:29,62,175` copy the C# client's prose ("Bible Explorer", the Sources and Concord intros). Until C# retires, every copy edit has to be made twice. Record each one in the ledger, so a C# change to the text reopens it.

## Feasibility and risk

### Bundle size and startup

Both clients were published with Release settings. The figures count only the files that `dotnet.js` references.

| Build | Files | Raw | Brotli | Largest |
|---|---|---|---|---|
| C# client (complete, non-AOT) | 45 | 10.6 MB | 3.18 MB | |
| F# client (one slice, non-AOT) | 62 | 13.5 MB | 4.49 MB | FSharp.Core.wasm 2.4 MB, not trimmed |
| F# client, AOT (Codex's publish of `86c9232`) | 62 | 36.6 MB | 8.3 MB | dotnet.native.wasm 26 MB |

- The F# client is already **41% larger brotli-compressed than the whole C# client**, before most features exist. With AOT it is **2.6×** the size of the C# client.
- Nobody has measured startup time. The completion gates should include it: time to first meaningful paint and first-read latency, for both clients, cold and warm, in Chromium and WebKit, with a budget the owner rules on.

**Options:**

- `InvariantGlobalization`, if the served text needs no ICU. Both builds ship ICU data now.
- Trimming FSharp.Core, using `TrimMode` and IL-trimming analyzers.
- Deciding whether AOT pays for its download size. The owner should rule on this.

### Tracking FOCUS-3 to FOCUS-9

FOCUS-3 to FOCUS-9 will rewrite most of `client/Legacy` (20 files) and much of `client/Exploring` and `client/Components`. If the F# port follows the C# client file by file, it will port code that FOCUS is about to delete, and then port it again.

**Recommendations:**

1. Mark each Legacy entry "superseded by FOCUS-n" rather than `pending`, and port only the post-FOCUS shapes.
2. For each FOCUS change, take the C# change *and* its F# change in the same review cycle, so the behavioural ledger (C3) never drifts by more than one batch.
3. Ask the owner whether new FOCUS work should land in F# first once the F# client has the focus surface. Otherwise every FOCUS batch is built twice.

### What will hurt at full parity

- The model's product-of-fields shape (C2).
- Bare identities (C1).
- The view-side joins (I2).

All three grow with each new route. Map interop (Leaflet plus five custom JS modules) has not been started; it is the riskiest remaining slice and should come before the remaining text surfaces. Split/follow and storage migration (C# `LocalStore` and `ViewStateService` formats) must read the C# client's existing saved data, so a migration test must load C#-written payloads.

## What is good

- **Elmish discipline:** a pure `update` returns `Effect list`, and one interpreter (`Runtime.command`) turns effects into Cmd. No Http, NavigationManager or mutable state is reachable from Core. `Back` makes no transport call, and that is tested.
- **Closed vocabularies are generated from `contracts/openapi.yaml`.** Edge duality is generated from `graph-vocabulary.json` (`Vocabulary.fs`), with a law that it is involutive and matches the fixture. Unsupported shapes are generator errors, never `obj` or `JsonElement`. Inexhaustive matches fail the build.
- **F-81 is closed properly on the F# side,** at the one JSON door. It is backed by a law that walks all 79 generated records (84 optional fields as exactly `None`, 245 required fields refused when omitted or null). This is the closure shape rule 24b asks for, and the C# client should copy it.
- **Stale and duplicate completions cannot change state,** through request ids (`LoadState`, `ReadSession`, `FocusState`). A failed page's Retry re-sends exactly the same typed request. The 10,000-page Concord law checks the whole model.
- **The graph resolver refuses** root changes, missing elements, identity substitution, nonterminal empty pages and extra pages. It never accepts a partial journey.
- **F-65 is not copied.** The F# client parses no served reference string: Concord `next` is passed through opaque, and chapters are found through served `Contents` loci, not by formatting a reference.
- **Process:** there are no comments, no symlinks and no checked-in output. Assets come in through MSBuild links. The C# client is untouched, and the browser tests use whole-value assertions over real fixtures.
