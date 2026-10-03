namespace BibleAtlas.Domain

module NonEmpty =
    val singleton: 'a -> NonEmpty<'a>
    val append: NonEmpty<'a> -> NonEmpty<'a> -> NonEmpty<'a>
    val map: ('a -> 'b) -> NonEmpty<'a> -> NonEmpty<'b>
    val toList: NonEmpty<'a> -> 'a list

module Positive =
    val create: int -> Result<Positive, DomainFailure>
    val value: Positive -> int

module Years =
    val year: YearEra -> Positive -> Year
    val compare: Year -> Year -> int
    val singleton: Year -> YearSpan
    val between: Endpoints<Year> -> Result<YearSpan, DomainFailure>
    val endpoints: YearSpan -> Endpoints<Year>
    val contains: YearSpan -> Year -> bool
    val cover: YearSpan -> YearSpan -> YearSpan
    val empty: YearCoverage
    val includeSpan: YearSpan -> YearCoverage
    val union: YearCoverage -> YearCoverage -> YearCoverage
    val spans: YearCoverage -> YearSpan list

module References =
    val compareBible: BibleVerse -> BibleVerse -> int
    val compareConcord: ConcordParagraph -> ConcordParagraph -> int
    val between: Endpoints<UnitReference> -> Result<Passage, DomainFailure>
    val contains: Passage -> UnitReference -> bool
    val unitPosition: UnitReference -> Position
    val passagePosition: Passage -> Position

module Nodes =
    val id: Node -> NodeId
    val label: Node -> DisplayText
    val kind: Entity -> EntityKind
    val entityKinds: NonEmpty<EntityKind>
    val relations: NonEmpty<RelationKind>
    val dual: RelationDirection -> RelationDirection

module History =
    val accountPassage: HistoricalAccount -> Passage
    val accounts: Event -> NonEmpty<HistoricalAccount>
    val context: Rooted<EventContext> -> EventContext
    val concurrent: Chronology -> EventId list

module Rooted =
    val root: Rooted<'a> -> ArtifactRoot
    val value: Rooted<'a> -> 'a

module Resolution =
    val position: Resolved -> Position
    val root: Resolved -> ArtifactRoot
    val value: Resolved -> Rooted<ResolvedValue>

module Trails =
    val capacity: Positive -> TrailCapacity
    val beginAt: TrailCapacity -> Resolved -> Trail
    val transition: Rooted<Edge> -> Resolved -> Resolved -> Result<Transition, DomainFailure>
    val step: Transition -> Trail -> Result<StepOutcome, DomainFailure>
    val back: Trail -> BackOutcome
    val restoreEarlier: TrailPage -> EarlierSteps -> Result<Trail, DomainFailure>
    val renew: Rooted<RenewalEvidence> -> Renewal -> Result<Trail, DomainFailure>
    val current: Trail -> Resolved
    val steps: Trail -> Transition list
    val breadcrumb: Trail -> Transition list
    val requestedPositions: Renewal -> NonEmpty<Position>

module ReadingPages =
    val key: ReadingPage -> ReadingKey
    val cacheKey: ReadingPage -> PageCacheKey
    val units: ReadingPage -> TextUnit list
    val before: ReadingPage -> TextCursor option
    val after: ReadingPage -> TextCursor option
    val pageSize: Positive
    val windowSize: Positive

module ReadingWindows =
    val empty: ReadingKey -> ReadingWindow
    val openAt: ReadingPage -> ReadingWindow
    val append: ReadingPage -> ReadingWindow -> Result<ReadingWindow, DomainFailure>
    val prepend: ReadingPage -> ReadingWindow -> Result<ReadingWindow, DomainFailure>
    val key: ReadingWindow -> ReadingKey
    val units: ReadingWindow -> TextUnit list
    val before: ReadingWindow -> TextCursor option
    val after: ReadingWindow -> TextCursor option

module WholeChapters =
    val chapter: WholeChapter -> BibleChapter
    val units: WholeChapter -> NonEmpty<TextUnit>

module Neighbours =
    val capacity: Positive -> NeighbourCapacity
    val empty: NeighbourCapacity -> NeighbourKey -> NeighbourWindow
    val openAt: NeighbourCapacity -> NeighbourPage -> Result<NeighbourWindow, DomainFailure>
    val append: NeighbourPage -> NeighbourWindow -> Result<NeighbourWindow, DomainFailure>
    val key: NeighbourWindow -> NeighbourKey
    val positions: NeighbourWindow -> Position list
    val after: NeighbourWindow -> EdgeCursor option

module PageKeys =
    val reading: ReadingKey -> TextCursor option -> Result<PageCacheKey, DomainFailure>
    val neighbours: NeighbourKey -> EdgeCursor option -> Result<PageCacheKey, DomainFailure>

module PageCaches =
    val capacity: Positive -> CacheCapacity
    val empty: CacheCapacity -> ArtifactRoot -> PageCache
    val put: CachedPage -> PageCache -> Result<PageCache, DomainFailure>
    val find: PageCacheKey -> PageCache -> CachedPage option * PageCache
    val rebase: ArtifactRoot -> PageCache -> PageCache
    val entries: PageCache -> (PageCacheKey * CachedPage) list

module Focuses =
    val capacity: Positive -> FrontierCapacity
    val beginAt: FrontierCapacity -> Resolved -> Focus
    val current: Focus -> Resolved
    val frontier: Focus -> Frontier
    val showNeighbours: NeighbourWindow -> Focus -> Result<Focus, DomainFailure>

module Validations =
    val map: ('a -> 'b) -> Validation<'a> -> Validation<'b>
    val combine: ('a -> 'b -> 'c) -> Validation<'a> -> Validation<'b> -> Validation<'c>
