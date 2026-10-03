namespace BibleAtlas.FSharp.Domain

type RootMismatch<'root> =
    private RootMismatch of expected: 'root * observed: 'root
    with

    member mismatch.Expected = match mismatch with RootMismatch (expected, _) -> expected
    member mismatch.Observed = match mismatch with RootMismatch (_, observed) -> observed

type Rooted<'root, 'value> = private Rooted of root: 'root * value: 'value

module Rooted =
    let admit expected observed value : Result<Rooted<'root, 'value>, RootMismatch<'root>> =
        if expected = observed then Ok (Rooted (observed, value))
        else Error (RootMismatch (expected, observed))

    let map mapping (Rooted (root, value)) = Rooted (root, mapping value)

    let map2 mapping (Rooted (leftRoot, left)) (Rooted (rightRoot, right)) =
        admit leftRoot rightRoot right
        |> Result.map (fun (Rooted (_, admittedRight)) -> Rooted (leftRoot, mapping left admittedRight))

    let root (Rooted (root, _)) = root
    let value (Rooted (_, value)) = value
