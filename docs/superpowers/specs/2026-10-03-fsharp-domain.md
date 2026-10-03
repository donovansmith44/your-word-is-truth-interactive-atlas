# F# domain: compiling skeleton for owner review

Owner directive ops `e65c377` supersedes the partial leaf implementation sequence.
The whole module map now builds in Core and the Debug WebAssembly client. Types,
private representations and operation signatures live in the files below; this
page declares none of them. **Skeleton review, not implemented behavior or full
client parity.** Operations awaiting laws and implementation call the one
[DomainSkeleton.pending](../../../client-fsharp/Core/Domain/Skeleton.fs) marker.
The [inventory gate](../../../scripts/fsharp-client/domain-skeleton.py) counts and
names every pending operation; none is called by the existing app.

| Real file, in project compile order | What the owner can inspect |
|---|---|
| [Positive](../../../client-fsharp/Core/Domain/Positive.fs), [NonEmpty](../../../client-fsharp/Core/Domain/NonEmpty.fs), [Rooted](../../../client-fsharp/Core/Domain/Rooted.fs) | Existing tested positive counts, FSharpPlus nonempty collections and coherent-root values, now with explicit signatures throughout. |
| [Time](../../../client-fsharp/Core/Domain/Time.fs) | A served Year retains its node, label and numeric value; a span has ordered ends; dating has exact/circa precision. Span comparison, containment, cover, overlap, intersection, inclusion, adjacency and filtering of supplied years have signatures. No operation creates the next Year node. |
| [Text](../../../client-fsharp/Core/Domain/Text.fs) | Producer reference parameters, anchored parts, a body, event headings and a text unit with its compiled edge summary. `GEN.1.1` remains the served verse identity everywhere it occurs. Ordering takes the producer's comparison, never a parsed reference or an invented book ordinal. No domain copy of the generated reference union. |
| [Containers](../../../client-fsharp/Core/Domain/Containers.fs) | Distinct Bible and Concord containers with children, previous and next. Both have an instance of one navigation typeclass, including whole reading. Passages retain the served node, span and mark; one-unit and reversed passages have their own refusals. A Concord document/article is not disguised as a Bible book/chapter. |
| [Graph](../../../client-fsharp/Core/Domain/Graph.fs) | Entities, specialized nodes and elements; directed/symmetric relations; node-to-node ends and the separate justification-to-edge ends. Edges retain parentage, votes, narrative, provenance and summaries. Contract NodeKind/EdgeKind/Parentage/NarrativeId are reused. |
| [History](../../../client-fsharp/Core/Domain/History.fs) | Kind-admitted event/story ids; an account run is an existing single verse or a multi-verse passage, behind one history-admission signature and a total projection. One-unit passages remain refused; no account node is minted. Previous/next chronology, the short complete concurrent set and story steps stay separate. Undated has no chronology field. Accounts are a list: an absent attestation cannot make an otherwise readable event impossible to represent. |
| [YearReading](../../../client-fsharp/Core/Domain/YearReading.fs) | Served facet counts, a span composed from year contexts, and individual office terms with their grounds. Several terms allow a person to hold several offices. |
| [Position](../../../client-fsharp/Core/Domain/Position.fs) | A producer element identity or composed Year span; graph/event/year/span resolutions sealed to a root. A passage and an individual Year open as their served node, without a second independent position identity. No domain copy of the generated element identity. |
| [ReadingWindow](../../../client-fsharp/Core/Paging/ReadingWindow.fs) | Container-only query keys, page cursors, previous/next, admitted pages, page/window bounds, append/prepend and whole-container reading. Default policy is 20 units per page and at most 40 visible; its operation awaits implementation. |
| [NeighbourWindow](../../../client-fsharp/Core/Paging/NeighbourWindow.fs) | Query-bound relation/cursor keys and bounded windows. Entries retain edge evidence alongside the target position, so a trail can follow what a focus shows. |
| [PageCache](../../../client-fsharp/Core/Paging/PageCache.fs) | Reading/neighbour keys and page alternatives, plus the existing tested immutable cache. The cache owns one root; keys do not repeat it. Its positive budget is measured in pages. |
| [Trail](../../../client-fsharp/Core/Exploration/Trail.fs) | Rooted transitions, last-N retention, Back/AtStart, root renewal and chain evidence; no server receipts or unlimited old-step history. |
| [Focus](../../../client-fsharp/Core/Exploration/Focus.fs) | Current resolution, bounded relation windows and one `change` operation for arrows and scrolling. |
| [ArrayIndex](../../../client-fsharp/Core/Admission/ArrayIndex.fs), [DocumentPosition](../../../client-fsharp/Core/Admission/DocumentPosition.fs), [Coordinates](../../../client-fsharp/Core/Admission/Coordinates.fs), [HttpUrl](../../../client-fsharp/Core/Admission/HttpUrl.fs) | Existing tested admission leaves and their narrow failures. |
| [Scalars](../../../client-fsharp/Core/Admission/Scalars.fs), [TextSpans](../../../client-fsharp/Core/Admission/TextSpans.fs) | Listed colors, distinct client/server refusal statuses, scalar/UTF-16 offsets and spans, and conversion within supplied text. |
| [Paths](../../../client-fsharp/Core/Admission/Paths.fs), [WireFailure](../../../client-fsharp/Core/Admission/WireFailure.fs), [ReadFailure](../../../client-fsharp/Core/Admission/ReadFailure.fs) | Structured JSON locations and closed failures. A user cancellation is separate from retryable failures; received unfamiliar text is retained as evidence, not called an admitted vocabulary. |

**Producer dependency:** the pinned contract predates WIREID, Year and FOCUS-3.
Their unavailable generated identities, reference unions, cursor types, container
levels, passage marks, text-part roles, time evidence, facets and offices occupy
explicit type parameters. These parameters will bind to those generated types
when their owning work lands. They are not alternate handwritten schemas or
claims that arbitrary instantiations carry domain guarantees. In particular,
calendar evidence comes from the producer, including Scripture's within-day
sequence; the client converts no calendar and infers no concurrency. A final
concrete admission review is required when the producer types exist. Strings in
labels, prose, JSON property names and rejected input carry exactly those
meanings, without costume wrappers.

The account correction follows signed Year revision `9bf4f64`, §2.8/§3.4:
185 of the 2,082 measured account runs are single verses, and the other 1,897
are passages of two or more verses. The two cases reuse `TextUnit` and `Passage`
respectively, retaining their served identities. The history predicate at the
shared pending admission signature must consume the producer's narration/mark
evidence for both cases; it must not infer history by scanning their text. This
is a visible shape correction for sign-off, not an implemented admission law.

After owner review, laws fill the named operations: span/reference laws under
served ordering; private-door refusals; symmetric dual involution; 20/40 paging
and backward slide restoration; query/root coherence; bounded cache/frontier
accessors; trail current/back inversion (full-state inversion only without
eviction), callable same-root renewal idempotence, and scrolling/arrow equality
through the single focus change. No new behavioral law or adapter is implemented
in this skeleton step. Existing leaf/cache laws remain. The
[checkpoint report](../reports/2026-10-02-fsharp-client-domain-skeleton.md) records
red inventory, green build, placeholder count, dependencies and remaining limits.
The [account and identity correction](../reports/2026-10-03-fsharp-account-skeleton.md)
records the single-verse shape repair, removal of handwritten wire shapes and
compiler/build evidence; pending behavior and concrete producer admission remain
for the subsequent sign-off and implementation steps.
