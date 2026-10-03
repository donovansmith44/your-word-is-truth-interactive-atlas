module rec BibleAtlas.FSharp.Tests.TransportTests

open System
open System.Net
open System.Net.Http
open System.Threading
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Fact>]
let ``generated requests compose escaped paths and the contract enum spellings`` () =
    let actual = [Request.uri (Reads.nodeRecord (WireFixtures.identity<NodeId> "text-unit:BoC 7.2.1")); Request.uri (Reads.nodeEdges (WireFixtures.identity<ElementId> "Person:god_1324") EdgeKind.MentionedIn (Some (WireFixtures.identity<EdgePageCursor> 0)) (Some 20)); Request.uri (Reads.textWindow (WireFixtures.identity<TextWindowReference> "JHN.3") None None (Some TextScope.Chapter) None); Request.uri (Reads.elements [(WireFixtures.identity<ElementId> "Event:a"); (WireFixtures.identity<ElementId> "Place:b")] None)]
    Assert.Equal<string list>(["api/node/text-unit%3ABoC%207.2.1"; "api/node/Person%3Agod_1324/edges?kind=mentioned-in&cursor=0&limit=20"; "api/text?ref=JHN.3&scope=chapter"; "api/elements?ids=Event%3Aa%2CPlace%3Ab"], actual)

[<Fact>]
let ``a typed HTTP read refuses a record missing its required fields`` () =
    use response = new HttpResponseMessage(HttpStatusCode.OK, Content = new StringContent("""{"id":"Person:a","kind":"Person","label":"A"}"""))
    use handler = new Handler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let request = Reads.nodeRecord (WireFixtures.identity<NodeId> "Person:a")
    let actual = Api.read http CancellationToken.None request |> Async.RunSynchronously
    Assert.True(Result.isError actual)

[<Fact>]
let ``a typed HTTP read returns the whole element page`` () =
    use response = new HttpResponseMessage(HttpStatusCode.OK, Content = new StringContent("""{"elements":[{"element":"missing","id":"Person:absent"}],"version":"root"}"""))
    use handler = new Handler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let actual = Api.read http CancellationToken.None (Reads.elements [(WireFixtures.identity<ElementId> "Person:absent")] None) |> Async.RunSynchronously
    Assert.Equal(Ok { Elements = [Element.Missing { Id = (WireFixtures.identity<ElementId> "Person:absent") }]; Version = (WireFixtures.identity<ArtifactRoot> "root"); Next = None; Previous = None }, actual)

[<Fact>]
let ``a served refusal stays an explicit failure instead of an empty collection`` () =
    use response = new HttpResponseMessage(HttpStatusCode.NotFound, Content = new StringContent("""{"error":{"code":"not_found","message":"node not found"}}"""))
    use handler = new Handler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let actual = Api.read http CancellationToken.None (Reads.nodeRecord (WireFixtures.identity<NodeId> "Person:absent")) |> Async.RunSynchronously
    Assert.Equal(Error(Transport "404: node not found"), actual)

[<Fact>]
let ``an interrupted HTTP request completes with an explicit retryable failure`` () =
    use handler = new InterruptedHandler()
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let actual = Api.read http CancellationToken.None (Reads.sources()) |> Async.RunSynchronously
    Assert.Equal(Error(Transport "read interrupted"), actual)

type Handler(response: HttpResponseMessage) =
    inherit HttpMessageHandler()
    override _.SendAsync(_, _) = System.Threading.Tasks.Task.FromResult response

type InterruptedHandler() =
    inherit HttpMessageHandler()
    override _.SendAsync(_, _) = System.Threading.Tasks.Task.FromException<HttpResponseMessage>(System.OperationCanceledException "read interrupted")
