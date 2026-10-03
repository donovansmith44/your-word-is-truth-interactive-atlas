# Structured graph-answer failure checkpoint

All twelve existing non-JSON Contract string producers now return typed graph
answer failures. The scoped category went from **14 failures / 41 passes** to
**68 passing properties**. The broader suite passed **182 properties** and the
standalone domain runner passed **42**. Final refactor evidence is recorded
alongside these runs. This is a correction checkpoint, not whole-domain approval
or 100% parity.

Base: `5ce950f`, `lane/codex/CX-FSHARP-wire`, producer `da7e00d`.
Ops `2cb83f7` records the correction scope. Existing F# application/tests and
their project inputs changed; no new feature, pending domain body, application
view, C#, contract, producer, data or Claude worktree edit. The pending count
stays **59**.

## Category, side and all sites

The failure abstraction had lost graph evidence by turning it into prose.
The client owns validation of answers against its requested identities, trail
and reading. Generated ids, kinds, corpus and cursors remain the source of the
domain vocabulary; no new parsing, identity syntax or graph fact was invented.

Actual declarations: [GraphFailure](../../../client-fsharp/Core/Admission/GraphFailure.fs),
[ReadFailure](../../../client-fsharp/Core/Admission/ReadFailure.fs), and
[Failures.graph](../../../client-fsharp/Core/Admission/Failure.fs). The
[actual-module spec](../specs/2026-10-03-fsharp-domain.md) links the full model.
One shared wrapper carries GraphFailure into terminal ReadFailure; tests retain
an independent fixture constructor. The observation records hold requested and
received values, without promising refined inequality. Cardinalities use uint64
because an observed collection size cannot be negative.

| Existing sites migrated | Evidence retained |
| --- | --- |
| GraphRead's five refusal branches | Whole requested/received counts, both PositionRef values including kind, exact empty/excess continuation cursor |
| Explorable.ofElement | The generated ElementId of the missing answer |
| Explore.follow and Trail.resume | Requested and received element counts, including the journey origin |
| Presenter.node | The generated NodeId of the TextUnit lacking its served body |
| ReadingSurface's three refusal branches | Requested/received Corpus or the Corpus with no opening |

GraphRead keeps the original request count through page append, including an
excess answer after earlier pages. Identity refusal preserves kind as well as
id; a same-id answer of another kind remains a refusal. Text validation retains
the first corpus mismatch; valid and empty windows behave as before. Model
states/effects, stale completion refusal, request sequencing and successful
trail/presentation behavior remain covered by the whole-result properties.

There are nine closed GraphFailure cases. Compiled reflection checks their
entire payload vocabulary, and a generated law carries every category through
terminal ReadFailure unchanged. The existing retry law includes InvalidGraph.
No non-JSON application site still produces Contract prose. This is migration
of the twelve current sites, **not full rule-24b closure**: Failure.Contract of
string remains publicly writable until the JSON correction removes it. Typed
UI error presentation, retry policy and controller cancellation I8/I3/M2 remain
open; no finding is marked closed by this author checkpoint.

## Test-first evidence and limits

`graph-vocabulary-red.log` contains missing-type/member compilation failures,
not behavior reds. After introducing the compilable vocabulary/fixture but
before migrating producers, `graph-behavior-red.log` ran **55 properties**:
**14 failed / 41 passed**. Whole-result expectations retained every current
failure site; new generated laws covered excess elements/cursors, non-singleton
Follow, empty Resume and wrong-corpus Contents. `graph-green.log` ran those
groups plus source order: **68 pass / zero failures/skips**.

Additional green coverage enumerates all nine graph failure cases and preserves
the requested/received kind when the id agrees. It is supplementary coverage,
not claimed as another independently executed behavioral red. The final suite
has **182 properties / zero failures/skips**. The standalone runner has **42**;
its project now reads the existing shared WireFixtures rather than copying an
identity constructor. Its initial no-restore run lacked an assets file; restoring
the already selected packages resolved that infrastructure error. It is not a
behavior red. Same-file constructor qualification during the final shared-wrapper
refactor was a compilation error and is likewise not counted as a behavior red.

