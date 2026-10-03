#r "../../../../../client-fsharp/Core/bin/Debug/net10.0/BibleAtlas.FSharp.Core.dll"

let shadow (reference: int) : BibleAtlas.FSharp.Domain.UnitReference<int, byte> =
    BibleAtlas.FSharp.Domain.UnitReference.Bible reference

printfn "A handwritten domain reference union remains constructible."
