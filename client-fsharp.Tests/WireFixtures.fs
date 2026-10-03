namespace BibleAtlas.FSharp.Tests

open System.Text.Json
open System.Net
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Admission

module WireFixtures =
    let identity<'a> (value: obj) : 'a = JsonSerializer.Deserialize<'a>(JsonSerializer.Serialize value)

    let readFailure (code: ErrorCode) : BibleAtlas.FSharp.Failure =
        match RefusalStatuses.server (int HttpStatusCode.InternalServerError) with
        | Ok status -> BibleAtlas.FSharp.Failure.Read(ReadFailure.Transient(TransientFailure.ServerRefusal(status, Some code)))
        | Error failure -> failwithf "%A" failure
