namespace BibleAtlas.FSharp

type Failure =
    | Transport of string
    | Contract of string
    | ArtifactMoved of resolved: string * serving: string

type RequestId = private RequestId of int64

module RequestId =
    let initial = RequestId 0L
    let next (RequestId current) = RequestId(current + 1L)

type LoadState<'a> =
    | Empty
    | Loading of RequestId * previous: 'a option
    | Ready of 'a
    | Failed of RequestId * Failure * previous: 'a option

module LoadState =
    let beginRead request state =
        let previous =
            match state with
            | Ready value -> Some value
            | Loading(_, previous) | Failed(_, _, previous) -> previous
            | Empty -> None
        Loading(request, previous)

    let complete request answer state =
        match state with
        | Loading(pending, previous) when pending = request ->
            match answer with
            | Ok value -> Ready value
            | Error failure -> Failed(request, failure, previous)
        | Empty | Ready _ | Failed _ | Loading _ -> state
