namespace BibleAtlas.FSharp.ContractGenerator

open System
open System.IO
open System.Text
open System.Text.RegularExpressions
open YamlDotNet.RepresentationModel

module Generator =
    let internal name (wire: string) =
        let words = Regex.Split(wire, "[^A-Za-z0-9]+") |> Array.filter (String.IsNullOrEmpty >> not)
        let joined = words |> Array.map (fun word -> string (Char.ToUpperInvariant word[0]) + word[1..]) |> String.concat ""
        if joined.Length = 0 then invalidOp $"empty identifier: {wire}"
        elif Char.IsDigit joined[0] then "N" + joined
        else joined

    let generate (document: string) =
        try
            let yaml = YamlStream()
            use reader = new StringReader(document)
            yaml.Load(reader)
            let get key (node: YamlNode) =
                match node with
                | :? YamlMappingNode as mapping ->
                    match mapping.Children.TryGetValue(YamlScalarNode(key)) with
                    | true, value -> Some value
                    | _ -> None
                | _ -> None
            let scalar (node: YamlNode) =
                match node with
                | :? YamlScalarNode as value -> value.Value
                | _ -> invalidOp $"expected a scalar, received {node}"
            let values (node: YamlNode) =
                match node with
                | :? YamlSequenceNode as sequence -> Seq.toList sequence.Children
                | _ -> invalidOp $"expected a sequence, received {node}"
            let fields (node: YamlNode) =
                match node with
                | :? YamlMappingNode as mapping -> mapping.Children |> Seq.map (fun entry -> scalar entry.Key, entry.Value) |> Seq.toList
                | _ -> invalidOp $"expected a mapping, received {node}"
            let quote (value: string) = System.Text.Json.JsonSerializer.Serialize(value)
            let schemas = yaml.Documents[0].RootNode |> get "components" |> Option.bind (get "schemas") |> Option.map fields |> Option.defaultWith (fun () -> invalidOp "the document has no components.schemas")
            let definitions = ResizeArray<string * YamlNode>(schemas)
            let rec shape (context: string) (node: YamlNode) =
                match get "$ref" node with
                | Some reference -> scalar reference |> fun reference -> reference.Split('/') |> Array.last
                | None ->
                    match get "oneOf" node with
                    | Some cases ->
                        let cases = values cases
                        let present = cases |> List.filter (fun case -> get "type" case |> Option.exists (fun kind -> scalar kind = "null") |> not)
                        match present, cases.Length with
                        | [value], 2 -> shape context value + " option"
                        | _ -> invalidOp $"{context}: unsupported union {node}"
                    | None ->
                        let kinds =
                            match get "type" node with
                            | Some (:? YamlSequenceNode as list) -> values list |> List.map scalar
                            | Some kind -> [scalar kind]
                            | None -> []
                        let nullable = List.contains "null" kinds
                        let kind = kinds |> List.filter ((<>) "null")
                        let underlying =
                            match get "enum" node with
                            | Some _ ->
                                let typeName = context.Split('.') |> Array.map name |> String.concat ""
                                if definitions |> Seq.exists (fst >> (=) typeName) |> not then definitions.Add(typeName, node)
                                typeName
                            | None ->
                                match kind with
                                | ["string"] -> "string"
                                | ["integer"] -> if get "format" node |> Option.exists (scalar >> (=) "int64") then "int64" else "int"
                                | ["number"] -> "float"
                                | ["boolean"] -> "bool"
                                | ["array"] -> get "items" node |> Option.map (shape (context + "Item")) |> Option.map (fun item -> $"({item}) list") |> Option.defaultWith (fun () -> invalidOp $"{context}: an array has no item schema")
                                | ["object"] ->
                                    match get "additionalProperties" node with
                                    | Some (:? YamlMappingNode as item) ->
                                        let valueType = shape (context + "Value") item
                                        $"Map<string, {valueType}>"
                                    | _ -> invalidOp $"{context}: unsupported object {node}"
                                | _ -> invalidOp $"{context}: unsupported schema {node}"
                        underlying + if nullable then " option" else ""
            let requests =
                yaml.Documents[0].RootNode |> get "paths" |> Option.map fields |> Option.defaultValue []
                |> List.choose (fun (path, endpoint) ->
                    get "get" endpoint |> Option.bind (fun operation ->
                        let response = get "responses" operation |> Option.bind (get "200") |> Option.bind (get "content") |> Option.bind (get "application/json") |> Option.bind (get "schema")
                        response |> Option.map (fun response ->
                            let operationName = get "operationId" operation |> Option.map scalar |> Option.defaultWith (fun () -> invalidOp $"{path}: no operationId")
                            let parameters = get "parameters" operation |> Option.map values |> Option.defaultValue []
                            let parameters = parameters |> List.map (fun parameter ->
                                let wire = get "name" parameter |> Option.map scalar |> Option.defaultWith (fun () -> invalidOp $"{path}: unnamed parameter")
                                let required = get "required" parameter |> Option.exists (scalar >> (=) "true")
                                let location = get "in" parameter |> Option.map scalar |> Option.defaultValue "query"
                                let schema = get "schema" parameter |> Option.defaultWith (fun () -> invalidOp $"{path}: {wire} has no schema")
                                wire, shape (operationName + "." + wire) schema, required, location)
                            operationName, path.TrimStart('/'), shape (operationName + "Response") response, parameters)))
            let ownFields (node: YamlNode) =
                match get "allOf" node with
                | Some inherited -> values inherited |> List.filter (get "$ref" >> Option.isNone)
                | None -> [node]
            let output = StringBuilder("// Generated from contracts/openapi.yaml.\nnamespace BibleAtlas.FSharp.Contract\n\nopen System.Text.Json.Serialization\n\n")
            let mutable index = 0
            while index < definitions.Count do
                let typeName, node = definitions[index]
                let prefix = if index = 0 then "type" else "and"
                let enumValues = get "enum" node
                let discriminator = get "discriminator" node
                let attributes =
                    match discriminator, enumValues with
                    | Some tag, _ ->
                        let property = get "propertyName" tag |> Option.map scalar |> Option.defaultWith (fun () -> invalidOp $"{typeName}: a discriminator has no propertyName")
                        $"[<RequireQualifiedAccess; JsonFSharpConverter(UnionEncoding = (JsonUnionEncoding.InternalTag ||| JsonUnionEncoding.UnwrapRecordCases ||| JsonUnionEncoding.AllowUnorderedTag), UnionTagName = {quote property})>]"
                    | None, Some _ -> "[<RequireQualifiedAccess; JsonFSharpConverter(UnionEncoding = JsonUnionEncoding.UnwrapFieldlessTags)>]"
                    | _ -> ""
                if index = 0 then
                    if attributes <> "" then output.AppendLine(attributes) |> ignore
                    output.AppendLine($"{prefix} {typeName} =") |> ignore
                else
                    output.AppendLine($"{prefix} {attributes} {typeName} =".Replace("and  ", "and ")) |> ignore
                match discriminator, enumValues with
                | Some tag, _ ->
                    let mapping = get "mapping" tag |> Option.map fields |> Option.defaultWith (fun () -> invalidOp $"{typeName}: a discriminator has no mapping")
                    for wire, reference in mapping do
                        let payload = scalar reference |> fun reference -> reference.Split('/') |> Array.last
                        output.AppendLine($"    | [<JsonName({quote wire})>] {name wire} of {payload}") |> ignore
                | None, Some cases ->
                    for case in values cases |> List.map scalar do
                        output.AppendLine($"    | [<JsonName({quote case})>] {name case}") |> ignore
                | _ ->
                    let properties = ownFields node |> List.collect (fun own -> get "properties" own |> Option.map (fun properties -> fields properties |> List.map (fun (wire, property) -> wire, property, own)) |> Option.defaultValue [])
                    match properties with
                    | [] -> output.AppendLine($"    {shape typeName node}") |> ignore
                    | properties ->
                        output.AppendLine("    {") |> ignore
                        for wire, property, own in properties do
                            let required = get "required" own |> Option.map (values >> List.map scalar >> List.contains wire) |> Option.defaultValue false
                            let propertyType = shape (typeName + "." + wire) property
                            let optional = if required || propertyType.EndsWith(" option") then propertyType else propertyType + " option"
                            output.AppendLine($"        [<JsonPropertyName({quote wire})>] {name wire}: {optional}") |> ignore
                        output.AppendLine("    }") |> ignore
                index <- index + 1
            if not requests.IsEmpty then
                output.AppendLine("\ntype Request<'a> = private Request of uri: string\n\nmodule Request =\n    let uri (Request uri) = uri\n\nmodule Reads =") |> ignore
                output.AppendLine("    let private value item =\n        use document = System.Text.Json.JsonDocument.Parse(System.Text.Json.JsonSerializer.Serialize item)\n        match document.RootElement.ValueKind with\n        | System.Text.Json.JsonValueKind.String -> document.RootElement.GetString()\n        | System.Text.Json.JsonValueKind.Array -> document.RootElement.EnumerateArray() |> Seq.map (fun value -> value.GetString()) |> String.concat \",\"\n        | _ -> document.RootElement.GetRawText()\n\n    let private encode item = System.Uri.EscapeDataString(value item)") |> ignore
                for operation, path, response, parameters in requests do
                    let operation = name operation
                    let functionName = string (Char.ToLowerInvariant operation[0]) + operation[1..]
                    let arguments = parameters |> List.map (fun (wire, shape, required, _) ->
                        let suffix = if required then "" else " option"
                        $"(``{wire}``: {shape}{suffix})") |> String.concat " "
                    let arguments = if arguments = "" then "()" else arguments
                    output.AppendLine($"    let {functionName} {arguments} : Request<{response}> =") |> ignore
                    let template = parameters |> List.filter (fun (_, _, _, location) -> location = "path") |> List.fold (fun path (wire, _, _, _) ->
                        let placeholder = quote ("{" + wire + "}")
                        path + $".Replace({placeholder}, encode ``{wire}``)") (quote path)
                    output.AppendLine($"        let path = {template}") |> ignore
                    output.AppendLine("        let query =\n            [") |> ignore
                    for wire, _, required, location in parameters do
                        if location = "query" then
                            let key = quote (wire + "=")
                            if required then output.AppendLine($"                yield {key} + encode ``{wire}``") |> ignore
                            else output.AppendLine($"                match ``{wire}`` with Some item -> yield {key} + encode item | None -> ()") |> ignore
                    output.AppendLine("            ]\n        Request(path + (if query.IsEmpty then \"\" else \"?\" + String.concat \"&\" query))") |> ignore
            Ok(output.ToString().Replace("\r\n", "\n"))
        with error -> Error error.Message

    let generateFile document output =
        generate (File.ReadAllText document)
        |> Result.map (fun generated ->
            let directory = Path.GetDirectoryName(Path.GetFullPath output)
            Directory.CreateDirectory directory |> ignore
            File.WriteAllText(output, generated))
