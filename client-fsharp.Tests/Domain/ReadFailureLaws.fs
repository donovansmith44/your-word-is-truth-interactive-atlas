module rec BibleAtlas.FSharp.Tests.Domain.ReadFailureLaws

open System.Net
open FsCheck.Xunit
open Xunit
open BibleAtlas.FSharp.Admission
open BibleAtlas.FSharp.Contract

[<Property>]
let ``transient refusals alone are retryable while every terminal category and cancellation stays distinct`` (offset: byte) (code: ErrorCode) =
    let client, server = clientStatus offset, serverStatus offset
    let failures =
        [ ReadFailure.Transient TransientFailure.Unreachable
          ReadFailure.Transient(TransientFailure.ServerRefusal(server, Some code))
          ReadFailure.Transient(TransientFailure.InvalidStatus(enum<HttpStatusCode> invalidStatus, Some code))
          ReadFailure.Terminal(TerminalFailure.ClientRefusal(client, Some code))
          ReadFailure.Terminal(TerminalFailure.InvalidAnswer WireFailure.NullAnswer)
          ReadFailure.Terminal(TerminalFailure.UnexpectedStatus(HttpStatusCode.Found, Some code))
          ReadFailure.Terminal(TerminalFailure.InvalidGraph(GraphFailure.MissingOpening Corpus.Bible))
          ReadFailure.Cancelled ]
    Assert.Equal<bool list>([true; true; true; false; false; false; false; false], failures |> List.map ReadFailures.retryable)

let private clientStatus offset =
    match RefusalStatuses.client (clientMinimum + int offset % refusalClassSize) with
    | Ok status -> status
    | Error failure -> failwithf "%A" failure

let private serverStatus offset =
    match RefusalStatuses.server (serverMinimum + int offset % refusalClassSize) with
    | Ok status -> status
    | Error failure -> failwithf "%A" failure

let private clientMinimum = 400
let private serverMinimum = 500
let private refusalClassSize = 100
let private invalidStatus = 600
