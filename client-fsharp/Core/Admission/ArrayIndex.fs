namespace BibleAtlas.FSharp.Admission

[<RequireQualifiedAccess>]
type ArrayIndexFailure = NegativeIndex

type ArrayIndex = private ArrayIndex of int

module ArrayIndices =
    let admit position =
        if position >= 0 then Ok (ArrayIndex position)
        else Error ArrayIndexFailure.NegativeIndex
    let value (ArrayIndex position) = position
