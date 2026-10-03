namespace BibleAtlas.Domain

type Rooted<'a>
type Resolved
type Transition
type TrailCapacity
type TrailCursor
type TrailPage
type EarlierSteps
type Trail
type Renewal
type RenewalEvidence
[<RequireQualifiedAccess>]
type StepOutcome = Continued of Trail | NeedsRenewal of Renewal
[<RequireQualifiedAccess>]
type BackOutcome = Backed of Trail | NeedsEarlierSteps of EarlierSteps

[<RequireQualifiedAccess>]
type Reading = Bible of selection: BibleSelection * translation: TranslationId | Concord of ConcordSelection
type ReadingKey = { Root: ArtifactRoot; Reading: Reading }
type NeighbourKey = { Root: ArtifactRoot; At: Position; Through: RelationDirection }
type TextCursor
type EdgeCursor
type ReadingPage
type NeighbourPage
type ReadingWindow
type NeighbourWindow
type NeighbourCapacity
type FrontierCapacity
type WholeChapter
[<RequireQualifiedAccess>]
type ReadingExtent = Paged of ReadingWindow | CompleteChapter of WholeChapter

type ReadingPageKey = private { Reading: ReadingKey; After: TextCursor option }
type NeighbourPageKey = private { Neighbours: NeighbourKey; After: EdgeCursor option }
[<RequireQualifiedAccess>]
type PageCacheKey = Reading of ReadingPageKey | Neighbours of NeighbourPageKey
[<RequireQualifiedAccess>]
type CachedPage = Reading of ReadingPage | Neighbours of NeighbourPage
type CacheCapacity
type PageCache
type Frontier
type Focus
