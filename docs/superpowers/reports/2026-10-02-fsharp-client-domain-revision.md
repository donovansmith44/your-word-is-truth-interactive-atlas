# F# domain revision after Claude's pre-review

The 6118be2 proposal is **not ready for sign-off**. Claude's exact pre-review
[4f6eb12](https://github.com/donovansmith44/your-word-is-truth-interactive-atlas/blob/4f6eb12/docs/superpowers/reports/2026-10-03-fsharp-domain-prereview.md)
identifies 6 Critical, 10 Important and 8 Minor issues. None is claimed closed by
this response. Opaque names hid the proposed data; the trail-history receipts
invented an unserved mechanism; references and years did not match the server
specifications. Those are defects in my proposal, not implementation guarantees.

Owner ruling on ops 0057d41: declarations belong in the actual modules where they
will live, with property laws beside those modules. The spec becomes a tour of
linked real files and decisions; it contains no second copy of the types. The
four standalone proposal signatures are deleted in the revision, rather than
retained as a second authority. Their current parsing result proves no closure.

The revision follows Claude's dependency map, with actual F# compilation deciding
ordering: generated contract first; domain collections/root/time/text/containers;
graph/history/year reading/position; reading/neighbour windows/cache;
trail/focus; admission adapters. Cross-module dependencies are checked before
adopting that order; Graph's node sum must not forward-reference Event declared
later. Signatures must accompany real, stub-free implementations. No dummy
factory, not-implemented exception or second hand-written wire declaration is
used to make a future producer appear present.

| Pre-review | Revision obligation |
|---|---|
| C1 / M8 | Private representations and their checks in real files; plain-language examples and links in the tour. No opaque-name or syntax-only sign-off claim. |
| C2 / I5 | Admit the served Year node, its label, atlas membership and counted facets/offices. No client year factory or computed previous/next. Pure span operations match the signed Year design. |
| C3 / M3 | Match dated/undated event representation and precision; chronology only for dated events. Check every real event's attestation availability before requiring a nonempty account collection. |
| C4 / I9 / I10 | Shared Bible/Concord containers and served passage marks; at least two units; parts/headings/summaries retained. No client passage minting. Text query/cache keys match the actual container read. |
| C5 / I6 / M7 | Use the generated reference/cursor/vocabulary names once, including the distinct approved cursors. Narrow node kinds from served discriminants, never id prefixes. Stories use Previous/Next. |
| C6 / I7 / M4 | Remove fictional server trail receipts. Await the recorded trail-retention ruling; expose one step sequence. State inverses with their capacity/eviction preconditions and a callable renewal law. |
| I1 / M5 | Each door returns its own reachable closed failure; user cancellation is separate from retryable transport failure. |
| I2 / I3 | One focus-change operation for scrolling and arrows; neighbour entries retain the served edge evidence needed to follow. |
| I4 | At-the-same-time rows use the actual served paged window/cursor, not an eagerly retained complete set. |
| I8 | Involution/symmetry, vocabulary and accessor/capacity laws; reference ordering requires an actual served/contract ordering guarantee. No invented ordinals. |
| M2 / M6 | Retain edge metadata and restrict edge-to-edge endpoints by relation. Cache capacity has one named unit; root/query/cursor equality and invalidation have observable laws. |
| M1 | Complete library survey and package/WASM gates before implementing collection/composition/cache machinery. |

The six choices already presented by Claude remain owner decisions; this response
neither answers them on the owner's behalf nor asks them again. Existing rulings
on shared containers, canonical verse identity, served vocabulary and bounded
reads are respected now. Unlanded WIREID/TIME/FOCUS-3 shapes are contract
dependencies, not permission to copy their declarations into the client.

Library-survey additions: Microsoft.OpenApi + YamlReader **3.10.2**, Thoth.Json.Core
**0.9.1** + System.Text.Json **0.4.0** (both pre-1.0), FsToolkit.ErrorHandling
**5.2.0**, Bolero **0.25.65** (Apache-2.0) and FsCheck.Xunit **3.4.0**.
[NuGet FSharpPlus 1.9.1](https://www.nuget.org/packages/FSharpPlus/1.9.1)
provides a maintained collection candidate, targets .NET Standard 2.0/.NET 6 and
is **Apache-2.0**, not MIT. Its NonEmptyList is the preferred candidate to verify
instead of assuming a bespoke NonEmpty is necessary. Package metadata alone
proves neither trimming size nor WASM/AOT behavior; run those gates before
adoption or rejection. Reuse FsToolkit validation/composition through the one
checked boundary, rather than installing a second general-purpose algebra.
The immutable LRU survey remains to complete; no rejection or custom-cache
implementation is authorized by an unmeasured size claim.

The initial 89b6283 response changed no production source, compile list or
package. The partial real-module revision below changes only the owned F# lane;
no artifact, shared contract or C# change is made. Further view/exemplar work stays
behind the owner-directed domain gate. Next work is the real-module revision,
representation-specific invalid-input properties and complete all-source costume
closure, with the server disagreements reconciled at their owning side.


Before implementing the independent collection/admission modules (2026-10-03):
FSharpPlus 1.9.1, released 2026-01-11, is selected for NonEmptyList (Apache-2.0;
[package metadata](https://www.nuget.org/packages/FSharpPlus/1.9.1),
[upstream implementation](https://github.com/fsprojects/FSharpPlus/blob/master/src/FSharpPlus/Data/NonEmptyList.fs)).
Use only its safe singleton/create/map/append/tryOfList/toList operations in one
private wrapper module; its partial ofList/tail/indexers are not our public API.
No collection algorithm is reimplemented. FSharp.Core is already the immutable
list/Result library; FsToolkit will supply validation/combinators when needed.
The positive/index/position/coordinate checks define domain meaning at their own
single doors; no generic validator framework is being implemented. System.Uri
TryCreate supplies URL parsing (existing framework, MIT); Double.IsFinite supplies
IEEE classification (same). HTTP(S) scheme membership and coordinate ranges are
semantic checks, with closed reachable failures. All dependencies remain outside
Claude’s code tree; native tests and a Debug WASM build will establish this scoped
adoption gate, without making a full runtime/AOT or whole-client closure claim.

The constructor-accessibility property reuses Microsoft's FSharp.Compiler.Service
43.12.401 (MIT, the compiler matching the installed SDK's FSharp.Core 10.1.401),
already used by the separate style checkpoint. This native test-only compiler
library checks programs directly; no accessibility parser or new generic source
checker is hand-written. Its dependency is absent from browser output.


Partial real-module checkpoint (2026-10-03): the
[actual-module tour](../specs/2026-10-03-fsharp-domain.md) replaces the dense type
catalog. Six concrete files compile in Core, with eight private representations
and reachable door-specific failures. The four standalone proposal signatures
are deleted; the old report links its historical 6118be2 declarations and is
explicitly superseded. No stub, unchecked scalar factory, generic failure bucket
or future wire declaration remains in these new modules.

Test-first evidence is retained beside this report (trailing whitespace normalized;
original logs remain in the owned mut directory):

| Gate | Exact observed result |
|---|---|
| [Admission red](evidence/2026-10-03-fsharp-domain/admission-red.log) | Tests written first; always-refusing scaffolds compile but 9 of 16 properties fail on valid values, empty/nonempty classification, coordinate classification and supported/non-web URL schemes. Scaffolds were then replaced, not committed. |
| [Accessibility red](evidence/2026-10-03-fsharp-domain/constructors-red.log) | Temporarily exposing all eight new representations makes the public-surface property fail. A finally block restores every source file before the green gates. This is a bounded regression demonstration, not the deferred batch mutation gate. |
| [Domain green](evidence/2026-10-03-fsharp-domain/domain-green.log) | All 17 properties pass, no skips. Classification retains the complete admitted scalar/collection or exact closed refusal. Ten generated script-name variants compile all eight public doors and reject all eight forbidden constructors with the compiler's accessibility diagnostic. |
| [Regression green](evidence/2026-10-03-fsharp-domain/regression-green.log) | Normal F# test project includes Domain/*.fs: 126 pass, no skips. New properties are discovered by the ordinary test command, not only a separate spike. Its FCS dependency is pinned to 43.12.401 rather than an unversioned SDK file. |
| [Debug WASM](evidence/2026-10-03-fsharp-domain/wasm-build.log) | Client builds with FSharpPlus in its dependency graph; zero warnings/errors. This checks build compatibility only, not browser execution, AOT, trimming size or full parity. |

Reproduce with the task environment sourced, from this lane worktree:

```sh
. ~/.bible-atlas-env
dotnet test client-fsharp.Tests/Domain/DomainLaws.fsproj
dotnet test client-fsharp.Tests/BibleAtlas.FSharp.Tests.fsproj
dotnet build client-fsharp/BibleAtlas.FSharp.Client.fsproj
```

The small domain project and normal regression project discover the same physical
property files through compile globs; they declare no second law implementation.
No new Fact/Theory, fake recursive module or application comment was added. The
existing source-order gate covers the new files and generated contract output;
its existing limitations are not claimed closed by these leaf modules. All 17
new tests are properties; older example tests still require migration.

Private constructors now carry meaningful guarantees for this scoped set, but
all-source/all-door/keyword closure and the remaining costume inventory are open.
Whole-domain pre-review C1/I1/M1/M8 is advanced, not collectively closed. Served
root/identity/time/container/history modules, paging/cache/trail/focus and full
structured failures are still owed; six owner choices are still pending. The
new leaves have no production consumers until the domain integration is approved;
they are an owner-requested compiling design checkpoint, not completed feature
code or a claim against the no-dead-code landing gate. Further view/exemplar work
remains gated. The C# app and Claude's server/data/contract files are untouched.

Disk: keep compact red/green evidence in these source reports, remove stopped
owned disposable mutation/build output under CX-I3's retention proposal, and
check the Windows VHD-host volume as well as guest df. Guest cleanup does not
compact the VHD. No fresh Cargo/AOT target, shared-artifact write, server or Codex
lock was needed for this checkpoint.
