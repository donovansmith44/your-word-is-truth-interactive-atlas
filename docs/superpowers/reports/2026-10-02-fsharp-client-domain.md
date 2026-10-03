# F# client domain proposal: detailed laws and library survey

**Proposal for domain sign-off; no implementation or parity claim.** Owner rulings:
ops `2e2be49`, `570f5b9`, `7572dae`. This replaces further view/exemplar work.
The four small signature files under `client-fsharp/Domain/` are the complete
proposed interfaces. They are not wired into the client or shared contract.

| Domain types — every declaration in [Model.fsi](../../../client-fsharp/Domain/Model.fsi) | Meaning and invariant |
|---|---|
| `NonEmpty<'a>`, `Positive`, `DisplayText`, `Endpoints<'a>` | A witnessed nonempty collection; a strictly positive count; explicitly unparsed prose; two endpoints. |
| `ArtifactRoot`, `NodeId`, `EdgeId`, `EventId`, `StoryId`, `BookId`, `TranslationId`, `DocumentId`, `RelationKind`, `EntityKind` | Opaque admitted identities/vocabularies. Event/story ids cannot substitute a person/place id. Leaves/vocabularies are generated from the single shared authority; no copied id grammar, canon, kind list or wire DTO imports. `EntityKind` excludes text/event because `Node` carries those cases separately; no catechism-item identity survives the approved replacement. |
| `BibleBook`, `BibleChapter`, `BibleVerse`, `BiblePassage`, `ConcordParagraph`, `ConcordPassage`, `UnitReference`, `Passage`, `BibleSelection`, `ConcordSelection` | References carry admitted identities/order from the compiler. Ranges have ordered endpoints in one corpus; cross-chapter Scripture ranges are allowed. Paragraph numbering comes from the Triglot source. A catechism paragraph **is** the same Concord text unit, with no second item/hop. Every citation to a Bible verse names its canonical unit; translation changes its text, not that identity. |
| `YearEra`, `Year`, `YearSpan`, `YearCoverage` | BC/AD with positive year number, no year zero. Inclusive ordered nonempty span. Coverage is a normalized collection of disjoint spans, including empty coverage. A year is atomic; a span composes years, not a second date convention. |
| `TextBody`, `TextUnit`, `HistoricalAccount`, `Event`, `Entity`, `Node`, `ElementId`, `Edge`, `Element`, `Position`, `ResolvedValue`, `PassageContext`, `YearContext`, `SpanContext` | Text/reference/content admission is coherent. Node discriminants require their payload. An account is a passage with compiled evidence of recording history; an event groups **nonempty** accounts. A graph element is a node or edge. Passage/year/span positions compose admitted values and never invent graph ids. Contexts hold admitted bounded facts, not a new UI/domain authority. |
| `Direction`, `RelationDirection`, `Chronology`, `StoryStep`, `EventContext`, `ConcurrentEvents` | One earlier event, one later event, a concurrent event set; no self/concurrent overlap. Stories have their own identity and previous/next step. Story order never substitutes chronology. Relation directions are typed and their dual comes from the relation declaration. |
| `JourneyProblem`, `WindowProblem`, `DomainFailure`, `Validation<'a>` | Closed typed failures; independent validation can accumulate a nonempty list. No HTTP status, exception, JSON object, DTO, renderer or effect lives in this model. |

