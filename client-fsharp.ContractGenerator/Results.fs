namespace BibleAtlas.FSharp.ContractGenerator

type internal ResultBuilder() =
    member _.Bind(value, next) = Result.bind next value
    member _.Return(value) = Ok value
    member _.ReturnFrom(value: Result<_, _>) = value

[<AutoOpen>]
module internal Results =
    let result = ResultBuilder()
