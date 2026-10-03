namespace BibleAtlas.FSharp

open System.Net.Http
open System.Threading
open BibleAtlas.FSharp.Contract

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
                match append page.Version page.Elements reading.Remaining reading.Reverse with
                | Error failure -> return Error failure
                | Ok(remaining, gathered) ->
                    match page.Next with
                    | None when List.isEmpty remaining -> return Ok(List.rev gathered)
                    | None -> return Error(Contract $"the element read returned {gathered.Length} elements for {positions.Length} positions")
                    | Some _ when List.isEmpty page.Elements -> return Error(Contract "the element read returned a nonterminal page with no positions")
                    | Some _ when List.isEmpty remaining -> return Error(Contract "the element read returned a next page after all requested positions")
                    | Some cursor -> return! read http positions { Cursor = Some cursor; Root = Some page.Version; Remaining = remaining; Reverse = gathered }
    }

    let private append (root: ArtifactRoot) (elements: Element list) (remaining: PositionRef list) (gathered: Explorable list) : Result<PositionRef list * Explorable list, Failure> =
        match elements, remaining with
        | [], _ -> Ok(remaining, gathered)
        | _ :: _, [] -> Error(Contract "the element read returned more elements than requested positions")
        | element :: rest, position :: remaining ->
            Explorable.ofElement root element |> Result.bind (fun resolved ->
                if Positions.sameIdentity position (Explorable.position resolved) then append root rest remaining (resolved :: gathered)
                else
                    let actual = Positions.id (Explorable.position resolved)
                    let wanted = Positions.id position
                    Error(Contract $"the element read returned {actual} for {wanted}"))

    type private Reading =
        { Cursor: ElementPageCursor option
          Root: ArtifactRoot option
          Remaining: PositionRef list
          Reverse: Explorable list }
