#r "../../../../../client-fsharp/Core/bin/Debug/net10.0/BibleAtlas.FSharp.Core.dll"

open BibleAtlas.FSharp.Domain

let singleVerse (verse: TextUnit<uint16, int, byte>) : AccountRun<uint16, int, byte, unit> =
    AccountRun.Verse verse

let multipleVerses (passage: Passage<uint16, int, unit>) : AccountRun<uint16, int, byte, unit> =
    AccountRun.Passage passage

let admittedRun : HistoricalAccount<uint16, int, byte, unit> -> AccountRun<uint16, int, byte, unit> =
    History.accountRun

let admit : (AccountRun<uint16, int, byte, unit> -> bool) -> AccountRun<uint16, int, byte, unit> -> Result<HistoricalAccount<uint16, int, byte, unit>, HistoricalAccountFailure> =
    History.admitAccount

let eventAccounts (context: EventContext<uint16, int, byte, unit, string>) : HistoricalAccount<uint16, int, byte, unit> list =
    context |> History.context |> History.accounts

let resolvedEvent (context: EventContext<uint16, int, byte, unit, string>) : ResolvedValue<uint16, byte, int, byte, unit, unit, unit, string, int> =
    ResolvedValue.Event context

printfn "Both account-run shapes, the shared admission signature, total projection and event-resolution propagation compile; no pending operation invoked."
