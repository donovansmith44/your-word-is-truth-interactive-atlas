# CX-FSHARP: Claude's second review, increment `9adb89e..7da4d70`

> Owner, 2026-10-03: "Readability and clarity and capturing everything under the type system are still our priorities."

- **Reviewed:** `origin/lane/codex/CX-FSHARP`, one commit, `7da4d70` ("Generate private scalar identities with checked JSON boundaries and property laws"). 24 files, +450 / −275.
- **Against:** the first review (`2026-10-03-fsharp-client-claude-review.md`, C1–C3, I1–I10, Minor) and the refactor guidance (`2026-10-03-fsharp-client-refactor-guidance.md`, its order and "done when" lines).
- **Method:** read-only. Nothing on Codex's branch was changed.

## Verdict: the right direction, but the refactor has barely started. Changes are still required.

- **None of the 13 findings is fully addressed.** Three are partly addressed (C1, I9, I10). Ten are untouched. None is worse.
- **Codex followed the order.** It added no feature slice. C1 came first. The other work in the commit answers the owner's own steering: newspaper order, property tests, and the M1 and M10 minors.
- **C1 protects nothing the client uses yet.** The three identities now wrapped (`EraId`, `NarrativeId`, `PolityId`) appear in no client source file. Every identity the client actually handles is still a `string` or an `int`: node and edge ids, artifact roots, cursors and references. The rest of C1 waits on the contract item A-WIRE-IDENTITIES. That item is now on the critical path and needs an owner ruling.
- **The two changes that matter most are still open.** C2 (the model as a sum of surfaces) and C3 (behavioural parity) have not started. Neither has the boundary step (I8, I4, I6, I7).
- **New in this increment:**
  - The JSON door now depends on a positional downcast (N1).
  - The property count overstates the laws (N3).
  - Two definitions of "scalar identity" disagree (N4).
  - The new test dependencies are not recorded (N5).

## Independent gates (Claude, 2026-10-03)

- **F# tests:** `dotnet test client-fsharp.Tests`: **107 passed, 0 failed, 0 skipped.** The build is warning-free with TreatWarningsAsErrors on. This matches Codex's claim.
- **Comments:** I grepped every added line of the diff for `//` and `(*`. Nothing was added, apart from the allowed generated-file header.
- **Symlinks:** `find . -type l | wc -l` prints 0.
- **Checked-in outputs:** `git ls-files` shows no `bin/` or `obj/`.
- **Licences:**
  - FsCheck and FsCheck.Xunit 3.4.0 are BSD-3-Clause in their nuspecs. That is permissive, so it is allowed.
  - FSharp.Compiler.Service comes from the SDK and is MIT.
  - Neither is recorded (N5).
- **Laws seen red first:** the report records four reds with their test names and first failure lines (`docs/superpowers/reports/2026-10-02-fsharp-client-refactor.md:17-24`). That is real progress on I10. The converted and new properties have no recorded red (N3).

## Progress table