The checkpoint evidence folder retains complete category/full/native/WASM logs,
first failure lines, generated/source hashes and final declaration inventory.
Final measurements: **288 unused / 59 pending / zero compiler errors**; the
unused gate remains red. Source-order inventory: **71 authored .fs + 2 generated**,
green. Both generated files remain byte-identical to `5ce950f`. Debug WASM:
**zero warnings/errors**. No added application/test comments, example attributes
or diff whitespace errors. No Rust, AOT, browser differential, mutation or full
parity gate ran. The unused/order gates retain their documented semantic and
real-model reachability limitations.

## Existing tools and next JSON correction

The graph correction uses existing FSharp.Core immutable lists, generated
contract types, CLR reflection and installed FsCheck/FCS. No parser, validator,
format handler or new dependency was introduced. It names/migrates failures in
existing composition; it does not build general-purpose machinery.

Before replacing the remaining JSON door, the primary-source survey identifies:

| Candidate | License / maintenance / fit | Current choice |
| --- | --- | --- |
| System.Text.Json + installed FSharp.SystemTextJson | Runtime MIT; installed .NET 10 and current pinned F# serializer. Preserves current generated wire bytes, but the adapter presently reduces JsonException to a message. | Retain serialization; correct its failure mapping. |
| [JsonSchema.Net](https://docs.json-everything.net/schema/basics/) | [MIT repository source](https://github.com/json-everything/json-everything/blob/399f198431f65cf6896fe6038f833ef6d0b27a39/LICENSE); subsequent binary-license inspection found OSMFEULA on the 9.4.0 NuGet package, so the follow-up candidate is built from pinned MIT source. Maintained project with draft 2020-12 and documented build-once/evaluate-many support. [Results](https://docs.json-everything.net/api/JsonSchema.Net/EvaluationResults/) expose instance/schema locations and keyword errors. | First evaluation candidate against the original approved schema; no package adopted yet. |
| [NJsonSchema](https://github.com/RicoSuter/NJsonSchema) | [MIT](https://github.com/RicoSuter/NJsonSchema/blob/master/LICENSE.md); maintained OpenAPI/JSON Schema toolchain, typed validation kinds/paths, but introduces its Json.NET/reflection stack. | Reserve alternative; schema dialect, footprint and WASM fit need measurement. |
| [SDK JsonSchemaExporter](https://learn.microsoft.com/en-us/dotnet/standard/serialization/system-text-json/extract-schema) | Runtime MIT and already installed. Exports schemas from CLR metadata; does not itself evaluate answers against the approved OpenAPI. | Does not replace a validator; do not re-derive the producer schema from client types. |

This is a survey and tentative spike choice, not an executed package-fit verdict.
No candidate is rejected wholesale and no bespoke validator is justified by
this table. Next measure the candidate against real committed documents,
schema-derived required/null/pattern/union laws, error locations and bounded
WASM costs before adopting it. Closed library errors map at one adapter door;
do not parse exception message text or duplicate the contract's patterns/fields.
Current NodeId/EdgeId patterns are disjoint in the approved document; do not
assume overlapping identity-only oneOf branches from their primitive types.

## Handoff and disk retention

Continue newly submitted Claude exact-head reviews first, then JSON error
mapping/validation, frontier/explorer integration, unused reachability and
remaining semantic order before whole-domain review. Parent `88f0d26`, isolated
order `79671ab`, frozen STYLE `35f6797` and R2 `d17644f` remain unchanged.
No Codex lock/server/live build remains when handed off; Claude's ordinary
heavy/contract locks are preserved.

CX-I3 retains Donovan's request to prune obsolete stopped task-owned mutation
results and disposable builds after keeping compact outcomes. WSL guest space
does not establish Windows VHD-host headroom: check both before large gates.
This continuation adds bounded native/Debug output and compact evidence only,
with no bulk cleanup, raw/cache or other-agent deletion, retention automation,
fresh Cargo/AOT target or VHD compaction.
