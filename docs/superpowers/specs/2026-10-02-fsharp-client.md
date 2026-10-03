# F# client alongside C#

Owner authorization: 2026-10-02, whole frontend rewrite in F# WebAssembly using Bolero and Elmish MVU, coexistence before retirement. Initial reference: `ace063d`, approved FOCUS-2/style. Claude continues to own the C# client and FOCUS architecture. Codex writes only the migration item's separate paths.

The pinned reference now includes approved A-NOBLURB `e57542c`. The Reader and Concord view slice uses TextWindow's complete UnitText, NodeRef, UnitHeading and served Contents labels. Each row, heading and inline anchor emits OpenPosition with a generated PositionRef; no legacy reference/id parser supplies a domain fact. Inline rendering shares AnchoredText, keyboard activation and the existing explicit failure view. Internal anchor selectors retain opaque node ids and served offsets; legacy selector spelling is not yet a parity claim. Contents controls, paging, focus presentation, selections, split composition and persistence remain separate required slices.

## Boundaries and behavior

The Rust server, compiled artifact and OpenAPI stay authoritative. No server or artifact changes are required by the language migration. F# consumes generated immutable records, closed enum unions and tagged unions derived from the committed OpenAPI, built into `obj/`; it does not link a C# application assembly. Both clients consume the same server, JS map module, CSS and font files. Assets are linked by MSBuild, never copied into an independently maintained second source.

Bolero runs as a standalone Blazor WebAssembly app. Elmish owns state: views emit messages, a pure transition describes effects, and the Elmish adapter interprets effects into Cmd. Async completions carry request identity; obsolete completions cannot modify newer state. Browser/HTTP/map/storage interop is outside the transition. There are no mutable atoms, global service locator, swallowed exceptions or partial input matches.

## Types and signatures

The initial core API is shown below. Later rendering/runtime modules extend this catalog before their implementation. Generated types are enumerated after it; their full fields and cases are derived from `contracts/openapi.yaml`, not manually redeclared.

```fsharp
type Failure = Transport of string | Contract of string | ArtifactMoved of resolved: string * serving: string

type RequestId = private RequestId of int64
module RequestId =
    val initial: RequestId
    val next: RequestId -> RequestId

type LoadState<'a> = Empty | Loading of RequestId * previous: 'a option | Ready of 'a | Failed of RequestId * Failure * previous: 'a option
module LoadState =
    val beginRead: RequestId -> LoadState<'a> -> LoadState<'a>
    val complete: RequestId -> Result<'a, Failure> -> LoadState<'a> -> LoadState<'a>

type Cache<'key, 'value when 'key: comparison> = private Cache of capacity: int * entries: Map<'key, 'value> * recent: 'key list
module Cache =
    val empty: int -> Cache<'key, 'value>
    val find: 'key -> Cache<'key, 'value> -> 'value option * Cache<'key, 'value>
    val put: 'key -> 'value -> Cache<'key, 'value> -> Cache<'key, 'value>
    val count: Cache<'key, 'value> -> int

type Link = { Kind: EdgeKind; Target: PositionRef }
type Resolved = private NodeResolved of root: string * node: NodeRecord | EdgeResolved of root: string * edge: EdgeRecord
module Resolved =
    val ofElement: string -> Element -> Result<Resolved, Failure>
    val root: Resolved -> string
    val element: Resolved -> Element
    val position: Resolved -> PositionRef
type Step = { Kind: EdgeKind; Target: Resolved }
type Trail = private { Start: Resolved; Steps: Step list }
module Trail =
    val beginAt: Resolved -> Trail
    val follow: Step -> Trail -> Trail
    val current: Trail -> Resolved
    val walked: Trail -> Resolved list
    val breadcrumb: Trail -> Step list
    val onOneRoot: Trail -> bool
    val resume: Resolved list -> EdgeKind list -> Result<Trail, Failure>

type Explorer = { Resolve: PositionRef list -> Async<Result<Resolved list, Failure>> }
type Explore<'a> = private Explore of (Explorer -> Trail -> Async<Result<'a * Trail, Failure>>)
module Explore =
    val result: 'a -> Explore<'a>
    val bind: ('a -> Explore<'b>) -> Explore<'a> -> Explore<'b>
    val map: ('a -> 'b) -> Explore<'a> -> Explore<'b>
    val run: Explorer -> Trail -> Explore<'a> -> Async<Result<'a * Trail, Failure>>
    val here: Explore<Resolved>
    val follow: Link -> Explore<Resolved>
    val renew: Explore<Resolved>
    val back: Explore<Resolved>
    val beginAt: Explorer -> PositionRef -> Async<Result<Trail, Failure>>
    val resume: Explorer -> PositionRef -> Link list -> Async<Result<Trail, Failure>>

type ExploreBuilder =
    new: unit -> ExploreBuilder
    member Return: 'a -> Explore<'a>
    member ReturnFrom: Explore<'a> -> Explore<'a>
    member Bind: Explore<'a> * ('a -> Explore<'b>) -> Explore<'b>

module Json =
    val encode: 'a -> string
    val decode<'a>: string -> Result<'a, Failure>
module EdgeKinds =
    val dual: EdgeKind -> EdgeKind

module Generator =
    val generate: document: string -> Result<string, string>
    val generateFile: document: string -> output: string -> Result<unit, string>
```

