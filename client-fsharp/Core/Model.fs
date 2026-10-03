namespace BibleAtlas.FSharp

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
type ReadingState = Idle | Unavailable of RequestId * Failure | Active of ReadSession<TextWindow>

type Model =
    { Route: Route
      Serial: RequestId
      Contents: Map<Corpus, LoadState<Contents>>
      ReadingSession: ReadingState
      Sources: LoadState<SourcesDocument>
      Focus: FocusState }
    member this.Reading =
        match this.ReadingSession with
        | ReadingState.Idle -> Empty
        | ReadingState.Unavailable(identity, failure) -> Failed(identity, failure, None)
        | ReadingState.Active session -> ReadSession.state session

type Message =
    | Navigate of Route
    | Retry
    | ReadNext
    | ContentsLoaded of Corpus * RequestId * Result<Contents, Failure>
    | TextLoaded of RequestId * Result<TextWindow, Failure>
    | SourcesLoaded of RequestId * Result<SourcesDocument, Failure>
    | OpenPosition of PositionRef
    | Traverse of Traversal
    | CloseFocus
    | RetryFocus
    | FocusLoaded of RequestId * Result<Trail, Failure>

type Effect =
    | ReadContents of Corpus * RequestId
    | ReadText of RequestId * Request<TextWindow>
    | ReadSources of RequestId
    | ReadOpening of RequestId * PositionRef
    | WalkFocus of RequestId * Trail * Traversal

module Model =
    let private concordPageSize = 20

    let rec init route =
        loadView { Route = route; Serial = RequestId.initial; Contents = Map.empty; ReadingSession = ReadingState.Idle; Sources = Empty; Focus = FocusState.Closed }

    and update message model =
        match message with
        | Navigate route -> loadView { model with Route = route; Serial = RequestId.next model.Serial; ReadingSession = ReadingState.Idle; Focus = FocusState.Closed }
        | ReadNext ->
            match model.Route, model.Reading with
            | Route.Concord _, Ready window ->
                match window.Next with
                | Some reference ->
                    beginReading (Reads.textWindow reference (Some concordPageSize) (Some WindowDir.Onward) None (Some Corpus.Concord)) { model with Serial = RequestId.next model.Serial; Focus = FocusState.Closed }
                | None -> model, []
            | Route.Reader, _ | Route.Read _, _ | Route.World, _ | Route.Kretzmann, _ | Route.Sources, _ | Route.NotFound, _
            | Route.Concord _, Empty | Route.Concord _, Loading _ | Route.Concord _, Failed _ -> model, []
        | Retry ->
            let reset state = match state with Failed _ -> Empty | Empty | Loading _ | Ready _ -> state
            let model = { model with Serial = RequestId.next model.Serial; Contents = Map.map (fun _ state -> reset state) model.Contents; Sources = reset model.Sources }
            match model.ReadingSession with
            | ReadingState.Active session ->
                match ReadSession.state session with
                | Failed _ ->
                    let session = ReadSession.retry model.Serial session
                    { model with ReadingSession = ReadingState.Active session }, [ReadText(model.Serial, ReadSession.request session)]
                | Empty | Loading _ | Ready _ -> loadView model
            | ReadingState.Unavailable _ -> loadView { model with ReadingSession = ReadingState.Idle }
            | ReadingState.Idle -> loadView model
        | ContentsLoaded(corpus, request, answer) ->
            let previous = Map.tryFind corpus model.Contents |> Option.defaultValue Empty
            let complete = LoadState.complete request answer previous
            if previous = complete then model, []
            else loadView { model with Contents = Map.add corpus complete model.Contents }
        | TextLoaded(request, answer) ->
            match model.ReadingSession with
            | ReadingState.Active session -> { model with ReadingSession = ReadingState.Active(ReadSession.complete request answer session) }, []
            | ReadingState.Idle | ReadingState.Unavailable _ -> model, []
        | SourcesLoaded(request, answer) -> { model with Sources = LoadState.complete request answer model.Sources }, []
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

    and private loadView model =
        match model.Route with
        | Route.Sources ->
            match model.Sources with
            | Empty -> { model with Sources = Loading(model.Serial, None) }, [ReadSources model.Serial]
            | Loading _ | Ready _ | Failed _ -> model, []
        | Route.Reader | Route.Read _ -> reading Corpus.Bible model
        | Route.Concord _ -> reading Corpus.Concord model
        | Route.World | Route.Kretzmann | Route.NotFound -> model, []

    and private reading corpus model =
        let contents = Map.tryFind corpus model.Contents |> Option.defaultValue Empty
        match contents, model.Reading with
        | Empty, _ -> { model with Contents = Map.add corpus (Loading(model.Serial, None)) model.Contents }, [ReadContents(corpus, model.Serial)]
        | Ready contents, Empty ->
            match opening model.Route contents with
            | Some(reference, scope) ->
                let size = match corpus with Corpus.Bible -> None | Corpus.Concord -> Some concordPageSize
                beginReading (Reads.textWindow reference size None scope (Some corpus)) model
            | None -> { model with ReadingSession = ReadingState.Unavailable(model.Serial, Contract "the requested reading has no opening in the served contents") }, []
        | Loading _, _ | Failed _, _ | Ready _, Loading _ | Ready _, Ready _ | Ready _, Failed _ -> model, []

    and private beginReading request model =
        let session = ReadSession.beginRead model.Serial request model.Reading
        { model with ReadingSession = ReadingState.Active session }, [ReadText(model.Serial, ReadSession.request session)]

    and private opening route (contents: Contents) =
        match route with
        | Route.Reader -> contents.Roots |> List.tryHead |> Option.bind (fun root -> root.Children |> List.tryHead) |> Option.map (fun child -> child.Ref, Some TextScope.Chapter)
        | Route.Read location ->
            contents.Roots |> List.collect _.Children |> List.tryFind (fun child ->
                match child.Locus with
                | TextRef.Bible locus -> locus.Book = location.Book && locus.Chapter = location.Chapter
                | TextRef.Concord _ -> false) |> Option.map (fun child -> child.Ref, Some TextScope.Chapter)
        | Route.Concord(Some reference) -> Some(reference, None)
        | Route.Concord None -> contents.Roots |> List.tryHead |> Option.map (fun root -> root.Ref, None)
        | Route.World | Route.Kretzmann | Route.Sources | Route.NotFound -> None
