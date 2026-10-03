namespace rec BibleAtlas.FSharp

open System
open System.Text.Json
open BibleAtlas.FSharp.Contract

type ReadingLocation = { Book: BookId; Chapter: int }

[<RequireQualifiedAccess>]
type Route =
    | Reader
    | Read of ReadingLocation
    | World
    | Kretzmann
    | Concord of string option
    | Sources
    | NotFound

module Routes =
    let parse (uri: Uri) =
        match uri.AbsolutePath.Trim('/').Split('/') |> Array.toList with
        | [""] -> Route.Reader
        | ["read"; book; chapter] ->
            match Json.decode<BookId>(Json.encode book), Int32.TryParse chapter with
            | Ok book, (true, chapter) when chapter > 0 -> Route.Read { Book = book; Chapter = chapter }
            | _ -> Route.NotFound
        | ["world"] -> Route.World
        | ["kretzmann"] -> Route.Kretzmann
        | ["concord"] ->
            let reference = uri.Query.TrimStart('?').Split('&') |> Array.tryPick (fun parameter ->
                match parameter.Split('=', 2) with
                | [|"ref"; value|] -> Some(Uri.UnescapeDataString(value.Replace('+', ' ')))
                | _ -> None)
            Route.Concord reference
        | ["sources"] -> Route.Sources
        | _ -> Route.NotFound

    let url route =
        match route with
        | Route.Reader -> "/"
        | Route.Read location ->
            let book = JsonSerializer.Deserialize<string>(Json.encode location.Book)
            $"/read/{book}/{location.Chapter}"
        | Route.World -> "/world"
        | Route.Kretzmann -> "/kretzmann"
        | Route.Concord None -> "/concord"
        | Route.Concord(Some reference) -> "/concord?ref=" + Uri.EscapeDataString reference
        | Route.Sources -> "/sources"
        | Route.NotFound -> "/not-found"

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

[<RequireQualifiedAccess>]
type ReadingState =
    | LoadingContents of RequestId
    | CouldNotLoadContents of Failure
    | Unavailable of Failure
    | Active of Contents * ReadSession<TextWindow>

type ReaderPage = private { Location: ReadingLocation option; State: ReadingState }
type ConcordPage = private { Reference: string option; State: ReadingState }

module ReaderPage =
    let state (page: ReaderPage) = page.State
    let location (page: ReaderPage) = page.Location

module ConcordPage =
    let state (page: ConcordPage) = page.State
    let reference (page: ConcordPage) = page.Reference

[<RequireQualifiedAccess>]
type Surface =
    | Reader of ReaderPage
    | Concord of ConcordPage
    | Sources of LoadState<SourcesDocument>
    | World
    | Kretzmann
    | NotFound

type Model = { Surface: Surface; Serial: RequestId; Focus: FocusState }

[<RequireQualifiedAccess>]
type ReadingMessage =
    | RetryContents
    | RetryText
    | ContentsLoaded of RequestId * Result<Contents, Failure>
    | TextLoaded of RequestId * Result<TextWindow, Failure>

[<RequireQualifiedAccess>]
type ConcordMessage = Reading of ReadingMessage | Next

[<RequireQualifiedAccess>]
type SourcesMessage = Retry | Loaded of RequestId * Result<SourcesDocument, Failure>

[<RequireQualifiedAccess>]
type SurfaceMessage =
    | Reader of ReadingMessage
    | Concord of ConcordMessage
    | Sources of SourcesMessage

type Message =
    | Navigate of Route
    | Page of SurfaceMessage
    | OpenPosition of PositionRef
    | Traverse of Traversal
    | CloseFocus
    | RetryFocus
    | FocusLoaded of RequestId * Result<Trail, Failure>

type Effect =
    | ReadContents of Corpus * RequestId
    | ReadText of Corpus * RequestId * Request<TextWindow>
    | ReadSources of RequestId
    | ReadOpening of RequestId * PositionRef
    | WalkFocus of RequestId * Trail * Traversal