| Data structures — every declaration in [Structures.fsi](../../../client-fsharp/Domain/Structures.fsi) | Invariant and bound |
|---|---|
| `Rooted<'a>`, `Resolved`, `Transition` | A private root seal, admitted position/value correspondence and a transition witnessed by a served edge. No kind/identity/root mismatch can be installed through public constructors. |
| `TrailCapacity`, `TrailCursor`, `TrailPage`, `EarlierSteps`, `Trail`, `Renewal`, `RenewalEvidence`, `StepOutcome`, `BackOutcome` | One coherent resident journey with explicit finite step capacity. Old steps are paged through opaque history receipts, rather than forgotten or held forever. Back either returns the previous trail or a typed requirement for earlier steps; at the actual start it is a no-op. `RenewalEvidence` is an admitted coherent batch containing the resident positions **and edge evidence**, on one new root; its decoder validates exact requested identities and path order, rather than accepting an arbitrary nonempty list. Renewal resolves them together. A missing/changed step refuses the replacement. |
| `Reading`, `ReadingKey`, `NeighbourKey`, `TextCursor`, `EdgeCursor`, `ReadingPage`, `NeighbourPage`, `ReadingWindow`, `NeighbourWindow`, `NeighbourCapacity`, `FrontierCapacity`, `WholeChapter`, `ReadingExtent` | Bible selection always has a translation; Concord selection has none. Distinct cursors carry their root/query witness. Reading pages default to **20**, the resident paged reading to **40** units. Sliding evicts whole oldest page segments and preserves served before/after receipts; it never invents offsets/references. Neighbour windows have a positive explicit `NeighbourCapacity` in units; `Focuses.beginAt` takes a separate `FrontierCapacity` limiting aggregate resident neighbour units. A page/window exceeding either budget refuses without changing the previous value. The finite generated relation vocabulary bounds empty-window metadata; no history of evicted windows is retained. `CompleteChapter` is the approved separate mode: exactly one whole chapter, not a whole corpus and not falsely subject to 40. |
| `ReadingPageKey`, `NeighbourPageKey`, `PageCacheKey`, `CachedPage`, `CacheCapacity`, `PageCache`, `Frontier`, `Focus` | Cache keys include root, selection/relation/direction **and cursor**. Capacity is a finite positive policy parameter; immutable LRU eviction. Focus holds a resolved subject and its bounded lazy relation windows; no CSS/HTML or re-derived fact. No whole-collection prefetch. |

Every operation is declared in [Algebras.fsi](../../../client-fsharp/Domain/Algebras.fsi).
The following are the property families to implement **test-first after sign-off**;
they are executable specifications to be written, not tests already passed.

| Algebra | Properties over generated admitted values, including boundary refusals |
|---|---|
| Nonempty / positive | `toList(singleton x)=[x]`; append associative; map identity/composition; every list remains nonempty. `create n` accepts iff `n>0`, otherwise returns the whole `NonPositive` refusal without smuggling a primitive into the error. |
| Years | Comparison is total/transitive; BC order reverses its positive count, 1 BC precedes AD 1 with no year-zero value. `between` admits iff endpoints are ordered. Singleton containment is equality. `cover` is associative, commutative, idempotent and contains both inputs; it is the **convex hull**, not set union, and has no empty identity. Coverage union is associative/commutative/idempotent with empty identity, preserves gaps and yields sorted disjoint spans. |
| References / history | Ordering comes from admitted compiler ordinals, never parsed labels. Passage contains both endpoints and exactly admitted units in its interval. Reversal/corpus mismatch is a whole typed refusal. Equivalent Bible citations map to the identical verse position for every active translation and citation source. Account nonemptiness/history witness, chronology cardinality and chronology/story separation are generated invariant laws. |
| Trail | Begin has no steps. A valid same-root step extends exactly one coherent journey. Back in the resident range undoes the last breadcrumb step; inverse pairs cancel without erasing actual journey evidence. Back at the true origin is identity; at a history boundary it requests earlier steps. Resident steps never exceed capacity. Different-root continuation produces `NeedsRenewal`, never mixed-root success. Accepted renewal preserves positions/edge identities/kinds/order on one new root; refusal preserves the original valid trail. Renew twice on the same complete root is idempotent. |
| Reading / neighbours | Opening and valid append/prepend preserve the complete page and query. Empty window is identity only for a same-query legal page. Resident paged reading never exceeds 40 after >=10,000 generated turns; an oversize page, wrong root/query/cursor, or empty continuation is refused without altering the original. Append/prepend are **not** inverses after eviction; rereading a saved boundary receipt recovers the same page on the same root. Whole chapter returns every admitted unit of that chapter and nothing from another. |
| Cache / focus | Count never exceeds capacity, hits/put update LRU order and one eviction policy, keys differ for root/query/direction/cursor. Rebase to same root is identity; rebase to a different root discards all old-root entries and is idempotent. Focus accepts only windows for its subject/root; all other windows refuse without changing it. Work/retention stay bounded at ten-times graph size. |
| Validation | map identity/composition; independent error concatenation associative and preserves declaration order; no empty refusal. Sequential `Result` bind is associative with `Ok` as identity and first failure propagated whole. Failed transitions never partially install state. |

