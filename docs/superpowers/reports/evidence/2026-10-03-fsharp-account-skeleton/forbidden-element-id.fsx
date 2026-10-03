#r "../../../../../client-fsharp/Core/bin/Debug/net10.0/BibleAtlas.FSharp.Core.dll"

let shadow (id: uint16) : BibleAtlas.FSharp.Domain.ElementId<uint16, byte> =
    BibleAtlas.FSharp.Domain.ElementId.Node id

printfn "A handwritten domain element identity remains constructible."
