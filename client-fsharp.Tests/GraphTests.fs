module rec BibleAtlas.FSharp.Tests.GraphTests

open System
open System.Net
open System.Net.Http
open System.Threading.Tasks
open Xunit
open FsCheck.Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Admission

[<Property>]
let ``an ordered resolution reads the whole element page once`` (suffix: uint16) =
    let first = first suffix
    let second = second suffix
    let root = root suffix
    let page = { Elements = [Explorable.element first; Explorable.element second]; Version = root; Next = None; Previous = None }
    let actual = resolve [page] [Explorable.position first; Explorable.position second]
    Assert.Equal((Ok [first; second], [($"/api/elements?ids=Person%%3Afirst-{suffix}%%2CPerson%%3Asecond-{suffix}")]), actual)

[<Property>]
let ``a paged resolution retains the requested order across a single artifact`` (suffix: uint16) =
    let first = first suffix
    let second = second suffix
    let root = root suffix
    let pages = [{ Elements = [Explorable.element first]; Version = root; Next = Some (WireFixtures.identity<ElementPageCursor> 1); Previous = None }; { Elements = [Explorable.element second]; Version = root; Next = None; Previous = Some (WireFixtures.identity<ElementPageCursor> 0) }]
    let actual = resolve pages [Explorable.position first; Explorable.position second]
    Assert.Equal((Ok [first; second], [($"/api/elements?ids=Person%%3Afirst-{suffix}%%2CPerson%%3Asecond-{suffix}"); ($"/api/elements?ids=Person%%3Afirst-{suffix}%%2CPerson%%3Asecond-{suffix}&cursor=1")]), actual)

[<Property>]
let ``a root change between pages refuses the mixed journey`` (suffix: uint16) =
    let first = first suffix
    let second = second suffix
    let root = root suffix
    let moved = ExplorationTests.node $"Person:second-{suffix}" $"new-{suffix}"
    let pages = [{ Elements = [Explorable.element first]; Version = root; Next = Some (WireFixtures.identity<ElementPageCursor> 1); Previous = None }; { Elements = [Explorable.element moved]; Version = (WireFixtures.identity<ArtifactRoot> $"new-{suffix}"); Next = None; Previous = Some (WireFixtures.identity<ElementPageCursor> 0) }]
    let actual = resolve pages [Explorable.position first; Explorable.position second]
    let expected = Error(ArtifactMoved(root, WireFixtures.identity<ArtifactRoot> $"new-{suffix}")), [$"/api/elements?ids=Person%%3Afirst-{suffix}%%2CPerson%%3Asecond-{suffix}"; $"/api/elements?ids=Person%%3Afirst-{suffix}%%2CPerson%%3Asecond-{suffix}&cursor=1"]
    Assert.Equal(expected, actual)

[<Property>]
let ``a missing element cannot become a resolved position`` (suffix: uint16) =
    let first = first suffix
    let root = root suffix
    let page = { Elements = [Element.Missing { Id = Positions.id (Explorable.position first) }]; Version = root; Next = None; Previous = None }
    let actual = resolve [page] [Explorable.position first]
    Assert.Equal((Error(WireFixtures.graphFailure(GraphFailure.MissingElement(Positions.id (Explorable.position first)))), [$"/api/elements?ids=Person%%3Afirst-{suffix}"]), actual)

[<Property>]
let ``an incomplete element answer cannot shift the remaining journey`` (suffix: uint16) =
    let first = first suffix
    let second = second suffix
    let root = root suffix
    let page = { Elements = [Explorable.element first]; Version = root; Next = None; Previous = None }
    let actual = resolve [page] [Explorable.position first; Explorable.position second]
    Assert.Equal((Error(WireFixtures.graphFailure(GraphFailure.ElementCountMismatch { Requested = pairElementCount; Received = singleElementCount })), [$"/api/elements?ids=Person%%3Afirst-{suffix}%%2CPerson%%3Asecond-{suffix}"]), actual)

