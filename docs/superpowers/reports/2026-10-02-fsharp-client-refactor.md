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
