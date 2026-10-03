namespace BibleAtlas.FSharp

open System.Net.Http
open System.Threading
open BibleAtlas.FSharp.Contract

module Graph =
    type private Reading =
        { Cursor: int option
          Root: string option
          Remaining: PositionRef list
          Reverse: Resolved list }

    let rec explorer (http: HttpClient) =
        { Resolve = fun positions ->
            if List.isEmpty positions then async { return Ok [] }
            else read http positions { Cursor = None; Root = None; Remaining = positions; Reverse = [] } }

    and private read http positions reading = async {
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

    and private append root elements remaining gathered =
        match elements, remaining with
        | [], _ -> Ok(remaining, gathered)
        | _ :: _, [] -> Error(Contract "the element read returned more elements than requested positions")
        | element :: rest, position :: remaining ->
            Resolved.ofElement root element |> Result.bind (fun resolved ->
                if Positions.sameIdentity position (Resolved.position resolved) then append root rest remaining (resolved :: gathered)
                else
                    let actual = Positions.id (Resolved.position resolved)
                    let wanted = Positions.id position
                    Error(Contract $"the element read returned {actual} for {wanted}"))
