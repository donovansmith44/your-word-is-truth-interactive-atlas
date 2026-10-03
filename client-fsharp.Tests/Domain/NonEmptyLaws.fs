module BibleAtlas.FSharp.Tests.Domain.NonEmptyLaws

open FsCheck.Xunit
open BibleAtlas.FSharp.Domain

[<Property>]
let ``nonempty admission refuses empty input and preserves every nonempty list`` (values: int list) =
    let expected = match values with [] -> Error NonEmptyFailure.EmptyCollection | _ -> Ok values
    let actual = NonEmpty.admit values |> Result.map NonEmpty.toList
    actual = expected

[<Property>]
let ``singleton has exactly its supplied element`` (value: int) =
    (NonEmpty.singleton value |> NonEmpty.toList) = [value]

[<Property>]
let ``append preserves both complete operands in order`` (left: int) (leftTail: int list) (right: int) (rightTail: int list) =
    let first = NonEmpty.create left leftTail
    let second = NonEmpty.create right rightTail
    (NonEmpty.append first second |> NonEmpty.toList) = (left :: leftTail) @ (right :: rightTail)

[<Property>]
let ``append is associative`` (first: int) (second: int) (third: int) (tail: int list) =
    let a = NonEmpty.singleton first
    let b = NonEmpty.singleton second
    let c = NonEmpty.create third tail
    NonEmpty.append (NonEmpty.append a b) c = NonEmpty.append a (NonEmpty.append b c)

[<Property>]
let ``map preserves identity and composition without empty output`` (head: int) (tail: int list) =
    let original = NonEmpty.create head tail
    let first = string
    let second = String.length
    NonEmpty.map id original = original
    && NonEmpty.map (first >> second) original = (original |> NonEmpty.map first |> NonEmpty.map second)
    && (NonEmpty.map first original |> NonEmpty.toList) = List.map first (head :: tail)

