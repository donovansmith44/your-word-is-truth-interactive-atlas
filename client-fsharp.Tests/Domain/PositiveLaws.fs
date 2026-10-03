module BibleAtlas.FSharp.Tests.Domain.PositiveLaws

open FsCheck.Xunit
open BibleAtlas.FSharp.Domain

[<Property>]
let ``positive admission accepts exactly positive integers and preserves their value`` (number: int) =
    let expected = if number > 0 then Ok number else Error PositiveFailure.NonPositive
    let actual = Positive.admit number |> Result.map Positive.value
    actual = expected

[<Property>]
let ``positive admission refuses zero and every generated negative integer`` (number: uint32) =
    let candidate = -(int (number % uint32 System.Int32.MaxValue))
    Positive.admit candidate = Error PositiveFailure.NonPositive
