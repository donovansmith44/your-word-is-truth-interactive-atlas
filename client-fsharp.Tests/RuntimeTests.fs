module BibleAtlas.FSharp.Tests.RuntimeTests

open System
open System.Net
open System.Net.Http
open System.Threading.Tasks
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Client
open BibleAtlas.FSharp.Contract

[<Fact>]
let ``the opening command resolves its typed position and delivers the whole trail`` () =
    let start = ExplorationTests.start
    let body = Json.encode { Elements = [Explorable.element start]; Version = "root"; Next = None; Previous = None }
    use response = new HttpResponseMessage(HttpStatusCode.OK, Content = new StringContent(body))
    use handler = new TransportTests.Handler(response)
    use http = new HttpClient(handler, BaseAddress = Uri "http://example.test/")
    let completion = TaskCompletionSource<Message>()
    let command = Runtime.command http (ReadOpening(RequestId.initial, Explorable.position start))
    for effect in command do effect (fun message -> completion.SetResult message)
    Assert.Equal(FocusLoaded(RequestId.initial, Ok(Trail.beginAt start)), completion.Task.GetAwaiter().GetResult())

[<Fact>]
let ``the Back command interprets the algebra without a transport read`` () =
    let start = ExplorationTests.start
    let target = ExplorationTests.node "Person:target" "root"
    let outward = Trail.beginAt start |> Trail.follow { Kind = EdgeKind.Contains; Target = target }
    let expected = outward |> Trail.follow { Kind = EdgeKind.MemberOf; Target = start }
    use http = new HttpClient(BaseAddress = Uri "http://example.test/")
    let completion = TaskCompletionSource<Message>()
    for effect in Runtime.command http (WalkFocus(RequestId.initial, outward, Traversal.Back)) do
        effect (fun message -> completion.SetResult message)
    Assert.Equal(FocusLoaded(RequestId.initial, Ok expected), completion.Task.GetAwaiter().GetResult())
