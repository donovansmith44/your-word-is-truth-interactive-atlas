module rec BibleAtlas.FSharp.Tests.ExplorationTests

open System.IO
open System.Text.Json
open Microsoft.FSharp.Reflection
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Fact>]
let ``exploration obeys the monad left identity`` () =
    let f value = Explore.result (value + 1)
    Assert.Equal(run (f 7), run (Explore.result 7 |> Explore.bind f))

[<Fact>]
let ``exploration obeys the monad right identity including its trail`` () =
    Assert.Equal(run Explore.here, run (Explore.here |> Explore.bind Explore.result))

[<Fact>]
let ``exploration obeys associativity including its whole trail`` () =
    let f value = Explore.result (value + 1)
    let g value = Explore.result (string value)
    let m = Explore.result 7
    Assert.Equal(run (m |> Explore.bind f |> Explore.bind g), run (m |> Explore.bind (fun value -> f value |> Explore.bind g)))

[<Fact>]
let ``the exploration computation expression composes reads without losing the trail`` () =
    let explore = ExploreBuilder()
    let action = explore {
        let! current = Explore.here
        let! same = explore { return current }
        return! Explore.result same
    }
    Assert.Equal(run Explore.here, run action)

[<Fact>]
let ``an exploration failure short circuits later reads and preserves no false arrival`` () =
    let action = Explore.follow { Kind = EdgeKind.Contains; Target = Explorable.position start } |> Explore.bind (fun _ -> Explore.result 7)
    Assert.Equal(Error(Contract "unexpected read"), run action)

[<Fact>]
let ``Back records the dual step while cancelling the visible breadcrumb`` () =
    let target = node "Person:target" "root"
    let outward = trail |> Trail.follow { Kind = EdgeKind.Contains; Target = target }
    let actual = Explore.run explorer outward Explore.back |> Async.RunSynchronously
    let expectedTrail = outward |> Trail.follow { Kind = EdgeKind.MemberOf; Target = start }
    Assert.Equal(Ok(start, expectedTrail), actual)
    Assert.Equal<Step list>([], Trail.breadcrumb expectedTrail)
    Assert.Equal<Explorable list>([start; target; start], Trail.walked expectedTrail)

[<Fact>]
let ``Resume resolves the entire saved journey in one read and retains step kinds`` () =
    let target = node "Person:target" "root"
    let mutable calls = []
    let resolver = { Resolve = fun asked -> async { calls <- asked :: calls; return Ok [start; target] } }
    let saved: Link list = [{ Kind = EdgeKind.Contains; Target = Explorable.position target }]
    let actual = Explore.resume resolver (Explorable.position start) saved |> Async.RunSynchronously
    Assert.Equal(Ok(Trail.beginAt start |> Trail.follow { Kind = EdgeKind.Contains; Target = target }), actual)
    Assert.Equal<PositionRef list list>([[Explorable.position start; Explorable.position target]], List.rev calls)

[<Fact>]
let ``a resumed journey refuses wrong cardinality and mixed artifact roots`` () =
    let target = node "Person:target" "other-root"
    Assert.Equal(Error(Contract "the resolved journey has 1 elements for 1 steps"), Trail.resume [start] [EdgeKind.Contains])
    Assert.Equal(Error(ArtifactMoved((WireFixtures.identity<ArtifactRoot> "root"), (WireFixtures.identity<ArtifactRoot> "other-root"))), Trail.resume [start; target] [EdgeKind.Contains])

[<Fact>]
let ``a missing element cannot become a resolved position`` () =
    Assert.Equal(Error(Contract "the element read names nothing for Person:absent"), Explorable.ofElement (WireFixtures.identity<ArtifactRoot> "root") (Element.Missing { Id = (WireFixtures.identity<ElementId> "Person:absent") }))

[<Fact>]
let ``every generated edge kind has a dual and duality is involutive`` () =
    let all = FSharpType.GetUnionCases(typeof<EdgeKind>) |> Array.map (fun case -> FSharpValue.MakeUnion(case, [||]) :?> EdgeKind) |> Array.toList
    Assert.Equal<EdgeKind list>(all, all |> List.map (EdgeKinds.dual >> EdgeKinds.dual))
    let source = JsonDocument.Parse(File.ReadAllText(Path.Combine(__SOURCE_DIRECTORY__, "../contracts/atlas-graph-contract/fixtures/graph-vocabulary.json")))
    let expected = [ for relation in source.RootElement.GetProperty("relations").EnumerateArray() do yield relation.GetProperty("forward").GetString(), relation.GetProperty("inverse").GetString(); yield relation.GetProperty("inverse").GetString(), relation.GetProperty("forward").GetString()
                     for relation in source.RootElement.GetProperty("symmetric").EnumerateArray() do yield relation.GetProperty("label").GetString(), relation.GetProperty("label").GetString() ] |> List.sort
    let spelling kind = Json.encode kind |> fun value -> JsonSerializer.Deserialize<string> value
    Assert.Equal<(string * string) list>(expected, all |> List.map (fun kind -> spelling kind, spelling (EdgeKinds.dual kind)) |> List.sort)

[<Fact>]
let ``a node record cannot be stamped with a different element page root`` () =
    let served = Explorable.element start
    Assert.Equal(Error(ArtifactMoved((WireFixtures.identity<ArtifactRoot> "page-root"), (WireFixtures.identity<ArtifactRoot> "root"))), Explorable.ofElement (WireFixtures.identity<ArtifactRoot> "page-root") served)
let run<'a> (action: Explore<'a>) : Result<'a * Trail, Failure> = Explore.run explorer trail action |> Async.RunSynchronously
let explorer = { Resolve = fun _ -> async { return Error(Contract "unexpected read") } }
let node (id: string) (root: string) : Explorable =
    let record = { Id = (WireFixtures.identity<NodeId> id); Kind = NodeKind.Person; Label = id; Provenance = { Id = "test"; Title = "test" }; EdgeSummary = []; Version = (WireFixtures.identity<ArtifactRoot> root); Book = None; Catechism = None; Description = None; Era = None; Event = None; Map = None; Person = None; Place = None; Polity = None; Text = None }
    match Explorable.ofElement (WireFixtures.identity<ArtifactRoot> root) (Element.Node { Node = record }) with
    | Ok node -> node
    | Error failure -> failwithf "%A" failure

let start = node "Person:start" "root"
let trail = Trail.beginAt start
