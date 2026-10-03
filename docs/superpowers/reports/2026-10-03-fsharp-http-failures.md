# Structured HTTP failure correction

The F# HTTP door now returns the existing structured ReadFailure vocabulary.
All **174 property tests pass**, and Debug WASM builds with **zero warnings and
errors**. The old Transport string constructor is removed. This checkpoint
does not close JSON/graph failures, failure presentation, controller cancellation,
whole-domain approval or 100% parity.

Base: `f7bfeb9`, `lane/codex/CX-FSHARP-wire`, approved producer `da7e00d`.
The explicit correction scope is recorded on ops `208a64d`: the complete current
HTTP refusal/interruption/cancellation category, its two existing status doors
and every current consumer/fixture. No unrelated pending operation or feature
was filled. No application view, C#, producer, contract, data or Claude
worktree changed.

## Category and actual interfaces

The failed abstraction was Failure.Transport of string: status, published error
code, retry classification and user cancellation were flattened into prose.
This belongs at the client HTTP adapter. The SDK owns transport; the adapter
maps its result once into the client's closed types. Every read still enters
through Api.read, and all existing transport fixtures now use typed failures.

Inspect the actual declarations in [Failure](../../../client-fsharp/Core/Admission/Failure.fs),
[ReadFailure](../../../client-fsharp/Core/Admission/ReadFailure.fs),
[status admission](../../../client-fsharp/Core/Admission/Scalars.fs), and
[Api](../../../client-fsharp/Core/Api.fs). The whole actual-module tour is
[the domain spec](../specs/2026-10-03-fsharp-domain.md).

| Boundary result | Domain result |
| --- | --- |
| 4xx | Terminal ClientRefusal, private admitted status, generated ErrorCode option |
| 5xx | Transient ServerRefusal, private admitted status, generated ErrorCode option |
| HttpRequestException or cancellation exception without supplied-token cancellation | Transient Unreachable |
| Cancellation exception with supplied token cancelled | Cancelled |
| Invalid received status outside 100–599 | Transient InvalidStatus, exact received HttpStatusCode and code option |
| Informational/redirect response surfaced as a final SDK response | Terminal UnexpectedStatus, exact received status and code option |

Malformed or unknown error-body vocabulary cannot invent an ErrorCode. Its
status classification survives with None. Success decoding is unchanged.
ClientStatus and ServerStatus remain distinct private representations, admitted
only by their respective total range doors. Two pending bodies became these
doors; **59 remain**. Failure.Contract of string remains explicit unfinished
JSON/graph work, rather than a compatibility Transport constructor.

Invalid status classification follows [RFC 9110 §15](https://httpwg.org/specs/rfc9110.html#status.codes):
valid codes are 100–599, and clients should process invalid codes as server
errors. This does not claim every 5xx is automatically retried: the current
model/UI retry policy still needs integration. Actual cancellation tests cover
the API token; navigation/CloseFocus cancellation ownership remains open.

## Existing-tool choice

The installed .NET 10 System.Net.Http and FSharp.SystemTextJson remain the
transport and JSON implementations. No HTTP parser, response decoder, new
package or general-purpose validator was written. The domain-specific range
doors admit the existing private types. [HttpClient.GetAsync](https://learn.microsoft.com/en-us/dotnet/api/system.net.http.httpclient.getasync?view=net-10.0)
documents the SDK exception/token behavior; the [runtime license](https://github.com/dotnet/runtime/blob/main/LICENSE.TXT)
is MIT. Framework maintenance follows the installed supported .NET toolchain.

The supplementary primary-source check of [Flurl error handling](https://flurl.dev/docs/error-handling/)
confirms it wraps HTTP failures and timeout exceptions. Inference for this
application: an additional wrapper would still need the same application-type
mapping. It was not installed or tested, and no WASM-fit claim is made. This
is validation of the already selected framework adapter, not a claim that a
complete alternative-library survey preceded the original client implementation.

## Executed red/green evidence

| Gate | Actual result |
| --- | --- |
| Status laws against the two pending doors | **3 real failures**, NotImplementedException |
| Typed HTTP properties against the previous string path, after making their types compilable | **7 fail / 3 pass** |
| First HTTP/status/source-order green | **26 pass**, no failures/skips |
| Invalid-status retry property before adding its Api branch | **1 fail / 14 pass** |
| Final normal suite | **174 pass / 0 fail / 0 skip**, 31.6 seconds |
| External constructor vocabulary law | 15 cases × positive/negative × 3 runs = **90 compiler checks** within that suite |
| Source-order inventory | **69 authored .fs + 2 generated**, green within the suite |
| Debug WASM build | **0 warnings / 0 errors** |
| Final declaration inventory | **288 unused / 59 pending / 0 compiler errors**; gate remains red |
| Generated files | Both byte-identical to the preceding checkpoint |
| Added comments / example test attributes / diff whitespace | None / none / clean |

The HTTP red includes exact-status/client/server failures, interrupted and
user-cancelled reads, unknown error-body vocabulary, unexpected status and the
compiled Failure vocabulary. The supplementary invalid-status red catches
classifying invalid received codes as terminal. Complete executed logs are in
`evidence/2026-10-03-fsharp-http-failures/`, with exact first failure lines,
source hashes, category counts and the final unused inventory. Compile errors
while introducing types or qualifying Failure.Read are not behavior reds.

Property-only closure remains gated against compiled Xunit/FsCheck attributes.
The Failure vocabulary property enumerates its entire union; an external
compiler consumer proves the old Transport constructor is unwritable. Status
properties compare whole admission results and their class boundaries. Retry
laws enumerate every current inner failure case. Every current application
transport reference is migrated; no string constructor remains to reuse.
This structural closure is specifically the former HTTP Transport category.

## Remaining work and handoff

I8/I3/M2 remain open for JSON/graph failures, typed failure presentation,
manual retry policy and navigation/CloseFocus cancellation. The temporary
Contract string case, pending frontier/explorer integration, unused reachability,
remaining producer binding and semantic newspaper order still prevent
whole-domain sign-off. The unused scanner does not yet establish real-model
reachability, dead-island closure, full generic coverage or framework dispatch.
The order gate is syntactic and does not prove arbitrary alias/overload/shadow
or full symbol/type ordering. No AOT, browser differential, mutation, Rust or
full-parity gate ran. No finding is marked closed by the author checkpoint.

Continue newly submitted Claude exact-head reviews first, then these authorized
domain corrections before feature expansion. Parent `88f0d26`, isolated order
`79671ab`, frozen STYLE `35f6797` and R2 `d17644f` remain unchanged. Claude's
ordinary heavy/contract locks were preserved. No Codex lock/server remains.

CX-I3 retains Donovan's instruction to prune obsolete stopped mutation results
and unnecessary build output after keeping compact outcomes and verifying
ownership. Check WSL and the Windows VHD-host volume before large builds;
guest free space alone does not establish host headroom. This checkpoint adds
only bounded native/Debug output, with no bulk cleanup, raw/cache or other-agent
deletion, retention automation or VHD compaction.