module Model =
    let init (route: Route) : Model * Effect list =
        initialize route RequestId.initial

    let update (message: Message) (model: Model) : Model * Effect list =
        match message with
        | Navigate target when target = route model -> model, []
        | Navigate target -> initialize target (RequestId.next model.Serial)
        | Page message ->
            let request = RequestId.next model.Serial
            let surface, effects = Surfaces.update request message model.Surface
            let focus =
                match message, effects with
                | SurfaceMessage.Concord ConcordMessage.Next, _ :: _ -> FocusState.Closed
                | _ -> model.Focus
            { model with Surface = surface; Serial = (if effects.IsEmpty then model.Serial else request); Focus = focus }, effects
        | OpenPosition position ->
            let request = RequestId.next model.Serial
            { model with Serial = request; Focus = FocusState.Opening(request, position) }, [ReadOpening(request, position)]
        | Traverse traversal ->
            match model.Focus with
            | FocusState.Opened trail | FocusState.Walking(_, trail, _) | FocusState.CouldNotWalk(trail, _, _) ->
                let request = RequestId.next model.Serial
                { model with Serial = request; Focus = FocusState.Walking(request, trail, traversal) }, [WalkFocus(request, trail, traversal)]
            | FocusState.Closed | FocusState.Opening _ | FocusState.CouldNotOpen _ -> model, []
        | CloseFocus -> { model with Serial = RequestId.next model.Serial; Focus = FocusState.Closed }, []
        | RetryFocus ->
            match model.Focus with
            | FocusState.CouldNotOpen(position, _) -> update (OpenPosition position) model
            | FocusState.CouldNotWalk(_, traversal, _) -> update (Traverse traversal) model
            | FocusState.Closed | FocusState.Opening _ | FocusState.Opened _ | FocusState.Walking _ -> model, []
        | FocusLoaded(request, answer) ->
            match model.Focus with
            | FocusState.Opening(pending, position) when pending = request ->
                { model with Focus = match answer with Ok trail -> FocusState.Opened trail | Error failure -> FocusState.CouldNotOpen(position, failure) }, []
            | FocusState.Walking(pending, trail, traversal) when pending = request ->
                { model with Focus = match answer with Ok trail -> FocusState.Opened trail | Error failure -> FocusState.CouldNotWalk(trail, traversal, failure) }, []
            | FocusState.Closed | FocusState.Opening _ | FocusState.Opened _ | FocusState.Walking _ | FocusState.CouldNotOpen _ | FocusState.CouldNotWalk _ -> model, []


    let route (model: Model) : Route =
        match model.Surface with
        | Surface.Reader page -> ReaderPage.location page |> Option.map Route.Read |> Option.defaultValue Route.Reader
        | Surface.Concord page -> Route.Concord(ConcordPage.reference page)
        | Surface.Sources _ -> Route.Sources
        | Surface.World -> Route.World
        | Surface.Kretzmann -> Route.Kretzmann
        | Surface.NotFound -> Route.NotFound

    let private initialize (route: Route) (request: RequestId) : Model * Effect list =
        let surface, effects = Surfaces.initialize request route
        { Surface = surface; Serial = request; Focus = FocusState.Closed }, effects

module private Surfaces =
    let initialize (request: RequestId) (route: Route) : Surface * Effect list =
        match route with
        | Route.Reader ->
            Surface.Reader { Location = None; State = ReadingState.LoadingContents request }, [ReadContents(Corpus.Bible, request)]
        | Route.Read location ->
            Surface.Reader { Location = Some location; State = ReadingState.LoadingContents request }, [ReadContents(Corpus.Bible, request)]
        | Route.Concord reference ->
            Surface.Concord { Reference = reference; State = ReadingState.LoadingContents request }, [ReadContents(Corpus.Concord, request)]
        | Route.Sources -> Surface.Sources(Loading(request, None)), [ReadSources request]
        | Route.World -> Surface.World, []
        | Route.Kretzmann -> Surface.Kretzmann, []
        | Route.NotFound -> Surface.NotFound, []

    let update (request: RequestId) (message: SurfaceMessage) (surface: Surface) : Surface * Effect list =
        match message, surface with
        | SurfaceMessage.Reader message, Surface.Reader page ->
            let state, effects = ReadingSurface.update (ReadingChoice.Bible page.Location) request message page.State
            Surface.Reader { page with State = state }, effects
        | SurfaceMessage.Concord message, Surface.Concord page ->
            let state, effects = ConcordSurface.update page.Reference request message page.State
            Surface.Concord { page with State = state }, effects
        | SurfaceMessage.Sources message, Surface.Sources state ->
            let state, effects = SourcesSurface.update request message state
            Surface.Sources state, effects
        | _ -> surface, []

module private SourcesSurface =
    let update (request: RequestId) (message: SourcesMessage) (state: LoadState<SourcesDocument>) : LoadState<SourcesDocument> * Effect list =
        match message, state with
        | SourcesMessage.Retry, Failed _ -> LoadState.beginRead request state, [ReadSources request]
        | SourcesMessage.Loaded(identity, answer), _ -> LoadState.complete identity answer state, []
        | SourcesMessage.Retry, Empty | SourcesMessage.Retry, Loading _ | SourcesMessage.Retry, Ready _ -> state, []

