namespace BibleAtlas.FSharp.ContractGenerator

open System.Text
open System.Text.Json

module Vocabulary =
    let generate (source: string) =
        try
            use document = JsonDocument.Parse(source)
            let text key (element: JsonElement) = element.GetProperty(key: string).GetString()
            let directed =
                [ for relation in document.RootElement.GetProperty("relations").EnumerateArray() do
                      let forward, inverse = text "forward" relation, text "inverse" relation
                      yield forward, inverse
                      yield inverse, forward ]
            let symmetric =
                [ for relation in document.RootElement.GetProperty("symmetric").EnumerateArray() do
                      let label = text "label" relation
                      yield label, label ]
            let cases = directed @ symmetric
            if cases |> List.map fst |> List.distinct |> List.length <> cases.Length then
                Error "the graph vocabulary repeats an edge kind"
            else
                let output = StringBuilder("// Generated from contracts/atlas-graph-contract/fixtures/graph-vocabulary.json.\nnamespace BibleAtlas.FSharp.Contract\n\nmodule EdgeKinds =\n    let dual kind =\n        match kind with\n")
                for kind, dual in cases do
                    output.AppendLine($"        | EdgeKind.{Generator.name kind} -> EdgeKind.{Generator.name dual}") |> ignore
                Ok(output.ToString().Replace("\r\n", "\n"))
        with error -> Error error.Message
