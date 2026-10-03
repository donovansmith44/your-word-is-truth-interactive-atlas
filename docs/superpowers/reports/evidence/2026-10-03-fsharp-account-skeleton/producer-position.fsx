#r "../../../../../client-fsharp/Core/bin/Debug/net10.0/BibleAtlas.FSharp.Core.dll"

open BibleAtlas.FSharp.Domain

let elementPosition (identity: uint64) : Position<uint64, uint16> =
    Position.Element identity

let yearPosition (span: YearSpan<uint16>) : Position<uint64, uint16> =
    Position.Years span

printfn "Position takes the producer element identity parameter without reconstructing its wire cases."
