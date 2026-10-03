module rec BibleAtlas.FSharp.Tests.GeneratorTests

open Xunit
open FsCheck.Xunit
open System.IO
open BibleAtlas.FSharp.ContractGenerator

[<Property>]
let ``a schema generates immutable fields with their exact wire names and optionality`` (suffix: uint16) required nullable =
    let identity = $"Entry{suffix}"
    let fields = if required then "[display_name, count]" else "[display_name]"
    let kind = if nullable then "[integer, 'null']" else "integer"
    let count = if nullable || not required then "int option" else "int"
    let source = document $"    {identity}:\n      type: object\n      required: {fields}\n      properties:\n        display_name: {{type: string}}\n        count: {{type: {kind}}}\n"
    let expected = $"// Generated from contracts/openapi.yaml.\nnamespace BibleAtlas.FSharp.Contract\n\nopen System.Text.Json.Serialization\n\ntype {identity} =\n    {{\n        [<JsonPropertyName(\"display_name\")>] DisplayName: string\n        [<JsonPropertyName(\"count\")>] Count: {count}\n    }}\n"
    Assert.Equal(Ok expected, Generator.generate source)

[<Property(MaxTest = 1)>]
let ``every schema in the current published contract generates without an untyped fallback`` () =
    let source = File.ReadAllText(Path.Combine(__SOURCE_DIRECTORY__, "../contracts/openapi.yaml"))
    match Generator.generate source with
    | Ok generated -> Assert.DoesNotContain("JsonElement", generated); Assert.DoesNotContain(": obj", generated)
    | Error error -> Assert.Fail(error)

[<Property>]
let ``an unsupported wire shape is refused rather than replaced with an untyped value`` (suffix: uint16) =
    let source = document $"    Opaque{suffix}:\n      type: object\n      properties:\n        data: {{}}\n"
    Assert.Equal(Error $"Opaque{suffix}.data: unsupported schema {{  }}", Generator.generate source)

[<Property>]
let ``closed vocabulary spells each case from its wire enum`` (suffix: uint16) =
    let source = document $"    Kind:\n      type: string\n      enum: [cited-by-{suffix}, contains]\n"
    let expected = $"// Generated from contracts/openapi.yaml.\nnamespace BibleAtlas.FSharp.Contract\n\nopen System.Text.Json.Serialization\n\n[<RequireQualifiedAccess; JsonFSharpConverter(UnionEncoding = JsonUnionEncoding.UnwrapFieldlessTags)>]\ntype Kind =\n    | [<JsonName(\"cited-by-{suffix}\")>] CitedBy{suffix}\n    | [<JsonName(\"contains\")>] Contains\n"
    Assert.Equal(Ok expected, Generator.generate source)

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
    let source = document $"    {identity}:\n      {schema}\n"
    let expected = $"// Generated from contracts/openapi.yaml.\nnamespace BibleAtlas.FSharp.Contract\n\nopen System.Text.Json.Serialization\n\n[<Struct; JsonConverter(typeof<{identity}JsonConverter>)>]\ntype {identity} =\n    private | {identity} of {primitive}\nand private {identity}JsonConverter() =\n    inherit JsonConverter<{identity}>()\n    override _.Read(reader: byref<System.Text.Json.Utf8JsonReader>, _, options) =\n        if reader.TokenType = System.Text.Json.JsonTokenType.Null then\n            raise (System.Text.Json.JsonException(\"{identity} cannot be null\"))\n        {identity}(System.Text.Json.JsonSerializer.Deserialize<{primitive}>(&reader, options))\n    override _.Write(writer, {identity} value, options) =\n        System.Text.Json.JsonSerializer.Serialize<{primitive}>(writer, value, options)\n"
    Assert.Equal(Ok expected, Generator.generate source)

[<Property>]
let ``a cursor default is exposed only as its own typed first value`` (suffix: uint16) (start: byte) (wide: bool) =
    let identity = $"Cursor{suffix}"
    let format = if wide then "      format: int64\n" else ""
    let primitive = if wide then "int64" else "int"
    let literal = string start + if wide then "L" else ""
    let source = document $"    {identity}:\n      type: integer\n{format}      default: {int start}\n"
    let expected = $"// Generated from contracts/openapi.yaml.\nnamespace BibleAtlas.FSharp.Contract\n\nopen System.Text.Json.Serialization\n\n[<Struct; JsonConverter(typeof<{identity}JsonConverter>)>]\ntype {identity} =\n    private | {identity} of {primitive}\nand private {identity}JsonConverter() =\n    inherit JsonConverter<{identity}>()\n    override _.Read(reader: byref<System.Text.Json.Utf8JsonReader>, _, options) =\n        if reader.TokenType = System.Text.Json.JsonTokenType.Null then\n            raise (System.Text.Json.JsonException(\"{identity} cannot be null\"))\n        {identity}(System.Text.Json.JsonSerializer.Deserialize<{primitive}>(&reader, options))\n    override _.Write(writer, {identity} value, options) =\n        System.Text.Json.JsonSerializer.Serialize<{primitive}>(writer, value, options)\n\nmodule {identity} =\n    let first : {identity} = {identity} {literal}\n"
    Assert.Equal(Ok expected, Generator.generate source)

[<Property>]
let ``a named identity union has its own representation and a widening from its declared leaf`` (suffix: uint16) =
    let leaf = $"Leaf{suffix}"
    let union = $"Union{suffix}"
    let source = document $"    {leaf}:\n      type: string\n    {union}:\n      oneOf: [{{$ref: '#/components/schemas/{leaf}'}}]\n"
    let expected = $"// Generated from contracts/openapi.yaml.\nnamespace BibleAtlas.FSharp.Contract\n\nopen System.Text.Json.Serialization\n\n[<Struct; JsonConverter(typeof<{leaf}JsonConverter>)>]\ntype {leaf} =\n    private | {leaf} of string\nand [<Struct; JsonConverter(typeof<{union}JsonConverter>)>] {union} =\n    private | {union} of string\nand private {leaf}JsonConverter() =\n    inherit JsonConverter<{leaf}>()\n    override _.Read(reader: byref<System.Text.Json.Utf8JsonReader>, _, options) =\n        if reader.TokenType = System.Text.Json.JsonTokenType.Null then\n            raise (System.Text.Json.JsonException(\"{leaf} cannot be null\"))\n        {leaf}(System.Text.Json.JsonSerializer.Deserialize<string>(&reader, options))\n    override _.Write(writer, {leaf} value, options) =\n        System.Text.Json.JsonSerializer.Serialize<string>(writer, value, options)\nand private {union}JsonConverter() =\n    inherit JsonConverter<{union}>()\n    override _.Read(reader: byref<System.Text.Json.Utf8JsonReader>, _, options) =\n        if reader.TokenType = System.Text.Json.JsonTokenType.Null then\n            raise (System.Text.Json.JsonException(\"{union} cannot be null\"))\n        {union}(System.Text.Json.JsonSerializer.Deserialize<string>(&reader, options))\n    override _.Write(writer, {union} value, options) =\n        System.Text.Json.JsonSerializer.Serialize<string>(writer, value, options)\n\nmodule {union} =\n    let of{leaf} ({leaf} value) : {union} = {union} value\n"
    Assert.Equal(Ok expected, Generator.generate source)

let document schemas = "openapi: 3.1.0\ncomponents:\n  schemas:\n" + schemas
