module rec BibleAtlas.FSharp.Tests.SchemaSpike.SurveyInputs

open System.IO
open System.Text.Json
open System.Text.Json.Nodes
open YamlDotNet.Serialization
open BibleAtlas.FSharp.Tests.SchemaSpike.SurveyData

let export (root: string) (output: string) =
    let inputs = load root
    Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath output)) |> ignore
    File.WriteAllText(output, JsonSerializer.Serialize inputs)

let load (root: string) : Inputs =
    { Document = document root; Examples = examples root }

let private document (root: string) : JsonNode =
    let deserialize = DeserializerBuilder().WithAttemptingUnquotedStringTypeDeserialization().Build()
    let serialize = SerializerBuilder().JsonCompatible().Build()
    let body = File.ReadAllText(Path.Combine(root, "contracts/openapi.yaml")) |> deserialize.Deserialize<obj> |> serialize.Serialize
    JsonNode.Parse body

let private examples (root: string) : QueryExample list =
    Directory.GetFiles(Path.Combine(root, "contracts/atlas-query-contract/fixtures"), "*.json") |> Array.sort |> Array.toList |> List.choose (fun file ->
        let name = Path.GetFileNameWithoutExtension file
        let source = JsonNode.Parse(File.ReadAllText file)
        let shape =
            if name = "index" || name = "contract" then None
            elif source["status"].GetValue<int>() >= clientRefusalMinimum then Some "ErrorBody"
            elif name.StartsWith "focus-" then Some "NodeRecord"
            elif name.StartsWith "contents-" then Some "Contents"
            elif name.StartsWith "text-window-" then Some "TextWindow"
            elif name.StartsWith "traversal-" then Some "EdgePage"
            elif name.StartsWith "scene-" then Some "Scene"
            elif name.StartsWith "event-page-" then Some "EventPage"
            elif name = "element-read" then Some "ElementPage"
            else invalidOp $"Unclassified query fixture {name}"
        shape |> Option.map (fun shape -> { SchemaName = shape; FileName = name; Body = JsonSerializer.Deserialize<JsonElement>(source["body"].ToJsonString()) }))

let private clientRefusalMinimum = 400
