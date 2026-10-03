namespace BibleAtlas.FSharp.ContractGenerator

module rec ContractEmitter =
    let emit (model: ContractModel) : GeneratedSource =
        "// Generated from contracts/openapi.yaml.\nnamespace BibleAtlas.FSharp.Contract\n\nopen System.Text.Json.Serialization\n\n"
        + (model.Schemas |> List.mapi definition |> String.concat "")
        + (model.Schemas |> List.map identityConverter |> String.concat "")
        + converterRegistry model + requests model
        |> GeneratedSource.ofText

    let private definition index (schema: Schema) =
        let schemaName = name schema.Name
        let attributes =
            match schema.Definition with
            | Tagged(discriminator, _) -> "[<RequireQualifiedAccess; JsonFSharpConverter(UnionEncoding = (JsonUnionEncoding.InternalTag ||| JsonUnionEncoding.UnwrapRecordCases ||| JsonUnionEncoding.AllowUnorderedTag), UnionTagName = " + quote discriminator + ")>]"
            | Enumeration _ -> "[<RequireQualifiedAccess; JsonFSharpConverter(UnionEncoding = JsonUnionEncoding.UnwrapFieldlessTags)>]"
            | Identity _ -> "[<JsonConverter(typeof<" + schemaName + "JsonConverter>)>]"
            | Alias _ | Record _ -> ""
        let heading =
            if index = 0 then (if attributes = "" then "" else attributes + "\n") + "type " + schemaName + " =\n"
            else "and " + (if attributes = "" then "" else attributes + " ") + schemaName + " =\n"
        let body =
            match schema.Definition with
            | Identity value -> "    private | " + schemaName + " of " + primitive value + "\n"
            | Alias value -> "    " + shape value + "\n"
            | Enumeration cases ->
                cases |> List.map (fun case -> "    | [<JsonName(" + quote case.WireName + ")>] " + name case.Name + "\n") |> String.concat ""
            | Tagged(_, cases) ->
                cases |> List.map (fun case -> "    | [<JsonName(" + quote case.Tag.WireName + ")>] " + name case.Tag.Name + " of " + name case.Payload + "\n") |> String.concat ""
            | Record fields ->
                "    {\n" + (fields |> List.map (fun field -> "        [<JsonPropertyName(" + quote field.WireName + ")>] " + name field.Name + ": " + shape field.Shape + "\n") |> String.concat "") + "    }\n"
        heading + body

    let private identityConverter (schema: Schema) =
        match schema.Definition with
        | Alias _ | Record _ | Enumeration _ | Tagged _ -> ""
        | Identity kind ->
            let schemaName = name schema.Name
            let primitive = primitive kind
            let nullMessage = quote (schemaName + " cannot be null")
            $"and private {schemaName}JsonConverter() =\n    inherit JsonConverter<{schemaName}>()\n    override _.HandleNull = true\n    override _.Read(reader: byref<System.Text.Json.Utf8JsonReader>, _, options) =\n        if reader.TokenType = System.Text.Json.JsonTokenType.Null then\n            raise (System.Text.Json.JsonException({nullMessage}))\n        {schemaName}(System.Text.Json.JsonSerializer.Deserialize<{primitive}>(&reader, options))\n    override _.Write(writer, {schemaName} value, options) =\n        System.Text.Json.JsonSerializer.Serialize<{primitive}>(writer, value, options)\n"

    let private converterRegistry (model: ContractModel) =
        let converters =
            model.Schemas |> List.choose (fun schema ->
                match schema.Definition with
                | Identity _ -> Some("        " + name schema.Name + "JsonConverter() :> JsonConverter\n")
                | Alias _ | Record _ | Enumeration _ | Tagged _ -> None)
        "\nmodule IdentityConverters =\n    let all: JsonConverter list =\n        [\n" + String.concat "" converters + "        ]\n"

    let private requests (model: ContractModel) =
        match model.Operations with
        | [] -> ""
        | operations ->
            "\ntype Request<'a> = private Request of uri: string\n\nmodule Request =\n    let uri (Request uri) = uri\n\ntype private RequestEncoding =\n    static member Encode<'a>(item: 'a) : string =\n        use document = System.Text.Json.JsonDocument.Parse(System.Text.Json.JsonSerializer.Serialize item)\n        let value =\n            match document.RootElement.ValueKind with\n            | System.Text.Json.JsonValueKind.String -> document.RootElement.GetString()\n            | System.Text.Json.JsonValueKind.Array -> document.RootElement.EnumerateArray() |> Seq.map (fun value -> value.GetString()) |> String.concat \",\"\n            | _ -> document.RootElement.GetRawText()\n        System.Uri.EscapeDataString value\n\nmodule Reads =\n" + (operations |> List.map request |> String.concat "\n")

    let private request (operation: Operation) =
        let operationName = name operation.Name
        let functionName =
            match Seq.tryHead operationName with
            | None -> "request"
            | Some first -> string (System.Char.ToLowerInvariant first) + operationName.Substring(1)
        let arguments =
            match operation.Parameters with
            | [] -> "()"
            | parameters ->
                parameters |> List.map (fun parameter ->
                    let parameterShape = shape parameter.Shape + if parameter.Required then "" else " option"
                    "(``" + parameter.WireName + "``: " + parameterShape + ")") |> String.concat " "
        let path =
            operation.Parameters |> List.fold (fun path parameter ->
                match parameter.Location with
                | Query -> path
                | Path -> path + ".Replace(" + quote ("{" + parameter.WireName + "}") + ", RequestEncoding.Encode ``" + parameter.WireName + "``)") (quote (operation.Path.TrimStart('/')))
        let query =
            operation.Parameters |> List.choose (fun parameter ->
                match parameter.Location with
                | Path -> None
                | Query ->
                    let key = quote (parameter.WireName + "=")
                    Some(if parameter.Required then "                yield " + key + " + RequestEncoding.Encode ``" + parameter.WireName + "``\n"
                         else "                match ``" + parameter.WireName + "`` with Some item -> yield " + key + " + RequestEncoding.Encode item | None -> ()\n"))
            |> String.concat ""
        "    let " + functionName + " " + arguments + " : Request<" + shape operation.Response + "> =\n        let path = " + path + "\n        let query =\n            [\n" + query + "            ]\n        Request(path + (if query.IsEmpty then \"\" else \"?\" + String.concat \"&\" query))\n"

    let private shape value =
        match value with
        | Primitive value -> primitive value
        | Named value -> name value
        | Optional value -> shape value + " option"
        | Many value -> "(" + shape value + ") list"
        | StringMap value -> "Map<string, " + shape value + ">"

    let private primitive value =
        match value with
        | Text -> "string"
        | Int32 -> "int"
        | Int64 -> "int64"
        | Number -> "float"
        | Boolean -> "bool"

    let private name value = SchemaName.text value

    let private quote (value: string) =
        let escaped = value |> Seq.map (fun letter ->
            match letter with
            | '"' -> "\\\""
            | '\\' -> "\\\\"
            | '\n' -> "\\n"
            | '\r' -> "\\r"
            | '\t' -> "\\t"
            | letter when System.Char.IsControl letter -> $"\\u{int letter:X4}"
            | letter -> string letter) |> String.concat ""
        "\"" + escaped + "\""
