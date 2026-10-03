module BibleAtlas.FSharp.Tests.Domain.DocumentPositionLaws

open FsCheck.Xunit
open BibleAtlas.FSharp.Admission

[<Property>]
let ``document lines are one based and preserve every valid value`` (number: int64) =
    let expected = if number > 0L then Ok number else Error LineNumberFailure.NonPositiveLine
    (LineNumbers.admit number |> Result.map LineNumbers.value) = expected

[<Property>]
let ``document byte columns are one based and preserve every valid value`` (number: int64) =
    let expected = if number > 0L then Ok number else Error ByteColumnFailure.NonPositiveColumn
    (ByteColumns.admit number |> Result.map ByteColumns.value) = expected
