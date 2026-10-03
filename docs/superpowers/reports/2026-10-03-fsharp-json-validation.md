# Typed JSON failure checkpoint and validator fit

The JSON door now returns distinct syntax, null-answer and unreadable-answer
failures with the library's exact optional path, line and byte observations.
`Failure.Contract of string` is removed. Current JSON/graph/HTTP failure
producers no longer turn evidence into free-form failure prose. The scoped
category went from **8 failures / 10 passes** to **40 passing properties**;
the final normal suite passed **190 properties / zero failures or skips**.
Debug WASM builds with zero warnings/errors. This is a correction checkpoint,
not whole-domain approval, complete schema validation or 100% parity.

Base: `5748bf9`, producer `da7e00d`, lane `lane/codex/CX-FSHARP-wire`.
Ops `8586327`/`60cea89` record the source-license correction, measured native
fit and authorized JSON error migration. No C#, contract, artifact, data,
Claude worktree, new feature/view or pending domain body changed. Pending
bodies remain **59**. Evidence is in
[evidence/2026-10-03-fsharp-json-validation](evidence/2026-10-03-fsharp-json-validation/manifest.json).

## Category and the shared door

The failed abstraction was the conversion from library errors into application
failures: it discarded observations and exposed prose as a public constructor.
The client owns interpreting its JSON transport at one library boundary.
Serialization and generated contract types remain the existing
System.Text.Json/FSharp.SystemTextJson implementation.

Actual types and signatures are in [WireFailure](../../../client-fsharp/Core/Admission/WireFailure.fs),
[Failures.wire](../../../client-fsharp/Core/Admission/Failure.fs) and
[Json](../../../client-fsharp/Core/Admission/Json.fs); the
[actual-module domain tour](../specs/2026-10-03-fsharp-domain.md) links them
rather than declaring another model.

`Json.decode` first uses the SDK JSON parser to distinguish invalid JSON and
whole-answer null. It disposes the parsed document, and the installed serializer
reads the original body for typed deserialization. Reading the original body
preserves the serializer's original whitespace coordinates for primitive
failures. This makes two library reads of successful JSON; no speed or frame
budget claim is made here. Encoding options and generated output bytes are
unchanged. No hand-written JSON/parser/schema/discriminator implementation or
new application package was added.

All existing callers already use this door. Its error becomes terminal
`ReadFailure.InvalidAnswer` through one shared `Failures.wire` wrapper.
The HTTP missing-field expectation and complete failure-vocabulary law migrate
with it. The compiler accessibility law proves a public read construction
compiles and `Failure.Contract "raw reason"` fails with FS0039. The older
Transport constructor remains forbidden. Reflection pins the complete current
Failure and WireFailure payload vocabulary.

The old, unimplemented semantic JSON cases and JsonKind/IdentityFailure
vocabularies are removed: this adapter cannot claim MissingField, UnknownCase,
WrongKind or InvalidIdentity from a message it does not interpret. It reports
what the selected library supplies. JsonErrorLocation holds observations;
it does not promise a complete JSON path, positive coordinate, or a coordinate
in the original document. Optional fields stay optional, with no invented
fallback value. Paths/TextPosition remain separate unfinished proposal APIs,
not a representation supplied by this serializer.

A measured limit matters: a nested F# field converter reports `$`, line 0 and
the byte length of its numeric fragment, rather than `$.label` in the complete
record. The initial stronger expectation fails in
`json-location-characterization.log`; the final property states and preserves
the actual fragment observation. This is not a fabricated field path or a
claim that precise nested diagnostics are solved.

Closure here is **raw failure-prose construction and migration of every current
producer**. It is not full JSON/schema semantics, typed UI error presentation,
retry/controller cancellation I8/I3/M2, or the whole domain's rule-24b closure.

## Existing validator survey, measured fit and license correction

The [previous survey](2026-10-03-fsharp-graph-failures.md) remains the candidate
comparison. This continuation executes the first candidate, using the original
approved OpenAPI 3.1 document and draft 2020-12. The spike adds only a root `$ref`
to select a component; it does not alter the published component schemas.
All **35** response fixtures are classified and evaluated. Unknown fixture
names fail the test harness rather than being silently omitted.

