module BibleAtlas.FSharp.Tests.StateTests

open Xunit
open BibleAtlas.FSharp

[<Fact>]
let ``an obsolete completion leaves the pending newer request unchanged`` () =
    let first = RequestId.initial
    let next = RequestId.next first
    let pending = Ready "old" |> LoadState.beginRead first |> LoadState.beginRead next
    Assert.Equal(Loading(next, Some "old"), LoadState.complete first (Ok "stale") pending)

[<Fact>]
let ``a failed read preserves the last successful value and permits a fresh retry`` () =
    let first = RequestId.initial
    let failure = Transport "offline"
    let failed = Ready "old" |> LoadState.beginRead first |> LoadState.complete first (Error failure)
    let next = RequestId.next first
    Assert.Equal(Failed(first, failure, Some "old"), failed)
    Assert.Equal(Ready "new", failed |> LoadState.beginRead next |> LoadState.complete next (Ok "new"))

[<Fact>]
let ``a completion after leaving a view cannot reopen its state`` () =
    Assert.Equal(Empty, LoadState.complete RequestId.initial (Ok "stale") Empty)

[<Fact>]
let ``the element cache retains only its capacity after a future sized walk`` () =
    let capacity = 32
    let cache = [1..100000] |> List.fold (fun cache key -> Cache.put key key cache) (Cache.empty capacity)
    Assert.Equal((capacity, None, Some 100000), (Cache.count cache, Cache.find 1 cache |> fst, Cache.find 100000 cache |> fst))

[<Fact>]
let ``access promotes an entry and replacement does not consume another slot`` () =
    let cache = Cache.empty 2 |> Cache.put "a" 1 |> Cache.put "b" 2
    let found, cache = Cache.find "a" cache
    let cache = cache |> Cache.put "a" 3 |> Cache.put "c" 4
    Assert.Equal((Some 1, 2, None, Some 3, Some 4), (found, Cache.count cache, Cache.find "b" cache |> fst, Cache.find "a" cache |> fst, Cache.find "c" cache |> fst))

[<Fact>]
let ``a zero capacity cache holds no entries`` () =
    let cache = Cache.empty 0 |> Cache.put "a" 1
    Assert.Equal((0, None), (Cache.count cache, Cache.find "a" cache |> fst))
