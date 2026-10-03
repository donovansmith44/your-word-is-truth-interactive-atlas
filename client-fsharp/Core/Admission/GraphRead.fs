namespace BibleAtlas.FSharp

open System.Net.Http
open System.Threading
open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Admission

module rec GraphRead =
    let explorer (http: HttpClient) : Explorer =
        { Resolve = fun positions ->
            if List.isEmpty positions then async { return Ok [] }
            else read http positions { Cursor = None; Root = None; Remaining = positions; Reverse = [] } }

    let private read (http: HttpClient) (positions: PositionRef list) (reading: Reading) : Async<Result<Explorable list, Failure>> = async {
        let! answer = Api.read http CancellationToken.None (Reads.elements (List.map Positions.id positions) reading.Cursor)
        match answer with
        | Error failure -> return Error failure
        | Ok page ->
            match reading.Root with
            | Some root when root <> page.Version -> return Error(ArtifactMoved(root, page.Version))
            | _ ->
                match append (uint64 positions.Length) page.Version page.Elements reading.Remaining reading.Reverse with
                | Error failure -> return Error failure
                | Ok(remaining, gathered) ->
                    match page.Next with
                    | None when List.isEmpty remaining -> return Ok(List.rev gathered)
                    | None -> return Error(Failures.graph(GraphFailure.ElementCountMismatch { Requested = uint64 positions.Length; Received = uint64 gathered.Length }))
                    | Some cursor when List.isEmpty page.Elements -> return Error(Failures.graph(GraphFailure.EmptyContinuation cursor))
                    | Some cursor when List.isEmpty remaining -> return Error(Failures.graph(GraphFailure.ExcessContinuation cursor))
                    | Some cursor -> return! read http positions { Cursor = Some cursor; Root = Some page.Version; Remaining = remaining; Reverse = gathered }
    }

    let private append (requested: uint64) (root: ArtifactRoot) (elements: Element list) (remaining: PositionRef list) (gathered: Explorable list) : Result<PositionRef list * Explorable list, Failure> =
        match elements, remaining with
        | [], _ -> Ok(remaining, gathered)
        | _ :: _, [] -> Error(Failures.graph(GraphFailure.ElementCountMismatch { Requested = requested; Received = uint64 gathered.Length + uint64 elements.Length }))
        | element :: rest, position :: remaining ->
            Explorable.ofElement root element |> Result.bind (fun resolved ->
                if Positions.sameIdentity position (Explorable.position resolved) then append requested root rest remaining (resolved :: gathered)
                else
                    Error(Failures.graph(GraphFailure.UnexpectedElement { Requested = position; Received = Explorable.position resolved })))

    type private Reading =
        { Cursor: ElementPageCursor option
          Root: ArtifactRoot option
          Remaining: PositionRef list
          Reverse: Explorable list }