Exploration is StateT Trail over Async/Result: the trail is append-only, breadcrumbs cancel a step followed by its dual back to its source, and Back records a retracing step. Resolve is one batched read. Begin/resume refuse wrong cardinality, missing elements and mixed roots. The monad laws compare whole values and trails; no identity equality shortcut hides a root mismatch. Edge duality comes from the existing generated vocabulary fixture, not a transcribed list.

Cache and page state are bounded; no cursor history grows with the journey. Failures remain retryable. Text rendering slices only served Unicode scalar spans and preserves served labels, kinds and words-of-Christ spans. Client interaction derivations stay on the client; domain derivations stay with the existing compiler/server.

## Contract-generated catalog

| Schema | F# shape |
|---|---|
| `Anchor` | object |
| `BibleRef` | record payload |
| `BookDetail` | object |
| `BookId` | closed union |
| `CanonBook` | object |
| `CatechismDetail` | object |
| `CatechismItem` | object |
| `CatechismProofVerse` | object |
| `CatechismRef` | object |
| `Chapter` | object |
| `ConcordRef` | record payload |
| `Confidence` | closed union |
| `Contents` | object |
| `ContentsChild` | object |
| `ContentsChildKind` | closed union |
| `ContentsRoot` | object |
| `ContentsRootKind` | closed union |
| `Contract` | object |
| `Corpus` | closed union |
| `CrossRef` | object |
| `DateClaim` | object |
| `EdgeElement` | record payload |
| `EdgeEntry` | object |
| `EdgeKind` | closed union |
| `EdgePage` | object |
| `EdgePosition` | record payload |
| `EdgeRecord` | object |
| `EdgeRef` | object |
| `EdgeSummaryEntry` | object |
| `Element` | tagged union |
| `ElementPage` | object |
| `Era` | object |
| `EraDetail` | object |
| `EraId` | string |
| `ErrorBody` | object |
| `ErrorCode` | closed union |
| `ErrorInner` | object |
| `EventAnalogue` | object |
| `EventDetail` | object |
| `EventKind` | closed union |
| `EventPage` | object |
| `EventWitness` | object |
| `Heading` | object |
| `KretzmannChapter` | object |
| `KretzmannChapterItem` | object |
| `KretzmannChapterVerse` | object |
| `LandMask` | object |
| `Landmark` | object |
| `LandmarkKind` | closed union |
| `LandmarkSize` | closed union |
| `MapDetail` | object |
| `MissingElement` | record payload |
| `Narrative` | object |
| `NarrativeAdjacentEvent` | object |
| `NarrativeEventPositions` | object |
| `NarrativeId` | string |
| `NarrativePosition` | object |
| `NodeElement` | record payload |
| `NodeKind` | closed union |
| `NodePosition` | record payload |
| `NodeRecord` | object |
| `NodeRef` | object |
| `Parentage` | closed union |
| `PersonLife` | object |
| `PersonRef` | object |
| `PlaceDetail` | object |
| `PlaceRef` | object |
| `Point` | array |
| `Polities` | object |
| `Polity` | object |
| `PolityDelta` | object |
| `PolityDetail` | object |
| `PolityId` | string |
| `PositionRef` | tagged union |
| `ProvenanceEntry` | object |
| `QuietPlace` | object |
| `Scene` | object |
| `SceneArrow` | object |
| `SceneEvent` | object |
| `SceneMode` | closed union |
| `SceneNarrative` | object |
| `ScenePlace` | object |
| `SourceCategory` | object |
| `SourceEntry` | object |
| `SourcesDocument` | object |
| `Testament` | closed union |
| `TextPoint` | object |
| `TextRef` | tagged union |
| `TextScope` | closed union |
| `TextSpan` | object |
| `TextUnit` | object |
| `TextWindow` | object |
| `TimeRange` | object |
| `TimelinePosition` | object |
| `UnitHeading` | object |
| `UnitText` | object |
| `Verse` | object |
| `VerseGroup` | object |
| `WindowDir` | closed union |
| `WordsOfChristSpan` | object |
| `Year` | object |
| `YearSpan` | object |

