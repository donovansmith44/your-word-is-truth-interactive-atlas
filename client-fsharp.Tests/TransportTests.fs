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
let ``a served refusal stays an explicit failure instead of an empty collection`` (NonNull reason: NonNull<string>) (notFound: bool) =
    let status, code = if notFound then HttpStatusCode.NotFound, ErrorCode.NotFound else HttpStatusCode.BadRequest, ErrorCode.BadRef
    let body = Json.encode { Error = { Code = code; Message = reason } }
    use response = new HttpResponseMessage(status, Content = new StringContent(body))
    use handler = new Handler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let actual = Api.read http CancellationToken.None (Reads.nodeRecord (WireFixtures.identity<NodeId> "Person:absent")) |> Async.RunSynchronously
    Assert.Equal(Error(Transport $"{int status}: {reason}"), actual)

[<Property>]
let ``an interrupted HTTP request completes with an explicit retryable failure`` (NonNull reason: NonNull<string>) =
    use handler = new InterruptedHandler(reason)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let actual = Api.read http CancellationToken.None (Reads.sources()) |> Async.RunSynchronously
    Assert.Equal(Error(Transport reason), actual)

type Handler(response: HttpResponseMessage) =
    inherit HttpMessageHandler()
    override _.SendAsync(_, _) = System.Threading.Tasks.Task.FromResult response

type InterruptedHandler(reason: string) =
    inherit HttpMessageHandler()
    override _.SendAsync(_, _) = System.Threading.Tasks.Task.FromException<HttpResponseMessage>(System.OperationCanceledException reason)
