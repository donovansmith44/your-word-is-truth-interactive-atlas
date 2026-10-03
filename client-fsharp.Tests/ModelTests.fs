module rec BibleAtlas.FSharp.Tests.ModelTests

open System
open Xunit
open FsCheck
open FsCheck.Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Admission
open Microsoft.FSharp.Reflection

[<Property>]
let ``navigating to the current route preserves its pending request instead of sending it twice`` (route: Route) =
    let model, _ = Model.init route
    Assert.Equal((model, []), Model.update (Navigate route) model)

[<Property(MaxTest = 1)>]
let ``reading page constructors are private to the surface transition boundary`` () =
    let actual =
        [typeof<ReaderPage>; typeof<ConcordPage>]
        |> List.map (fun shape -> shape.GetConstructors(Reflection.BindingFlags.Instance ||| Reflection.BindingFlags.Public ||| Reflection.BindingFlags.NonPublic) |> Array.map _.IsPublic |> Array.toList)
    Assert.Equal<bool list list>([[false]; [false]], actual)

[<Property>]
let ``a text answer from another corpus is refused before entering either reading surface`` (corpus: Corpus) (NonNull text: NonNull<string>) =
    let route, served, read, other =
        match corpus with
        | Corpus.Bible -> Route.Reader, contents, Reads.textWindow (TextWindowReference.ofContentsReference firstChapter.Ref) None None (Some TextScope.Chapter) (Some Corpus.Bible), TextRef.Concord { Part = 1; Article = 1; Paragraph = 1 }
        | Corpus.Concord -> Route.Concord None, concordContents "served", Reads.textWindow (WireFixtures.identity<TextWindowReference> "served") (Some concordPageSize) None None (Some Corpus.Concord), firstChapter.Locus
    let model, _ = Model.init route
    let pending, _ = Model.update (readingMessage corpus (ReadingMessage.ContentsLoaded(model.Serial, Ok served))) model
    let request = pending.Serial
    let unit: TextUnit = { Ref = (WireFixtures.identity<UnitReference> "opaque"); Node = { Id = (WireFixtures.identity<NodeId> "served"); Kind = NodeKind.TextUnit; Label = "served" }; Heading = None; EdgeSummary = []; Body = { Text = text; Locus = other; Anchors = []; WordsOfChrist = [] } }
    let window: TextWindow = { Units = [unit]; Next = None; Version = (WireFixtures.identity<ArtifactRoot> "root") }
    let failed = ReadSession.beginRead request read Empty |> ReadSession.complete request (Error(WireFixtures.graphFailure(GraphFailure.TextCorpusMismatch { Requested = corpus; Received = (if corpus = Corpus.Bible then Corpus.Concord else Corpus.Bible) })))
    let expected = withState (ReadingState.Active(served, failed)) pending
    Assert.Equal((expected, []), Model.update (readingMessage corpus (ReadingMessage.TextLoaded(request, Ok window))) pending)

[<Property>]
let ``contents from the other corpus preserve the mismatch and perform no text read`` (corpus: Corpus) (NonNull title: NonNull<string>) =
    let route, received = match corpus with Corpus.Bible -> Route.Reader, Corpus.Concord | Corpus.Concord -> Route.Concord None, Corpus.Bible
    let model, _ = Model.init route
    let offered = { contents with Corpus = received; Roots = contents.Roots |> List.map (fun root -> { root with Title = title }) }
    let failure = WireFixtures.graphFailure(GraphFailure.ContentsCorpusMismatch { Requested = corpus; Received = received })
    let expected = withState (ReadingState.Unavailable failure) model
    Assert.Equal((expected, []), Model.update (readingMessage corpus (ReadingMessage.ContentsLoaded(model.Serial, Ok offered))) model)

