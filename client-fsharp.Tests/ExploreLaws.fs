module rec BibleAtlas.FSharp.Tests.ExploreLaws

open FsCheck.Xunit
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Property>]
let ``exploration left identity preserves the complete result and trail for every continuation`` (value: int) (fails: bool) =
    let continueWith value = if fails then Explore.follow link |> Explore.map (fun _ -> value) else Explore.result value
    Assert.Equal(run (continueWith value), run (Explore.result value |> Explore.bind continueWith))

[<Property>]
let ``exploration right identity preserves a successful or failed stateful walk`` (fails: bool) =
    let walk = if fails then Explore.follow link else (Explore.back ())
    Assert.Equal(run walk, run (walk |> Explore.bind Explore.result))

[<Property>]
let ``exploration associativity preserves failures and the entire trail`` (value: int) (firstFails: bool) (secondFails: bool) =
    let first value = if firstFails then Explore.follow link |> Explore.map (fun _ -> value) else (Explore.back ()) |> Explore.map (fun _ -> value)
    let second value = if secondFails then Explore.follow link |> Explore.map (fun _ -> string value) else Explore.result (string value)
    let walk = Explore.result value
    Assert.Equal(run (walk |> Explore.bind first |> Explore.bind second), run (walk |> Explore.bind (fun result -> first result |> Explore.bind second)))

[<Property>]
let ``constructing an exploration defers its whole computation until execution`` (value: int) =
    let mutable evaluated = false
    let action = ExploreBuilder() {
        evaluated <- true
        return value
    }
    let before = evaluated
    let answer = run action
    Assert.Equal((false, true, Ok(value, trail)), (before, evaluated, answer))

[<Property>]
let ``following across a root change renews every retained identity on the new root`` (suffix: uint16) =
    let root = $"root-{suffix}"
    let start = ExplorationTests.node "Person:start" "old-root"
    let target = ExplorationTests.node "Person:target" root
    let renewedStart = ExplorationTests.node "Person:start" root
    let explorer = { Resolve = fun positions -> async {
        return positions |> List.map (fun position -> if position = Explorable.position start then renewedStart else target) |> Ok
    } }
    let actual = Explore.run explorer (Trail.beginAt start) (Explore.follow { Kind = EdgeKind.Contains; Target = Explorable.position target }) |> Async.RunSynchronously
    let expected = Ok(target, Trail.beginAt renewedStart |> Trail.follow { Kind = EdgeKind.Contains; Target = target })
    Assert.Equal(expected, actual)

[<Property>]
let ``the trail retains only its most recent budgeted steps and the source of that suffix`` (extra: uint16) =
    let count = Trail.retainedSteps + 1 + int extra % 100
    let targets = [1..count] |> List.map (fun index -> ExplorationTests.node $"Person:{index}" "root")
    let steps = targets |> List.map (fun target -> { Kind = EdgeKind.Contains; Target = target })
    let actual = steps |> List.fold (fun trail step -> Trail.follow step trail) trail |> Trail.walked
    let expected = (ExplorationTests.start :: targets) |> List.skip (count - Trail.retainedSteps)
    Assert.Equal<Explorable list>(expected, actual)

let private run<'a> (walk: Explore<'a>) : Result<'a * Trail, Failure> = Explore.run ExplorationTests.explorer trail walk |> Async.RunSynchronously
let private link: Link = { Kind = EdgeKind.Contains; Target = Explorable.position ExplorationTests.start }
let private trail: Trail = ExplorationTests.trail
