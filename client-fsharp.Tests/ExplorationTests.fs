module rec BibleAtlas.FSharp.Tests.ExplorationTests

open System.IO
open System.Text.Json
open Microsoft.FSharp.Reflection
open Xunit
open FsCheck
open FsCheck.Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Property>]
let ``the exploration expression composes nested reads with the complete generated trail`` (suffix: uint16) (value: int) (kind: EdgeKind) =
    let start = node $"Person:start-{suffix}" $"root-{suffix}"
    let target = node $"Person:target-{suffix}" $"root-{suffix}"
    let trail = Trail.beginAt start |> Trail.follow { Kind = kind; Target = target }
    let action = explore {
        let! current = Explore.here ()
        let! same = explore { return current }
        return same, value
    }
    let actual = Explore.run explorer trail action |> Async.RunSynchronously
    Assert.Equal(Ok((target, value), trail), actual)

[<Property>]
let ``a failed exploration stops the continuation and makes exactly its requested read`` (suffix: uint16) (kind: EdgeKind) (NonNull reason: NonNull<string>) =
    let start = node $"Person:start-{suffix}" $"root-{suffix}"
    let requested = Explorable.position (node $"Person:target-{suffix}" $"root-{suffix}")
    let mutable calls = []
    let mutable continued = false
    let failure = Contract reason
    let resolver = { Resolve = fun asked -> async { calls <- calls @ [asked]; return Error failure } }
    let action = Explore.follow { Kind = kind; Target = requested } |> Explore.bind (fun _ -> continued <- true; Explore.result start)
    let actual = Explore.run resolver (Trail.beginAt start) action |> Async.RunSynchronously
    Assert.Equal((Error failure, [[requested]], false), (actual, calls, continued))

[<Property>]
let ``Back records the served dual edge and cancels the whole visible breadcrumb`` (suffix: uint16) (kind: EdgeKind) =
    let start = node $"Person:start-{suffix}" $"root-{suffix}"
    let target = node $"Person:target-{suffix}" $"root-{suffix}"
    let outward = Trail.beginAt start |> Trail.follow { Kind = kind; Target = target }
    let actual = Explore.run explorer outward (Explore.back ()) |> Async.RunSynchronously
    let expectedTrail = outward |> Trail.follow { Kind = EdgeKinds.dual kind; Target = start }
    Assert.Equal((Ok(start, expectedTrail), [], [start; target; start]), (actual, Trail.breadcrumb expectedTrail, Trail.walked expectedTrail))

[<Property>]
let ``Back at the beginning preserves the complete trail and does no graph read`` (suffix: uint16) =
    let start = node $"Person:start-{suffix}" $"root-{suffix}"
    let trail = Trail.beginAt start
    let mutable calls = []
    let resolver = { Resolve = fun asked -> async { calls <- calls @ [asked]; return Error(Contract "unexpected read") } }
    let actual = Explore.run resolver trail (Explore.back ()) |> Async.RunSynchronously
    Assert.Equal((Ok(start, trail), []), (actual, calls))

[<Property>]
let ``Resume resolves every saved identity once and retains the bounded whole journey with its kinds`` (suffix: uint16) (kinds: EdgeKind list) =
    let start = node $"Person:start-{suffix}" $"root-{suffix}"
    let targets = kinds |> List.mapi (fun index _ -> node $"Person:target-{suffix}-{index}" $"root-{suffix}")
    let saved: Link list = List.map2 (fun kind target -> { Kind = kind; Target = Explorable.position target }) kinds targets
    let mutable calls = []
    let resolver = { Resolve = fun asked -> async { calls <- calls @ [asked]; return Ok(start :: targets) } }
    let actual = Explore.resume resolver (Explorable.position start) saved |> Async.RunSynchronously
    let expectedTrail = List.map2 (fun kind target -> { Kind = kind; Target = target }) kinds targets |> List.fold (fun trail step -> Trail.follow step trail) (Trail.beginAt start)
    let expectedRead = Explorable.position start :: List.map Explorable.position targets
    let expectedWalked = (start :: targets) |> List.skip (max 0 (targets.Length - Trail.retainedSteps))
    Assert.Equal((Ok expectedTrail, [expectedRead], Ok expectedWalked), (actual, calls, actual |> Result.map Trail.walked))

