namespace BibleAtlas.FSharp.Domain

open FSharpPlus.Data

[<RequireQualifiedAccess>]
type NonEmptyFailure = EmptyCollection

type NonEmpty<'a> = private NonEmpty of NonEmptyList<'a>

module NonEmpty =
    let admit values =
        match NonEmptyList.tryOfList values with
        | Some nonempty -> Ok (NonEmpty nonempty)
        | None -> Error NonEmptyFailure.EmptyCollection
    let singleton value = NonEmptyList.singleton value |> NonEmpty
    let create head tail = NonEmptyList.create head tail |> NonEmpty
    let append (NonEmpty left) (NonEmpty right) = NonEmptyList.append left right |> NonEmpty
    let map mapping (NonEmpty values) = NonEmptyList.map mapping values |> NonEmpty
    let toList (NonEmpty values) = NonEmptyList.toList values
