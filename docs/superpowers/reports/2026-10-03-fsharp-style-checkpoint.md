# F# style exemplar checkpoint

2026-10-03, base `2e85a19`, `lane/codex/CX-FSHARP-STYLE`.

The generator now reads a typed immutable contract model, then emits source.
Its public door is `ContractDocument -> Result<GeneratedSource, ContractError>`;
twelve closed failure cases carry typed document paths or library diagnostics.
YAML library objects/exceptions stay in `YamlAdapter`. Reader and emitter use
actual schema/type recursion and put private helpers below their first caller.
The reader uses the Result algebra rather than exceptions or mutation. The
emitter folds immutable definitions rather than appending to a mutable builder.

The SourceEntry decoder returns `WireFailure` values: malformed syntax, invalid
contract value, or null payload. A single shared JSON adapter constructs the
F# converter directly, registering generated identity converters explicitly
before it. The positional cast/exact attribute-type comparison is gone.
Nominal scalar identities are reference DUs, with private constructors and
`HandleNull = true`; no default struct can contain an unchecked null. The five
scalar primitive forms compile and round trip in a generated test fixture.
The existing identity law discovers identities from the same reader model,
rather than a second YAML/string-only predicate. Dependency notes now record
FsCheck/FsCheck.Xunit BSD-3-Clause, FCS MIT and Elmish Apache-2.0.

Verified: **11 exemplar properties pass; 108 existing regression tests pass**,
zero failures/skips. The regression command builds the Debug WASM client and
its generated current OpenAPI model. The previous 109-test suite's parameterless
current-contract example was replaced by the generated complete-document
extension law; the count change is not a lost contract check. Each error-value
property walks all eleven non-YAML failure fixtures per generated input; malformed
YAML exercises the twelfth. Other laws vary field count, required/nullable
combinations, list nesting, enum cardinality, unsupported shapes, primitive
kinds and wire payloads. Existing example tests outside these exemplars are
still present; a whole-client property-only claim would be false.

Commands:

```bash
. /home/donovan/.bible-atlas-env
dotnet test client-fsharp.Tests/StyleGeneration/BibleAtlas.FSharp.StyleGeneration.Tests.fsproj --no-restore --nologo
dotnet test client-fsharp.Tests/BibleAtlas.FSharp.Tests.fsproj --no-restore --nologo
```

Meaningful red evidence is retained below. Reader laws were written before the
reader, against the typed placeholder refusal. Wire laws were written before
the new adapter, against a compatibility wrapper that discarded classification
and structured diagnostics. Disabling the generated HandleNull override then
made the null-refusal law red, confirming it catches the library's null shortcut;
the deliberate change was restored. This is local red evidence, not a full
mutation-gate result. An initial expectation of `$.title` was wrong: the pinned
library reports `$` and a position within the field token. Final laws assert the
whole actual diagnostic, including generated token-length/whitespace positions,
and no finer path is claimed.

## Remaining before style review

This is a pushed checkpoint, **not the completed exemplar set or full parity**.
Still required: one Sources model/message/update/Cmd/view over presentation,
exhaustive transition and bounded-paging laws, source closure over the exemplars,
and comprehensive local/member newspaper-order enforcement. Sources-only glue
paths are named in the ops item. No broad repaint/new feature slice has begun.
The original Vocabulary generator still uses its old JSON parsing/builder/error
style; only its name conversion was adapted to the new checked API. The old
JSON compatibility door renders typed wire faults into the existing textual
`Failure.Contract`; whole-client I8 is still open. SDK-bound FCS N6 is still open.
A-WIRE-IDENTITIES remains owner-blocked; this checkpoint cannot make the client's
currently unnamed wire IDs nominal. Request style/explode/I7 remains unclosed.

**14b:** the contract reader owns scalar classification; the JSON adapter owns
converter registration. The emitter reads typed primitives/optionality/cases,
not YAML. Reviewed shared fields retain their generated contract shapes.
**24a/24b:** this closes the generator's exception-as-domain-error path for the
supported shape reader and the positional JSON converter dependency; it does
not establish every legacy module's closure. Comprehensive source laws, external
model construction/unsupported contract variations and whole-client category
review remain necessary before style sign-off.

No AOT, live differential browser, fresh Rust, full workspace, mutation gate,
or 100% parity result is claimed. Windows C: remains about 3.5 GB free while WSL
reports hundreds of GB free; only small .NET builds ran. CX-I3 on ops records
retention/pruning and host-capacity checks. Durable review evidence is retained;
no raw/cache/worktree or Claude output was deleted. No Codex lock/server remains.

