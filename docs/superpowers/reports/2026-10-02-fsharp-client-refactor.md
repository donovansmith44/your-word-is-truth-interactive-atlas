# CX-FSHARP review refactor checkpoint, 2026-10-03

Reference review: Claude's `9dd57f0`, `2026-10-03-fsharp-client-claude-review.md` and its refactor guidance. This checkpoint addresses the already named identity portion of C1, starts the requested newspaper-order/property migration, and removes M1's unused cache. The task remains claimed. It is not a full rewrite or parity verdict.

## What changed and what closes

The failed abstraction was primitive type aliases for nominal identities. Every named scalar schema now generates a distinct private single-case struct union and a generated JSON converter. The current published identities are `EraId`, `NarrativeId` and `PolityId`. Ordinary callers cannot construct them from strings or substitute one identity type for another; decoding preserves their primitive wire representation and refuses null or the wrong JSON primitive kind. The generated converter owns that boundary. Json's F# converter delegates schemas with their own standard JsonConverter attribute to those generated converters; it retains the configured F# option/tagged-union behavior for the other types.

A schema-derived property enumerates every currently published scalar identity, checks its private constructor/field type, roundtrips generated string payloads, and refuses the entire null/object/array/boolean/number input family. Generator properties additionally vary names and all five primitive representations (string, int, int64, float, bool). The optional/required field properties vary complete record payloads and enumerate every generated record field, so the identity change is checked inside records too.

C1 remains open for unnamed identities, roots, cursors and references. The proposed `A-WIRE-IDENTITIES` queue item belongs to the contract lane; Codex did not edit the shared contract or invent a second client vocabulary. Contract constraints beyond the currently unconstrained named IDs remain part of the later boundary work.

The SDK compiler parses every authored F# file in the migration paths and the two generated files. The order law checks module-level private helpers and test fixtures against their first same-scope caller. Generated Reads helpers and test fixtures now descend from their callers. Generated-identifier properties prove the gate refuses ascending helpers/fixtures and accepts descending ones, including a fixture accessed through a qualified record field. Cached JSON options initialize lazily without suppressing initialization warnings. The artificial `let rec`/`and` function groups in Model, Graph, Presenter, Runtime and View have been replaced with separate, typed bindings in recursive module/namespace scopes.

This order law is a checkpoint, not a complete proof of newspaper order: public dependency ordering, local bindings and class-member ordering still need the full sweep. The generator's mutable intermediate representation remains M7 work. Fifteen properties now cover generator, decoding, loading and source order; the remaining example tests still need conversion into meaningful generated or exhaustive laws. The dead Cache abstraction, compile entry and its example tests were removed because no application code used it.

## Durable red evidence

- `named scalar schemas generate distinct immutable identities`: before nominal generation, FsCheck falsified the whole generated-source expectation after one test, shrinking the identity suffix to zero; the primitive alias differed from the expected private union.
- `every published scalar identity has its own private constructor and preserves its wire value`: after nominal generation with the library's unwrapped-union converter, the malformed-input assertion failed after one generated string, shrinking to `NonNull ""`. **Expected `[True, True, True, True, True]`; actual `[False, True, True, True, True]`.** JSON `null` was accepted. The generated converter plus correct converter precedence makes that category green.
- `private helpers and test fixtures follow their first caller in every FSharp source`: the initial red found 30 ascending helper/fixture sites. A later red still named generated Reads helpers before their callers; after moving those, the remaining fixture failures exposed that qualified references were not counted correctly.
- `the order gate finds a fixture used through a served record field`: the gate's own red shrank to suffix zero: **expected an ascending `served0` fixture violation; actual `[]`**. Reading the head identifier in a qualified local reference fixes both the synthetic false negative and the real fixture false positives.

These test names and first failure lines are retained here; correctness evidence no longer depends only on cleared `/tmp` files.

## Verification and practical limits

`DOTNET_PROCESSOR_COUNT=4 nice -n 10 dotnet test client-fsharp.Tests --no-restore -m:4`: **107 passed, zero failed/skipped**. This builds the generator, generated core, and Debug Blazor WASM application. The narrowed generator property run also passed all five properties. No new application comments or C# application references were introduced. FsCheck/FsCheck.Xunit 3.4.0 declare BSD-3-Clause in their installed package metadata; FSharp.Compiler.Service is referenced from the installed SDK rather than adding an incompatible second FSharp.Core package.

No fresh Release/AOT publish, full browser suite, live differential parity or mutation gate is claimed for this checkpoint. Claude holds heavy for A-F39; its workspace gate is not touched. Existing mocked/browser evidence belongs to the earlier checkpoint and does not establish current behavioral parity. C2's surface product, C3's hash-based ledger, boundary/presentation/trail findings and the unimplemented feature slices remain open, in Claude's requested order.

## Disk hygiene

