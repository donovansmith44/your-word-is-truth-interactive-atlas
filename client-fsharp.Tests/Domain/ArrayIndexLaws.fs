module BibleAtlas.FSharp.Tests.Domain.ArrayIndexLaws

open FsCheck.Xunit
open BibleAtlas.FSharp.Admission

[<Property>]
let ``array index admission accepts exactly nonnegative integers`` (number: int) =
    let expected = if number >= 0 then Ok number else Error ArrayIndexFailure.NegativeIndex
    (ArrayIndices.admit number |> Result.map ArrayIndices.value) = expected
