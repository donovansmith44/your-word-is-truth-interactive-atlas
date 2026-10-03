module rec BibleAtlas.FSharp.Tests.Domain.StatusLaws

open FsCheck.Xunit
open Xunit
open BibleAtlas.FSharp.Admission

[<Property>]
let ``client refusal admission preserves exactly the whole 400 through 499 range`` (status: int) =
    let expected = if status >= clientMinimum && status <= clientMaximum then Ok status else Error ClientStatusFailure.NotClientRefusal
    Assert.Equal(expected, RefusalStatuses.client status |> Result.map RefusalStatuses.clientValue)

[<Property>]
let ``server refusal admission preserves exactly the whole 500 through 599 range`` (status: int) =
    let expected = if status >= serverMinimum && status <= serverMaximum then Ok status else Error ServerStatusFailure.NotServerRefusal
    Assert.Equal(expected, RefusalStatuses.server status |> Result.map RefusalStatuses.serverValue)

[<Property>]
let ``both refusal doors preserve generated codes and their complete class boundaries`` (offset: byte) =
    let codes = [clientMinimum, serverMinimum; clientMaximum, serverMaximum; clientMinimum + int offset % refusalClassSize, serverMinimum + int offset % refusalClassSize]
    let expected = codes |> List.map (fun (client, server) -> Ok client, Ok server)
    let actual = codes |> List.map (fun (client, server) -> RefusalStatuses.client client |> Result.map RefusalStatuses.clientValue, RefusalStatuses.server server |> Result.map RefusalStatuses.serverValue)
    Assert.Equal<(Result<int, ClientStatusFailure> * Result<int, ServerStatusFailure>) list>(expected, actual)

[<Property>]
let ``invalid HTTP status admission recognizes exactly the complement of 100 through 599`` (status: int) =
    let expected = [status, status < informationalMinimum || status > serverMaximum; informationalMinimum, false; serverMaximum, false; informationalMinimum - 1, true; serverMaximum + 1, true]
    Assert.Equal<(int * bool) list>(expected, expected |> List.map (fun (status, _) -> status, RefusalStatuses.invalid status))

let private informationalMinimum = 100
let private clientMinimum = 400
let private clientMaximum = 499
let private serverMinimum = 500
let private serverMaximum = 599
let private refusalClassSize = 100
