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
    let page = { Elements = [Explorable.element first; Explorable.element second]; Version = (WireFixtures.identity<ArtifactRoot> "root"); Next = None; Previous = None }
    let actual = resolve [page] [Explorable.position first; Explorable.position second]
    Assert.Equal((Ok [first; second], ["/api/elements?ids=Person%3Afirst%2CPerson%3Asecond"]), actual)

[<Fact>]
let ``a paged resolution retains the requested order across a single artifact`` () =
    let pages = [{ Elements = [Explorable.element first]; Version = (WireFixtures.identity<ArtifactRoot> "root"); Next = Some (WireFixtures.identity<ElementPageCursor> 1); Previous = None }; { Elements = [Explorable.element second]; Version = (WireFixtures.identity<ArtifactRoot> "root"); Next = None; Previous = Some (WireFixtures.identity<ElementPageCursor> 0) }]
    let actual = resolve pages [Explorable.position first; Explorable.position second]
    Assert.Equal((Ok [first; second], ["/api/elements?ids=Person%3Afirst%2CPerson%3Asecond"; "/api/elements?ids=Person%3Afirst%2CPerson%3Asecond&cursor=1"]), actual)

[<Fact>]
let ``a root change between pages refuses the mixed journey`` () =
    let moved = ExplorationTests.node "Person:second" "new"
    let pages = [{ Elements = [Explorable.element first]; Version = (WireFixtures.identity<ArtifactRoot> "root"); Next = Some (WireFixtures.identity<ElementPageCursor> 1); Previous = None }; { Elements = [Explorable.element moved]; Version = (WireFixtures.identity<ArtifactRoot> "new"); Next = None; Previous = Some (WireFixtures.identity<ElementPageCursor> 0) }]
    let actual, _ = resolve pages [Explorable.position first; Explorable.position second]
    Assert.Equal(Error(ArtifactMoved((WireFixtures.identity<ArtifactRoot> "root"), (WireFixtures.identity<ArtifactRoot> "new"))), actual)

[<Fact>]
let ``a missing element cannot become a resolved position`` () =
    let page = { Elements = [Element.Missing { Id = (WireFixtures.identity<ElementId> "Person:first") }]; Version = (WireFixtures.identity<ArtifactRoot> "root"); Next = None; Previous = None }
    let actual, _ = resolve [page] [Explorable.position first]
    Assert.Equal(Error(Contract "the element read names nothing for Person:first"), actual)

[<Fact>]
let ``an incomplete element answer cannot shift the remaining journey`` () =
    let page = { Elements = [Explorable.element first]; Version = (WireFixtures.identity<ArtifactRoot> "root"); Next = None; Previous = None }
    let actual, _ = resolve [page] [Explorable.position first; Explorable.position second]
    Assert.Equal(Error(Contract "the element read returned 1 elements for 2 positions"), actual)

[<Fact>]
let ``a different answered identity cannot silently replace the requested position`` () =
    let page = { Elements = [Explorable.element second]; Version = (WireFixtures.identity<ArtifactRoot> "root"); Next = None; Previous = None }
    let actual, _ = resolve [page] [Explorable.position first]
    Assert.Equal(Error(Contract "the element read returned Person:second for Person:first"), actual)

[<Fact>]
let ``an empty resolution does no transport work`` () =
    Assert.Equal((Ok [], []), resolve [] [])

[<Fact>]
let ``a nonterminal empty page cannot keep a resolution reading forever`` () =
    let page = { Elements = []; Version = (WireFixtures.identity<ArtifactRoot> "root"); Next = Some (WireFixtures.identity<ElementPageCursor> 1); Previous = None }
    let actual, _ = resolve [page] [Explorable.position first]
    Assert.Equal(Error(Contract "the element read returned a nonterminal page with no positions"), actual)

let resolve (pages: ElementPage list) (positions: PositionRef list) : Result<Explorable list, Failure> * string list =
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
    let actual = (GraphRead.explorer http).Resolve positions |> Async.RunSynchronously
    actual, requests

let first = ExplorationTests.node "Person:first" "root"
let second = ExplorationTests.node "Person:second" "root"