[<Property>]
let ``a completion from a departed surface cannot alter any replacement surface`` (NonNull title: NonNull<string>) =
    let departed, _ = Model.init Route.Sources
    let answer: SourcesDocument = { Categories = [{ Id = "generated"; Label = title }]; Sources = []; Provenances = None }
    let destinations = [Route.Reader; Route.Read { Book = BookId.GEN; Chapter = 1 }; Route.World; Route.Kretzmann; Route.Concord None; Route.NotFound]
    let replacements = destinations |> List.map (fun route -> Model.update (Navigate route) departed |> fst)
    let actual = replacements |> List.map (Model.update (Page(SurfaceMessage.Sources(SourcesMessage.Loaded(departed.Serial, Ok answer)))))
    let expected = replacements |> List.map (fun replacement -> replacement, [])
    Assert.Equal<(Model * Effect list) list>(expected, actual)

[<Property>]
let ``retry with no failed operation preserves every surface and pending request`` (route: Route) =
    let model, _ = Model.init route
    let messages = [Page(SurfaceMessage.Reader ReadingMessage.RetryContents); Page(SurfaceMessage.Reader ReadingMessage.RetryText); Page(SurfaceMessage.Concord(ConcordMessage.Reading ReadingMessage.RetryContents)); Page(SurfaceMessage.Concord(ConcordMessage.Reading ReadingMessage.RetryText)); Page(SurfaceMessage.Sources SourcesMessage.Retry)]
    let expected = messages |> List.map (fun _ -> model, [])
    Assert.Equal<(Model * Effect list) list>(expected, messages |> List.map (fun message -> Model.update message model))

[<Property>]
let ``reader startup uses the complete served contents to open its selected chapter`` (NonNull title: NonNull<string>) =
    let served = { contents with Roots = contents.Roots |> List.map (fun root -> { root with Title = title }) }
    let model, effects = Model.init Route.Reader
    Assert.Equal((Surface.Reader(readerPage None (ReadingState.LoadingContents model.Serial)), [ReadContents(Corpus.Bible, model.Serial)]), (model.Surface, effects))
    let request = RequestId.next model.Serial
    let read = Reads.textWindow (TextWindowReference.ofContentsReference firstChapter.Ref) None None (Some TextScope.Chapter) (Some Corpus.Bible)
    let expected = { model with Serial = request; Surface = Surface.Reader(readerPage None (ReadingState.Active(served, ReadSession.beginRead request read Empty))) }
    Assert.Equal((expected, [ReadText(Corpus.Bible, request, read)]), Model.update (Page(SurfaceMessage.Reader(ReadingMessage.ContentsLoaded(model.Serial, Ok served)))) model)

[<Property>]
let ``a text completion after navigation cannot reopen a departed reading`` (code: ErrorCode) =
    let model, _ = Model.init Route.Reader
    let model, _ = Model.update (Page(SurfaceMessage.Reader(ReadingMessage.ContentsLoaded(model.Serial, Ok contents)))) model
    let replacement, _ = Model.update (Navigate Route.Sources) model
    Assert.Equal((replacement, []), Model.update (Page(SurfaceMessage.Reader(ReadingMessage.TextLoaded(model.Serial, Error(WireFixtures.readFailure code))))) replacement)

[<Property>]
let ``source retry advances only its failed operation and keeps its entire last value`` (NonNull title: NonNull<string>) (code: ErrorCode) =
    let model, _ = Model.init Route.Sources
    let prior: SourcesDocument = { Categories = [{ Id = "served"; Label = title }]; Sources = []; Provenances = None }
    let model = { model with Surface = Surface.Sources(Failed(model.Serial, WireFixtures.readFailure code, Some prior)) }
    let request = RequestId.next model.Serial
    let expected = { model with Serial = request; Surface = Surface.Sources(Loading(request, Some prior)) }
    Assert.Equal((expected, [ReadSources request]), Model.update (Page(SurfaceMessage.Sources SourcesMessage.Retry)) model)

[<Property>]
let ``reader and Concord routes roundtrip their generated vocabulary and escaped payloads`` (book: BookId) (PositiveInt chapter) (NonNull reference: NonNull<string>) =
    let expected = [Route.Read { Book = book; Chapter = chapter }; Route.Concord(Some reference)]
    let actual = expected |> List.map (fun route -> Routes.parse (Uri("http://example.test" + Routes.url route)))
    Assert.Equal<Route list>(expected, actual)

