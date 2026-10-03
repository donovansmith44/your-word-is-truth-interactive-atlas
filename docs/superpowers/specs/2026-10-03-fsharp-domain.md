# F# domain: actual modules and remaining decisions

**Partial revision; not ready for whole-domain sign-off.** The declarations below
compile in Core. Each link is the sole declaration of its types and operations;
the spec does not duplicate them. The standalone proposal signatures from
6118be2 have been removed under owner ruling ops 0057d41. Claude's
[pre-review](https://github.com/donovansmith44/your-word-is-truth-interactive-atlas/blob/4f6eb12/docs/superpowers/reports/2026-10-03-fsharp-domain-prereview.md)
and the [revision obligations](../reports/2026-10-02-fsharp-client-domain-revision.md)
remain the full open inventory.

| Actual module | Meaning and evidence |
|---|---|
| [Positive](../../../client-fsharp/Core/Domain/Positive.fs) | A positive count, rather than an integer with a label. Zero and negative inputs return its own closed refusal. [Properties](../../../client-fsharp.Tests/Domain/PositiveLaws.fs). |
| [NonEmpty](../../../client-fsharp/Core/Domain/NonEmpty.fs) | A private adapter around FSharpPlus's nonempty list. Empty input is refused; singleton, create, append and map retain complete ordered values. Append is associative; map preserves identity and composition. [Properties](../../../client-fsharp.Tests/Domain/NonEmptyLaws.fs). |
| [Rooted](../../../client-fsharp/Core/Domain/Rooted.fs) | An admitted value retains the version it came from. Combining values from different roots is refused with both identities intact. Mapping preserves the root, identity and composition; same-root append is associative. RootMismatch is private, so callers cannot invent a mismatch between equal roots. [Properties](../../../client-fsharp.Tests/Domain/RootedLaws.fs). |
| [PageCache](../../../client-fsharp/Core/Paging/PageCache.fs) | A private immutable cache holds one root and a positive budget measured in pages. Put rejects another root; a hit promotes the complete page; replacement keeps one entry per key; eviction forgets the oldest page. Rebase preserves the same-root cache or clears it for a new root. The private lookup result retains the returned page and the updated cache together. [Properties](../../../client-fsharp.Tests/Domain/PageCacheLaws.fs). |
| [ArrayIndex](../../../client-fsharp/Core/Admission/ArrayIndex.fs) | A zero-based, nonnegative array position, distinct from document positions. [Properties](../../../client-fsharp.Tests/Domain/ArrayIndexLaws.fs). |
| [DocumentPosition](../../../client-fsharp/Core/Admission/DocumentPosition.fs) | One-based line and byte column, separately admitted and retained as a structured location. Neither door accepts zero or a negative number. [Properties](../../../client-fsharp.Tests/Domain/DocumentPositionLaws.fs). |
| [Coordinates](../../../client-fsharp/Core/Admission/Coordinates.fs) | Finite latitude/longitude inside their respective inclusive bounds. Nonfinite input and finite out-of-range input have distinct refusals. [Properties](../../../client-fsharp.Tests/Domain/CoordinatesLaws.fs). |
| [HttpUrl](../../../client-fsharp/Core/Admission/HttpUrl.fs) | An absolute HTTP(S) visit URL parsed by System.Uri. Relative/malformed input and other schemes return distinct refusals; admitted spelling is preserved. [Properties](../../../client-fsharp.Tests/Domain/HttpUrlLaws.fs). |

These are independent domain/admission leaves, not a new UI or a replacement
wire vocabulary. For example, a supplied count of zero fails admission; an empty
collection cannot become a nonempty account collection; a latitude of NaN cannot
enter a map position. The application will consume the approved modules through
its admission adapters after the full model is reviewed. No future contract type
or producer is copied into the client to fill a missing dependency.

The [public-surface property](../../../client-fsharp.Tests/Domain/AdmissionSurfaceLaws.fs)
compiles twelve public-use programs and separately attempts each forbidden
constructor, including RootMismatch, PageCache and CacheLookup. No constructor's
diagnostics can hide another constructor's accessibility. The normal regression
project discovers these domain properties; the small DomainLaws project also
runs them without a browser build.

For the cache, a budget of two pages admits A and B; looking up A makes it newest,
so adding C retains C and A and forgets B. Changing the root empties the cache;
an old-root response is then refused. The previous immutable cache is unchanged.
Ten thousand distinct puts retain exactly the latest budgeted pages, without
prefix history. Query/cursor keys remain structured equality values; the cache
does not stringify, parse or flatten them.

Rooted and PageCache take the admitted identity, key and payload types as
parameters. This proves coherence and bounded page retention, not identity
syntax or wire admission. The concrete generated root/query/cursor types are
producer dependencies; they are not redeclared here. A bounded count of pages
does not by itself bound each page's size: the reading/neighbour window must
carry that separate invariant. The cache policy uses FSharp.Core's immutable
list operations through one private Remember member; its callers precede it.
The mutually dependent cache/lookup types form a real type cycle, not an
artificial recursive function group.

The remaining tour must cover concrete served identities and root admission; Year and spans;
Bible/Concord containers, passage/history/text parts; graph elements, chronology
and stories; bounded reading/neighbour windows and cache; trail renewal and
focus; structured wire/contract/read failures. Those actual modules and their
laws are still owed. Each private scalar must have one validating Result door
with its own reachable closed failure and independent invalid-input properties.

The owner answered Claude's six choices on ops 75c9925: Years are served
nodes, stepped by served links; an event has one exact/circa span or is undated
with no chronology; the trail keeps the last N steps; Bible and Concord have
distinct types sharing one navigation interface/typeclass; contract names and
the owner's vocabulary win. Concurrency is a short whole list of genuinely
simultaneous events, not every event in the same year. This owner ruling
supersedes pre-review I4's proposed paging. An account is one unbroken verse run;
a fragmented story has multiple accounts of the same event.

Ops a5f02a1 further rules that Scripture-grounded months/days, festivals and
weekdays refine events within a Year; the client must not infer those dates or
concurrency. Festivals are explorable. The amended Year/FOCUS-3 and unlanded
WIREID contracts remain producer dependencies. There are no invented client
year/passage constructors or server trail receipts. Canonical verse identities,
one focus-change path and default 20/40 reading bounds remain binding.

Ops 780e628 further requires ordering by the finest served evidence, including
Scripture's sequence within a day. Sharing a day does not itself establish
concurrency. Use the Bible's calendar without conversion; the producer curates
the events with explicit month/day/festival/weekday evidence first. Those facts
and ordering links belong to the artifact and contract, not client inference.

The [survey and validation record](../reports/2026-10-02-fsharp-client-domain-revision.md)
states the dependency choices and what has actually run. The 89-instance costume
audit, all-source admission/keyword closure, whole-domain sign-off, all-file
newspaper order and 100% parity remain open. No further view/exemplar expansion
or retirement of the C# client is claimed by this checkpoint.