| Finding | Status | Evidence at `7da4d70` |
|---|---|---|
| **C1** Bare primitive identities | **Partly** | Named scalar schemas now generate a private single-case struct with a checked converter (`client-fsharp.ContractGenerator/Generator.fs:103-106,120,139,150-153`). A property enumerates them (`client-fsharp.Tests/ContractShapeTests.fs:47-58`). Still bare: `Failure.ArtifactMoved of string * string` (`client-fsharp/Core/Loading.fs:8`), the `Resolved` roots (`Core/Exploration.fs:21-22`), `Graph.Reading.Cursor: int option` and `Root: string option` (`Core/Graph.fs:45-46`), `append (root: string)` (`Graph.fs:32`), and `Route.Concord of string option` (`Core/Model.fs:15`). None of the new types is used by client code. The guidance's "done when" source law is absent. The unnamed identities are blocked on A-WIRE-IDENTITIES (contract side). |
| **C2** Model is a product of fields | **Untouched** | `Model` still holds `Contents`, `ReadingSession`, `Sources` and `Focus` side by side (`Core/Model.fs:66-72`). `ReadingState.Unavailable of RequestId * Failure` still invents a request id (`:64,179`). There is one global `Retry` (`:81,115-125`). `View.readingState` still merges states in the view (`client-fsharp/View.fs:113`). |
| **C3** Ledger cannot detect behaviour | **Untouched** | `scripts/fsharp-client/` and `tests/parity-fsharp/` are unchanged in the diff. The ledger is still file hashes, with five unused JS modules counted as shared assets. There is no differential harness. |
| **I1** No rule-25 closure laws | **Untouched** | The new `SourceOrderTests.fs` is a newspaper-order gate, not a rule-25 or field-usage law. There is still no law against parsing, prefix filtering or unread generated fields. `Int32.TryParse` is still at `Core/Model.fs:24`. |
| **I2** View joins, formats and drops | **Untouched** | `readerTitle` (`View.fs:118`), `string locus.Chapter` (`:134`), the verse number from `locus.Verse` (`:106`), and `List.groupBy _.Category` (`:72`). |
| **I3** View presents; contract fault reads as network fault | **Untouched** | `Presenter.popover` is called during render (`View.fs:336`), and its `Error` still renders as "check your connection" (`:382`). |
| **I4** `AnchoredText.runs` hides bad offsets | **Untouched** | It still clamps with `max 0 (min …)` (`Core/AnchoredText.fs:10`) and keeps the first overlap with `tryPick` (`:22`). The diff only moves a fixture in `AnchoredTextTests.fs`. |
| **I5** Trail and renew grow with the journey | **Untouched** | `Steps @ [step]` (`Core/Exploration.fs:60`). `renew` still resolves every walked position (`:103-106`). There is no long-journey law. |
| **I6** Wire spellings via JSON round trips | **Untouched** | `Json.decode<BookId>(Json.encode book)` (`Core/Model.fs:24`), `Deserialize<string>(Json.encode …)` (`Model.fs:42`, `client-fsharp/Runtime.fs:14`). The generated `Reads.value` round trip moved to the bottom of the module but is unchanged (`Generator.fs:175`). |
| **I7** Positional reads permit refused requests | **Untouched** | `Reads.textWindow reference (Some concordPageSize) (Some WindowDir.Onward) None (Some Corpus.Concord)` (`Core/Model.fs:111,178`). Arrays are still comma-joined whatever `style`/`explode` says (`Generator.fs:175`). |
| **I8** Failures are strings | **Untouched** | `Transport of string \| Contract of string` (`Core/Loading.fs:6-7`). The status code and served `ErrorCode` are still flattened into a string (`Core/Api.fs:20`). One message for every failure (`View.fs:382`). |
| **I9** Examples, not laws | **Partly** | FsCheck was added. `StateTests.fs` is now four real properties, and the contract-shape laws vary their payloads. However, the monad "laws" are still single examples over `Explore.result 7` (`client-fsharp.Tests/ExplorationTests.fs:11-24`). There is no `Message` × `Model` enumeration and no `parse ∘ url` law. `Some 20` is still repeated (`ModelTests.fs:58,60,119`). New and old assertions still check only `Result.isError` (`ContractShapeTests.fs:43,57`, `ContractTests.fs:23,28,47`, `TransportTests.fs:23`). See N3. |
| **I10** Red evidence uncheckable | **Partly** | New reds are durable, with names and first failure lines, in the committed report (`2026-10-02-fsharp-client-refactor.md:17-24`). The original slice's reds are still lost. The converted laws record no red. |

**Totals: 0 addressed, 3 partly addressed, 10 untouched, 0 worse.**

### Minor findings

| Minor | Status | Evidence |
|---|---|---|
| **M1** Dead `Cache` | **Addressed** | `Core/Cache.fs`, its compile entry, its tests and its spec block are all deleted. |
| **M10** `let rec … and private` chains | **Addressed, owner-ruled** | The owner ruled newspaper order. Codex replaced the chains with `module rec` / `namespace rec` and separately typed bindings (`Model.fs`, `Graph.fs`, `Presentation.fs`, `Runtime.fs`, `View.fs`). |
| **M4** Licence records | **Untouched, and wider** | Elmish is still recorded as MIT (`client-fsharp/THIRD-PARTY.md:3`). See N5. |
| **M7** Generator readability | **Worse** | The `while`/`mutable index` loop remains (`Generator.fs:108-109`). The new converter is one nine-line string literal (`:153`). |
| **M2, M3, M5, M6, M8, M9, M11, M12** | **Untouched** | `CancellationToken.None` (`Runtime.fs:15-17`, `Graph.fs:14`). Ports 8100/5100 are still hard-coded. `Kind = NodeKind.Person` / `EventKind.General` comparisons (`View.fs:165,243,253`). `concordPageSize = 20` (`Model.fs`). WebAssembly 10.0.12. |

## Order

**Codex followed the guidance's order.**

- No feature slice landed.
- The only C-step work is C1, the first item in the order.
- The non-ordered work (the source-order gate, the property conversion, M1 and M10) answers the owner's steering of 2026-10-03, which outranks the guidance.
- The commit's plan addendum restates the order (`docs/superpowers/plans/2026-10-02-fsharp-client.md:16-18`).

**The cost is pace.** One checkpoint closed none of the 13 findings. C2 is the change every later item builds on, and it has not begun. The guidance asked for C2 straight after C1. C1's remaining part is blocked on the contract, so C2 should start now rather than wait for C1 to finish.

## New findings, ranked by the owner's priorities

### N1 (Important, readability and type system): the one JSON door depends on a positional downcast

**Where:** `client-fsharp/Core/Json.fs:16-22`.

**Defect:** Codex replaces `read.Converters[0]` with a wrapper.

- The wrapper assumes FSharp.SystemTextJson put its factory first, and `:?> JsonConverterFactory` throws if it did not.
- It then decides whether a type "owns" its converter by testing `attribute.GetType() = typeof<JsonConverterAttribute>` exactly. That test excludes the subclass `JsonFSharpConverterAttribute` without saying so.
- So the boundary that F-81 closed now rests on a library's internal ordering, and on an exact-type test the reader has to decode.