module private ConcordSurface =
    let update (reference: string option) (request: RequestId) (message: ConcordMessage) (state: ReadingState) : ReadingState * Effect list =
        match message, state with
        | ConcordMessage.Reading message, _ -> ReadingSurface.update (ReadingChoice.Concord reference) request message state
        | ConcordMessage.Next, ReadingState.Active(contents, session) ->
            match ReadSession.state session with
            | Ready window ->
                match window.Next with
                | Some reference -> ReadingSurface.beginRead Corpus.Concord request contents session (Reads.textWindow (TextWindowReference.ofUnitReference reference) (Some ReadingAffordances.concordPageSize) (Some WindowDir.Onward) None (Some Corpus.Concord))
                | None -> state, []
            | Empty | Loading _ | Failed _ -> state, []
        | ConcordMessage.Next, ReadingState.LoadingContents _ | ConcordMessage.Next, ReadingState.CouldNotLoadContents _ | ConcordMessage.Next, ReadingState.Unavailable _ -> state, []

module private ReadingSurface =
    let update (choice: ReadingChoice) (request: RequestId) (message: ReadingMessage) (state: ReadingState) : ReadingState * Effect list =
        let corpus = corpus choice
        match message, state with
        | ReadingMessage.RetryContents, ReadingState.CouldNotLoadContents _ ->
            ReadingState.LoadingContents request, [ReadContents(corpus, request)]
        | ReadingMessage.ContentsLoaded(identity, answer), ReadingState.LoadingContents pending when identity = pending ->
            match answer with
            | Error failure -> ReadingState.CouldNotLoadContents failure, []
            | Ok contents when contents.Corpus <> corpus -> ReadingState.Unavailable(Contract "the contents answer names a different corpus"), []
            | Ok contents ->
                match opening choice contents with
                | Error failure -> ReadingState.Unavailable failure, []
                | Ok None -> ReadingState.Unavailable(Contract "the requested reading has no opening in the served contents"), []
                | Ok(Some(reference, scope)) ->
                    let size = match corpus with Corpus.Bible -> None | Corpus.Concord -> Some ReadingAffordances.concordPageSize
                    let read = Reads.textWindow reference size None scope (Some corpus)
                    ReadingState.Active(contents, ReadSession.beginRead request read Empty), [ReadText(corpus, request, read)]
        | ReadingMessage.TextLoaded(identity, answer), ReadingState.Active(contents, session) ->
            let answer = answer |> Result.bind (validate corpus)
            ReadingState.Active(contents, ReadSession.complete identity answer session), []
        | ReadingMessage.RetryText, ReadingState.Active(contents, session) ->
            match ReadSession.state session with
            | Failed _ ->
                let retried = ReadSession.retry request session
                ReadingState.Active(contents, retried), [ReadText(corpus, request, ReadSession.request retried)]
            | Empty | Loading _ | Ready _ -> state, []
        | _ -> state, []

    let beginRead (corpus: Corpus) (request: RequestId) (contents: Contents) (session: ReadSession<TextWindow>) (read: Request<TextWindow>) : ReadingState * Effect list =
        ReadingState.Active(contents, ReadSession.beginRead request read (ReadSession.state session)), [ReadText(corpus, request, read)]

    let private opening (choice: ReadingChoice) (contents: Contents) : Result<(TextWindowReference * TextScope option) option, Failure> =
        match choice with
        | ReadingChoice.Bible None -> contents.Roots |> List.tryHead |> Option.bind (fun root -> root.Children |> List.tryHead) |> Option.map (fun child -> TextWindowReference.ofContentsReference child.Ref, Some TextScope.Chapter) |> Ok
        | ReadingChoice.Bible(Some location) ->
            contents.Roots |> List.collect _.Children |> List.tryFind (fun child ->
                match child.Locus with
                | TextRef.Bible locus -> locus.Book = location.Book && locus.Chapter = location.Chapter
                | TextRef.Concord _ -> false) |> Option.map (fun child -> TextWindowReference.ofContentsReference child.Ref, Some TextScope.Chapter) |> Ok
        | ReadingChoice.Concord(Some reference) -> Json.decode<ConcordReference>(Json.encode reference) |> Result.map (fun reference -> Some(TextWindowReference.ofConcordReference reference, None))
        | ReadingChoice.Concord None -> contents.Roots |> List.tryHead |> Option.map (fun root -> TextWindowReference.ofContentsReference root.Ref, None) |> Ok

    let private validate (corpus: Corpus) (window: TextWindow) : Result<TextWindow, Failure> =
        let matches =
            window.Units |> List.forall (fun unit ->
                match corpus, unit.Body.Locus with
                | Corpus.Bible, TextRef.Bible _ | Corpus.Concord, TextRef.Concord _ -> true
                | Corpus.Bible, TextRef.Concord _ | Corpus.Concord, TextRef.Bible _ -> false)
        if matches then Ok window else Error(Contract "the text answer names a different corpus")

    let private corpus choice =
        match choice with
        | ReadingChoice.Bible _ -> Corpus.Bible
        | ReadingChoice.Concord _ -> Corpus.Concord

[<RequireQualifiedAccess>]
type private ReadingChoice = Bible of ReadingLocation option | Concord of string option

module private ReadingAffordances =
    let concordPageSize = 20
