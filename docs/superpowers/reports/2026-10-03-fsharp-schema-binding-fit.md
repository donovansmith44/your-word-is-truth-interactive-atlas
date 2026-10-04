# F# generated-type schema binding fit

The existing validating serializer converter is useful but its current
name-based registration does **not** qualify for application adoption.
The separate native fit has **5 properties pass / 2 adoption gates fail**.
Both failures stay red. Base: `9fa90fc`; approved producer: `da7e00d`;
lane: `lane/codex/CX-FSHARP-wire`.

The experiment uses JsonSchema.Net's maintained
[ValidatingJsonConverter](https://docs.json-everything.net/schema/serialization/),
with the already pinned MIT source and existing F# serializer. No custom
validator or converter implementation was written. Registration is derived
from the original component names and actual generated assembly. The
candidate's private test setup uses the serializer's existing type-selection
options: records, collections, optional types and tuples are handled globally;
generated converter attributes own unions and opaque identities. This keeps
the handwritten factory override out of the candidate setup. It is not a
change to the application's serializer configuration.

## Executed fit

| Property | Result |
|---|---|
| Complete component-to-runtime binding | **Fail**: 120 of 121 components bind; `Point` has no named CLR type |
| Every whole original query fixture decodes | Pass: all 35 |
| Opaque identity patterns and closed leaf-record extras | Pass: 100 generated trials |
| Original `Point` requires exactly two numeric coordinates | Pass: 100 generated length/value trials |
| Complete structured NodeRef refusal hierarchy | Pass: original root `properties` and `/id` `pattern` errors retained |
| Selected union payload constraints survive decoding | **Fail** across TextRef, PositionRef and Element |
| Every compiled test is a property | Pass |

The complete run returns exit **1** in **1.7015 seconds**. The
[whole comparisons](evidence/2026-10-03-fsharp-schema-binding-fit/whole-comparisons.json)
retain all 121 component results and the full eight-answer refusal matrix.
The binding failure does not require turning every alias into a nominal type.
The candidate guessed a CLR name from the schema name; that strategy cannot
discover `Point = float list`. The original Point schema has `minItems: 2` and
`maxItems: 2`, so metadata must preserve that schema association even though
the F# type erases. A generated schema/response binding is the next candidate;
hardcoding a Point exception or silently skipping it would leave the category
open.

The selected-payload matrix is:

| Input | Expected accepted | Actual accepted |
|---|---|---|
| Invalid book enum in direct BibleRef | false | false |
| Same invalid book in TextRef | false | false |
| Same invalid book in UnitText.locus | false | false |
| Chapter -1 in direct BibleRef | false | false |
| Same chapter in TextRef | false | **true** |
| Same chapter in UnitText.locus | false | **true** |
| Invalid NodeId in PositionRef.node | false | **true** |
| Invalid NodeId in Element.node | false | **true** |

The invalid Element is a clone of the actual element-read fixture's node
answer with only its id changed. The book enum remains enforced by the
generated enum serializer; that success is not evidence that arbitrary schema
constraints survive a selected branch. The negative chapter and opaque id
examples expose the distinction.

## Source and standards checks

At the pinned source, the validating converter's
[default options factory](https://github.com/json-everything/json-everything/blob/399f198431f65cf6896fe6038f833ef6d0b27a39/src/JsonSchema/Serialization/ValidatingJsonConverter.cs)
removes validating converters before delegating to the normal serializer. A
parent schema therefore cannot rely on that delegation to validate each
concrete payload. The pinned F# serializer's
[unwrapped-record handling](https://github.com/Tarmil/FSharp.SystemTextJson/blob/ac6fd9a3770590e3260bca94395a884c24cf2227/src/FSharp.SystemTextJson/Union.fs)
also constructs a record converter directly and reads through
`ReadRestOfObject`. Merely registering concrete record converters is not a
verified remedy for that path.

This is an integration limitation, not a producer or validator finding.
[OAS 3.1.2 §4.8.25](https://spec.openapis.org/oas/v3.1.2.html#discriminator-object)
clarifies that a discriminator does not change schema validation and that an
allOf parent's validation does not search its children. The client must
preserve the constraints of the concrete schema selected by its generated
union model. Do not change the producer document to make this candidate pass,
replace child validation with base validation, parse error prose, or invent
an original-document location from fragment-relative serializer diagnostics.

The maintained validator remains the selected evaluation engine. Its
MapType/default-converter combination is insufficient as the entire admission
door. The alternative-tool survey remains in the prior reports; this result
does not reject all alternatives or authorize a handwritten generic decoder.
Before application adoption, bind declared schema/response metadata through
the generator and test selected payload constraints under the maintained
validator, including aliases, nullable/collection cases and every discriminator
family. Keep the original null and concrete serializer checks as well.

## Shared preparation and regression gates

[SchemaSurvey.fs](../../../client-fsharp.Tests/SchemaSpike/SchemaSurvey.fs)
now owns the input/schema cache shared by the original native properties,
browser host and separate binding fit. SchemaLaws no longer repeats schema
building; the browser primes this same cache. Original producer/fixture bytes
and application sources are unchanged.

Fresh regression evidence after extraction: original native **9 pass** in
**1.2260 s**; Debug host **0 warnings/errors**, **8.28 s**; Chromium original
**8 properties / 701 trials / 35 fixtures pass**, **8,434 ms** in properties,
**10.629 s** including navigation. Invalid-root control again fails the actual
fixture property as expected. Positive/control runs have no page or HTTP
faults, and the owned 5100 host is stopped. Final source-order **13 pass** in
**1.9305 s**, covering **79 authored + 2 generated** F# files with the previously
recorded semantic limitations.

The initial syntax/recursive-initialization failures and a null dictionary-key
mistake belong to test harness preparation, not production behavioral reds.
The runtime registration attempt initially threw for the erased Point alias;
the final experiment lets other registrations run while keeping the complete
binding assertion red. A local test helper initially preceded its caller;
the order gate caught it, and moving it below the property restored the gate.
No warnings were suppressed. Saved logs normalize trailing whitespace; compact
browser records summarize resource inventories, with original hashes/logs
retained. The [manifest](evidence/2026-10-03-fsharp-schema-binding-fit/manifest.json)
records inputs, exact source hashes and executed outcomes.

## Reproduce and handoff

Use the pinned source checkout/SourceLink patch from the
[native fit instructions](2026-10-03-fsharp-json-validation.md). Run:

```bash
. /home/donovan/.bible-atlas-env
dotnet test client-fsharp.Tests/SchemaSpike/Binding/BindingFit.fsproj -p:JsonSchemaSource=/tmp/codex-json-schema-fit-source -p:GeneratePackageOnBuild=false --logger 'console;verbosity=normal'
```

Expect **5 pass / 2 fail**, exit **1**. This standalone experimental project
is deliberately outside the normal application test assembly. It is not a
gate-clean adoption. The previous normal 190-property run and 271-unused /
59-pending inventory were not rerun here; production behavior did not change.

Continue newly submitted Claude exact-head reviews first, then the generated
binding and concrete-payload correction before adoption or feature expansion.
Whole-domain sign-off, precise diagnostics, frontier/read integration,
root-aware liveness, semantic order and 100% parity remain unfinished. Parent
`88f0d26`, order `79671ab`, frozen STYLE `35f6797` and R2 `d17644f` remain
unchanged. No Codex lock remains; Claude's F3 contract lock is preserved.
CX-I3 retains the owner's cleanup reminder. Current capacity is about
**773 GB WSL free / 1.9 GB Windows VHD-host free**. No fresh Cargo/AOT,
full browser/mutation gate, shared artifact/schema, raw/cache or Claude edit.

After pushing `42c30c4`, six completed native/export/browser bin/obj trees
were pruned: **205,241,554 logical bytes**. The
[cleanup inventory](evidence/2026-10-03-fsharp-schema-binding-fit/cleanup.json)
retains ownership checks and the system-service process-visibility limit.
The active Binding/bin and Binding/obj caches remain for the next correction;
source, original logs, compact results, raw/cache/scratch and Claude outputs
remain untouched. Latest Windows host headroom is only **1.7 GB**, while WSL
still has about **773 GB** free. Guest cleanup has not compacted the VHD or
restored equivalent Windows space; continue with existing bounded native
output and avoid fresh WASM/AOT/Cargo growth while host capacity is critical.