[<Property>]
let ``Resume preserves a failed whole-journey read without constructing a false trail`` (suffix: uint16) (kinds: EdgeKind list) (NonNull reason: NonNull<string>) =
    let start = node $"Person:start-{suffix}" $"root-{suffix}"
    let targets: Link list = kinds |> List.mapi (fun index kind -> { Kind = kind; Target = Explorable.position (node $"Person:target-{suffix}-{index}" $"root-{suffix}") })
    let mutable calls = []
    let failure = Transport reason
    let resolver = { Resolve = fun asked -> async { calls <- calls @ [asked]; return Error failure } }
    let actual = Explore.resume resolver (Explorable.position start) targets |> Async.RunSynchronously
    Assert.Equal((Error failure, [Explorable.position start :: List.map (fun (link: Link) -> link.Target) targets]), (actual, calls))

[<Property>]
let ``a resumed journey refuses every generated missing target cardinality`` (suffix: uint16) (kind: EdgeKind) (kinds: EdgeKind list) =
    let start = node $"Person:start-{suffix}" $"root-{suffix}"
    let targets = kinds |> List.mapi (fun index _ -> node $"Person:target-{suffix}-{index}" $"root-{suffix}")
    let requestedKinds = kind :: kinds
    let actual = Trail.resume (start :: targets) requestedKinds
    Assert.Equal(Error(Contract $"the resolved journey has {targets.Length + 1} elements for {requestedKinds.Length} steps"), actual)

[<Property>]
let ``a resumed journey refuses a final target from a different generated artifact root`` (suffix: uint16) (kind: EdgeKind) =
    let root = $"root-{suffix}"
    let movedRoot = $"moved-{suffix}"
    let start = node $"Person:start-{suffix}" root
    let target = node $"Person:target-{suffix}" movedRoot
    Assert.Equal(Error(ArtifactMoved(WireFixtures.identity<ArtifactRoot> root, WireFixtures.identity<ArtifactRoot> movedRoot)), Trail.resume [start; target] [kind])

[<Property>]
let ``a missing element cannot become a resolved position for any requested identity`` (suffix: uint16) =
    let root = WireFixtures.identity<ArtifactRoot> $"root-{suffix}"
    let missing = $"Person:absent-{suffix}"
    let actual = Explorable.ofElement root (Element.Missing { Id = WireFixtures.identity<ElementId> missing })
    Assert.Equal(Error(Contract $"the element read names nothing for {missing}"), actual)

[<Property>]
let ``every generated edge kind has an involutive dual from the published vocabulary`` (kind: EdgeKind) =
    let all = FSharpType.GetUnionCases(typeof<EdgeKind>) |> Array.map (fun case -> FSharpValue.MakeUnion(case, [||]) :?> EdgeKind) |> Array.toList
    use source = JsonDocument.Parse(File.ReadAllText(Path.Combine(__SOURCE_DIRECTORY__, "../contracts/atlas-graph-contract/fixtures/graph-vocabulary.json")))
    let expected = [ for relation in source.RootElement.GetProperty("relations").EnumerateArray() do yield relation.GetProperty("forward").GetString(), relation.GetProperty("inverse").GetString(); yield relation.GetProperty("inverse").GetString(), relation.GetProperty("forward").GetString()
                     for relation in source.RootElement.GetProperty("symmetric").EnumerateArray() do yield relation.GetProperty("label").GetString(), relation.GetProperty("label").GetString() ] |> List.sort
    let spelling kind = Json.encode kind |> fun value -> JsonSerializer.Deserialize<string> value
    Assert.Equal((kind, expected), (EdgeKinds.dual (EdgeKinds.dual kind), all |> List.map (fun kind -> spelling kind, spelling (EdgeKinds.dual kind)) |> List.sort))

[<Property>]
let ``a node record cannot be stamped with a different generated page root`` (suffix: uint16) =
    let actualRoot = $"root-{suffix}"
    let pageRoot = $"page-root-{suffix}"
    let served = Explorable.element (node $"Person:start-{suffix}" actualRoot)
    Assert.Equal(Error(ArtifactMoved(WireFixtures.identity<ArtifactRoot> pageRoot, WireFixtures.identity<ArtifactRoot> actualRoot)), Explorable.ofElement (WireFixtures.identity<ArtifactRoot> pageRoot) served)

let explorer = { Resolve = fun _ -> async { return Error(Contract "unexpected read") } }
let node (id: string) (root: string) : Explorable =
    let record = { Id = (WireFixtures.identity<NodeId> id); Kind = NodeKind.Person; Label = id; Provenance = { Id = "test"; Title = "test" }; EdgeSummary = []; Version = (WireFixtures.identity<ArtifactRoot> root); Book = None; Catechism = None; Description = None; Era = None; Event = None; Map = None; Person = None; Place = None; Polity = None; Text = None }
    match Explorable.ofElement (WireFixtures.identity<ArtifactRoot> root) (Element.Node { Node = record }) with
    | Ok node -> node
    | Error failure -> failwithf "%A" failure

let start = node "Person:start" "root"
let trail = Trail.beginAt start
