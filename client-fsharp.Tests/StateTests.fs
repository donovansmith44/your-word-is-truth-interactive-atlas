module rec BibleAtlas.FSharp.Tests.StateTests

open Xunit
open FsCheck
open FsCheck.Xunit
open BibleAtlas.FSharp

[<Property>]
let ``every obsolete completion leaves the newer pending state unchanged`` (old: int) (stale: int) (steps: byte) =
    let first = requestAfter steps
    let next = RequestId.next first
    let pending = Ready old |> LoadState.beginRead first |> LoadState.beginRead next
    Assert.Equal(Loading(next, Some old), LoadState.complete first (Ok stale) pending)

[<Property>]
let ``every failed read preserves the entire last value and permits a fresh retry`` (old: int list) (replacement: int list) (NonNull reason: NonNull<string>) (steps: byte) =
    let first = requestAfter steps
    let failure = Transport reason
    let failed = Ready old |> LoadState.beginRead first |> LoadState.complete first (Error failure)
    let next = RequestId.next first
    Assert.Equal(Failed(first, failure, Some old), failed)
    Assert.Equal(Ready replacement, failed |> LoadState.beginRead next |> LoadState.complete next (Ok replacement))

[<Property>]
let ``a completion after leaving a view cannot reopen any old value`` (stale: int list) (steps: byte) =
    Assert.Equal(Empty, LoadState.complete (requestAfter steps) (Ok stale) Empty)

[<Property>]
let ``beginning a new request preserves the last complete value through every pending state`` (old: int list option) (failed: bool) (steps: byte) =
    let first = requestAfter steps
    let next = RequestId.next first
    let prior = if failed then Failed(first, Transport "prior failure", old) else Loading(first, old)
    Assert.Equal(Loading(next, old), LoadState.beginRead next prior)

let private requestAfter steps = [1 .. int steps] |> List.fold (fun identity _ -> RequestId.next identity) RequestId.initial
