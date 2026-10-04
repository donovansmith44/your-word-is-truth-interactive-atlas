module rec BibleAtlas.FSharp.Tests.SchemaSpike.SchemaLaws

open System.Text.Json
open System.Text.Json.Nodes
open FsCheck
open FsCheck.Xunit
open global.Xunit
open Json.Schema
open BibleAtlas.FSharp.Tests.SchemaSpike.SchemaSurvey

[<Property(MaxTest = 1)>]
let ``the original approved OpenAPI builds and accepts every recorded query answer without schema edits`` () =
    let actual = fixtures () |> List.map (fun (shape, name, body) -> shape, name, (schema shape).Evaluate(body).IsValid)
    Assert.Equal<(string * string * bool) list>(fixtures () |> List.map (fun (shape, name, _) -> shape, name, true), actual)

[<Property>]
let ``generated valid roots and invalid root shapes are distinguished by the original pattern`` (suffix: uint16) =
    let valid = JsonSerializer.SerializeToElement(suffix.ToString("x32"))
    let invalid = JsonSerializer.SerializeToElement($"root-{suffix}")
    Assert.Equal((true, false), ((schema "ArtifactRoot").Evaluate(valid).IsValid, (schema "ArtifactRoot").Evaluate(invalid).IsValid))

[<Property>]
let ``both published element identity branches accept exactly their declared opaque wire shapes`` (suffix: uint16) (edge: bool) =
    let address = suffix.ToString("x32")
    let valid = if edge then $"Contains:{address}" else $"Person:served-{suffix}"
    let invalid = $"untyped-{suffix}"
    Assert.Equal((true, false), ((schema "ElementId").Evaluate(JsonSerializer.SerializeToElement(valid)).IsValid, (schema "ElementId").Evaluate(JsonSerializer.SerializeToElement(invalid)).IsValid))

[<Property>]
let ``all required node fields reject omission and null and the full record accepts its generated label`` (suffix: uint16) (NonNull label: NonNull<string>) =
    let source = JsonSerializer.SerializeToNode {| id = $"Person:served-{suffix}"; kind = "Person"; label = label |}
    let components = (document ())["components"]
    let definitions = components["schemas"]
    let definition = definitions["NodeRef"]
    let fields = definition["required"]
    let required = fields.AsArray() |> Seq.map (fun field -> field.GetValue<string>()) |> Seq.toList
    let actual = required |> List.map (fun field ->
        let missing, explicitNull = source.DeepClone().AsObject(), source.DeepClone().AsObject()
        missing.Remove field |> ignore
        explicitNull[field] <- null
        field, (schema "NodeRef").Evaluate(element missing).IsValid, (schema "NodeRef").Evaluate(element explicitNull).IsValid)
    Assert.Equal((true, required |> List.map (fun field -> field, false, false)), ((schema "NodeRef").Evaluate(element source).IsValid, actual))

[<Property>]
let ``closed records reject extra properties through the original additionalProperties keyword`` (suffix: uint16) =
    let body = JsonSerializer.SerializeToElement {| id = $"Person:served-{suffix}"; kind = "Person"; label = "served"; extra = suffix |}
    Assert.Equal(false, (schema "NodeRef").Evaluate(body).IsValid)

[<Property>]
let ``structured evaluation retains the exact failing instance location and keyword without reading message text`` (suffix: uint16) =
    let body = JsonSerializer.SerializeToElement {| id = $"untyped-{suffix}"; kind = "Person"; label = "served" |}
    let answer = (schema "NodeRef").Evaluate(body, EvaluationOptions(OutputFormat = OutputFormat.List))
    let actual = answer.Details |> Seq.filter (fun detail -> not detail.IsValid && not (isNull detail.Errors) && detail.Errors.Count > 0) |> Seq.collect (fun detail -> detail.Errors.Keys |> Seq.map (fun keyword -> detail.InstanceLocation.ToString(), keyword)) |> Seq.toList
    let expected = false, [("", "properties"); ("/id", "pattern")]
    Assert.True((expected = (answer.IsValid, actual)), sprintf "Expected %A; actual %A" expected (answer.IsValid, actual))

[<Property>]
let ``nullable heading and every whole TextWindow fixture remain valid under generated artifact roots`` (suffix: uint16) =
    let examples = fixtures () |> List.filter (fun (shape, _, _) -> shape = "TextWindow")
    let actual = examples |> List.map (fun (_, name, body) ->
        let changed = JsonNode.Parse(body.GetRawText())
        changed["version"] <- JsonValue.Create(suffix.ToString("x32"))
        for unit in changed["units"].AsArray() do unit["heading"] <- null
        name, (schema "TextWindow").Evaluate(element changed).IsValid)
    Assert.NotEmpty(examples)
    Assert.Equal<(string * bool) list>(examples |> List.map (fun (_, name, _) -> name, true), actual)

[<Property>]
let ``base schema acceptance cannot replace the concrete branch validation performed by the generated serializer`` (suffix: uint16) =
    let incomplete = JsonSerializer.SerializeToElement {| corpus = "bible"; probe = suffix |}
    let actual = (schema "TextRef").Evaluate(incomplete).IsValid, (schema "BibleRef").Evaluate(incomplete).IsValid, BibleAtlas.FSharp.Json.decode<BibleAtlas.FSharp.Contract.TextRef>(incomplete.GetRawText()) |> Result.isOk
    Assert.Equal((true, false, false), actual)

let private fixtures () : (string * string * JsonElement) list =
    SchemaSurvey.examples () |> List.map (fun example -> example.SchemaName, example.FileName, example.Body)

let private element (node: JsonNode) : JsonElement = JsonSerializer.Deserialize<JsonElement>(node.ToJsonString())
