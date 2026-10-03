module rec BibleAtlas.FSharp.Tests.TransportTests

open System
open System.Net
open System.Net.Http
open System.Threading
open Xunit
open FsCheck
open FsCheck.Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Admission
open Microsoft.FSharp.Reflection

[<Property>]
let ``generated requests compose escaped paths and the contract enum spellings`` (suffix: uint16) =
    let actual = [Request.uri (Reads.nodeRecord (WireFixtures.identity<NodeId> $"text-unit:BoC 7.2.{suffix}")); Request.uri (Reads.nodeEdges (WireFixtures.identity<ElementId> $"Person:served-{suffix}") EdgeKind.MentionedIn (Some EdgePageCursor.first) (Some 20)); Request.uri (Reads.textWindow (TextWindowReference.ofChapterReference (WireFixtures.identity<ChapterReference> "JHN.3")) None None (Some TextScope.Chapter) None); Request.uri (Reads.elements [ElementId.ofNodeId (WireFixtures.identity<NodeId> $"Event:a-{suffix}"); ElementId.ofEdgeId (WireFixtures.identity<EdgeId> $"edge:b-{suffix}")] None)]
    Assert.Equal<string list>([$"api/node/text-unit%%3ABoC%%207.2.{suffix}"; $"api/node/Person%%3Aserved-{suffix}/edges?kind=mentioned-in&cursor=0&limit=20"; "api/text?ref=JHN.3&scope=chapter"; $"api/elements?ids=Event%%3Aa-{suffix}%%2Cedge%%3Ab-{suffix}"], actual)

[<Property>]
let ``a typed HTTP read refuses a record missing its required fields`` (suffix: uint16) =
    let body = System.Text.Json.JsonSerializer.Serialize {| id = $"Person:{suffix}"; kind = "Person"; label = $"Label {suffix}" |}
    use response = new HttpResponseMessage(HttpStatusCode.OK, Content = new StringContent(body))
    use handler = new Handler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let request = Reads.nodeRecord (WireFixtures.identity<NodeId> $"Person:{suffix}")
    let actual = Api.read http CancellationToken.None request |> Async.RunSynchronously
    Assert.Equal(Error(Contract "Missing field for record type BibleAtlas.FSharp.Contract.NodeRecord: edge_summary"), actual)

[<Property>]
let ``a typed HTTP read returns the whole element page`` (suffix: uint16) =
    let identity = ElementId.ofNodeId (WireFixtures.identity<NodeId> $"Person:absent-{suffix}")
    let expected: ElementPage = { Elements = [Element.Missing { Id = identity }]; Version = WireFixtures.identity<ArtifactRoot> $"root-{suffix}"; Next = None; Previous = None }
    use response = new HttpResponseMessage(HttpStatusCode.OK, Content = new StringContent(Json.encode expected))
    use handler = new Handler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let actual = Api.read http CancellationToken.None (Reads.elements [identity] None) |> Async.RunSynchronously
    Assert.Equal(Ok expected, actual)

[<Property>]
let ``every client refusal retains its exact admitted status and published error code`` (offset: byte) (code: ErrorCode) (NonNull reason: NonNull<string>) =
    let status = enum<HttpStatusCode> (clientRefusalMinimum + int offset % refusalClassSize)
    let body = Json.encode { Error = { Code = code; Message = reason } }
    use response = new HttpResponseMessage(status, Content = new StringContent(body))
    use handler = new Handler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let actual = Api.read http CancellationToken.None (Reads.nodeRecord (WireFixtures.identity<NodeId> "Person:absent")) |> Async.RunSynchronously
    Assert.Equal(Error(BibleAtlas.FSharp.Failure.Read(ReadFailure.Terminal(TerminalFailure.ClientRefusal(clientStatus (int status), Some code)))), actual)

[<Property>]
let ``an interrupted HTTP request completes with an explicit retryable failure`` (NonNull reason: NonNull<string>) (network: bool) =
    use handler = new InterruptedHandler(reason, network)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let actual = Api.read http CancellationToken.None (Reads.sources()) |> Async.RunSynchronously
    Assert.Equal(Error(BibleAtlas.FSharp.Failure.Read(ReadFailure.Transient TransientFailure.Unreachable)), actual)


