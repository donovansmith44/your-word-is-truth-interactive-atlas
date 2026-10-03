namespace BibleAtlas.FSharp.Domain

type RootMismatch<'root> =
    private RootMismatch of expected: 'root * observed: 'root
    with

    member mismatch.Expected : 'root = match mismatch with RootMismatch (expected, _) -> expected
    member mismatch.Observed : 'root = match mismatch with RootMismatch (_, observed) -> observed

type Rooted<'root, 'value> = private Rooted of root: 'root * value: 'value

module rec Rooted =
    let map2 (mapping: 'a -> 'b -> 'c) (Rooted (leftRoot, left): Rooted<'root, 'a>) (Rooted (rightRoot, right): Rooted<'root, 'b>) : Result<Rooted<'root, 'c>, RootMismatch<'root>> =
        admit leftRoot rightRoot right
        |> Result.map (fun (Rooted (_, admittedRight)) -> Rooted (leftRoot, mapping left admittedRight))

    let admit<'root, 'value when 'root: equality> (expected: 'root) (observed: 'root) (value: 'value) : Result<Rooted<'root, 'value>, RootMismatch<'root>> =
        if expected = observed then Ok (Rooted (observed, value))
        else Error (RootMismatch (expected, observed))

    let map (mapping: 'a -> 'b) (Rooted (root, value): Rooted<'root, 'a>) : Rooted<'root, 'b> = Rooted (root, mapping value)

    let root (Rooted (root, _): Rooted<'root, 'value>) : 'root = root

    let value (Rooted (_, value): Rooted<'root, 'value>) : 'value = value
