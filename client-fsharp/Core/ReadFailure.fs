namespace BibleAtlas.FSharp

open System.Net
open BibleAtlas.FSharp.Contract

type HttpRejection = { Status: HttpStatusCode; Reason: string option; Body: Result<ErrorBody, WireFailure> }

[<RequireQualifiedAccess>]
type ReadFailure =
    | Unreachable of diagnostic: string
    | Cancelled of diagnostic: string
    | HttpRejected of HttpRejection
    | InvalidAnswer of WireFailure

module ReadFailure =
    let canRetry failure =
        match failure with
        | ReadFailure.Unreachable _ | ReadFailure.Cancelled _ -> true
        | ReadFailure.HttpRejected refusal ->
            let serverErrorsStart = int HttpStatusCode.InternalServerError
            let httpStatusClassWidth = 100
            int refusal.Status >= serverErrorsStart && int refusal.Status < serverErrorsStart + httpStatusClassWidth
        | ReadFailure.InvalidAnswer _ -> false

    let describe failure =
        match failure with
        | ReadFailure.Unreachable diagnostic | ReadFailure.Cancelled diagnostic -> diagnostic
        | ReadFailure.InvalidAnswer failure -> WireFailure.render failure
        | ReadFailure.HttpRejected refusal ->
            let message =
                match refusal.Body with
                | Ok body -> body.Error.Message
                | Error _ -> refusal.Reason |> Option.defaultValue "The server refused this request."
            $"{int refusal.Status}: {message}"
