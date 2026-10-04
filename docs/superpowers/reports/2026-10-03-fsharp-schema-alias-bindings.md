# Generated schema bindings preserve erased F# aliases

The isolated wire lane now binds all **121** original OpenAPI components, including `Point = float list`, using compiled F# type references. The complete-binding property has turned green. The application validation migration remains unfinished: the separate selected-payload adoption property still fails, so this experiment is not suitable for application adoption yet.

Base: `cb272c1` on `lane/codex/CX-FSHARP-wire`. Approved producer: `da7e00d`. Validator MIT source pin: `399f198431f65cf6896fe6038f833ef6d0b27a39`; serializer source pin: `ac6fd9a3770590e3260bca94395a884c24cf2227`. The existing [source-build and licensing survey](2026-10-03-fsharp-json-validation.md) still applies. Claude's producer work, C# client, contract, assets and frozen style lane are untouched.

## Change

`Generator.generateBindings` emits an internal `SchemaTypes.all : Map<string,System.Type>` from the same component definitions used by wire generation. Each schema name refers to `typeof<Component>`, so the F# compiler resolves aliases without guessing their CLR names. `--schema-bindings` writes that table to the native candidate's own `obj/Contract/SchemaTypes.g.fs`; no table, validator dependency or converter is added to the application. YAML loading and component extraction are shared with existing generation, as is file writing.

The candidate's existing maintained validating converter registers those generated type references. Its complete-binding property compares the entire key list with original component definitions and checks converter coverage for every name. The `Point` property now exercises actual deserialization rather than schema evaluation alone, retaining the exactly-two-coordinate requirement.

Three new property laws compare entire emitted documents: two differently constrained collection aliases retain both names, every component appears once in source order, and a document without components is refused. The source-order gate always checks the third generated source in memory and, when its native build output exists, checks that output against the generator. It therefore does not require the separate validator project to have been built before ordinary client tests.

## Gates

| Gate | Result |
|---|---|
| Test first: three new laws before the API existed | Expected FS0039 compile refusal, exit 1; retained log |
| Complete native client suite | **193 properties pass**, exit 0, 31.5541 s |
| Generator laws included above | **10 pass**, including the three new laws |
| Source-order laws included above | **13 pass**, covering **79 authored + 3 generated** sources |
| Separate native binding experiment | **7 properties: 6 pass / 1 fail**, exit 1, 1.6959 s |
| Complete generated component registration | **121/121**, no missing names; `Point` included |
| Original complete query fixtures | **35 accepted**, unchanged |
| Application wire/vocabulary and input contract | SHA-256 unchanged from the prior binding checkpoint |

The binding experiment's remaining red comparison is unchanged:

| Answer / invalid input | Expected acceptance | Actual acceptance |
|---|---|---|
| BibleRef / unknown book | false | false |
| TextRef / unknown book | false | false |
| UnitText.locus / unknown book | false | false |
| BibleRef / chapter -1 | false | false |
| TextRef / chapter -1 | false | **true** |
| UnitText.locus / chapter -1 | false | **true** |
| PositionRef.node / malformed NodeId | false | **true** |
| Element.node / malformed NodeId | false | **true** |

The [previous binding fit](2026-10-03-fsharp-schema-binding-fit.md) explains the maintained converter's default delegation and F# unwrapped-record behavior. OpenAPI discriminator metadata is an annotation, not extra JSON Schema assertions. No producer/library defect or new finding is asserted for that standards behavior.

## Limits and handoff

Keeping both names in metadata does not solve context-dependent constraints for two aliases that erase to the same CLR type: the candidate converter caches registration by CLR type. The generated schema/response context and selected concrete payload constraints still need a measured fit before the application door can adopt this candidate. Do not make Point nominal, copy a constraint list or weaken the remaining property.

No new browser host, AOT publish, live side-by-side parity or mutation run was performed. Ordinary native tests used the retained incremental Debug client build. Earlier browser results remain historical. Whole-domain sign-off, frontier/read integration, precise diagnostics, root-aware unused closure and full F# parity are still outstanding. The queue item remains claimed.

Windows C: still has only about **1.7 GB** free while WSL shows **768 GB**. CX-I3 records the owner's instruction to prune obsolete mutation results/targets and finished build output after retaining compact outcomes and verifying ownership/stopped processes. This checkpoint retains the small active native caches, avoids a fresh large WASM/AOT/Cargo target, and does not claim guest cleanup compacts the host VHD. No Codex lock, server or build process remains.

Compact evidence, complete logs, the actual generated table snapshot, source hashes and unchanged-output comparisons are in [the evidence manifest](evidence/2026-10-03-fsharp-schema-alias-bindings/manifest.json). Original unnormalized logs remain under `/home/donovan/mut/codex-CX-FSHARP-wire-integration-logs`.

Reproduce using the documented retained MIT source checkout:

```sh
. /home/donovan/.bible-atlas-env
dotnet test client-fsharp.Tests/BibleAtlas.FSharp.Tests.fsproj --no-restore
dotnet test client-fsharp.Tests/SchemaSpike/Binding/BindingFit.fsproj --no-restore \
  -p:JsonSchemaSource=/home/donovan/mut/codex-json-schema-fit-source \
  -p:GeneratePackageOnBuild=false
```

The first command succeeds; the second intentionally remains red at the selected-payload adoption requirement.