[<Property>]
let ``unknown routes book codes and invalid chapter input are refused`` (suffix: uint16) =
    let paths = [$"/read/NEW{suffix}/3"; "/read/JHN/0"; $"/read/JHN/nope{suffix}"; $"/unknown{suffix}"]
    Assert.Equal<Route list>([Route.NotFound; Route.NotFound; Route.NotFound; Route.NotFound], paths |> List.map (fun path -> Routes.parse (Uri("http://example.test" + path))))

[<Property>]
let ``Concord opens its bounded reading from the served corpus beginning`` (NonNull reference: NonNull<string>) =
    let served = concordContents reference
    let model, _ = Model.init (Route.Concord None)
    let request = RequestId.next model.Serial
    let read = Reads.textWindow (WireFixtures.identity<TextWindowReference> reference) (Some concordPageSize) None None (Some Corpus.Concord)
    let expected = { model with Serial = request; Surface = Surface.Concord(concordPage None (ReadingState.Active(served, ReadSession.beginRead request read Empty))) }
    Assert.Equal((expected, [ReadText(Corpus.Concord, request, read)]), Model.update (Page(SurfaceMessage.Concord(ConcordMessage.Reading(ReadingMessage.ContentsLoaded(model.Serial, Ok served))))) model)

[<Property>]
let ``a missing chapter is unavailable without a manufactured request identity or retry`` (PositiveInt chapter) =
    let location = { Book = BookId.GEN; Chapter = 2 + chapter % 100 }
    let model, _ = Model.init (Route.Read location)
    let expected = { model with Surface = Surface.Reader(readerPage (Some location) (ReadingState.Unavailable(WireFixtures.graphFailure(GraphFailure.MissingOpening Corpus.Bible)))) }
    let actual = Model.update (Page(SurfaceMessage.Reader(ReadingMessage.ContentsLoaded(model.Serial, Ok contents)))) model
    Assert.Equal((expected, []), actual)
    Assert.Equal((expected, []), Model.update (Page(SurfaceMessage.Reader ReadingMessage.RetryContents)) expected)
    Assert.Equal((expected, []), Model.update (Page(SurfaceMessage.Reader ReadingMessage.RetryText)) expected)

[<Property>]
let ``Concord Next and failed-page retry preserve exactly the served continuation request`` (suffix: uint16) (code: ErrorCode) =
    let reference = $"route-{suffix}"
    let continuation = $"next-{suffix}"
    let window: TextWindow = { Units = []; Next = Some (WireFixtures.identity<UnitReference> continuation); Version = (WireFixtures.identity<ArtifactRoot> "root") }
    let model, _ = Model.init (Route.Concord(Some reference))
    let model = withReading window model
    let request = RequestId.next model.Serial
    let read = Reads.textWindow (WireFixtures.identity<TextWindowReference> continuation) (Some concordPageSize) (Some WindowDir.Onward) None (Some Corpus.Concord)
    let served, session = concordSession model
    let expected = { model with Serial = request; Surface = Surface.Concord(concordPage (Some reference) (ReadingState.Active(served, ReadSession.beginRead request read (Ready window)))); Focus = FocusState.Closed }
    let pending, effects = Model.update (Page(SurfaceMessage.Concord ConcordMessage.Next)) model
    Assert.Equal((expected, [ReadText(Corpus.Concord, request, read)]), (pending, effects))
    Assert.Equal((pending, []), Model.update (Page(SurfaceMessage.Concord ConcordMessage.Next)) pending)
    let failure = WireFixtures.readFailure code
    let failed, _ = Model.update (Page(SurfaceMessage.Concord(ConcordMessage.Reading(ReadingMessage.TextLoaded(request, Error failure))))) pending
    let retryId = RequestId.next request
    let expectedRetry = { failed with Serial = retryId; Surface = Surface.Concord(concordPage (Some reference) (ReadingState.Active(served, ReadSession.beginRead retryId read (Failed(request, failure, Some window))))) }
    let retryMessage = Page(SurfaceMessage.Concord(ConcordMessage.Reading ReadingMessage.RetryText))
    let retried, effects = Model.update retryMessage failed
    Assert.Equal((expectedRetry, [ReadText(Corpus.Concord, retryId, read)]), (retried, effects))
    Assert.Equal((retried, []), Model.update (Page(SurfaceMessage.Concord(ConcordMessage.Reading(ReadingMessage.TextLoaded(request, Ok window))))) retried)
    Assert.Equal(Ready window, ReadSession.state session)

