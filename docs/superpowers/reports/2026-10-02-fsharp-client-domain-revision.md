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

No production source, project compile list, package installation, artifact,
contract or C# change is made by this response. Further view/exemplar work stays
behind the owner-directed domain gate. Next work is the real-module revision,
representation-specific invalid-input properties and complete all-source costume
closure, with the server disagreements reconciled at their owning side.
