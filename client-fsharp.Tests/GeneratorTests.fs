module BibleAtlas.FSharp.Tests.GeneratorTests

open Xunit
open System.IO
open BibleAtlas.FSharp.ContractGenerator

let document schemas = "openapi: 3.1.0\ncomponents:\n  schemas:\n" + schemas

[<Fact>]
let ``a schema generates immutable fields with their exact wire names and optionality`` () =
    let source = document "    Entry:\n      type: object\n      required: [display_name]\n      properties:\n        display_name: {type: string}\n        count: {type: [integer, 'null']}\n"
    let expected = "// Generated from contracts/openapi.yaml.\nnamespace BibleAtlas.FSharp.Contract\n\nopen System.Text.Json.Serialization\n\ntype Entry =\n    {\n        [<JsonPropertyName(\"display_name\")>] DisplayName: string\n        [<JsonPropertyName(\"count\")>] Count: int option\n    }\n"
    Assert.Equal(Ok expected, Generator.generate source)

[<Fact>]
let ``every schema in the current published contract generates without an untyped fallback`` () =
    let source = File.ReadAllText(Path.Combine(__SOURCE_DIRECTORY__, "../contracts/openapi.yaml"))
    match Generator.generate source with
    | Ok generated -> Assert.DoesNotContain("JsonElement", generated); Assert.DoesNotContain(": obj", generated)
    | Error error -> Assert.Fail(error)

[<Fact>]
let ``an unsupported wire shape is refused rather than replaced with an untyped value`` () =
    let source = document "    Opaque:\n      type: object\n      properties:\n        data: {}\n"
    Assert.Equal(Error "Opaque.data: unsupported schema {  }", Generator.generate source)

[<Fact>]
let ``closed vocabulary spells each case from its wire enum`` () =
    let source = document "    Kind:\n      type: string\n      enum: [cited-by, contains]\n"
    let expected = "// Generated from contracts/openapi.yaml.\nnamespace BibleAtlas.FSharp.Contract\n\nopen System.Text.Json.Serialization\n\n[<RequireQualifiedAccess; JsonFSharpConverter(UnionEncoding = JsonUnionEncoding.UnwrapFieldlessTags)>]\ntype Kind =\n    | [<JsonName(\"cited-by\")>] CitedBy\n    | [<JsonName(\"contains\")>] Contains\n"
    Assert.Equal(Ok expected, Generator.generate source)
