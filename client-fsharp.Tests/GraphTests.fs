module rec BibleAtlas.FSharp.Tests.GraphTests

open System
open System.Net
open System.Net.Http
open System.Threading.Tasks
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Fact>]
let ``an ordered resolution reads the whole element page once`` () =
    let page = { Elements = [Resolved.element first; Resolved.element second]; Version = "root"; Next = None; Previous = None }
    let actual = resolve [page] [Resolved.position first; Resolved.position second]
    Assert.Equal((Ok [first; second], ["/api/elements?ids=Person%3Afirst%2CPerson%3Asecond"]), actual)

[<Fact>]
let ``a paged resolution retains the requested order across a single artifact`` () =
    let pages = [{ Elements = [Resolved.element first]; Version = "root"; Next = Some 1; Previous = None }; { Elements = [Resolved.element second]; Version = "root"; Next = None; Previous = Some 0 }]
    let actual = resolve pages [Resolved.position first; Resolved.position second]
    Assert.Equal((Ok [first; second], ["/api/elements?ids=Person%3Afirst%2CPerson%3Asecond"; "/api/elements?ids=Person%3Afirst%2CPerson%3Asecond&cursor=1"]), actual)

[<Fact>]
let ``a root change between pages refuses the mixed journey`` () =
    let moved = ExplorationTests.node "Person:second" "new"
    let pages = [{ Elements = [Resolved.element first]; Version = "root"; Next = Some 1; Previous = None }; { Elements = [Resolved.element moved]; Version = "new"; Next = None; Previous = Some 0 }]
    let actual, _ = resolve pages [Resolved.position first; Resolved.position second]
    Assert.Equal(Error(ArtifactMoved("root", "new")), actual)

[<Fact>]
let ``a missing element cannot become a resolved position`` () =
    let page = { Elements = [Element.Missing { Id = "Person:first" }]; Version = "root"; Next = None; Previous = None }
    let actual, _ = resolve [page] [Resolved.position first]
    Assert.Equal(Error(Contract "the element read names nothing for Person:first"), actual)

[<Fact>]
let ``an incomplete element answer cannot shift the remaining journey`` () =
    let page = { Elements = [Resolved.element first]; Version = "root"; Next = None; Previous = None }
    let actual, _ = resolve [page] [Resolved.position first; Resolved.position second]
    Assert.Equal(Error(Contract "the element read returned 1 elements for 2 positions"), actual)

[<Fact>]
let ``a different answered identity cannot silently replace the requested position`` () =
    let page = { Elements = [Resolved.element second]; Version = "root"; Next = None; Previous = None }
    let actual, _ = resolve [page] [Resolved.position first]
    Assert.Equal(Error(Contract "the element read returned Person:second for Person:first"), actual)

[<Fact>]
let ``an empty resolution does no transport work`` () =
    Assert.Equal((Ok [], []), resolve [] [])

[<Fact>]
let ``a nonterminal empty page cannot keep a resolution reading forever`` () =
    let page = { Elements = []; Version = "root"; Next = Some 1; Previous = None }
    let actual, _ = resolve [page] [Resolved.position first]
    Assert.Equal(Error(Contract "the element read returned a nonterminal page with no positions"), actual)

let resolve (pages: ElementPage list) (positions: PositionRef list) : Result<Resolved list, Failure> * string list =
    let mutable answers = pages
    let mutable requests = []
    use handler = { new HttpMessageHandler() with
        override _.SendAsync(request, _) =
            requests <- requests @ [request.RequestUri.PathAndQuery]
            match answers with
            | page :: rest ->
                answers <- rest
                Task.FromResult(new HttpResponseMessage(HttpStatusCode.OK, Content = new StringContent(Json.encode page)))
            | [] -> Task.FromException<HttpResponseMessage>(InvalidOperationException "unexpected read") }
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let actual = (Graph.explorer http).Resolve positions |> Async.RunSynchronously
    actual, requests

let first = ExplorationTests.node "Person:first" "root"
let second = ExplorationTests.node "Person:second" "root"