String id schemas remain wire aliases where the contract declares them open; enum schemas become qualified DUs. Optional/nullable properties become option, arrays become immutable lists, tagged inheritance becomes a DU over immutable payload records. JSON encodings retain every published property/discriminator/case spelling. An unsupported shape is a generator error, never JsonElement or obj as a fallback. A new schema/enum case is generated automatically; match warnings become build errors.

## Parity and rollout

The migration ledger pins the reference commit, source-file hashes, contract hash, route inventory and every browser scenario. A source entry is pending until implementation and evidence identify its F# replacement; no source deletion marks it complete automatically. Runtime parity compares whole served payloads, visible text, links, accessibility, controls, URL/history, network request bounds, saved trails, split/follow state, map interaction/lifecycle and degraded/failure states. Chromium and WebKit runs include the owner's Mac styling regression.

Reader, World, Kretzmann, Concord, Sources and NotFound all migrate. Contents/pickers, text and linked anchors, legacy nodes still awaiting FOCUS, popover/trail/save/selection, time controls, map, split panes and storage migrations all count. Adopt every reviewed/landed FOCUS change and update the pinned reference deliberately. Known reference defects are recorded separately from intended behavior; do not reproduce a known defect to make an assertion pass.

C# stays runnable throughout. Completion requires zero pending parity entries against the then-current approved reference, both clients tested side by side, independent review, and a Claude handoff. C# removal is a later owner-approved rollout step.