The owner reported WSL exhaustion. `CX-I3` records that host capacity must be checked as well as guest filesystem capacity, and that compact mutation outcomes/reports must survive pruning of disposable targets/results. During this session C: had approximately 2.3 GB free while WSL reported approximately 793 GB free. Those are different capacity constraints.

Only stopped, owned generated outputs `client-fsharp/obj/Release` (325,290,494 bytes) and `client-fsharp/bin/Release` (240,398,597 bytes) were removed after checking tracked files, process command lines and open descriptors. This frees about 566 MB in the guest; no Windows VHD compaction or host-space recovery is claimed. Sources, durable reports, raw/cache data, worktrees and Claude's active outputs were preserved. Earlier review cleanup removed its own stopped 444 MB partial Rust target. Large new Rust/AOT growth stays deferred while host space is critical.

## Handoff

Continue C1 through the named-wire contract proposal when authorized/landed, and begin C2's closed surfaces and operation-specific messages/retries now; finish the remaining boundary, presentation/trail, behavioral parity and closure work before new feature slices. Preserve newspaper order and convert tests to generated laws alongside each change. The full 100% parity goal remains active. No Codex server or lock is held.

## C2 surface checkpoint, 2026-10-03

The application now has one `Surface` (`Reader`, `Concord`, `Sources`, `World`, `Kretzmann`, `NotFound`) rather than simultaneous Route/Contents/ReadingSession/Sources fields. Route is derived from the surface. Private ReaderPage and ConcordPage construction plus read-only projections keep page states behind the transition boundary; a private-constructor law covers both types. The earlier direct View field reads fail with compiler **FS1093** after construction is hidden, and the view now uses only the projections. Tests use reflection only in explicit expected-value fixtures.

Each surface has typed messages; RetryContents, RetryText and Sources Retry are separate operations. A private shared reading transition implements Reader/Concord behavior; Concord alone accepts Next. Runtime routes each corpus completion to its corresponding message, and the view renders the closed ReadingState without Model.Reading or View.readingState. Unavailable has no request identity. Contents and text requests have separate tickets. Text answers with any opposite-corpus unit are refused, and contents answers must name the requested corpus. Same-route Navigate preserves the live state/read instead of sending another request. Focus keeps its existing orthogonal state machine.

The contents cache is currently removed rather than kept dormant: only the active reading owns its served Contents, and navigation reads it again. Shared root-aware caching belongs to the later validated boundary/resource work. This is a deliberate interim tradeoff; a network/performance parity claim is not made. The top-level focus message algebra/routing, complete matching-message transition enumeration and the rest of I9 still need the follow-up sweep.

### C2 durable reds

- `a completion from a departed surface cannot alter any replacement surface`: before the sum type, FsCheck failed after one generated title, shrinking to `NonNull ""`: **Assert.Equal collections differ**. The old hidden Sources field changed from Loading to Ready in every replacement route. There is no off-screen Sources field now.
- `retry with no failed operation preserves every surface and pending request`: before operation-specific messages, FsCheck shrank to `NotFound`: **expected RequestId 0L; actual RequestId 1L**, even with no failed operation or emitted effect.
- `a Reader text answer from another corpus is refused before entering reading state`: before corpus validation, a generated Concord UnitText sent to Reader was accepted. **Expected an Active session with Failed(Contract "the text answer names a different corpus"); actual Ready containing the Concord unit.** The input shrank to empty text. The generalized green property now exercises both corpora.
- `navigating to the current route preserves its pending request instead of sending it twice`: before same-route refusal, **expected the existing serial; actual the next serial**, after one generated route (NotFound). The real Bolero Sources entry test also exposed duplicate reads: its one-response handler was reused for a second request and the Sources category never rendered. A route no-op fixes the entire duplicate-navigation category without increasing the test timeout.

### C2 verification and remaining scope

Full current F# suite: **109 passed, zero failed/skipped**, including **38 properties**. Every ModelTests test is now a generated or finite enumeration law. The cross-surface matrix spans **19 state fixtures × 16 message values**, asserting all **214 foreign-surface pairs** are whole-model/effect no-ops; the noncurrent-completion law checks every fixture and completion variant with varied request identities, failures and focus. Matching startup/contents failure/sources success/retry/Next/stale/focus transitions have whole-value properties, plus escaped Route round trips and four generated journeys of at least 10,000 Concord turns. Source-order gate and Debug WASM compilation pass. No new application comments or edits to C# code, contracts, data or Claude's outputs.

Remaining example tests, local/public/member newspaper ordering, typed HTTP failures (I8), anchor refusal (I4), generated enum spellings/request records (I6/I7), presentation/trail fixes (I2/I3/I5), behavioral parity/usage closure (C3/I1), cancellation/root-aware working sets and all unimplemented features remain open. AOT, mutation and browser differential gates are not rerun or claimed. C: now reports about 3.5 GB free, still critical; the contract lock belongs to Claude/A-PROVENANCE and is untouched.