**Handoff:** continue Sources/paging/source-order exemplars in this worktree,
then submit the complete set for Claude review and Donovan style sign-off. Keep
parent CX-FSHARP at its existing checkpoint until these patterns are approved.
A-F39 review was separately published as changes requested (F-85), report
785c9de, with durable production probes; do not consume its root changes yet.

## reader-red.log

```text
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.ReaderLaws.the complete published document remains readable after harmless description changes [102 ms]
  Error Message:
   FsCheck.Xunit.PropertyFailedException : 
Falsifiable, after 1 test (1 shrink) (1511400736537700014,16630849011668829867). 
Last step was invoked with size of 2 and seed of (3101332749803302318,6826378926586505659):
Original:
139uy
Shrunk:
0uy

---- $.components.schemas: no schemas
  
----- Inner Stack Trace -----
   at BibleAtlas.FSharp.Tests.StyleGeneration.ReaderLaws.the complete published document remains readable after harmless description changes(Byte length) in /home/donovan/w/CX-FSHARP-STYLE/client-fsharp.Tests/StyleGeneration/ReaderLaws.fs:line 79
   at InvokeStub_ReaderLaws.the complete published document remains readable after harmless description changes(Object, Span`1)
   at System.Reflection.MethodBaseInvoker.InvokeWithOneArg(Object obj, BindingFlags invokeAttr, Binder binder, Object[] parameters, CultureInfo culture)
--- End of stack trace from previous location ---
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.ReaderLaws.reading preserves generated field types and optionality as whole typed values [28 ms]
  Error Message:
   FsCheck.Xunit.PropertyFailedException : 
Falsifiable, after 1 test (1 shrink) (10211877699308308750,5993457981417674575). 
Last step was invoked with size of 2 and seed of (18249642798722196855,16834879469524040515):
Original:
(156uy, true, true)
Shrunk:
(0uy, true, true)

---- Assert.Equal() Failure: Values differ
Expected: Microsoft.FSharp.Core.FSharpResult`2[BibleAtlas.FSharp.ContractGenerator.ContractModel,BibleAtlas.FSharp.ContractGenerator.ContractError]
Actual:   Microsoft.FSharp.Core.FSharpResult`2[BibleAtlas.FSharp.ContractGenerator.ContractModel,BibleAtlas.FSharp.ContractGenerator.ContractError]
  
----- Inner Stack Trace -----
   at BibleAtlas.FSharp.Tests.StyleGeneration.ReaderLaws.reading preserves generated field types and optionality as whole typed values(Byte count, Boolean required, Boolean nullable) in /home/donovan/w/CX-FSHARP-STYLE/client-fsharp.Tests/StyleGeneration/ReaderLaws.fs:line 25
   at InvokeStub_ReaderLaws.reading preserves generated field types and optionality as whole typed values(Object, Span`1)
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.ReaderLaws.nullable unions retain their element shape and do not flatten nominal identities [2 ms]
  Error Message:
   FsCheck.Xunit.PropertyFailedException : 
Falsifiable, after 1 test (1 shrink) (5615598144391173447,8809304245932356189). 
Last step was invoked with size of 2 and seed of (9325525407455532756,1767439095290563245):
Original:
17uy
Shrunk:
0uy

---- Assert.Equal() Failure: Values differ
Expected: Microsoft.FSharp.Core.FSharpResult`2[BibleAtlas.FSharp.ContractGenerator.ContractModel,BibleAtlas.FSharp.ContractGenerator.ContractError]
Actual:   Microsoft.FSharp.Core.FSharpResult`2[BibleAtlas.FSharp.ContractGenerator.ContractModel,BibleAtlas.FSharp.ContractGenerator.ContractError]
  
----- Inner Stack Trace -----
   at BibleAtlas.FSharp.Tests.StyleGeneration.ReaderLaws.nullable unions retain their element shape and do not flatten nominal identities(Byte depth) in /home/donovan/w/CX-FSHARP-STYLE/client-fsharp.Tests/StyleGeneration/ReaderLaws.fs:line 42
   at InvokeStub_ReaderLaws.nullable unions retain their element shape and do not flatten nominal identities(Object, Span`1)
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.ReaderLaws.reader errors are reachable values with their exact document paths [6 ms]
  Error Message:
   FsCheck.Xunit.PropertyFailedException : 
Falsifiable, after 1 test (9 shrinks) (16156127213302470618,15709013812580771171). 
Last step was invoked with size of 2 and seed of (18334742856978129485,11661371202310855151):
Original:
(185uy, 9uy)
Shrunk:
(1uy, 0uy)

