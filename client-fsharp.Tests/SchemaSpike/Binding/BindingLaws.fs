module rec BibleAtlas.FSharp.Tests.SchemaSpike.Binding.BindingLaws

open System.Text.Json
open System.Text.Json.Nodes
open System.Text.Json.Serialization
open FsCheck.Xunit
open Xunit
open Json.Schema
open Json.Schema.Serialization
open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Tests.SchemaSpike

[<Property(MaxTest = 1)>]
let ``adoption requires a complete schema binding including components whose FSharp aliases erase at runtime`` () =
    let converter = (options ()).Converters[0]
    let names = definitions () |> Seq.map _.Key |> Seq.sort |> Seq.toList
    Assert.Equal<string list>(names, Map.keys SchemaTypes.all |> Seq.toList)
    let actual = names |> List.map (fun name ->
        let shape = Map.tryFind name SchemaTypes.all
        name, shape.IsSome, shape |> Option.exists converter.CanConvert)
    let expected = names |> List.map (fun name -> name, true, true)
    Assert.True((expected = actual), sprintf "Expected %s; actual %s" (JsonSerializer.Serialize expected) (JsonSerializer.Serialize actual))

[<Property(MaxTest = 1)>]
let ``the original schema bound validating converter accepts every complete recorded query fixture`` () =
    let fixtures = SchemaSurvey.examples ()
    let actual = fixtures |> List.map (fun example -> example.SchemaName, example.FileName, accepts example.SchemaName example.Body)
    Assert.Equal<(string * string * bool) list>(fixtures |> List.map (fun example -> example.SchemaName, example.FileName, true), actual)

[<Property>]
let ``the schema bound converter refuses opaque identity patterns and closed leaf record extras`` (suffix: uint16) =
    let answers =
        [ "ArtifactRoot", JsonSerializer.SerializeToElement($"root-{suffix}")
          "NodeRef", JsonSerializer.SerializeToElement {| id = $"untyped-{suffix}"; kind = "Person"; label = "served" |}
          "NodeRef", JsonSerializer.SerializeToElement {| id = $"Person:served-{suffix}"; kind = "Person"; label = "served"; extra = suffix |} ]
    let actual = answers |> List.map (fun (name, body) -> name, accepts name body)
    Assert.Equal<(string * bool) list>(["ArtifactRoot", false; "NodeRef", false; "NodeRef", false], actual)

[<Property>]
let ``the original Point schema enforces exactly two numeric coordinates despite its erased runtime alias`` (suffix: uint16) (length: byte) =
    let point = JsonSerializer.SerializeToElement(Array.init (int length) (fun _ -> float suffix))
    let expected = int length = coordinatePairLength
    Assert.Equal(expected, accepts "Point" point)

[<Property>]
let ``the existing converter reports the whole original node identity failure hierarchy through structured exception data`` (suffix: uint16) =
    let body = JsonSerializer.SerializeToElement {| id = $"untyped-{suffix}"; kind = "Person"; label = "served" |}
    let actual = refusal "NodeRef" body
    let expected = Some (false, [("", "properties"); ("/id", "pattern")])
    Assert.Equal(expected, actual)

[<Property>]
let ``adoption requires selected union payload identities and numeric bounds to remain enforced across every discriminator family`` (suffix: uint16) =
    let invalidBook = JsonSerializer.SerializeToElement {| corpus = "bible"; book = $"untyped-{suffix}"; chapter = 1; verse = 1 |}
    let invalidChapter = JsonSerializer.SerializeToElement {| corpus = "bible"; book = "JHN"; chapter = -1; verse = 1 |}
    let answers =
        [ "BibleRef", invalidBook
          "TextRef", invalidBook
          "UnitText", unitText invalidBook
          "BibleRef", invalidChapter
          "TextRef", invalidChapter
          "UnitText", unitText invalidChapter
          "PositionRef", JsonSerializer.SerializeToElement {| position = "node"; node = {| id = $"untyped-{suffix}"; kind = "Person"; label = "served" |} |}
          "Element", invalidElementId suffix ]
    let expected = answers |> List.map (fun (name, _) -> name, false)
    let actual = answers |> List.map (fun (name, body) -> name, accepts name body)
    Assert.True((expected = actual), sprintf "Expected %s; actual %s" (JsonSerializer.Serialize expected) (JsonSerializer.Serialize actual))

let private unitText (locus: JsonElement) : JsonElement =
    JsonSerializer.SerializeToElement {| locus = locus; text = "served"; anchors = ([||]: obj array); words_of_christ = ([||]: obj array) |}

let private invalidElementId (suffix: uint16) : JsonElement =
    let example = SchemaSurvey.examples () |> List.find (fun example -> example.SchemaName = "ElementPage")
    let page = JsonNode.Parse(example.Body.GetRawText())
    let elements = page["elements"].AsArray()
    let source = elements |> Seq.find (fun element -> element["element"].GetValue<string>() = "node")
    let changed = source.DeepClone()
    let node = changed["node"]
    node["id"] <- JsonValue.Create($"untyped-{suffix}")
    JsonSerializer.Deserialize<JsonElement>(changed.ToJsonString())

let private definitions () : JsonObject =
    let components = (SchemaSurvey.document ())["components"]
    components["schemas"].AsObject()

let private accepts (name: string) (body: JsonElement) =
    try
        JsonSerializer.Deserialize(body.GetRawText(), SchemaTypes.all[name], options ()) |> ignore
        true
    with :? JsonException -> false

let private refusal (name: string) (body: JsonElement) =
    try
        JsonSerializer.Deserialize(body.GetRawText(), SchemaTypes.all[name], options ()) |> ignore
        None
    with :? JsonException as error ->
        match error.Data["validation"] with
        | :? EvaluationResults as result ->
            let errors = result.Details |> Seq.filter (fun detail -> not detail.IsValid && not (isNull detail.Errors) && detail.Errors.Count > 0) |> Seq.collect (fun detail -> detail.Errors.Keys |> Seq.map (fun keyword -> detail.InstanceLocation.ToString(), keyword)) |> Seq.toList
            Some (result.IsValid, errors)
        | _ -> None

let private options () : JsonSerializerOptions = configurations.GetOrAdd(typeof<ArtifactRoot>.Assembly, fun _ -> configure ())

let private configure () : JsonSerializerOptions =
    let register = typeof<ValidatingJsonConverter>.GetMethod("MapType")
    for name, shape in Map.toList SchemaTypes.all do
        register.MakeGenericMethod(shape).Invoke(null, [|SchemaSurvey.schema name|]) |> ignore
    let shapes = JsonFSharpTypes.Records ||| JsonFSharpTypes.Collections ||| JsonFSharpTypes.OptionalTypes ||| JsonFSharpTypes.Tuples
    let result = JsonFSharpOptions.Default().WithTypes(shapes).WithAllowOverride().WithSkippableOptionFields(SkippableOptionFields.Always, deserializeNullAsNone = true).ToJsonSerializerOptions()
    result.Converters.Insert(0, ValidatingJsonConverter(EvaluationOptions = EvaluationOptions(OutputFormat = OutputFormat.List)))
    result

let private configurations: System.Collections.Concurrent.ConcurrentDictionary<System.Reflection.Assembly, JsonSerializerOptions> = System.Collections.Concurrent.ConcurrentDictionary<System.Reflection.Assembly, JsonSerializerOptions>()

let private coordinatePairLength = 2