[<Property>]
let ``a different answered identity cannot silently replace the requested position`` (suffix: uint16) =
    let first = first suffix
    let second = second suffix
    let root = root suffix
    let page = { Elements = [Explorable.element second]; Version = root; Next = None; Previous = None }
    let actual = resolve [page] [Explorable.position first]
    Assert.Equal((Error(WireFixtures.graphFailure(GraphFailure.UnexpectedElement { Requested = Explorable.position first; Received = Explorable.position second })), [$"/api/elements?ids=Person%%3Afirst-{suffix}"]), actual)

[<Property>]
let ``an empty resolution does no transport work`` (suffix: uint16) =
    let offered = [{ Elements = [Explorable.element (first suffix)]; Version = root suffix; Next = None; Previous = None }]
    Assert.Equal((Ok [], []), resolve offered [])

[<Property>]
let ``a nonterminal empty page cannot keep a resolution reading forever`` (suffix: uint16) =
    let first = first suffix
    let root = root suffix
    let page = { Elements = []; Version = root; Next = Some (WireFixtures.identity<ElementPageCursor> 1); Previous = None }
    let actual = resolve [page] [Explorable.position first]
    Assert.Equal((Error(WireFixtures.graphFailure(GraphFailure.EmptyContinuation(WireFixtures.identity<ElementPageCursor> 1))), [$"/api/elements?ids=Person%%3Afirst-{suffix}"]), actual)

[<Property>]
let ``extra answered elements retain the whole requested and received cardinalities`` (suffix: uint16) =
    let first, second = first suffix, second suffix
    let page = { Elements = [Explorable.element first; Explorable.element second]; Version = root suffix; Next = None; Previous = None }
    let actual = resolve [page] [Explorable.position first]
    let expected = Error(WireFixtures.graphFailure(GraphFailure.ElementCountMismatch { Requested = singleElementCount; Received = pairElementCount })), [$"/api/elements?ids=Person%%3Afirst-{suffix}"]
    Assert.Equal(expected, actual)

[<Property>]
let ``a continuation after all requested elements preserves its exact cursor and sends no further read`` (suffix: uint16) (cursor: uint16) =
    let first = first suffix
    let continuation = WireFixtures.identity<ElementPageCursor> (int cursor)
    let page = { Elements = [Explorable.element first]; Version = root suffix; Next = Some continuation; Previous = None }
    let actual = resolve [page] [Explorable.position first]
    let expected = Error(WireFixtures.graphFailure(GraphFailure.ExcessContinuation continuation)), [$"/api/elements?ids=Person%%3Afirst-{suffix}"]
    Assert.Equal(expected, actual)

[<Property>]
let ``a changed served kind cannot replace the same requested node identity`` (suffix: uint16) =
    let first = first suffix
    let position = Explorable.position first
    let served =
        match Explorable.element first with
        | Element.Node node -> Element.Node { Node = { node.Node with Kind = NodeKind.Event } }
        | Element.Edge _ | Element.Missing _ -> failwith "the fixture is a node"
    let answered =
        match Explorable.ofElement (root suffix) served with
        | Ok answered -> answered
        | Error failure -> failwithf "%A" failure
    let page = { Elements = [served]; Version = root suffix; Next = None; Previous = None }
    let actual = resolve [page] [position]
    let expected = Error(WireFixtures.graphFailure(GraphFailure.UnexpectedElement { Requested = position; Received = Explorable.position answered })), [$"/api/elements?ids=Person%%3Afirst-{suffix}"]
    Assert.Equal(expected, actual)

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

let private first suffix = ExplorationTests.node $"Person:first-{suffix}" $"root-{suffix}"
let private second suffix = ExplorationTests.node $"Person:second-{suffix}" $"root-{suffix}"
let private root suffix = WireFixtures.identity<ArtifactRoot> $"root-{suffix}"

let private singleElementCount = 1UL
let private pairElementCount = 2UL
