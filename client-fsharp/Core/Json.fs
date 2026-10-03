namespace BibleAtlas.FSharp

module Json =
    let encode value = JsonAdapter.Encode value

    let decode<'a> body : Result<'a, Failure> =
        JsonAdapter.Decode<'a> body |> Result.mapError (WireFailure.render >> Contract)
