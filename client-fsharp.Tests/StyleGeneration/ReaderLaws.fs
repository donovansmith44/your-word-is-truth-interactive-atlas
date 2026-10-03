module BibleAtlas.FSharp.Tests.StyleGeneration.ReaderLaws

open System.IO
open FsCheck.Xunit
open Xunit
open BibleAtlas.FSharp.ContractGenerator

[<Property>]
let ``reading preserves generated field types and optionality as whole typed values`` (count: byte) required nullable =
    let fieldCount = 1 + int count % 8
    let fields = [1 .. fieldCount] |> List.map (fun index -> $"field_{index}")
    let requiredFields = if required then String.concat ", " fields else ""
    let kind = if nullable then "[integer, 'null']" else "integer"
    let properties = fields |> List.map (fun field -> $"        {field}: {{type: {kind}}}") |> String.concat "\n"
    let source = $"components:\n  schemas:\n    Entry:\n      type: object\n      required: [{requiredFields}]\n      properties:\n{properties}\n"
    let path = DocumentPath.root |> DocumentPath.memberAt "components" |> DocumentPath.memberAt "schemas"
    let expectedFields =
        fields |> List.map (fun wire ->
            SchemaName.create path wire |> Result.map (fun name ->
                { WireName = wire; Name = name; Shape = if nullable || not required then Optional(Primitive Int32) else Primitive Int32 }))
        |> List.fold (fun state field -> Result.bind (fun fields -> Result.map (fun field -> fields @ [field]) field) state) (Ok [])
    let expected =
        SchemaName.create path "Entry"
        |> Result.bind (fun name -> expectedFields |> Result.map (fun fields -> { Schemas = [{ Name = name; Definition = Record fields }]; Operations = [] }))
    Assert.Equal(expected, ContractReader.read (ContractDocument.ofText source))

[<Property>]
let ``nullable unions retain their element shape and do not flatten nominal identities`` (depth: byte) =
    let depth = 1 + int depth % 6
    let item = "{$ref: '#/components/schemas/Id'}"
    let shape = [1 .. depth] |> List.fold (fun shape _ -> "{type: array, items: " + shape + "}") item
    let source = $"components:\n  schemas:\n    Id: {{type: string}}\n    Container:\n      type: object\n      required: [value]\n      properties:\n        value:\n          oneOf:\n          - {shape}\n          - {{type: 'null'}}\n"
    let path = DocumentPath.root
    let expected =
        SchemaName.create path "Id"
        |> Result.bind (fun identity ->
            SchemaName.create path "Container" |> Result.bind (fun container ->
                SchemaName.create path "Value" |> Result.map (fun field ->
                    { Schemas = [ { Name = identity; Definition = Identity Text }
                                  { Name = container; Definition = Record [ { WireName = "value"; Name = field; Shape = Optional([1 .. depth] |> List.fold (fun shape _ -> Many shape) (Named identity)) } ] } ]
                      Operations = [] })))
    Assert.Equal(expected, ContractReader.read (ContractDocument.ofText source))

[<Property>]
let ``reader errors are reachable values with their exact document paths`` (choice: byte) (depth: byte) =
    let path = DocumentPath.root
    let schemas = path |> DocumentPath.memberAt "components" |> DocumentPath.memberAt "schemas"
    let entry = schemas |> DocumentPath.memberAt "Entry"
    let document schema = "components:\n  schemas:\n    Entry: " + schema + "\n"
    let case index =
        match index with
        | 0 -> "{}", NoSchemas schemas
        | 1 -> document "[]", ExpectedMapping entry
        | 2 -> document "{type: {}}", ExpectedScalar(DocumentPath.memberAt "type" entry)
        | 3 -> document "{type: string, enum: {}}", ExpectedSequence(DocumentPath.memberAt "enum" entry)
        | 4 -> document "{type: array}", MissingField(entry, "items")
        | 5 -> document $"{{type: unknown_{depth}}}", UnsupportedType(entry, [$"unknown_{depth}"])
        | 6 -> document "{oneOf: [{type: string}, {type: integer}]}", UnsupportedUnion(DocumentPath.memberAt "oneOf" entry)
        | 7 -> document "{$ref: 'elsewhere/Id'}", InvalidReference(DocumentPath.memberAt "$ref" entry, "elsewhere/Id")
        | 8 -> "components:\n  schemas:\n    '---': {type: string}\n", InvalidName(schemas |> DocumentPath.memberAt "---", "---")
        | 9 -> document "{type: string, enum: []}", EmptyVocabulary(DocumentPath.memberAt "enum" entry)
        | _ -> "components:\n  schemas:\n    Entry: {type: string}\npaths:\n  /entry:\n    get:\n      operationId: entry\n      parameters:\n      - {name: x, in: cookie, schema: {type: string}}\n      responses:\n        '200':\n          content:\n            application/json:\n              schema: {$ref: '#/components/schemas/Entry'}\n", UnsupportedParameterLocation(path |> DocumentPath.memberAt "paths" |> DocumentPath.memberAt "/entry" |> DocumentPath.memberAt "get" |> DocumentPath.memberAt "parameters" |> DocumentPath.itemAt 0 |> DocumentPath.memberAt "in", "cookie")
    let cases = [0 .. 10] |> List.map (fun index -> case ((index + int choice) % 11))
    let expected = cases |> List.map (snd >> Error)
    let actual = cases |> List.map (fst >> ContractDocument.ofText >> ContractReader.read)
    Assert.Equal<Result<ContractModel, ContractError> list>(expected, actual)
    let paths = cases |> List.map (snd >> ContractError.render >> fun message -> message.Contains('$'))
    Assert.Equal<bool list>(List.replicate 11 true, paths)

[<Property>]
let ``malformed YAML is returned at the library boundary`` (indent: byte) =
    let source = "components:\n" + String.replicate (1 + int indent % 8) " " + "schemas: [unterminated"
    match ContractReader.read (ContractDocument.ofText source) with
    | Error(InvalidYaml diagnostic) -> Assert.Contains("$", ContractError.render (InvalidYaml diagnostic)); Assert.NotEmpty(diagnostic.Message)
    | other -> Assert.Fail($"expected InvalidYaml, received {other}")

[<Property>]
let ``adding a generated nominal schema to the complete published document preserves the rest`` (identity: uint16) =
    let source = File.ReadAllText(Path.Combine(__SOURCE_DIRECTORY__, "../../contracts/openapi.yaml"))
    let addedName = $"StyleIdentity{identity}"
    let extended = source.Replace("  schemas:\n", $"  schemas:\n    {addedName}: {{type: string}}\n")
    let expected =
        ContractReader.read (ContractDocument.ofText source)
        |> Result.bind (fun original -> SchemaName.create DocumentPath.root addedName |> Result.map (fun name ->
            { original with Schemas = { Name = name; Definition = Identity Text } :: original.Schemas }))
    Assert.Equal(expected, ContractReader.read (ContractDocument.ofText extended))