[<Property(MaxTest = 4)>]
let ``future sized Concord turns retain exactly the final page and current request`` (extra: byte) =
    let pageCount = futureJourneyLength + int extra
    let model, _ = Model.init (Route.Concord None)
    let first = page 0
    let model = withReading first model
    let actual =
        [1..pageCount] |> List.fold (fun model index ->
            let pending, _ = Model.update (Page(SurfaceMessage.Concord ConcordMessage.Next)) model
            Model.update (Page(SurfaceMessage.Concord(ConcordMessage.Reading(ReadingMessage.TextLoaded(pending.Serial, Ok(page index)))))) pending |> fst) model
    let request = [1..pageCount] |> List.fold (fun request _ -> RequestId.next request) model.Serial
    let read = Reads.textWindow (WireFixtures.identity<TextWindowReference> $"next-{pageCount - 1}") (Some concordPageSize) (Some WindowDir.Onward) None (Some Corpus.Concord)
    let served, _ = concordSession model
    let session = ReadSession.beginRead request read Empty |> ReadSession.complete request (Ok(page pageCount))
    let expected = { model with Serial = request; Surface = Surface.Concord(concordPage None (ReadingState.Active(served, session))) }
    Assert.Equal(expected, actual)

[<Property>]
let ``every message belonging to another surface preserves every page state`` (code: ErrorCode) (steps: byte) =
    let request = requestAfter steps
    let failure = WireFixtures.readFailure code
    let fixtures = surfaceFixtures request failure
    let messages = surfaceMessages request failure
    let cases = [for surface in fixtures do for message in messages do if owner surface <> messageOwner message then yield surface, message]
    let models = cases |> List.map (fun (surface, _) -> { Surface = surface; Serial = request; Focus = FocusState.Closed })
    let expected = models |> List.map (fun model -> model, [])
    let actual = List.map2 (fun model (_, message) -> Model.update (Page message) model) models cases
    Assert.Equal<(Model * Effect list) list>(expected, actual)

[<Property>]
let ``every noncurrent surface completion preserves every page state and its focus`` (code: ErrorCode) (steps: byte) focused =
    let request = requestAfter steps
    let fixtures = surfaceFixtures request (WireFixtures.readFailure code)
    let messages =
        surfaceMessages (RequestId.next request) (WireFixtures.readFailure code)
        |> List.filter (function
            | SurfaceMessage.Reader(ReadingMessage.ContentsLoaded _ | ReadingMessage.TextLoaded _)
            | SurfaceMessage.Concord(ConcordMessage.Reading(ReadingMessage.ContentsLoaded _ | ReadingMessage.TextLoaded _))
            | SurfaceMessage.Sources(SourcesMessage.Loaded _) -> true
            | _ -> false)
    let focus = if focused then FocusState.Opened ExplorationTests.trail else FocusState.Closed
    let cases = [for surface in fixtures do for message in messages do yield { Surface = surface; Serial = request; Focus = focus }, message]
    let expected = cases |> List.map (fun (model, _) -> model, [])
    let actual = cases |> List.map (fun (model, message) -> Model.update (Page message) model)
    Assert.Equal<(Model * Effect list) list>(expected, actual)