---- Assert.Equal() Failure: Values differ
Expected: Microsoft.FSharp.Core.FSharpResult`2[BibleAtlas.FSharp.ContractGenerator.ContractModel,BibleAtlas.FSharp.ContractGenerator.ContractError]
Actual:   Microsoft.FSharp.Core.FSharpResult`2[BibleAtlas.FSharp.ContractGenerator.ContractModel,BibleAtlas.FSharp.ContractGenerator.ContractError]
  
----- Inner Stack Trace -----
   at BibleAtlas.FSharp.Tests.StyleGeneration.ReaderLaws.reader errors are reachable values with their exact document paths(Byte choice, Byte depth) in /home/donovan/w/CX-FSHARP-STYLE/client-fsharp.Tests/StyleGeneration/ReaderLaws.fs:line 63
   at InvokeStub_ReaderLaws.reader errors are reachable values with their exact document paths(Object, Span`1)
```

## wire-red.log

```text
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.WireLaws.syntax and null failures remain distinct values at the record door [98 ms]
  Error Message:
   FsCheck.Xunit.PropertyFailedException : 
Falsifiable, after 1 test (1 shrink) (3195409681156108794,14226609898975764677). 
Last step was invoked with size of 2 and seed of (14431951445252903847,12487468625130239917):
Original:
215uy
Shrunk:
0uy

---- expected InvalidSyntax, received Microsoft.FSharp.Core.FSharpResult`2[BibleAtlas.FSharp.Contract.SourceEntry,BibleAtlas.FSharp.WireFailure]
  
----- Inner Stack Trace -----
   at BibleAtlas.FSharp.Tests.StyleGeneration.WireLaws.syntax and null failures remain distinct values at the record door(Byte indent) in /home/donovan/w/CX-FSHARP-STYLE/client-fsharp.Tests/StyleGeneration/WireLaws.fs:line 23
   at InvokeStub_WireLaws.syntax and null failures remain distinct values at the record door(Object, Span`1)
   at System.Reflection.MethodBaseInvoker.InvokeWithOneArg(Object obj, BindingFlags invokeAttr, Binder binder, Object[] parameters, CultureInfo culture)
--- End of stack trace from previous location ---
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.WireLaws.valid JSON with the wrong field type is a typed value failure [6 ms]
  Error Message:
   FsCheck.Xunit.PropertyFailedException : 
Falsifiable, after 1 test (2 shrinks) (3308572525105703920,4944313650388700513). 
Last step was invoked with size of 2 and seed of (1332775902431515786,5735476027153718169):
Original:
-2
Shrunk:
0

---- Assert.Equal() Failure: Strings differ
            ↓ (pos 1)
Expected: "$.title"
Actual:   "$"
  
----- Inner Stack Trace -----
   at BibleAtlas.FSharp.Tests.StyleGeneration.WireLaws.valid JSON with the wrong field type is a typed value failure(Int32 number) in /home/donovan/w/CX-FSHARP-STYLE/client-fsharp.Tests/StyleGeneration/WireLaws.fs:line 30
```

## handle-null-red.log

```text
  Failed BibleAtlas.FSharp.Tests.StyleGeneration.IdentityLaws.every nominal converter receives and refuses null instead of silently returning a default [8 ms]
  Error Message:
   FsCheck.Xunit.PropertyFailedException : 
Falsifiable, after 1 test (1 shrink) (14675914702436256293,4304700202359397301). 
Last step was invoked with size of 2 and seed of (10092768365617659429,16971881893349542107):
Original:
225uy
Shrunk:
0uy

---- Assert.Throws() Failure: No exception was thrown
Expected: typeof(System.Text.Json.JsonException)
  
----- Inner Stack Trace -----
   at BibleAtlas.FSharp.Tests.StyleGeneration.IdentityLaws.actual@35-1.Invoke(Type shape) in /home/donovan/w/CX-FSHARP-STYLE/client-fsharp.Tests/StyleGeneration/IdentityLaws.fs:line 36
   at Microsoft.FSharp.Primitives.Basics.List.map[T,TResult](FSharpFunc`2 mapping, FSharpList`1 x) in /__w/1/s/src/fsharp/src/FSharp.Core/local.fs:line 247
   at Microsoft.FSharp.Collections.ListModule.Map[T,TResult](FSharpFunc`2 mapping, FSharpList`1 list) in /__w/1/s/src/fsharp/src/FSharp.Core/list.fs:line 97
```
