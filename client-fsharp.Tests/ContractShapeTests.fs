module BibleAtlas.FSharp.Tests.ContractShapeTests

open System
open System.Text.Json.Nodes
open System.Text.Json.Serialization
open Microsoft.FSharp.Reflection
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

let isOption (shape: Type) = shape.IsGenericType && shape.GetGenericTypeDefinition() = typedefof<option<_>>

let rec sample (shape: Type) : obj =
    if shape = typeof<string> then box "served"
    elif shape = typeof<int> then box 1
    elif shape = typeof<int64> then box 1L
    elif shape = typeof<float> then box 1.
    elif shape = typeof<bool> then box true
    elif isOption shape then
        let some = FSharpType.GetUnionCases(shape) |> Array.find (fun case -> case.Name = "Some")
        FSharpValue.MakeUnion(some, [|sample (shape.GetGenericArguments()[0])|])
    elif shape.IsGenericType && shape.GetGenericTypeDefinition() = typedefof<list<_>> then
        let empty = FSharpType.GetUnionCases(shape) |> Array.find (fun case -> case.GetFields().Length = 0)
        FSharpValue.MakeUnion(empty, [||])
    elif FSharpType.IsRecord shape then FSharpValue.MakeRecord(shape, FSharpType.GetRecordFields(shape) |> Array.map (fun field -> sample field.PropertyType))
    elif FSharpType.IsUnion shape then
        let case = FSharpType.GetUnionCases(shape) |> Array.head
        FSharpValue.MakeUnion(case, case.GetFields() |> Array.map (fun field -> sample field.PropertyType))
    else failwith $"unsupported generated contract shape {shape}"

let records = typeof<NodeRecord>.Assembly.GetTypes() |> Array.filter (fun shape -> shape.Namespace = typeof<NodeRecord>.Namespace && FSharpType.IsRecord shape) |> Array.sortBy _.Name
let decodeMethod = typeof<NodeRecord>.Assembly.GetType("BibleAtlas.FSharp.Json").GetMethod("decode")

let decode (shape: Type) body =
    let result = decodeMethod.MakeGenericMethod(shape).Invoke(null, [|box body|])
    let case, fields = FSharpValue.GetUnionFields(result, result.GetType())
    if case.Name = "Ok" then Ok fields[0] else Error(unbox<Failure> fields[0])

let wireName (field: Reflection.PropertyInfo) = field.GetCustomAttributes(typeof<JsonPropertyNameAttribute>, false) |> Array.exactlyOne |> unbox<JsonPropertyNameAttribute> |> _.Name

[<Fact>]
let ``every generated optional field accepts null as exactly None`` () =
    for shape in records do
        let value = sample shape
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

[<Fact>]
let ``every generated required field rejects omission and null at the one JSON door`` () =
    for shape in records do
        let value = sample shape
        Assert.Equal(Ok value, decode shape (Json.encode value))
        for field in FSharpType.GetRecordFields shape do
            if not (isOption field.PropertyType) then
                let missing = JsonNode.Parse(Json.encode value).AsObject()
                missing.Remove(wireName field) |> ignore
                let explicitNull = JsonNode.Parse(Json.encode value).AsObject()
                explicitNull[wireName field] <- null
                let actual = [decode shape (missing.ToJsonString()); decode shape (explicitNull.ToJsonString())] |> List.map Result.isError
                Assert.Equal<bool list>([true; true], actual)
