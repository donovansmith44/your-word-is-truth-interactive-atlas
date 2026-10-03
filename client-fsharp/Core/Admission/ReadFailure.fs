namespace BibleAtlas.FSharp.Admission

open BibleAtlas.FSharp.Contract
open System.Net

[<RequireQualifiedAccess>]
type TransientFailure =
    | Unreachable
    | ServerRefusal of ServerStatus * ErrorCode option
    | InvalidStatus of received: HttpStatusCode * code: ErrorCode option

[<RequireQualifiedAccess>]
type TerminalFailure =
    | ClientRefusal of ClientStatus * ErrorCode option
    | InvalidAnswer of WireFailure
    | UnexpectedStatus of received: HttpStatusCode * code: ErrorCode option

[<RequireQualifiedAccess>]
type ReadFailure = Transient of TransientFailure | Terminal of TerminalFailure | Cancelled

module ReadFailures =
    let retryable (failure: ReadFailure) : bool =
        match failure with
        | ReadFailure.Transient _ -> true
        | ReadFailure.Terminal _ | ReadFailure.Cancelled -> false