Primary framework documentation: [Bolero Elmish](https://fsbolero.io/docs/Elmish), [WebAssembly hosting](https://fsbolero.io/docs/Hosting), [F# JSON encoding](https://github.com/Tarmil/FSharp.SystemTextJson/blob/master/docs/Customizing.md). Dependencies are pinned in project files with package license evidence in the migration report.

## MVU route/loading slice

```fsharp
type ReadingLocation = { Book: BookId; Chapter: int }
type Route = Reader | Read of ReadingLocation | World | Kretzmann | Concord of string option | Sources | NotFound
module Routes =
    val parse: System.Uri -> Route
    val url: Route -> string

type Model = { Route: Route; Serial: RequestId; Contents: Map<Corpus, LoadState<Contents>>; Reading: LoadState<TextWindow>; Sources: LoadState<SourcesDocument> }
type Message = Navigate of Route | Retry | ContentsLoaded of Corpus * RequestId * Result<Contents, Failure> | TextLoaded of RequestId * Result<TextWindow, Failure> | SourcesLoaded of RequestId * Result<SourcesDocument, Failure>
type Effect = ReadContents of Corpus * RequestId | ReadText of RequestId * Request<TextWindow> | ReadSources of RequestId
module Model =
    val init: Route -> Model * Effect list
    val update: Message -> Model -> Model * Effect list

type Request<'a> = private Request of uri: string
module Request =
    val uri: Request<'a> -> string
module Api =
    val read: System.Net.Http.HttpClient -> System.Threading.CancellationToken -> Request<'a> -> Async<Result<'a, Failure>>
```

Read constructors and parameter/response types in `Reads` are generated from every JSON GET operation in OpenAPI. Omitted optional parameters remain omitted; path/query values use standard URI encoding. Core produces only effect descriptions; Bolero's Elmish adapter is the sole Cmd interpreter. World/Kretzmann/split/focus/persistence extend this slice only with their own preceding tests and type catalog.

## Bolero entry and rendering

```fsharp
module View =
    val app: Model -> (Message -> unit) -> Bolero.Node
type AppView() =
    inherit Bolero.ElmishComponent<Model, Message>
type App() =
    inherit Bolero.ProgramComponent<Model, Message>
    member Http: System.Net.Http.HttpClient with get, set
    member Navigation: Microsoft.AspNetCore.Components.NavigationManager with get, set
    override Program: Elmish.Program<Bolero.ProgramComponent<Model, Message>, Model, Message, Bolero.Node>
module Runtime =
    val command: System.Net.Http.HttpClient -> Effect -> Elmish.Cmd<Message>
module Program =
    val main: string array -> int
```

The DI properties are confined to the Blazor entry component. Render functions accept immutable models and dispatch only; HTTP is interpreted through Elmish Cmd. Browser navigation delivers Navigate messages and has its subscription disposed with the component. Assets are linked from the independent worktree's pinned C# inputs, so CSS/JS/font bytes have one maintained source.

The text view segments only the served scalar offsets; it does not discover anchors or red-letter runs from text.

```fsharp
type TextPiece = { Text: string; IsWordsOfChrist: bool; Anchor: Anchor option }
module AnchoredText =
    val runs: UnitText -> TextPiece list list
```

Generated read signatures (the build derives these from the published operations):

```fsharp
module Reads =
    val books: unit -> Request<(CanonBook) list>
    val catechismItem: string -> Request<CatechismItem>
    val catechismForSpan: string -> Request<(CatechismRef) list>
    val chapter: string -> Request<Chapter>
    val contents: string -> Request<Contents>
    val contract: unit -> Request<Contract>
    val elements: string list -> int option -> Request<ElementPage>
    val eras: unit -> Request<(Era) list>
    val event: string -> Request<EventPage>
    val kretzmannChapter: string -> Request<KretzmannChapter>
    val landMask: unit -> Request<LandMask>
    val landmarks: unit -> Request<(Landmark) list>
    val narrativeEventPositions: string -> Request<NarrativeEventPositions>
    val narratives: unit -> Request<(Narrative) list>
    val nodeRecord: string -> Request<NodeRecord>
    val nodeEdges: string -> EdgeKind -> int option -> int option -> Request<EdgePage>
    val polities: int -> int -> Request<Polities>
    val sceneTime: int -> int -> Request<Scene>
    val sceneScripture: string -> Request<Scene>
    val sources: unit -> Request<SourcesDocument>
    val textWindow: string -> int option -> WindowDir option -> TextScope option -> Corpus option -> Request<TextWindow>
    val xrefs: string -> Request<(CrossRef) list>
```

The graph effect interpreter resolves an ordered list of served positions. The generic elements operation may page; the interpreter rejects a root change and malformed cardinality/identity instead of admitting a partial journey. A resolver has one public read, independent of the surface that requested it.

```fsharp
module Graph =
    val explorer: System.Net.Http.HttpClient -> Explorer
```

Position identity is the shared comparison door for traversal and the effect interpreter. Labels may change across roots; identity compares the typed kind and id.

```fsharp
module Positions =
    val id: PositionRef -> string
    val sameIdentity: PositionRef -> PositionRef -> bool
```

The model owns focus state. Commands retain a snapshot of the journey they started from; completions can replace only their current request identity. The traversal intent is data, interpreted through the exploration algebra in Runtime.

```fsharp
[<RequireQualifiedAccess>]
type Traversal = Follow of Link | Back | Renew
[<RequireQualifiedAccess>]
type FocusState =
    | Closed
    | Opening of RequestId * PositionRef
    | Opened of Trail
    | Walking of RequestId * Trail * Traversal
    | CouldNotOpen of PositionRef * Failure
    | CouldNotWalk of Trail * Traversal * Failure

type Model = { Route: Route; Serial: RequestId; Contents: Map<Corpus, LoadState<Contents>>; Reading: LoadState<TextWindow>; Sources: LoadState<SourcesDocument>; Focus: FocusState }
type Message =
    | Navigate of Route | Retry
    | ContentsLoaded of Corpus * RequestId * Result<Contents, Failure>
    | TextLoaded of RequestId * Result<TextWindow, Failure>
    | SourcesLoaded of RequestId * Result<SourcesDocument, Failure>
    | OpenPosition of PositionRef | Traverse of Traversal | CloseFocus | RetryFocus
    | FocusLoaded of RequestId * Result<Trail, Failure>
type Effect =
    | ReadContents of Corpus * RequestId
    | ReadText of RequestId * Request<TextWindow>
    | ReadSources of RequestId
    | ReadOpening of RequestId * PositionRef
    | WalkFocus of RequestId * Trail * Traversal
```