**Fix:**

1. Have the generator emit its identity converters as a list.
2. Register that list explicitly in the read options, ahead of the F# converter.
3. Alternatively, use the library's documented override hook.

No downcast and no attribute-type equality should be needed.

### N2 (Important, type system): the new identities guard nothing the client touches

**Where:** no file under `client-fsharp/` names `EraId`, `NarrativeId` or `PolityId`.

**Defect:** C1's illegal cross-identity uses still all compile, because every identity in the client's flow is unnamed in the contract.

There is also a smaller hole. The wrappers are `[<Struct>]`, so a default instance exists (`Unchecked.defaultof`, `Array.zeroCreate`) that wraps a null. The converter refuses exactly that value at the door.

**Fix:**

1. The owner rules on A-WIRE-IDENTITIES now. It is the critical path for C1.
2. Until it lands, record that C1 provides no protection today.
3. Drop `[<Struct>]`, or state why a default instance is acceptable.

### N3 (Important, laws): the property count overstates the laws

**Where and defect:**

- **Examples labelled as properties.** `[<Property(MaxTest = 1)>]` on parameterless tests (`client-fsharp.Tests/SourceOrderTests.fs:12`, `GeneratorTests.fs:18`) is an example under another name.
- **Variation that does not touch the behaviour.** The generator properties mostly vary a `uint16` name suffix, which is not what the law is about (`GeneratorTests.fs`, the "closed vocabulary" and "unsupported shape" laws).
- **The claim:** "15 properties" is reported as progress on I9. Codex's report does say that example tests remain (`2026-10-02-fsharp-client-refactor.md:15`). Still, the laws I9 named are examples or absent: the monad laws, the `update` transitions and the route round trip.
- **No red for the converted laws.** For example, `StateTests.fs` "beginning a new request preserves the last complete value" is a new law with no recorded red.

**Fix:**

1. Use `[<Fact>]` where nothing varies.
2. Vary the inputs the law is about: schema shapes, `Message` × `Model`, generated `Explore` actions over a fake `Explorer`.
3. Record a red for each new law, or a deliberate mutation it catches.

### N4 (Important, D.R.Y. and closure): two definitions of "scalar identity"

**Where:**

- The generator's `isScalarIdentity` accepts string, integer, number and boolean (`Generator.fs:103-106`).
- The law's own YAML walk accepts only `type: string` (`ContractShapeTests.fs:90-101`), and it asserts the field type is `string` (`:57`).

**Defect:** A future integer identity would be generated and silently skipped by the law. That is the opposite of an enumerating closure.

**Fix:** Discover the identities from the generated assembly, as private single-case unions carrying their own `JsonConverter`, or reuse the generator's predicate. Then assert each one's own primitive.

### N5 (Minor, licensing): the new test dependencies are not recorded

**Where:** `client-fsharp/THIRD-PARTY.md:5` still says the tests use "xUnit, bUnit and Playwright".

**Defect:** FsCheck and FsCheck.Xunit 3.4.0 (BSD-3-Clause) and FSharp.Compiler.Service (MIT) are now test dependencies but are not listed. Elmish is still recorded as MIT; it is Apache-2.0 (M4).

**Fix:** Record all three, and correct Elmish.

### N6 (Minor, reproducibility): the compiler service is an unpinned SDK-internal reference

**Where:** `client-fsharp.Tests/BibleAtlas.FSharp.Tests.fsproj:8` references `$(MSBuildBinPath)/FSharp/FSharp.Compiler.Service.dll`.

**Defect:** Its version moves with whichever SDK is installed, so the source-order gate is not pinned.

**Fix:** Pin a package version compatible with the SDK's FSharp.Core, or record this as an accepted owner exception.

### N7 (Minor, readability): `rec` scopes everywhere, with a gate that checks only part of the order

**Defect:**

- `module rec` / `namespace rec` now covers every source and test file. That gives up F#'s compile-order guarantee in exchange for newspaper order. The owner chose that trade.
- The gate checks only private helpers and test fixtures. Codex says so, and makes no wider claim (`2026-10-02-fsharp-client-refactor.md:15`, `docs/superpowers/plans/2026-10-02-fsharp-client.md:18`).

**Fix:** Extend the gate to public bindings, local bindings and members before claiming that every file complies, as Codex already plans.

## Next, in order

1. **C2 now.** Use closed surfaces and per-surface messages, with each Retry naming its operation. Add the `Message` × `Model` enumeration law. Do not wait for A-WIRE-IDENTITIES.
2. **The boundary: I8, then I4, I6 and I7.** N1 should be fixed with I6, because both are about the one JSON door.
3. **I2, I3 and I5, then C3 and I1.** Make I9's properties the real ones as you go (N3, N4).
4. **Owner:** rule on A-WIRE-IDENTITIES (N2).
