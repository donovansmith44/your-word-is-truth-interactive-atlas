namespace BibleAtlas.FSharp.Domain

[<RequireQualifiedAccess>]
type PositiveFailure = NonPositive

type Positive = private Positive of int

module Positive =
    let admit number =
        if number > 0 then Ok (Positive number)
        else Error PositiveFailure.NonPositive
    let value (Positive number) = number
