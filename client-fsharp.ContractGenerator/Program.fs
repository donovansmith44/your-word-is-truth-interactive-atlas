module BibleAtlas.FSharp.ContractGenerator.Program

open System.IO

[<EntryPoint>]
let main arguments =
    match arguments with
    | [|document; output|] ->
        match Generator.generateFile document output with
        | Ok () -> 0
        | Error error -> eprintfn "%s" error; 1
    | [|document; output; vocabulary; names|] ->
        match Generator.generateFile document output |> Result.bind (fun () -> Vocabulary.generate (File.ReadAllText vocabulary)) with
        | Ok generated -> File.WriteAllText(names, generated); 0
        | Error error -> eprintfn "%s" error; 1
    | _ -> eprintfn "usage: contract-generator <openapi.yaml> <Wire.g.fs>"; 1
