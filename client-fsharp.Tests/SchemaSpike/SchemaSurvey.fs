module rec BibleAtlas.FSharp.Tests.SchemaSpike.SchemaSurvey

open System.IO
open System.Text.Json
open System.Text.Json.Nodes
open Json.Schema

let initialize (inputs: SurveyData.Inputs) =
    inputSets[repository ()] <- inputs
    schemas.Clear()

let schema (name: string) : JsonSchema = schemas.GetOrAdd(name, buildSchema)

let examples () : SurveyData.QueryExample list = (inputs ()).Examples

let private buildSchema (name: string) =
    let source = (document ()).DeepClone()
    source["$ref"] <- JsonValue.Create<string>($"#/components/schemas/{name}")
    let options = BuildOptions(Dialect = Dialect.Draft202012, SchemaRegistry = SchemaRegistry())
    JsonSchema.Build(JsonSerializer.Deserialize<JsonElement>(source.ToJsonString()), options)

let document () : JsonNode = (inputs ()).Document

let private inputs () : SurveyData.Inputs = inputSets.GetOrAdd(repository (), SurveyInputs.load)

let private repository () = Path.GetFullPath(Path.Combine(__SOURCE_DIRECTORY__, "../.."))

let private schemas: System.Collections.Concurrent.ConcurrentDictionary<string, JsonSchema> = System.Collections.Concurrent.ConcurrentDictionary<string, JsonSchema>()
let private inputSets: System.Collections.Concurrent.ConcurrentDictionary<string, SurveyData.Inputs> = System.Collections.Concurrent.ConcurrentDictionary<string, SurveyData.Inputs>()
