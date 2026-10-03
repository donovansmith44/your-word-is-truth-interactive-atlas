module BibleAtlas.FSharp.Tests.GeneratorTests

open Xunit
open FsCheck.Xunit
open BibleAtlas.FSharp.ContractGenerator

[<Property>]
let ``a schema generates immutable fields with their exact wire names and optionality`` (suffix: uint16) required nullable =
    let identity = $"Entry{suffix}"
    let fields = if required then "[display_name, count]" else "[display_name]"
    let kind = if nullable then "[integer, 'null']" else "integer"
    let count = if nullable || not required then "int option" else "int"
    let source = "openapi: 3.1.0\ncomponents:\n  schemas:\n" + $"    {identity}:\n      type: object\n      required: {fields}\n      properties:\n        display_name: {{type: string}}\n        count: {{type: {kind}}}\n"
    let expected = $"// Generated from contracts/openapi.yaml.\nnamespace BibleAtlas.FSharp.Contract\n\nopen System.Text.Json.Serialization\n\ntype {identity} =\n    {{\n        [<JsonPropertyName(\"display_name\")>] DisplayName: string\n        [<JsonPropertyName(\"count\")>] Count: {count}\n    }}\n\nmodule IdentityConverters =\n    let all: JsonConverter list =\n        [\n        ]\n"
    Assert.Equal(Ok(GeneratedSource.ofText expected), Generator.generate (ContractDocument.ofText source))

[<Property>]
let ``unsupported wire alternatives remain distinct typed failures`` (suffix: uint16) (choice: byte) =
    let path = DocumentPath.root |> DocumentPath.memberAt "components" |> DocumentPath.memberAt "schemas" |> DocumentPath.memberAt $"Opaque{suffix}" |> DocumentPath.memberAt "properties" |> DocumentPath.memberAt "data"
    let shape, failure =
        match choice % 4uy with
        | 0uy -> "{}", UnsupportedType(path, [])
        | 1uy -> "{type: unknown}", UnsupportedType(path, ["unknown"])
        | 2uy -> "{type: [string, number]}", UnsupportedType(path, ["string"; "number"])
        | _ -> "{oneOf: [{type: string}, {type: integer}]}", UnsupportedUnion(DocumentPath.memberAt "oneOf" path)
    let source = $"components:\n  schemas:\n    Opaque{suffix}:\n      type: object\n      properties:\n        data: {shape}\n"
    Assert.Equal(Error failure, Generator.generate (ContractDocument.ofText source))

[<Property>]
let ``closed vocabulary preserves every generated wire case and its order`` (count: byte) =
    let indices = [1 .. 1 + int count % 8]
    let wires = indices |> List.map (fun index -> $"cited-by-{index}") |> String.concat ", "
    let source = $"components:\n  schemas:\n    Kind:\n      type: string\n      enum: [{wires}]\n"
    let cases = indices |> List.map (fun index -> $"    | [<JsonName(\"cited-by-{index}\")>] CitedBy{index}\n") |> String.concat ""
    let expected = "// Generated from contracts/openapi.yaml.\nnamespace BibleAtlas.FSharp.Contract\n\nopen System.Text.Json.Serialization\n\n[<RequireQualifiedAccess; JsonFSharpConverter(UnionEncoding = JsonUnionEncoding.UnwrapFieldlessTags)>]\ntype Kind =\n" + cases + "\nmodule IdentityConverters =\n    let all: JsonConverter list =\n        [\n        ]\n"
    Assert.Equal(Ok(GeneratedSource.ofText expected), Generator.generate (ContractDocument.ofText source))

[<Property>]
let ``named scalar schemas generate distinct immutable identities`` (suffix: uint16) (kind: byte) =
    let identity = $"Identity{suffix}"
    let schema, primitive =
        match kind % 5uy with
        | 0uy -> "type: string", "string"
        | 1uy -> "type: integer", "int"
        | 2uy -> "type: integer\n      format: int64", "int64"
        | 3uy -> "type: number", "float"
        | _ -> "type: boolean", "bool"
    let source = "openapi: 3.1.0\ncomponents:\n  schemas:\n" + $"    {identity}:\n      {schema}\n"
    let expected = $"// Generated from contracts/openapi.yaml.\nnamespace BibleAtlas.FSharp.Contract\n\nopen System.Text.Json.Serialization\n\n[<JsonConverter(typeof<{identity}JsonConverter>)>]\ntype {identity} =\n    private | {identity} of {primitive}\nand private {identity}JsonConverter() =\n    inherit JsonConverter<{identity}>()\n    override _.HandleNull = true\n    override _.Read(reader: byref<System.Text.Json.Utf8JsonReader>, _, options) =\n        if reader.TokenType = System.Text.Json.JsonTokenType.Null then\n            raise (System.Text.Json.JsonException(\"{identity} cannot be null\"))\n        {identity}(System.Text.Json.JsonSerializer.Deserialize<{primitive}>(&reader, options))\n    override _.Write(writer, {identity} value, options) =\n        System.Text.Json.JsonSerializer.Serialize<{primitive}>(writer, value, options)\n\nmodule IdentityConverters =\n    let all: JsonConverter list =\n        [\n        {identity}JsonConverter() :> JsonConverter\n        ]\n"
    Assert.Equal(Ok(GeneratedSource.ofText expected), Generator.generate (ContractDocument.ofText source))