Admission is the one future wire door: generated contract values enter these
private constructors with checked shape/root/references; all renderers consume
the resulting model. Domain factories compose values, never parse labels or
compute compiled chronology, event grouping, verse identity or source numbering.
The year/reference algebra compares admitted values; it does not format dates or
infer historical claims. Abstract context/identity details remain representation
choices, not unchecked factories. Internal construction and unsafe/default escape
hatches will be fenced by source and negative compile laws before adoption.

Producer gaps are explicit: TIME/year positions, passage contexts, canonical
reference identities and before/after/history receipts require their approved
server/contract/storage mechanisms. This proposal does not add endpoints, shadow
those mechanisms on the client, or bless an old artifact root. Cache/trail/read
budgets remain distinct typed policy parameters with future-size gates; 20/40 is
the already ruled reading policy. Validation of the eventual implementation,
100% parity and migration from current generated DTOs remain ahead.

The admission boundary is separate from the domain and contains no wire DTOs.
Every declaration in [BoundaryModel.fsi](../../../client-fsharp/Domain/BoundaryModel.fsi):
`ArrayIndex`, `LineNumber`, `ByteColumn`, `ScalarOffset`, `Utf16Offset`, `ScalarSpan`,
`Utf16Span`, `HttpUrl`, `Latitude`, `Longitude`, `ColorToken`, `WireField`,
`DocumentField`, `Vocabulary`, `Discriminator`, `Bound`, `Keyword`, `ClientStatus`,
`ServerStatus`, `RefusalCode`, `TextPosition`, `Path<'step>`, `JsonStep`,
`DocumentStep`, `JsonPath`, `DocumentPath`, `JsonKind`, `IdentityFailure`,
`WireFailure`, `ContractError`, `TransientFailure`, `TerminalFailure`,
`ReadFailure`, `BoundaryFailure`. Paths carry generated field steps and validated
indices, not library strings. Syntax failures have real one-based positions and
no fabricated path. URLs admit absolute HTTP(S) only. An unparseable document with no structured library location returns `UnreadableDocument`; no library pointer/message is parsed back into a fake typed path. Offsets preserve scalar vs
UTF-16 units and reject out-of-text spans, with no clamping. Coordinates reject
NaN/infinity and exceedance of latitude ±90/longitude ±180. Client/server refusal
ranges are separate validated types; only the transient case may retry.