[<Property>]
let ``a failed contents read retries only contents with a new identity`` (code: ErrorCode) =
    let model, _ = Model.init Route.Reader
    let failure = WireFixtures.readFailure code
    let failed, effects = Model.update (Page(SurfaceMessage.Reader(ReadingMessage.ContentsLoaded(model.Serial, Error failure)))) model
    let expectedFailed = { model with Surface = Surface.Reader(readerPage None (ReadingState.CouldNotLoadContents failure)) }
    Assert.Equal((expectedFailed, []), (failed, effects))
    Assert.Equal((failed, []), Model.update (Page(SurfaceMessage.Reader ReadingMessage.RetryText)) failed)
    let request = RequestId.next model.Serial
    let expectedRetry = { failed with Serial = request; Surface = Surface.Reader(readerPage None (ReadingState.LoadingContents request)) }
    Assert.Equal((expectedRetry, [ReadContents(Corpus.Bible, request)]), Model.update (Page(SurfaceMessage.Reader ReadingMessage.RetryContents)) failed)

[<Property>]
let ``a current source completion installs the whole served answer without starting another read`` (NonNull title: NonNull<string>) =
    let model, _ = Model.init Route.Sources
    let answer: SourcesDocument = { Categories = [{ Id = "served"; Label = title }]; Sources = []; Provenances = None }
    let expected = { model with Surface = Surface.Sources(Ready answer) }
    Assert.Equal((expected, []), Model.update (Page(SurfaceMessage.Sources(SourcesMessage.Loaded(model.Serial, Ok answer)))) model)

[<Property>]
let ``opening a focus has a new identity and resolves only its requested position`` (steps: byte) =
    let model, _ = seedModel Route.Sources steps
    let position = Explorable.position ExplorationTests.start
    let actual, effects = Model.update (OpenPosition position) model
    let request = RequestId.next model.Serial
    Assert.Equal({ model with Serial = request; Focus = FocusState.Opening(request, position) }, actual)
    Assert.Equal<Effect list>([ReadOpening(request, position)], effects)

[<Property>]
let ``closing focus refuses a late successful opening`` (steps: byte) =
    let model, _ = seedModel Route.Sources steps
    let opening, _ = Model.update (OpenPosition(Explorable.position ExplorationTests.start)) model
    let closed, _ = Model.update CloseFocus opening
    let actual, effects = Model.update (FocusLoaded(opening.Serial, Ok ExplorationTests.trail)) closed
    Assert.Equal({ opening with Serial = RequestId.next opening.Serial; Focus = FocusState.Closed }, actual)
    Assert.Equal<Effect list>([], effects)

[<Property>]
let ``following preserves the current trail while its replacement is in flight`` (steps: byte) =
    let model, _ = seedModel Route.Sources steps
    let model = { model with Focus = FocusState.Opened ExplorationTests.trail }
    let link: Link = { Kind = EdgeKind.Contains; Target = Explorable.position (ExplorationTests.node "Person:target" "root") }
    let actual, effects = Model.update (Traverse(Traversal.Follow link)) model
    let request = RequestId.next model.Serial
    Assert.Equal({ model with Serial = request; Focus = FocusState.Walking(request, ExplorationTests.trail, Traversal.Follow link) }, actual)
    Assert.Equal<Effect list>([WalkFocus(request, ExplorationTests.trail, Traversal.Follow link)], effects)

[<Property>]
let ``a superseded focus completion cannot replace a later traversal`` (steps: byte) =
    let model, _ = seedModel Route.Sources steps
    let model = { model with Focus = FocusState.Opened ExplorationTests.trail }
    let first, _ = Model.update (Traverse Traversal.Back) model
    let next, _ = Model.update (Traverse Traversal.Renew) first
    let actual, effects = Model.update (FocusLoaded(first.Serial, Error(WireFixtures.readFailure ErrorCode.NotFound))) next
    Assert.Equal(next, actual)
    Assert.Equal<Effect list>([], effects)

[<Property>]
let ``a failed opening retains exactly the position needed by Retry`` (steps: byte) =
    let model, _ = seedModel Route.Sources steps
    let position = Explorable.position ExplorationTests.start
    let opening, _ = Model.update (OpenPosition position) model
    let failed, _ = Model.update (FocusLoaded(opening.Serial, Error(WireFixtures.readFailure ErrorCode.NotFound))) opening
    Assert.Equal({ opening with Focus = FocusState.CouldNotOpen(position, WireFixtures.readFailure ErrorCode.NotFound) }, failed)
    let retry, effects = Model.update RetryFocus failed
    let request = RequestId.next failed.Serial
    Assert.Equal({ failed with Serial = request; Focus = FocusState.Opening(request, position) }, retry)
    Assert.Equal<Effect list>([ReadOpening(request, position)], effects)

