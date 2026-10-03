#r "../../../../../client-fsharp/Core/bin/Debug/net10.0/BibleAtlas.FSharp.Core.dll"

open BibleAtlas.FSharp.Domain

let bypass (run: AccountRun<uint16, int, byte, unit>) : HistoricalAccount<uint16, int, byte, unit> =
    HistoricalAccount run
