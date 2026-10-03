# F# domain: current compiling correction

This is the actual-module tour for `lane/codex/CX-FSHARP-wire`, on approved
producer `da7e00d`, carrying the parent skeleton forward. **Not ready for whole
owner sign-off or full parity:** structured JSON failures and failure presentation, frontier/read
integration, generated producer binding in the remaining skeleton, semantic
source order and unused reachability still need correction. The named pending
inventory is **59**; only the two existing HTTP status doors were filled by the
explicit failure-correction work. No new view or unrelated operation was added.

| Real files | Domain, structures and algebras to inspect |
| --- | --- |
| [Positive](../../../client-fsharp/Core/Domain/Positive.fs), [NonEmpty](../../../client-fsharp/Core/Domain/NonEmpty.fs), [Rooted](../../../client-fsharp/Core/Domain/Rooted.fs) | Positive bounds, maintained nonempty collections, one coherent-root Result door and root-aware composition. |
| [Time](../../../client-fsharp/Core/Domain/Time.fs), [YearReading](../../../client-fsharp/Core/Domain/YearReading.fs) | Served years, spans, finest producer-supplied dating and contexts; pending laws/bindings, no invented next year or concurrency. |
| [Text](../../../client-fsharp/Core/Domain/Text.fs), [Containers](../../../client-fsharp/Core/Domain/Containers.fs) | Generated references, text parts/units, distinct Bible and Concord navigation instances and served multi-unit passages. New FOCUS-3 producer shapes remain a dependency. |
| [Graph](../../../client-fsharp/Core/Domain/Graph.fs), [History](../../../client-fsharp/Core/Domain/History.fs) | Graph and history skeleton; single-verse TextUnit or multi-verse Passage accounts, chronology separate from stories. Remaining generic identity parameters and unused declarations need binding/removal before review. |
| [Explorable](../../../client-fsharp/Core/Domain/Position.fs) | The single live generated node/edge position, checked against its artifact root; no parallel Position/Resolved vocabulary. |
| [ReadingWindow](../../../client-fsharp/Core/Paging/ReadingWindow.fs), [NeighbourWindow](../../../client-fsharp/Core/Paging/NeighbourWindow.fs), [PageCache](../../../client-fsharp/Core/Paging/PageCache.fs) | Bounded root/query/cursor windows and immutable cache; pending window doors stay visible. Default reading 20/40 remains proposed; cache leaf laws already run. |
| [Trail](../../../client-fsharp/Core/Exploration/Trail.fs) | Last 40 steps plus their source, shared follow/resume retention, dual cancellation of the visible breadcrumb. Edge-evidence admission remains unfinished. |
| [Focus / Frontier](../../../client-fsharp/Core/Exploration/Focus.fs) | Focus is only the element; Frontier separately holds its neighbour windows. The frontier door/read integration remains pending. |
| [Explore](../../../client-fsharp/Core/Exploration/Explore.fs) | FSharpPlus reader/state/result over async, lawful bind/map, deferred CE, here/follow/back/renew and one Resume. Here/back/renew are unit functions. Links remains pending. |
| [ArrayIndex](../../../client-fsharp/Core/Admission/ArrayIndex.fs), [DocumentPosition](../../../client-fsharp/Core/Admission/DocumentPosition.fs), [Coordinates](../../../client-fsharp/Core/Admission/Coordinates.fs), [HttpUrl](../../../client-fsharp/Core/Admission/HttpUrl.fs) | Existing private admission leaves and narrow failure cases. |
| [Scalars](../../../client-fsharp/Core/Admission/Scalars.fs), [TextSpans](../../../client-fsharp/Core/Admission/TextSpans.fs) | Private HTTP client/server refusal statuses with total checked range doors and invalid-status predicate; color/text-span doors remain pending. Existing text-run clamping still needs correction. |
| [Paths](../../../client-fsharp/Core/Admission/Paths.fs), [WireFailure](../../../client-fsharp/Core/Admission/WireFailure.fs) | The live JSON door uses distinct syntax/null/unreadable-answer failures and exact optional serializer path/zero-based line/byte observations; it never parses message text. Paths/TextPosition remain separate unfinished proposal vocabulary, not a claim that the serializer supplies structured path steps or original-document positions for nested converters. |
| [GraphFailure](../../../client-fsharp/Core/Admission/GraphFailure.fs), [graph read](../../../client-fsharp/Core/Admission/GraphRead.fs) | Closed answer failures carry generated identities, requested/received PositionRef including kind, exact continuation cursor, corpus and nonnegative cardinality evidence. Explorable, Trail/Explore, presenter and reading validation use this same vocabulary. Evidence records carry observations, without claiming refined equality/inequality guarantees. |
| [ReadFailure](../../../client-fsharp/Core/Admission/ReadFailure.fs), [Failure](../../../client-fsharp/Core/Admission/Failure.fs), [API door](../../../client-fsharp/Core/Api.fs) | Live Failure.Read carries transient/terminal/cancelled cases. Exact 4xx/5xx status and generated ErrorCode survive the door. Invalid received codes have a transient InvalidStatus case; surfaced informational/redirect statuses have terminal UnexpectedStatus. InvalidGraph carries the graph-answer vocabulary. The old Transport string constructor is absent. The public Contract string constructor is removed, and the JSON adapter returns terminal InvalidAnswer through Failures.wire. This closes raw failure-prose construction; full schema validation and UI failure policy remain unfinished. |

These links are the type/signature declarations; this spec does not maintain a
second model. Current generated identities, roots, cursors and widenings come
from approved WIREID. Current provenance carries the served title. Year and
FOCUS-3 shapes are not copied from unapproved branches; remaining skeleton
parameters are unfinished producer dependencies, not guarantees about arbitrary
instantiations. Every introduced API must be read by the real model before the
zero-unused gate can pass. Tests-only references do not establish application
liveness.

Current evidence: [wire integration](../reports/2026-10-03-fsharp-wire-identity-generation.md),
[source order](../reports/2026-10-03-fsharp-order-integration.md),
[property-only tests](../reports/2026-10-03-fsharp-property-conversion.md), and
[structured HTTP correction](../reports/2026-10-03-fsharp-http-failures.md), and
[structured graph-answer correction](../reports/2026-10-03-fsharp-graph-failures.md).
The compiling test suite proves the recorded scope; it is not owner approval or
an independent parity verdict. After the remaining corrections, present these
real files for whole-domain review before feature/view expansion.

JSON correction and measured validator fit: [checkpoint report](../reports/2026-10-03-fsharp-json-validation.md). The source-built validator is a native test candidate only; the application retains its installed serializer, with schema pattern/closed-record enforcement and WASM validator binding still outstanding. JsonErrorLocation holds exact observations, not an asserted valid/complete source coordinate; the nested serializer can report a fragment location.
