# F# newspaper-order integration

Author continuation on `lane/codex/CX-FSHARP-wire`, starting at **f32b616**.
The isolated order coverage checkpoint **79671ab** is now consumed into the
actual current app's test project. Parent 88f0d26, frozen STYLE 35f6797,
producer/C#/contract/data and Claude worktrees remain untouched.

The expanded syntax gate passes across **66 authored .fs files and both generated
.fs files**. All **13 order properties**, the full **167-test suite**, and Debug
WebAssembly build pass: no failures/skips or build warnings/errors. Both generated
outputs match their entire pre-refactor SHA-256 hashes. This is a verified order
integration checkpoint, not whole-domain sign-off, complete semantic order closure,
property-only tests, side-by-side parity or C# retirement.

## Failed category and every migrated site

The old gate checked only private module helpers and shared test data. It missed
public/internal helpers, member/local functions and qualified calls. The expanded
FCS ParsedInput.fold visitor initially reported 27 candidates. Two fresh generated
properties exposed further gaps: a helper call inside a nested/destructured let
could be missed or assigned to a later fixture, and a shadowing lambda argument
could be mistaken for the outer helper. Both properties failed before their gate
repairs. The visitor now selects the enclosing caller in the callee's lexical
scope and inspects lambda argument patterns, including typed arguments.

Migrated the resulting category across Generator, Vocabulary, Rooted, Explorable,
Explore, Trail, ReadSession, TextRuns, the App command interpreter and presentation
fixtures. Generator's captured helper functions move below generate, receiving
the same definition/schema inputs explicitly; their algorithms and mutable emitter
are retained. Text offset conversion and date-claim fixtures likewise move below
their caller, without a second copy of their logic. App.Commands is a private
member below Program, shared by init/update. Public domain entries precede their
helpers. Existing module recursion supports genuine forward declarations; no
invented self-call or recursive algorithm was added to evade the gate.

Explicit complete generic signatures preserve polymorphism under forward
references. The library stack's eager values produced FS0040 initialization errors.
The domain operations are now functions:

```fsharp
Explore.here : unit -> Explore<Explorable>
Explore.back : unit -> Explore<Explorable>
Explore.renew : unit -> Explore<Explorable>
```

The [actual algebra](../../../client-fsharp/Core/Exploration/Explore.fs) is the
single signature home; interpreter and test callers are migrated. The operations
still produce the same walks, arrivals, dual Back step and root renewal. Their
bodies are delayed by the async interpretation, with no warning suppression or
unsafe recursive object initialization. The seven preexisting monad/retention
properties and the normal command/model/view checks remain green. Links and other
pending bodies were not filled before domain sign-off.

## Existing library fit checked

Reuse pinned FCS 43.12.401 (MIT) for syntax traversal, FSharp.Core for collection
operations, YamlDotNet 18.1.0 (MIT) for the existing generator input, and
FSharpPlus 1.9.1 (Apache-2.0) for the existing Reader–State–Result monad. No package
or new parser/formatter was installed. Attribution remains in THIRD-PARTY.md.

Checked the [upstream ReaderT delay API](https://fsprojects.github.io/FSharpPlus/reference/fsharpplus-data-readert-2.html)
and the installed DLL. A standalone ReaderT over Async succeeds, but the **actual
Reader–State–Result–Async stack throws NotSupportedException: dynamic Delay is not
supported**. Two existing whole-result/CE-delay tests caught this candidate; an old
async command consequently did not deliver its completion. The attempt was removed,
including its private alias/member. The existing small async delay adapter remains
because the surveyed library operation does not fit this concrete stack. The corrected
full suite completes in 28 seconds. The rejected old run was verified live by PID,
its exact session/log path and descendants, then those own processes were stopped;
its incomplete output is retained and is not reported as a completed test count.

## Current gates, reproducible evidence and limits

[Evidence](evidence/2026-10-03-fsharp-order-integration) retains original order
red, nested/lambda reds, scoped green, complete current normal green, build log,
generated hashes, all authored-source hashes and current unused inventories.
After sourcing ~/.bible-atlas-env in the worktree:

```sh
dotnet test client-fsharp.Tests/BibleAtlas.FSharp.Tests.fsproj --filter FullyQualifiedName~SourceOrderTests
dotnet test client-fsharp.Tests/BibleAtlas.FSharp.Tests.fsproj
dotnet build client-fsharp/BibleAtlas.FSharp.Client.fsproj
python3 scripts/fsharp-client/declarations.py
```

The final command remains **RED**, with zero compiler errors, **61 pending bodies**
and **294 unused findings**. Its first current run reported 295, including an
unneeded generator loop binder. Replaced that binder with a discard and rechecked:
294 remain. This scanner is not a whole-screen-root reachability proof; its known
dead-island, transitive/generic coverage and exemptions need closure before sign-off.
The previous 294 count pinned to parent 88f0d26 has been replaced by actual current
evidence here; it is not a waiver.

The order gate uses FCS syntax and lexical scopes. It covers the public/private/
internal, qualified, nested binding, member, local-helper, parameter/lambda-shadow
and data-vs-helper classes its generated properties enumerate. It does not prove
arbitrary pattern/match shadowing, aliases, overload resolution, cross-module/type
ordering or complete compiler symbol resolution. These limits stay open; a green
syntax gate does not silently certify them. Types, module data and local input
bindings precede computations that require them, as the data controls assert.

D.R.Y.: functions moved rather than copied; generated bytes and whole-result
behavior checks retained; no copied producer types or hand-written identity doors.
Haskell bar: concrete generated root/cursor/reference types retained, no casts,
partial admission or extra domain arithmetic added; actual unused/open string
failure categories remain explicit debt. Category pass: source-order entry/helper
relations were extended and migrated together; API primitive calls migrated in
all authored clients/tests; existing monad delay retained only after library fit
failed on the actual stack. No comments added to application code.

Next close the remaining semantic-order coverage and unused reachability, remaining
example tests and structured Failure/frontier skeleton before whole-domain review.
Then resume the feature/parity ledger. No Codex lock/server or active build remains.
No new Rust/AOT/browser/mutation run, bulk cleanup, retention automation or VHD
compaction occurred; CX-I3 retains the owner-requested stopped-output pruning policy.
