namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract

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

type ReadSession<'a> = private { Request: Request<'a>; State: LoadState<'a> }

module ReadSession =
    let beginRead identity request state = { Request = request; State = LoadState.beginRead identity state }
    let state session = session.State
    let request session = session.Request
    let retry identity session = beginRead identity session.Request session.State
    let complete identity answer session = { session with State = LoadState.complete identity answer session.State }
