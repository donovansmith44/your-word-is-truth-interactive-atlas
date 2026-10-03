# F# domain sign-off proposal

Proposal only: no DTOs, effects, UI or implementation. Owner rulings on ops
`2e2be49`, `570f5b9`, `7572dae`; costume audit `a78559f` remains open.
[Full laws, admission boundaries and library survey](../reports/2026-10-02-fsharp-client-domain.md).
Every proposed type is below; linked signatures show all cases, fields and operations.

| Module / every type | Required guarantee |
|---|---|
| [Model.fsi](../../../client-fsharp/Domain/Model.fsi): `NonEmpty<'a>`, `Positive`, `DisplayText`, `ArtifactRoot`, `NodeId`, `EdgeId`, `EventId`, `StoryId`, `BookId`, `TranslationId`, `DocumentId`, `RelationKind`, `EntityKind`, `BibleBook`, `BibleChapter`, `BibleVerse`, `BiblePassage`, `ConcordParagraph`, `ConcordPassage`, `Year`, `YearSpan`, `YearCoverage`, `TextBody`, `HistoricalAccount`, `PassageContext`, `YearContext`, `SpanContext`, `ConcurrentEvents`, `Endpoints<'a>`, `YearEra`, `Direction`, `RelationDirection`, `UnitReference`, `Passage`, `BibleSelection`, `ConcordSelection`, `TextUnit`, `Event`, `Entity`, `Node`, `ElementId`, `Edge`, `Element`, `Position`, `ResolvedValue`, `Chronology`, `StoryStep`, `EventContext`, `JourneyProblem`, `WindowProblem`, `DomainFailure`, `Validation<'a>` | Private admitted identities/references; closed kinds; events group nonempty historical passage accounts. Year is atomic, BC/AD has no zero, spans have ordered endpoints; coverage preserves gaps. Chronology has one earlier/one later plus concurrent set, separate from story steps. Bible citations share canonical verse identity; Small Catechism paragraphs are Concord units. No copied canon/id grammar or inferred historical facts. |
| [Structures.fsi](../../../client-fsharp/Domain/Structures.fsi): `Rooted<'a>`, `Resolved`, `Transition`, `TrailCapacity`, `TrailCursor`, `TrailPage`, `EarlierSteps`, `Trail`, `Renewal`, `RenewalEvidence`, `StepOutcome`, `BackOutcome`, `Reading`, `ReadingKey`, `NeighbourKey`, `TextCursor`, `EdgeCursor`, `ReadingPage`, `NeighbourPage`, `ReadingWindow`, `NeighbourWindow`, `NeighbourCapacity`, `FrontierCapacity`, `WholeChapter`, `ReadingExtent`, `ReadingPageKey`, `NeighbourPageKey`, `PageCacheKey`, `CachedPage`, `CacheCapacity`, `PageCache`, `Frontier`, `Focus` | One root per resolved value/trail/window/cache. Step needs served edge evidence; renewal needs coherent positions **and edges**. Trail holds at most its explicit capacity, earlier steps use server receipts. Reading pages 20, resident reading 40; WholeChapter holds exactly one chapter. Neighbour and aggregate frontier capacities are explicit and separate. Cache is finite immutable LRU keyed by root/query/direction/cursor; no visited-prefix history. |
| [BoundaryModel.fsi](../../../client-fsharp/Domain/BoundaryModel.fsi): `ArrayIndex`, `LineNumber`, `ByteColumn`, `ScalarOffset`, `Utf16Offset`, `ScalarSpan`, `Utf16Span`, `HttpUrl`, `Latitude`, `Longitude`, `ColorToken`, `WireField`, `DocumentField`, `Vocabulary`, `Discriminator`, `Bound`, `Keyword`, `ClientStatus`, `ServerStatus`, `RefusalCode`, `TextPosition`, `Path<'step>`, `JsonStep`, `DocumentStep`, `JsonPath`, `DocumentPath`, `JsonKind`, `IdentityFailure`, `WireFailure`, `ContractError`, `TransientFailure`, `TerminalFailure`, `ReadFailure`, `BoundaryFailure` | One generated wire-admission door. Private scalar wrappers have validating Result doors. Structured paths contain fields and checked indices; errors are closed cases without English. Syntax positions are real, no fabricated path. HTTP(S) URLs, coordinate bounds, palette tokens, status ranges and scalar/UTF16 spans are checked. Transient/terminal failures are separate; only transient can retry. |

[Algebras.fsi](../../../client-fsharp/Domain/Algebras.fsi) declares every pure operation.
Property families required **test-first after sign-off**:

| Algebra | Laws |
|---|---|
| Nonempty/positive | Nonempty output; append associativity; map identity/composition; positive admission iff >0. |
| Years/references | Total order; ordered same-corpus containment. Hull: associative, commutative, idempotent, no empty identity. Coverage union additionally has empty identity and preserves gaps. References use admitted ordinals, never labels. |
| Trail | Begin/back at origin identities; resident step/back inverse; bounded evidence. Cross-root step requests renewal. Accepted renewal preserves exact journey/order on one root; refusal leaves the old value. Same-root complete renewal idempotent. |
| Paging | Same-query identity; append/prepend preserve admitted pages; resident <=40 at >=10,000 turns. Wrong root/query/cursor, oversize and empty continuation refuse whole. Eviction means append/prepend are **not** inverses; saved server receipts recover pages. WholeChapter is a separate approved extent. |
| Cache/focus | Capacity always holds; deterministic LRU; root/query/direction/cursor distinguish keys. Same-root rebase identity, new-root rebase clears old entries and is idempotent. Focus accepts only subject/root-coherent windows within aggregate budget. |
| Failures/admission | Result bind associativity/identity; independent validation accumulates nonempty failures in declaration order. Every scalar door has valid and invalid generators. Paths preserve structured steps; syntax/value failures never invent missing information. Failed transitions never partially install state. |

Closure gates enumerate **all** client/generator/test/generated sources and every
scalar door: private wrapper, exactly one validating door, no unchecked alternate,
no primitive semantic fields or error prose, structural compound values, invalid
input coverage, negative compile cases, independent expected values, every schema
keyword consumed or explicitly refused. `DisplayText` is unparsed served prose;
framework-owned HTTP injection is the sole sanctioned unchecked shell exception.

Library choices: Microsoft.OpenApi + YamlReader for generation; Thoth Core + STJ
for generated codecs; FsToolkit.ErrorHandling for composition; existing Bolero
router and framework file/URI APIs. Survey links/licences/maintenance and adoption
gates are in the full notes. WASM/AOT compatibility still needs executable gates.

Server/contract gaps (TIME, passage contexts, canonical reference identity,
before/after/history receipts) remain explicit dependencies. No endpoints or
compiled facts are invented on the client. Abstract representations are proposals,
not evidence of admission. Four signatures pass syntax parsing only; domain laws,
all 89 costume fixes, parity and any further exemplar/view work await this sign-off.
