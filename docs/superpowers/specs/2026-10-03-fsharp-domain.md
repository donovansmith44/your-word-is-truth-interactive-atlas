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
compiles all eight public doors and independently attempts all eight forbidden
constructors. The normal regression project discovers these domain properties;
the small DomainLaws project also runs them without a browser build.

The remaining tour must cover served identities/root seals; Year and spans;
Bible/Concord containers, passage/history/text parts; graph elements, chronology
and stories; bounded reading/neighbour windows and cache; trail renewal and
focus; structured wire/contract/read failures. Those actual modules and their
laws are still owed. Each private scalar must have one validating Result door
with its own reachable closed failure and independent invalid-input properties.

The six choices already sent by Claude remain pending with the owner: Year
admission, event dating, trail retention, shared container model, served
vocabulary, and paged concurrent events. Existing owner rulings on shared
containers, canonical verse identities, focus change and bounded reads still
bind the design. Unlanded WIREID/Year/FOCUS-3 contracts are producer dependencies.
There are no invented client year/passage constructors or server trail receipts.

The [survey and validation record](../reports/2026-10-02-fsharp-client-domain-revision.md)
states the dependency choices and what has actually run. The 89-instance costume
audit, all-source admission/keyword closure, whole-domain sign-off, all-file
newspaper order and 100% parity remain open. No further view/exemplar expansion
or retirement of the C# client is claimed by this checkpoint.
