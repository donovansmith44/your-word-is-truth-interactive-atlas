namespace BibleAtlas.FSharp.Domain

open FSharpPlus.Data

[<RequireQualifiedAccess>]
type NonEmptyFailure = EmptyCollection

type NonEmpty<'a> = private NonEmpty of NonEmptyList<'a>

module NonEmpty =
    let admit (values: 'a list) : Result<NonEmpty<'a>, NonEmptyFailure> =
        match NonEmptyList.tryOfList values with
        | Some nonempty -> Ok (NonEmpty nonempty)
        | None -> Error NonEmptyFailure.EmptyCollection
    let singleton (value: 'a) : NonEmpty<'a> = NonEmptyList.singleton value |> NonEmpty
    let create (head: 'a) (tail: 'a list) : NonEmpty<'a> = NonEmptyList.create head tail |> NonEmpty
    let append (NonEmpty left: NonEmpty<'a>) (NonEmpty right: NonEmpty<'a>) : NonEmpty<'a> = NonEmptyList.append left right |> NonEmpty
    let map (mapping: 'a -> 'b) (NonEmpty values: NonEmpty<'a>) : NonEmpty<'b> = NonEmptyList.map mapping values |> NonEmpty
    let toList (NonEmpty values: NonEmpty<'a>) : 'a list = NonEmptyList.toList values
