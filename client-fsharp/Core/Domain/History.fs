namespace BibleAtlas.FSharp.Domain

open BibleAtlas.FSharp.Contract

type EventId<'nodeId> = private EventId of 'nodeId

type StoryId<'nodeId> = private StoryId of 'nodeId

[<RequireQualifiedAccess>]
type AccountRun<'nodeId, 'partRole, 'mark> =
    | Verse of TextUnit<'nodeId, 'partRole>
    | Passage of Passage<'nodeId, 'mark>

type HistoricalAccount<'nodeId, 'partRole, 'mark> =
    private HistoricalAccount of AccountRun<'nodeId, 'partRole, 'mark>

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

type Event<'nodeId, 'partRole, 'mark, 'timeEvidence> =
    private
        { Id: EventId<'nodeId>
          Label: string
          Accounts: HistoricalAccount<'nodeId, 'partRole, 'mark> list
          When: When<'nodeId, 'timeEvidence> }

type EventContext<'nodeId, 'partRole, 'mark, 'timeEvidence> =
    private { Event: Event<'nodeId, 'partRole, 'mark, 'timeEvidence>; Stories: StoryStep<'nodeId> list }

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
    let admitAccount (recordsHistory: AccountRun<'nodeId, 'partRole, 'mark> -> bool) (run: AccountRun<'nodeId, 'partRole, 'mark>) : Result<HistoricalAccount<'nodeId, 'partRole, 'mark>, HistoricalAccountFailure> =
        DomainSkeleton.pending "History.admitAccount"
    let admitChronology (event: EventId<'nodeId>) (previous: EventId<'nodeId> option) (next: EventId<'nodeId> option) (concurrent: EventId<'nodeId> list) : Result<Chronology<'nodeId>, ChronologyFailure> =
        DomainSkeleton.pending "History.admitChronology"
    let event (id: EventId<'nodeId>) (label: string) (accounts: HistoricalAccount<'nodeId, 'partRole, 'mark> list) (dating: When<'nodeId, 'timeEvidence>) : Event<'nodeId, 'partRole, 'mark, 'timeEvidence> =
        { Id = id; Label = label; Accounts = accounts; When = dating }
    let storyStep (story: StoryId<'nodeId>) (previous: EventId<'nodeId> option) (next: EventId<'nodeId> option) : StoryStep<'nodeId> =
        { Story = story; Previous = previous; Next = next }
    let eventContext (event: Event<'nodeId, 'partRole, 'mark, 'timeEvidence>) (stories: StoryStep<'nodeId> list) : EventContext<'nodeId, 'partRole, 'mark, 'timeEvidence> =
        { Event = event; Stories = stories }
    let accountRun (HistoricalAccount run) : AccountRun<'nodeId, 'partRole, 'mark> = run
    let accounts (event: Event<'nodeId, 'partRole, 'mark, 'timeEvidence>) : HistoricalAccount<'nodeId, 'partRole, 'mark> list = event.Accounts
    let concurrent (chronology: Chronology<'nodeId>) : EventId<'nodeId> list = chronology.AtTheSameTime
    let previous (chronology: Chronology<'nodeId>) : EventId<'nodeId> option = chronology.Previous
    let next (chronology: Chronology<'nodeId>) : EventId<'nodeId> option = chronology.Next
    let node (EventId id) : 'nodeId = id
    let storyNode (StoryId id) : 'nodeId = id
    let dating (event: Event<'nodeId, 'partRole, 'mark, 'timeEvidence>) : When<'nodeId, 'timeEvidence> = event.When
    let context (context: EventContext<'nodeId, 'partRole, 'mark, 'timeEvidence>) : Event<'nodeId, 'partRole, 'mark, 'timeEvidence> = context.Event
    let stories (context: EventContext<'nodeId, 'partRole, 'mark, 'timeEvidence>) : StoryStep<'nodeId> list = context.Stories
