namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract

[<RequireQualifiedAccess>]
type SourcesState =
    | Loading of RequestId
    | Available of SourcesPresentation
    | RetryableFailure of ReadFailure
    | ReadRejected of ReadFailure
    | PresentationRejected of SourcePresentationFailure

type SourcesModel = private SourcesModel of SourcesState

[<RequireQualifiedAccess>]
type SourcesMessage = Retry | Loaded of RequestId * Result<SourcesDocument, ReadFailure>

[<RequireQualifiedAccess>]
type SourcesEffect = Read of RequestId

module Sources =
    let update next message (SourcesModel state as model) : SourcesModel * SourcesEffect list =
        match message, state with
        | SourcesMessage.Retry, SourcesState.RetryableFailure _ -> SourcesModel(SourcesState.Loading next), [SourcesEffect.Read next]
        | SourcesMessage.Retry, (SourcesState.Loading _ | SourcesState.Available _ | SourcesState.ReadRejected _ | SourcesState.PresentationRejected _) -> model, []
        | SourcesMessage.Loaded(ticket, answer), SourcesState.Loading pending when ticket = pending ->
            let state =
                match answer with
                | Ok document ->
                    match SourcesPresenter.present document with
                    | Ok presentation -> SourcesState.Available presentation
                    | Error failure -> SourcesState.PresentationRejected failure
                | Error failure ->
                    if ReadFailure.canRetry failure then SourcesState.RetryableFailure failure
                    else SourcesState.ReadRejected failure
            SourcesModel state, []
        | SourcesMessage.Loaded _, (SourcesState.Loading _ | SourcesState.Available _ | SourcesState.RetryableFailure _ | SourcesState.ReadRejected _ | SourcesState.PresentationRejected _) -> model, []
    let init request = SourcesModel(SourcesState.Loading request), [SourcesEffect.Read request]
    let state (SourcesModel state) = state
