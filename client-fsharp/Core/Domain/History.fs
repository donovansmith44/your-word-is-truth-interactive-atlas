namespace BibleAtlas.FSharp.Domain

open BibleAtlas.FSharp.Contract

type EventId<'nodeId> = private EventId of 'nodeId

type StoryId<'nodeId> = private StoryId of 'nodeId

type HistoricalAccount<'nodeId, 'reference, 'mark> = private HistoricalAccount of Passage<'nodeId, 'reference, 'mark>

type Chronology<'nodeId> =
    private
        { Previous: EventId<'nodeId> option
          Next: EventId<'nodeId> option
          AtTheSameTime: EventId<'nodeId> list }

[<RequireQualifiedAccess>]
type When<'nodeId, 'timeEvidence> =
    | Undated
    | Dated of Dating<'nodeId> * evidence: 'timeEvidence list * chronology: Chronology<'nodeId>

type StoryStep<'nodeId> =
    private { Story: StoryId<'nodeId>; Previous: EventId<'nodeId> option; Next: EventId<'nodeId> option }

type Event<'nodeId, 'reference, 'mark, 'timeEvidence> =
    private
        { Id: EventId<'nodeId>
          Label: string
          Accounts: HistoricalAccount<'nodeId, 'reference, 'mark> list
          When: When<'nodeId, 'timeEvidence> }

type EventContext<'nodeId, 'reference, 'mark, 'timeEvidence> =
    private { Event: Event<'nodeId, 'reference, 'mark, 'timeEvidence>; Stories: StoryStep<'nodeId> list }

[<RequireQualifiedAccess>]
type EventIdentityFailure = NotAnEvent

[<RequireQualifiedAccess>]
type StoryIdentityFailure = NotAStory

[<RequireQualifiedAccess>]
type HistoricalAccountFailure = DoesNotRecordHistory

[<RequireQualifiedAccess>]
type ChronologyFailure = SelfLink | RepeatedConcurrentEvent | OrderedAndConcurrent

module History =
    let admitEventId (id: 'nodeId) (kind: NodeKind) : Result<EventId<'nodeId>, EventIdentityFailure> =
        DomainSkeleton.pending "History.admitEventId"
    let admitStoryId (id: 'nodeId) (kind: NodeKind) : Result<StoryId<'nodeId>, StoryIdentityFailure> =
        DomainSkeleton.pending "History.admitStoryId"
    let admitAccount (recordsHistory: 'mark -> bool) (passage: Passage<'nodeId, 'reference, 'mark>) : Result<HistoricalAccount<'nodeId, 'reference, 'mark>, HistoricalAccountFailure> =
        DomainSkeleton.pending "History.admitAccount"
    let admitChronology (event: EventId<'nodeId>) (previous: EventId<'nodeId> option) (next: EventId<'nodeId> option) (concurrent: EventId<'nodeId> list) : Result<Chronology<'nodeId>, ChronologyFailure> =
        DomainSkeleton.pending "History.admitChronology"
    let event (id: EventId<'nodeId>) (label: string) (accounts: HistoricalAccount<'nodeId, 'reference, 'mark> list) (dating: When<'nodeId, 'timeEvidence>) : Event<'nodeId, 'reference, 'mark, 'timeEvidence> =
        { Id = id; Label = label; Accounts = accounts; When = dating }
    let storyStep (story: StoryId<'nodeId>) (previous: EventId<'nodeId> option) (next: EventId<'nodeId> option) : StoryStep<'nodeId> =
        { Story = story; Previous = previous; Next = next }
    let eventContext (event: Event<'nodeId, 'reference, 'mark, 'timeEvidence>) (stories: StoryStep<'nodeId> list) : EventContext<'nodeId, 'reference, 'mark, 'timeEvidence> =
        { Event = event; Stories = stories }
    let accountPassage (HistoricalAccount passage) : Passage<'nodeId, 'reference, 'mark> = passage
    let accounts (event: Event<'nodeId, 'reference, 'mark, 'timeEvidence>) : HistoricalAccount<'nodeId, 'reference, 'mark> list = event.Accounts
    let concurrent (chronology: Chronology<'nodeId>) : EventId<'nodeId> list = chronology.AtTheSameTime
    let previous (chronology: Chronology<'nodeId>) : EventId<'nodeId> option = chronology.Previous
    let next (chronology: Chronology<'nodeId>) : EventId<'nodeId> option = chronology.Next
    let node (EventId id) : 'nodeId = id
    let storyNode (StoryId id) : 'nodeId = id
    let dating (event: Event<'nodeId, 'reference, 'mark, 'timeEvidence>) : When<'nodeId, 'timeEvidence> = event.When
    let context (context: EventContext<'nodeId, 'reference, 'mark, 'timeEvidence>) : Event<'nodeId, 'reference, 'mark, 'timeEvidence> = context.Event
    let stories (context: EventContext<'nodeId, 'reference, 'mark, 'timeEvidence>) : StoryStep<'nodeId> list = context.Stories