let withReading (window: TextWindow) (model: Model) : Model =
    match model.Surface with
    | Surface.Reader reader ->
        let read = Reads.textWindow (TextWindowReference.ofContentsReference firstChapter.Ref) None None (Some TextScope.Chapter) (Some Corpus.Bible)
        let session = ReadSession.beginRead model.Serial read Empty |> ReadSession.complete model.Serial (Ok window)
        { model with Surface = Surface.Reader(readerPage (ReaderPage.location reader) (ReadingState.Active(contents, session))) }
    | Surface.Concord concord ->
        let reference = ConcordPage.reference concord |> Option.defaultValue "BoC 1.1.1"
        let read = Reads.textWindow (WireFixtures.identity<TextWindowReference> reference) (Some concordPageSize) None None (Some Corpus.Concord)
        let session = ReadSession.beginRead model.Serial read Empty |> ReadSession.complete model.Serial (Ok window)
        { model with Surface = Surface.Concord(concordPage (ConcordPage.reference concord) (ReadingState.Active(concordContents reference, session))) }
    | Surface.Sources _ | Surface.World | Surface.Kretzmann | Surface.NotFound -> failwith "the fixture requires a reading surface"

let private readingMessage corpus message =
    match corpus with
    | Corpus.Bible -> Page(SurfaceMessage.Reader message)
    | Corpus.Concord -> Page(SurfaceMessage.Concord(ConcordMessage.Reading message))

let private withState (state: ReadingState) (model: Model) : Model =
    match model.Surface with
    | Surface.Reader page -> { model with Surface = Surface.Reader(readerPage (ReaderPage.location page) state) }
    | Surface.Concord page -> { model with Surface = Surface.Concord(concordPage (ConcordPage.reference page) state) }
    | _ -> failwith "the fixture requires a reading surface"

let readerPage (location: ReadingLocation option) (state: ReadingState) : ReaderPage =
    FSharpValue.MakeRecord(typeof<ReaderPage>, [|box location; box state|], true) |> unbox<ReaderPage>

let private concordPage (reference: string option) (state: ReadingState) : ConcordPage =
    FSharpValue.MakeRecord(typeof<ConcordPage>, [|box reference; box state|], true) |> unbox<ConcordPage>

let private seedModel route steps : Model * Effect list =
    [1..int steps] |> List.fold (fun (model, _) _ ->
        let away, _ = Model.update (Navigate Route.NotFound) model
        Model.update (Navigate route) away) (Model.init route)

let private concordSession (model: Model) : Contents * ReadSession<TextWindow> =
    match model.Surface with
    | Surface.Concord page ->
        match ConcordPage.state page with
        | ReadingState.Active(contents, session) -> contents, session
        | _ -> failwith "the fixture requires an active Concord reading"
    | _ -> failwith "the fixture requires an active Concord reading"

let private concordContents reference : Contents =
    { contents with Corpus = Corpus.Concord; Roots = [{ contents.Roots.Head with Ref = (WireFixtures.identity<ContentsReference> reference); Locus = TextRef.Concord { Part = 1; Article = 1; Paragraph = 1 }; Children = [] }] }

let private page index : TextWindow =
    { Units = [{ Ref = (WireFixtures.identity<UnitReference> $"page-{index}"); Node = { Id = (WireFixtures.identity<NodeId> $"TextUnit:page-{index}"); Kind = NodeKind.TextUnit; Label = $"Page {index}" }; Heading = None; EdgeSummary = []; Body = { Locus = TextRef.Concord { Part = 1; Article = 1; Paragraph = index }; Text = $"Page {index} body"; Anchors = []; WordsOfChrist = [] } }]
      Next = Some (WireFixtures.identity<UnitReference> $"next-{index}"); Version = (WireFixtures.identity<ArtifactRoot> "root") }

