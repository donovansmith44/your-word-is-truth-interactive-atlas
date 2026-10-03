module BibleAtlas.FSharp.Tests.StyleGeneration.HttpLaws

open System
open System.Net
open System.Net.Http
open System.Threading
open FsCheck.Xunit
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Property>]
let ``the Sources read delivers every field through the typed contract boundary`` (categories: byte) (cards: byte) (links: byte) reversed =
    let document = (SourcesFixtures.document categories cards links reversed).Document
    use response = new HttpResponseMessage(HttpStatusCode.OK, Content = new StringContent(Json.encode document))
    use handler = new HttpFixtures.ResponseHandler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "https://example.test/")
    Assert.Equal(Ok document, Api.readContract http CancellationToken.None (Reads.sources()) |> Async.RunSynchronously)

[<Property>]
let ``a served HTTP refusal preserves its status reason and whole typed body`` (status: byte) (code: ErrorCode) =
    let status = enum<HttpStatusCode>(400 + int status % 200)
    let body: ErrorBody = { Error = { Code = code; Message = "Served refusal" } }
    use response = new HttpResponseMessage(status, Content = new StringContent(Json.encode body), ReasonPhrase = "Reason")
    use handler = new HttpFixtures.ResponseHandler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "https://example.test/")
    let expected = Error(ReadFailure.HttpRejected { Status = status; Reason = Some "Reason"; Body = Ok body })
    Assert.Equal(expected, Api.readContract http CancellationToken.None (Reads.sources()) |> Async.RunSynchronously)

[<Property>]
let ``a successful null response remains a typed contract failure`` (spaces: byte) =
    use response = new HttpResponseMessage(HttpStatusCode.OK, Content = new StringContent(String.replicate (int spaces % 8) " " + "null"))
    use handler = new HttpFixtures.ResponseHandler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "https://example.test/")
    Assert.Equal(Error(ReadFailure.InvalidAnswer NullPayload), Api.readContract http CancellationToken.None (Reads.sources()) |> Async.RunSynchronously)

[<Property>]
let ``transport failures preserve their category and diagnostic`` (cancelled: bool) (suffix: uint16) =
    let diagnostic = $"transport-{suffix}"
    let error: exn = if cancelled then OperationCanceledException diagnostic else HttpRequestException diagnostic
    use handler = new HttpFixtures.FailureHandler(error)
    use http = new HttpClient(handler, BaseAddress = Uri "https://example.test/")
    let expected = Error(if cancelled then ReadFailure.Cancelled diagnostic else ReadFailure.Unreachable diagnostic)
    Assert.Equal(expected, Api.readContract http CancellationToken.None (Reads.sources()) |> Async.RunSynchronously)