**Costume closure, part of sign-off:** [audit a78559f](https://github.com/donovansmith44/your-word-is-truth-interactive-atlas/blob/a78559f/docs/superpowers/reports/2026-10-03-fsharp-costume-audit.md)
is the migration inventory (89 instances, 42 files), not a completed fix. The
implementation must satisfy these generated property/source families over **all**
client/generator/test sources and generated code: private primitive-backed values
have one validating Result door and no unchecked total alternative; structured
values retain their parts; every door has invalid-input generators and negative
compile cases; no primitive semantic fields or library prose enter domain/failures
(`DisplayText` is explicitly unparsed prose); expected values do not call the
constructor under test; every schema keyword is consumed or explicitly refused.
The only unchecked injection exception is the framework-owned HTTP client in the
shell. Public rendering must take witnessed presentations, not their unchecked
payload. Kind-specific node payloads, typed transient/terminal failures and root
witnesses are mandatory; a private name alone is not proof. These interfaces are
proposals: validating implementations and these laws must pass before adoption.

**Library survey (2026-10-03, before more machinery):**

| Candidate / evidence | Fit and proposed choice |
|---|---|
| [Microsoft.OpenApi + YamlReader](https://github.com/microsoft/OpenAPI.NET/blob/main/docs/upgrade-guide-2.md), [3.10.2 release](https://github.com/microsoft/OpenAPI.NET/releases/tag/v3.10.2) | MIT, active release 2026-08-20; typed OpenAPI document plus YAML reader. Build-time only, so no WASM dependency. Choose instead of hand-walking YAML; verify all schema constraints on the real 3.1 contract before replacement. |
| [Thoth.Json.Core + System.Text.Json](https://github.com/thoth-org/Thoth.Json), [decoder documentation](https://thoth-org.github.io/Thoth.Json/documentation/concept/decoder.html) | MIT; current .NET System.Text.Json backend, typed compositional decoders. Preferred candidate for generated codecs; browser/WASM/AOT fit is an inference from the backend/targets and requires a scoped executable spike. Reject Newtonsoft-based legacy line. Generated decoders build our structured paths as they descend; library string paths/English errors never become domain guarantees. |
| [FsToolkit.ErrorHandling](https://github.com/demystifyfp/FsToolkit.ErrorHandling) | MIT, .NET Standard targets; maintained Result/traverse/validation API. Choose for combinators instead of the bespoke Result CE/folds. Pin after verifying package version and no unwanted runtime dependencies; actual WASM use still requires a build gate. |
| [Bolero router](https://fsbolero.io/docs/Routing) / System.IO + System.Uri | Reuse the already selected Bolero routing machinery and framework file/URI libraries. Domain segment/URI checks remain one named admission door, with closed errors. No replacement file/parser framework is needed. |

This confirms the audit's preferred library direction, not completed installations,
codec parity or a permission to write more UI. Versions/attribution are pinned
before adoption; adapters map only documented library outcomes. Implementation
of general-purpose parsing/combinators proceeds after this domain sign-off.

The admission laws also enumerate ArrayIndex negatives, zero/negative line and
column numbers, negative/reversed/out-of-text offsets, relative/non-HTTP(S) URLs,
blank/malformed/wrong-kind identities, NaN/infinite/out-of-range coordinates,
unlisted palette tokens and statuses outside 400–499 / 500–599. Each operation
asserts the entire success/refusal value; no expectation calls its own door.
Paths have empty-list identity and append preserves the exact structural steps;
scalar-to-UTF16 conversion preserves selected Unicode text and refuses malformed
boundaries. Transient retry is a projection of the closed case, never an HTTP
reason or exception sentence. No exception string enters these failures.

These signatures are **not compiled into the app**. Four-file FCS syntax parsing
passes; this is not type-checking, an implementation, a passed domain property
suite, complete costume closure or complete parity. The existing STYLE checkpoint
35f6797 (30 exemplar properties / 108 regression tests, Debug WASM build) remains
separate and unconsumed. Its 89 audit instances remain open until the approved
model and every-source admission/source/property gates replace them. No source
of authority is guessed for opaque identities/references/cursors/contexts;
wire admission must be generated from the signed server contract. Compiler-only
facts such as event attestations, reference membership and chronology require
actual served witnesses, not client derivation. General-purpose validating and
composition machinery uses the surveyed libraries through thin adapters.

The SDK signature check caught an opaque-id constraint gap: `Set<EventId>`
requires exposed comparison, which `type EventId` deliberately does not promise.
`ConcurrentEvents` hides the standard immutable collection representation and is
admitted as distinct event identities; `History.concurrent` exposes its members.
No new general-purpose set implementation is proposed. Chronology properties
assert uniqueness and absence of the subject/earlier/later events. Rechecking with
the installed SDK reports only FS0240 for the four intentionally absent matching
implementations. That is **not a successful compile**. FCS syntax parsing has zero
diagnostics; the exploratory FCS project harness did not resolve its reference
imports and is not verification evidence. No assembled domain library is produced.