[<Property>]
let ``every server refusal retains its exact admitted status and published error code`` (offset: byte) (code: ErrorCode) (NonNull reason: NonNull<string>) =
    let status = serverRefusalMinimum + int offset % refusalClassSize
    let body = Json.encode { Error = { Code = code; Message = reason } }
    use response = new HttpResponseMessage(enum<HttpStatusCode> status, Content = new StringContent(body))
    use handler = new Handler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let actual = Api.read http CancellationToken.None (Reads.sources()) |> Async.RunSynchronously
    Assert.Equal(Error(BibleAtlas.FSharp.Failure.Read(ReadFailure.Transient(TransientFailure.ServerRefusal(serverStatus status, Some code)))), actual)

[<Property>]
let ``a user-cancelled HTTP read is distinct from a retryable interruption`` (NonNull reason: NonNull<string>) =
    use cancellation = new CancellationTokenSource()
    cancellation.Cancel()
    use handler = new InterruptedHandler(reason, false)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let actual = Api.read http cancellation.Token (Reads.sources()) |> Async.RunSynchronously
    Assert.Equal(Error(BibleAtlas.FSharp.Failure.Read ReadFailure.Cancelled), actual)

[<Property>]
let ``every malformed refusal body preserves its HTTP classification without inventing an error code`` (offset: byte) (server: bool) =
    let status = (if server then serverRefusalMinimum else clientRefusalMinimum) + int offset % refusalClassSize
    let body = $"{{\"error\":{{\"code\":\"unknown-{offset}\",\"message\":\"unrecognized\"}}}}"
    use response = new HttpResponseMessage(enum<HttpStatusCode> status, Content = new StringContent(body))
    use handler = new Handler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let expected = if server then ReadFailure.Transient(TransientFailure.ServerRefusal(serverStatus status, None)) else ReadFailure.Terminal(TerminalFailure.ClientRefusal(clientStatus status, None))
    let actual = Api.read http CancellationToken.None (Reads.sources()) |> Async.RunSynchronously
    Assert.Equal(Error(BibleAtlas.FSharp.Failure.Read expected), actual)

[<Property>]
let ``unexpected HTTP statuses retain their evidence and invalid codes have server-error retryability`` (offset: uint16) (family: byte) (code: ErrorCode) =
    let status =
        match family % 3uy with
        | 0uy -> informationalMinimum + int offset % refusalClassSize
        | 1uy -> redirectionMinimum + int offset % refusalClassSize
        | _ -> afterServerRefusals + int offset % unexpectedExtensionSize
    let body = Json.encode { Error = { Code = code; Message = "unexpected status" } }
    use response = new HttpResponseMessage(enum<HttpStatusCode> status, Content = new StringContent(body))
    use handler = new Handler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let actual = Api.read http CancellationToken.None (Reads.sources()) |> Async.RunSynchronously
    let expected =
        if status >= afterServerRefusals then ReadFailure.Transient(TransientFailure.InvalidStatus(enum<HttpStatusCode> status, Some code))
        else ReadFailure.Terminal(TerminalFailure.UnexpectedStatus(enum<HttpStatusCode> status, Some code))
    Assert.Equal(Error(BibleAtlas.FSharp.Failure.Read expected), actual)

[<Property(MaxTest = 1)>]
let ``the application failure vocabulary exposes structured reads and no raw transport string constructor`` () =
    let actual = FSharpType.GetUnionCases(typeof<Failure>) |> Array.map (fun case -> case.Name, case.GetFields() |> Array.map (fun field -> field.PropertyType) |> Array.toList) |> Array.toList
    let expected = ["Read", [typeof<ReadFailure>]; "Contract", [typeof<string>]; "ArtifactMoved", [typeof<ArtifactRoot>; typeof<ArtifactRoot>]]
    Assert.Equal<(string * Type list) list>(expected, actual)

let private clientStatus status =
    match RefusalStatuses.client status with
    | Ok status -> status
    | Error failure -> failwithf "%A" failure

let private serverStatus status =
    match RefusalStatuses.server status with
    | Ok status -> status
    | Error failure -> failwithf "%A" failure

let private clientRefusalMinimum = 400
let private serverRefusalMinimum = 500
let private refusalClassSize = 100
let private informationalMinimum = 100
let private redirectionMinimum = 300
let private afterServerRefusals = 600
let private unexpectedExtensionSize = 400

type Handler(response: HttpResponseMessage) =
    inherit HttpMessageHandler()
    override _.SendAsync(_, _) = System.Threading.Tasks.Task.FromResult response

type InterruptedHandler(reason: string, network: bool) =
    inherit HttpMessageHandler()
    override _.SendAsync(_, _) =
        let failure: exn = if network then HttpRequestException reason else OperationCanceledException reason
        System.Threading.Tasks.Task.FromException<HttpResponseMessage>(failure)