[JsonSchema.Net 9.4.0](https://docs.json-everything.net/schema/basics/) exposes
[structured evaluation results](https://docs.json-everything.net/api/JsonSchema.Net/EvaluationResults/).
The repository source is
[MIT at the pinned commit](https://github.com/json-everything/json-everything/blob/399f198431f65cf6896fe6038f833ef6d0b27a39/LICENSE).
However, license inspection found OSMFEULA on the downloaded JsonSchema.Net,
JsonPointer.Net and Json.More.Net NuGet binaries. An initial binary-package
restore succeeded before F# compilation failed; no behavioral tests executed
with that reference. It was replaced with ProjectReferences to the pinned MIT
source checkout. The NuGet binary license is not described as MIT and no owner
license exemption is assumed. The earlier survey row and task attribution now
distinguish source from packaged binaries.

The executed candidate compiles JsonSchema 9.4.0, JsonPointer 7.0.2 and
Json.More 3.0.1 from upstream commit
`399f198431f65cf6896fe6038f833ef6d0b27a39`. The only source-checkout changes are
three SourceLink build-tool version edits, 10.0.201 to 10.0.401. The original
restore reported NU1902 for Microsoft.Build.Tasks.Git 10.0.201;
the [primary advisory](https://github.com/advisories/GHSA-23fw-v26w-5fgq)
identifies 10.0.303 as the patched 10.0 release. The pinned
[SourceLink 10.0.401 package](https://www.nuget.org/packages/Microsoft.SourceLink.GitHub/10.0.401)
is used for this source build. No upstream C# library code changes, warning
suppression, packing or publishing occurred. Git-clean LF comparison accounts
for this checkout's CRLF conversion.

Source license, exact patch, source/build/package metadata and hashes are
retained. New source-build dependencies are permissive: Humanizer.Core 3.0.10,
PolySharp 1.15.0 and SourceLink/Build.Tasks.Git 10.0.401 have MIT package metadata.
The latter two are build-only. Existing YamlDotNet 18.1.0 reads YAML for the
native spike. None of these validator dependencies is added to the application.

| Measurement | Executed result |
| --- | --- |
| All 35 committed responses, original schemas | Accepted |
| Every NodeRef required field, omitted/null | Refused |
| Generated valid/invalid ArtifactRoot and both ElementId branches | Original patterns distinguish them |
| Closed NodeRef extra property | Refused |
| Nullable headings, whole TextWindow fixtures, generated valid roots | Accepted |
| Invalid NodeRef id | Full errors include root `properties` and `/id` `pattern`, without reading message text |
| Incomplete TextRef selected by `corpus=bible` | Base schema accepts; concrete BibleRef and generated serializer refuse |

The first compiling fit run was **7 pass / 2 fail**. One failure demanded
concrete discriminator validation from a base schema which does not express
that validation; the other expected a leaf-only error list instead of the
library's full parent-plus-leaf hierarchy. Those are candidate-fit/diagnostic
expectation findings, not production behavioral fixes. The final **9 native
properties pass**, including the compiled property-only gate and an explicit
base/concrete/serializer acceptance comparison. This does not bless the
candidate as a sole OpenAPI decoder. The upstream OpenAPI discriminator
keyword implementation at this commit only produces an annotation; it does not
validate the selected branch. No producer defect is filed for that behavior.

Next, measure contract-derived schema/type binding plus the existing typed
serializer, nested union/closed-object enforcement and bounded WASM costs.
Identity pattern enforcement remains absent from the live serializer; the
native fit is not an application enforcement claim. Do not duplicate schema
patterns, edit the producer schema to accommodate the client, or replace the
serializer with a less strict candidate.

## Reproduce the native fit

Run from the lane worktree. Use a fresh source destination; preserve any
existing source checkout. The source build needs no shared lock/artifact change.

```bash
. /home/donovan/.bible-atlas-env
git -c core.autocrlf=false clone --filter=blob:none --no-checkout https://github.com/json-everything/json-everything /tmp/codex-json-schema-fit-source
git -C /tmp/codex-json-schema-fit-source checkout --detach 399f198431f65cf6896fe6038f833ef6d0b27a39
git -C /tmp/codex-json-schema-fit-source apply "$PWD/docs/superpowers/reports/evidence/2026-10-03-fsharp-json-validation/json-source-link.patch"
dotnet test client-fsharp.Tests/SchemaSpike/SchemaSpike.fsproj -p:JsonSchemaSource=/tmp/codex-json-schema-fit-source -p:GeneratePackageOnBuild=false --logger 'console;verbosity=normal'
```

The standalone spike is not in the normal client test assembly. Its explicit
source parameter and report make this experimental dependency reproducible;
it is not an external absolute path embedded in the application project.

## Gates, limits and handoff

`json-vocabulary-red.log` is a compilation failure, not a behavior red.
After the vocabulary compiled, the native JSON properties ran **5 fail / 2
pass** against the old decoder. The broader current-category red ran **8 fail /
10 pass**, including the HTTP/vocabulary/compiler-construction checks.
The final scoped category/contract/retry/order run passed **40 properties**.
The final normal suite passed **190 properties** in **33.2684 seconds**;
its compiled assembly gate requires every discovered test to be a property.
The separate validator fit passed **9** in **2.3072 seconds** at the final tree.
The whitespace-coordinate property is supplementary green evidence, not
another independently executed production red. Saved logs normalize trailing
whitespace; the manifest retains the original output hashes, and the original
command outputs remain in the task-owned integration log directory. The patch
is checked against original LF source files.

Final declaration inventory: **271 unused / 59 pending / zero compiler errors**.
Unused is still red and is not a root-aware dead-island proof. Newspaper-order
coverage checks **73 authored .fs + 2 generated** and passes; its existing
compiler-symbol/alias/overload limitations remain. Generated hashes remain
`72656755da812fff3b4bce21ae7dddb2cfa12e2c96623b01719a613f6ef7f988`
(Wire) and
`70fa71de281c2f9f5dda56ccc35109364f82dfe378f98000233984863e507dca`
(Vocabulary), identical to the base. Debug WASM: **zero warnings/errors**, 1.10s.
No Rust, AOT, browser differential, mutation, performance or full parity gate ran.
No added application/test comments, example attributes or diff whitespace errors.

Continue newly submitted Claude exact-head reviews first, then schema binding,
frontier/read integration, unused reachability and remaining semantic order
before whole-domain review. Parent `88f0d26`, order `79671ab`, frozen STYLE
`35f6797` and R2 `d17644f` remain unchanged. No Codex lock/server/live build
remains at handoff; Claude's ordinary locks and author work are preserved.

CX-I3 retains the owner's disk reminder: keep compact mutation outcomes,
reports and equivalent decisions, then prune only obsolete stopped task-owned
mutation results and unnecessary build output. Recheck both WSL and the Windows
VHD-host volume before growth-heavy gates. Latest check: **786 GB WSL free**,
Windows C: **3.2 GB / 100% used**. This continuation retains about 230 KB of
compact evidence and uses small native/Debug output; no bulk deletion,
automated retention, fresh Cargo/AOT target, raw/cache or other-agent deletion,
or VHD compaction occurred.
