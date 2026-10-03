module BibleAtlas.FSharp.Tests.Domain.RootedLaws

open FsCheck.Xunit
open BibleAtlas.FSharp.Domain

[<Property>]
let ``root admission accepts exactly the requested data version and preserves the whole value`` (expected: uint16) (observed: uint16) (values: int list) =
    let actual =
        Rooted.admit expected observed values
        |> Result.map (fun admitted -> {| Root = Rooted.root admitted; Value = Rooted.value admitted |})
        |> Result.mapError (fun mismatch -> {| Expected = mismatch.Expected; Observed = mismatch.Observed |})
    let wanted =
        if expected = observed then Ok {| Root = observed; Value = values |}
        else Error {| Expected = expected; Observed = observed |}
    actual = wanted

[<Property>]
let ``every generated foreign root is refused with both identities intact`` (expected: uint16) (value: int) =
    let observed = expected + 1us
    let actual =
        Rooted.admit expected observed value
        |> Result.mapError (fun mismatch -> {| Expected = mismatch.Expected; Observed = mismatch.Observed |})
    actual = Error {| Expected = expected; Observed = observed |}

[<Property>]
let ``mapping preserves both the admitted root and the complete transformed value`` (root: uint16) (values: int list) =
    let actual =
        Rooted.admit root root values
        |> Result.map (Rooted.map (List.map string))
        |> Result.map (fun admitted -> {| Root = Rooted.root admitted; Value = Rooted.value admitted |})
        |> Result.mapError (fun mismatch -> {| Expected = mismatch.Expected; Observed = mismatch.Observed |})
    actual = Ok {| Root = root; Value = List.map string values |}

[<Property>]
let ``rooted mapping preserves identity and composition`` (root: uint16) (values: int list) =
    let admitted = Rooted.admit root root values
    let first = List.map string
    let second = List.map String.length
    (admitted |> Result.map (Rooted.map id)) = admitted
    && (admitted |> Result.map (Rooted.map (first >> second)))
       = (admitted |> Result.map (Rooted.map first >> Rooted.map second))

[<Property>]
let ``combining values is admitted only on one coherent root`` (leftRoot: uint16) (rightRoot: uint16) (left: int list) (right: int list) =
    let actual =
        Rooted.admit leftRoot leftRoot left
        |> Result.bind (fun first ->
            Rooted.admit rightRoot rightRoot right
            |> Result.bind (Rooted.map2 List.append first))
        |> Result.map (fun admitted -> {| Root = Rooted.root admitted; Value = Rooted.value admitted |})
        |> Result.mapError (fun mismatch -> {| Expected = mismatch.Expected; Observed = mismatch.Observed |})
    let expected =
        if leftRoot = rightRoot then Ok {| Root = leftRoot; Value = left @ right |}
        else Error {| Expected = leftRoot; Observed = rightRoot |}
    actual = expected

[<Property>]
let ``same root combination is associative and preserves the complete ordered payload`` (root: uint16) (first: int list) (second: int list) (third: int list) =
    let a = Rooted.admit root root first
    let b = Rooted.admit root root second
    let c = Rooted.admit root root third
    let left = a |> Result.bind (fun x -> b |> Result.bind (Rooted.map2 List.append x)) |> Result.bind (fun xy -> c |> Result.bind (Rooted.map2 List.append xy))
    let right = b |> Result.bind (fun y -> c |> Result.bind (Rooted.map2 List.append y)) |> Result.bind (fun yz -> a |> Result.bind (fun x -> Rooted.map2 List.append x yz))
    let project admitted = {| Root = Rooted.root admitted; Value = Rooted.value admitted |}
    left = right && (left |> Result.map project) = Ok {| Root = root; Value = first @ second @ third |}
