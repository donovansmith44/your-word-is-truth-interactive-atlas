module rec BibleAtlas.FSharp.Tests.ContractShapeTests

open System
open System.Text.Json.Nodes
open System.Text.Json.Serialization
open Microsoft.FSharp.Reflection
open Xunit
open FsCheck
open FsCheck.Xunit
open YamlDotNet.RepresentationModel
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Property>]
let ``every generated optional field accepts null as exactly None`` (NonNull text) (number: int) =
    for shape in records do
        let value = sample text number shape
        let fields = FSharpType.GetRecordFields shape
        for index in 0 .. fields.Length - 1 do
            let field = fields[index]
            if isOption field.PropertyType then
                let body = JsonNode.Parse(Json.encode value).AsObject()
                body[wireName field] <- null
                let expected = FSharpValue.GetRecordFields value
                let none = FSharpType.GetUnionCases(field.PropertyType) |> Array.find (fun case -> case.Name = "None")
                expected[index] <- FSharpValue.MakeUnion(none, [||])
                let expected = FSharpValue.MakeRecord(shape, expected)
                match decode shape (body.ToJsonString()) with
                | Ok actual -> Assert.Equal(expected, actual)
                | Error failure -> Assert.Fail($"{shape.Name}.{field.Name}: {failure}")

[<Property>]
let ``every generated required field rejects omission and null at the one JSON door`` (NonNull text) (number: int) =
    for shape in records do
        let value = sample text number shape
        Assert.Equal(Ok value, decode shape (Json.encode value))
        for field in FSharpType.GetRecordFields shape do
            if not (isOption field.PropertyType) then
                let missing = JsonNode.Parse(Json.encode value).AsObject()
                missing.Remove(wireName field) |> ignore
                let explicitNull = JsonNode.Parse(Json.encode value).AsObject()
                explicitNull[wireName field] <- null
                let actual = [decode shape (missing.ToJsonString()); decode shape (explicitNull.ToJsonString())] |> List.map Result.isError
                Assert.Equal<bool list>([true; true], actual)

[<Property>]
let ``every published scalar identity has its own private constructor and preserves its wire value`` (NonNull text: NonNull<string>) =
    let body = Json.encode text
    for name in scalarIdentities do
        let shape = typeof<NodeRecord>.Assembly.GetType($"BibleAtlas.FSharp.Contract.{name}", true)
        let case = FSharpType.GetUnionCases(shape, true) |> Array.exactlyOne
        let constructor = FSharpValue.PreComputeUnionConstructorInfo(case, true)
        let actual =
            decode shape body
            |> Result.map (fun value -> shape.IsValueType, case.Name, case.GetFields() |> Array.map _.PropertyType |> Array.toList, constructor.IsPublic, Json.encode value)
        Assert.Equal(Ok(true, name, [typeof<string>], false, body), actual)
        let invalid = ["null"; "{}"; "[]"; "true"; "42"] |> List.map (decode shape >> Result.isError)
        Assert.Equal<bool list>([true; true; true; true; true], invalid)

let rec sample (text: string) (number: int) (shape: Type) : obj =
    if shape = typeof<string> then box text
    elif shape = typeof<int> then box number
    elif shape = typeof<int64> then box (int64 number)
    elif shape = typeof<float> then box (float number)
    elif shape = typeof<bool> then box true
    elif isOption shape then
        let some = FSharpType.GetUnionCases(shape) |> Array.find (fun case -> case.Name = "Some")
        FSharpValue.MakeUnion(some, [|sample text number (shape.GetGenericArguments()[0])|])
    elif shape.IsGenericType && shape.GetGenericTypeDefinition() = typedefof<list<_>> then
        let empty = FSharpType.GetUnionCases(shape) |> Array.find (fun case -> case.GetFields().Length = 0)
        FSharpValue.MakeUnion(empty, [||])
    elif FSharpType.IsRecord shape then FSharpValue.MakeRecord(shape, FSharpType.GetRecordFields(shape) |> Array.map (fun field -> sample text number field.PropertyType))
    elif FSharpType.IsUnion(shape, true) then
        let case = FSharpType.GetUnionCases(shape, true) |> Array.head
        FSharpValue.MakeUnion(case, case.GetFields() |> Array.map (fun field -> sample text number field.PropertyType), true)
    else failwith $"unsupported generated contract shape {shape}"

let isOption (shape: Type) : bool = shape.IsGenericType && shape.GetGenericTypeDefinition() = typedefof<option<_>>

let decode (shape: Type) (body: string) : Result<obj, Failure> =
    let result = decodeMethod.MakeGenericMethod(shape).Invoke(null, [|box body|])
    let case, fields = FSharpValue.GetUnionFields(result, result.GetType())
    if case.Name = "Ok" then Ok fields[0] else Error(unbox<Failure> fields[0])
let decodeMethod: Reflection.MethodInfo = typeof<NodeRecord>.Assembly.GetType("BibleAtlas.FSharp.Json").GetMethod("decode")

let wireName (field: Reflection.PropertyInfo) : string = field.GetCustomAttributes(typeof<JsonPropertyNameAttribute>, false) |> Array.exactlyOne |> unbox<JsonPropertyNameAttribute> |> _.Name

let records = typeof<NodeRecord>.Assembly.GetTypes() |> Array.filter (fun shape -> shape.Namespace = typeof<NodeRecord>.Namespace && FSharpType.IsRecord shape) |> Array.sortBy _.Name

let scalarIdentities: string list =
    let yaml = YamlStream()
    use reader = new IO.StringReader(IO.File.ReadAllText(IO.Path.Combine(__SOURCE_DIRECTORY__, "../contracts/openapi.yaml")))
    yaml.Load reader
    let root = yaml.Documents[0].RootNode :?> YamlMappingNode
    let components = root.Children[YamlScalarNode "components"] :?> YamlMappingNode
    let schemas = components.Children[YamlScalarNode "schemas"] :?> YamlMappingNode
    schemas.Children
    |> Seq.choose (fun entry ->
        let schema = entry.Value :?> YamlMappingNode
        match schema.Children.TryGetValue(YamlScalarNode "type"), schema.Children.ContainsKey(YamlScalarNode "enum") with
        | (true, (:? YamlScalarNode as kind)), false when kind.Value = "string" -> Some((entry.Key :?> YamlScalarNode).Value)
        | _ -> None)
    |> Seq.toList
