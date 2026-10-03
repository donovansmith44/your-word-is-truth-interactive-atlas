namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract

module WireDecoder =
    let sourceEntry body : Result<SourceEntry, WireFailure> =
        JsonAdapter.Decode<SourceEntry> body
