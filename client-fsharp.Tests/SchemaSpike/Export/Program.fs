module BibleAtlas.FSharp.Tests.SchemaSpike.Export.Program

open BibleAtlas.FSharp.Tests.SchemaSpike

[<EntryPoint>]
let main arguments =
    match arguments with
    | [|root; output|] -> SurveyInputs.export root output; 0
    | _ -> eprintfn "usage: schema-survey-export <repository> <output.json>"; 1
