namespace BibleAtlas.Domain

type NonEmpty<'a>
type Positive
type DisplayText
type ArtifactRoot
type NodeId
type EdgeId
type EventId
type StoryId
type BookId
type TranslationId
type DocumentId
type RelationKind
type EntityKind
type BibleBook
type BibleChapter
type BibleVerse
type BiblePassage
type ConcordParagraph
type ConcordPassage
type Year
type YearSpan
type YearCoverage
type TextBody
type HistoricalAccount
type PassageContext
type YearContext
type SpanContext
type ConcurrentEvents

type Endpoints<'a> = { First: 'a; Last: 'a }
[<RequireQualifiedAccess>]
type YearEra = BC | AD
[<RequireQualifiedAccess>]
type Direction = Outward | Inward
type RelationDirection = { Relation: RelationKind; Direction: Direction }
[<RequireQualifiedAccess>]
type UnitReference = BibleVerse of BibleVerse | ConcordParagraph of ConcordParagraph
[<RequireQualifiedAccess>]
type Passage = Scripture of BiblePassage | Confession of ConcordPassage
[<RequireQualifiedAccess>]
type BibleSelection = Book of BibleBook | Chapter of BibleChapter | Verse of BibleVerse | Passage of BiblePassage
[<RequireQualifiedAccess>]
type ConcordSelection = Paragraph of ConcordParagraph | Passage of ConcordPassage

type TextUnit = private { Id: NodeId; Reference: UnitReference; Label: DisplayText; Body: TextBody }
type Event = private { Id: EventId; Label: DisplayText; Accounts: NonEmpty<HistoricalAccount>; Years: YearCoverage }
type Entity = private { Id: NodeId; Kind: EntityKind; Label: DisplayText }
[<RequireQualifiedAccess>]
type Node = TextUnit of TextUnit | Event of Event | Entity of Entity
[<RequireQualifiedAccess>]
type ElementId = Node of NodeId | Edge of EdgeId

type Edge = private { Id: EdgeId; Relation: RelationKind; Subject: ElementId; Object: ElementId; Label: DisplayText }
[<RequireQualifiedAccess>]
type Element = Node of Node | Edge of Edge
[<RequireQualifiedAccess>]
type Position = Element of ElementId | Passage of Passage | Year of Year | Years of YearSpan
[<RequireQualifiedAccess>]
type ResolvedValue = GraphElement of Element | Passage of PassageContext | Year of YearContext | Years of SpanContext

type Chronology = private { Earlier: EventId option; Later: EventId option; Concurrent: ConcurrentEvents }
type StoryStep = private { Story: StoryId; Earlier: EventId option; Later: EventId option }
type EventContext = private { Event: Event; Chronology: Chronology; Stories: StoryStep list }

[<RequireQualifiedAccess>]
type JourneyProblem = MissingStart | WrongLength | DifferentIdentity | InvalidStep
[<RequireQualifiedAccess>]
type WindowProblem = WrongQuery | WrongCursor | OversizedPage | EmptyContinuation
[<RequireQualifiedAccess>]
type DomainFailure =
    | NonPositive
    | ReversedYears of Endpoints<Year>
    | ReversedPassage of Endpoints<UnitReference>
    | DifferentCorpora of Endpoints<UnitReference>
    | MissingPosition of Position
    | ChangedRoot of expected: ArtifactRoot * received: ArtifactRoot
    | UnrelatedPositions of fromPosition: Position * relation: RelationKind * toPosition: Position
    | InvalidJourney of JourneyProblem
    | InvalidWindow of WindowProblem
    | OversizedFrontier

[<RequireQualifiedAccess>]
type Validation<'a> = Accepted of 'a | Refused of NonEmpty<DomainFailure>
