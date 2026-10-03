module BibleAtlas.FSharp.Tests.StyleGeneration.IdentityLaws

open System
open System.Text.Json
open Microsoft.FSharp.Reflection
open FsCheck
open FsCheck.Xunit
open global.Xunit
open BibleAtlas.FSharp.Contract

[<Property>]
let ``every scalar primitive compiles to a nominal reference identity and preserves its wire value`` (NonNull text: NonNull<string>) (number: int) (boolean: bool) =
    let cases =
        [ typeof<FixtureText>, JsonSerializer.Serialize text
          typeof<FixtureInt>, JsonSerializer.Serialize number
          typeof<FixtureLong>, JsonSerializer.Serialize (int64 number * int64 Int32.MaxValue)
          typeof<FixtureNumber>, JsonSerializer.Serialize (float number / 2.0)
          typeof<FixtureBoolean>, JsonSerializer.Serialize boolean ]
    let actual =
        cases |> List.map (fun (shape, body) ->
            let value = JsonSerializer.Deserialize(body, shape)
            let union = FSharpType.GetUnionCases(shape, true) |> Array.toList
            let privateConstructor =
                match union with
                | [case] -> not (FSharpValue.PreComputeUnionConstructorInfo(case, true).IsPublic)
                | _ -> false
            shape.IsValueType, privateConstructor, JsonSerializer.Serialize(value, shape))
    let expected = cases |> List.map (fun (_, body) -> false, true, body)
    Assert.Equal<(bool * bool * string) list>(expected, actual)

[<Property>]
let ``every nominal converter receives and refuses null instead of silently returning a default`` (spaces: byte) =
    let body = String.replicate (int spaces % 12) " " + "null"
    let shapes = [ typeof<FixtureText>; typeof<FixtureInt>; typeof<FixtureLong>; typeof<FixtureNumber>; typeof<FixtureBoolean> ]
    let actual = shapes |> List.map (fun shape ->
        Assert.Throws<JsonException>(fun () -> JsonSerializer.Deserialize(body, shape) |> ignore).Message)
    let expected = shapes |> List.map (fun shape -> shape.Name + " cannot be null")
    Assert.Equal<string list>(expected, actual)