let private surfaceFixtures request failure : Surface list =
    let read = Reads.textWindow (WireFixtures.identity<TextWindowReference> "served") None None None None
    let pending = ReadSession.beginRead request read Empty
    let window: TextWindow = { Units = []; Next = None; Version = (WireFixtures.identity<ArtifactRoot> "root") }
    let ready = ReadSession.complete request (Ok window) pending
    let failed = ReadSession.complete request (Error failure) pending
    let states = [ReadingState.LoadingContents request; ReadingState.CouldNotLoadContents failure; ReadingState.Unavailable failure; ReadingState.Active(contents, pending); ReadingState.Active(contents, ready); ReadingState.Active(contents, failed)]
    let source: SourcesDocument = { Categories = []; Sources = []; Provenances = None }
    [yield! states |> List.map (fun state -> Surface.Reader(readerPage None state))
     yield! states |> List.map (fun state -> Surface.Concord(concordPage None state))
     yield! [Empty; Loading(request, None); Ready source; Failed(request, failure, Some source)] |> List.map Surface.Sources
     yield Surface.World; yield Surface.Kretzmann; yield Surface.NotFound]

let private surfaceMessages request failure : SurfaceMessage list =
    let window: TextWindow = { Units = []; Next = None; Version = (WireFixtures.identity<ArtifactRoot> "root") }
    let source: SourcesDocument = { Categories = []; Sources = []; Provenances = None }
    let readings = [ReadingMessage.RetryContents; ReadingMessage.RetryText; ReadingMessage.ContentsLoaded(request, Ok contents); ReadingMessage.ContentsLoaded(request, Error failure); ReadingMessage.TextLoaded(request, Ok window); ReadingMessage.TextLoaded(request, Error failure)]
    [yield! readings |> List.map SurfaceMessage.Reader
     yield! readings |> List.map (ConcordMessage.Reading >> SurfaceMessage.Concord)
     yield SurfaceMessage.Concord ConcordMessage.Next
     yield SurfaceMessage.Sources SourcesMessage.Retry
     yield SurfaceMessage.Sources(SourcesMessage.Loaded(request, Ok source))
     yield SurfaceMessage.Sources(SourcesMessage.Loaded(request, Error failure))]

let private owner surface =
    match surface with
    | Surface.Reader _ -> ReaderOwner
    | Surface.Concord _ -> ConcordOwner
    | Surface.Sources _ -> SourcesOwner
    | Surface.World | Surface.Kretzmann | Surface.NotFound -> NoOwner

let private messageOwner message =
    match message with
    | SurfaceMessage.Reader _ -> ReaderOwner
    | SurfaceMessage.Concord _ -> ConcordOwner
    | SurfaceMessage.Sources _ -> SourcesOwner

let private requestAfter steps = [1 .. int steps] |> List.fold (fun identity _ -> RequestId.next identity) RequestId.initial

let firstChapter: ContentsChild = { Id = (WireFixtures.identity<NodeId> "Container:bible-chapter-GEN-1"); Title = "1"; Kind = ContentsChildKind.Chapter; Ref = (WireFixtures.identity<ContentsReference> "GEN.1"); Locus = TextRef.Bible { Book = BookId.GEN; Chapter = 1; Verse = 1 }; Count = 31 }
let contents: Contents = { Corpus = Corpus.Bible; Version = (WireFixtures.identity<ArtifactRoot> "root"); Roots = [{ Id = (WireFixtures.identity<NodeId> "Container:bible-book-GEN"); Title = "Genesis"; Kind = ContentsRootKind.Book; Group = Some Testament.OT; Ref = (WireFixtures.identity<ContentsReference> "GEN.1.1"); Locus = firstChapter.Locus; Children = [firstChapter] }] }
let private concordPageSize = 20
let private futureJourneyLength = 10_000
type private SurfaceOwner = ReaderOwner | ConcordOwner | SourcesOwner | NoOwner
