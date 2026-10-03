namespace BibleAtlas.FSharp.ContractGenerator

module rec ContractReader =
    let read document : Result<ContractModel, ContractError> =
        YamlAdapter.read document |> Result.bind model

    let private model (node: YamlValue) : Result<ContractModel, ContractError> =
        result {
            let schemasPath = DocumentPath.root |> memberPath "components" |> memberPath "schemas"
            let! schemas = descendant ["components"; "schemas"] DocumentPath.root node
            match schemas with
            | None -> return! Error(NoSchemas schemasPath)
            | Some schemas ->
                let! schemas = fields schemasPath schemas
                match schemas with
                | [] -> return! Error(NoSchemas schemasPath)
                | schemas ->
                    let! schemas = schemas |> traverse (fun (wire, node) -> result {
                        let path = memberPath wire schemasPath
                        let! name = SchemaName.create path wire
                        return! schema name path node
                    })
                    let! document = fields DocumentPath.root node
                    let! paths = optional "paths" DocumentPath.root document fields []
                    let! operations = paths |> traverse (fun (template, node) -> operation (DocumentPath.root |> memberPath "paths" |> memberPath template) template node)
                    let operations = operations |> List.choose id
                    let extra = List.collect snd schemas @ List.collect snd operations
                    return { Schemas = (List.map fst schemas @ extra) |> List.distinctBy (fun schema -> schema.Name)
                             Operations = List.map fst operations }
        }

    let private schema (name: SchemaName) (path: DocumentPath) (node: YamlValue) : Result<Schema * Schema list, ContractError> = result {
        let! entries = fields path node
        match find "discriminator" entries, find "enum" entries with
        | Some tag, _ ->
            let tagPath = memberPath "discriminator" path
            let! tag = fields tagPath tag
            let! property = required "propertyName" tagPath tag
            let! discriminator = scalar (memberPath "propertyName" tagPath) property
            let! mapping = required "mapping" tagPath tag
            let! mapping = fields (memberPath "mapping" tagPath) mapping
            let! cases = mapping |> traverse (fun (wire, target) -> result {
                let casePath = tagPath |> memberPath "mapping" |> memberPath wire
                let! name = SchemaName.create casePath wire
                let! payload = reference casePath target
                return { Tag = { WireName = wire; Name = name }; Payload = payload }
            })
            match cases with
            | [] -> return! Error(EmptyVocabulary(memberPath "mapping" tagPath))
            | cases -> return { Name = name; Definition = Tagged(discriminator, cases) }, []
        | None, Some cases ->
            let! cases = vocabulary (memberPath "enum" path) cases
            return { Name = name; Definition = Enumeration cases }, []
        | None, None ->
            let! ownNodes = optional "allOf" path entries sequence [node]
            let! parts =
                ownNodes
                |> List.mapi (fun index node ->
                    let ownPath = if find "allOf" entries |> Option.isSome then path |> memberPath "allOf" |> DocumentPath.itemAt index else path
                    ownPath, node)
                |> traverse (fun (ownPath, ownNode) -> result {
                    let! own = fields ownPath ownNode
                    match find "$ref" own with
                    | Some _ -> return [], []
                    | None ->
                        let! requiredFields = optional "required" ownPath own stringValues []
                        let! properties = optional "properties" ownPath own fields []
                        let! parsed = properties |> traverse (fun (wire, node) -> result {
                            let fieldPath = ownPath |> memberPath "properties" |> memberPath wire
                            let! fieldName = SchemaName.create fieldPath wire
                            let! context = contextName name wire fieldPath
                            let! fieldShape, extra = shape context fieldPath node
                            let fieldShape =
                                match fieldShape, List.contains wire requiredFields with
                                | Optional _, _ -> fieldShape
                                | _, true -> fieldShape
                                | _, false -> Optional fieldShape
                            return ({ WireName = wire; Name = fieldName; Shape = fieldShape }: Field), extra
                        })
                        return List.map fst parsed, List.collect snd parsed
                })
            let properties = parts |> List.collect fst
            let extra = parts |> List.collect snd
            match properties with
            | _ :: _ -> return { Name = name; Definition = Record properties }, extra
            | [] ->
                let! value, extra = shape name path node
                let definition =
                    match value with
                    | Primitive value -> Identity value
                    | Named _ | Optional _ | Many _ | StringMap _ -> Alias value
                return { Name = name; Definition = definition }, extra
    }

    let private operation (path: DocumentPath) (template: string) (node: YamlValue) : Result<(Operation * Schema list) option, ContractError> = result {
        let! get = descendant ["get"] path node
        match get with
        | None -> return None
        | Some node ->
            let path = memberPath "get" path
            let responseKeys = ["responses"; "200"; "content"; "application/json"; "schema"]
            let! response = descendant responseKeys path node
            match response with
            | None -> return None
            | Some response ->
                let! entries = fields path node
                let! name = required "operationId" path entries
                let! name = scalar (memberPath "operationId" path) name
                let! name = SchemaName.create (memberPath "operationId" path) name
                let! parameters = optional "parameters" path entries sequence []
                let! parameters =
                    parameters |> List.mapi (fun index node -> parameter name (path |> memberPath "parameters" |> DocumentPath.itemAt index) node) |> traverse id
                let! context = contextName name "Response" path
                let responsePath = responseKeys |> List.fold (fun path key -> memberPath key path) path
                let! response, extra = shape context responsePath response
                return Some({ Name = name; Path = template; Response = response; Parameters = List.map fst parameters }, List.collect snd parameters @ extra)
    }

    let private parameter (context: SchemaName) (path: DocumentPath) (node: YamlValue) : Result<Parameter * Schema list, ContractError> = result {
        let! entries = fields path node
        let! wire = required "name" path entries
        let! wire = scalar (memberPath "name" path) wire
        let! location = optional "in" path entries scalar "query"
        let! location =
            match location with
            | "path" -> Ok Path
            | "query" -> Ok Query
            | location -> Error(UnsupportedParameterLocation(memberPath "in" path, location))
        let! requiredValue = optional "required" path entries scalar "false"
        let! node = required "schema" path entries
        let! context = contextName context wire path
        let! shape, extra = shape context (memberPath "schema" path) node
        return { WireName = wire; Shape = shape; Required = requiredValue = "true"; Location = location }, extra
    }

    let private shape (context: SchemaName) (path: DocumentPath) (node: YamlValue) : Result<TypeShape * Schema list, ContractError> = result {
        let! entries = fields path node
        match find "$ref" entries with
        | Some target ->
            let! name = reference (memberPath "$ref" path) target
            return Named name, []
        | None ->
            match find "oneOf" entries with
            | Some cases ->
                let unionPath = memberPath "oneOf" path
                let! cases = sequence unionPath cases
                let present = cases |> List.mapi (fun index node -> index, node) |> List.filter (snd >> nullable >> not)
                match present, cases with
                | [(index, node)], [_; _] ->
                    let! value, extra = shape context (DocumentPath.itemAt index unionPath) node
                    return Optional value, extra
                | _, _ -> return! Error(UnsupportedUnion unionPath)
            | None ->
                let! kinds = kinds path entries
                let optional = List.contains "null" kinds
                let present = kinds |> List.filter ((<>) "null")
                let! value, extra = result {
                    match find "enum" entries with
                    | Some _ ->
                        let! definition, extra = schema context path node
                        return Named context, definition :: extra
                    | None ->
                        match present with
                        | ["array"] ->
                            let! node = required "items" path entries
                            let! value, extra = shape context (memberPath "items" path) node
                            return Many value, extra
                        | ["object"] ->
                            let! node = required "additionalProperties" path entries
                            let! value, extra = shape context (memberPath "additionalProperties" path) node
                            return StringMap value, extra
                        | [kind] ->
                            let! value = primitive path entries kind
                            return Primitive value, []
                        | kinds -> return! Error(UnsupportedType(path, kinds))
                }
                return (if optional then Optional value else value), extra
    }

    let private descendant (keys: string list) (path: DocumentPath) (node: YamlValue) : Result<YamlValue option, ContractError> =
        match keys with
        | [] -> Ok(Some node)
        | key :: keys -> fields path node |> Result.bind (fun entries ->
            match find key entries with
            | None -> Ok None
            | Some node -> descendant keys (memberPath key path) node)

    let private vocabulary (path: DocumentPath) (node: YamlValue) : Result<EnumCase list, ContractError> =
        stringValues path node |> Result.bind (fun values ->
            match values with
            | [] -> Error(EmptyVocabulary path)
            | values -> values |> traverse (fun wire -> SchemaName.create path wire |> Result.map (fun name -> { WireName = wire; Name = name })))

    let private reference (path: DocumentPath) (node: YamlValue) : Result<SchemaName, ContractError> =
        scalar path node |> Result.bind (fun value ->
            match value.Split('/') |> Array.toList with
            | ["#"; "components"; "schemas"; name] -> SchemaName.create path name
            | _ -> Error(InvalidReference(path, value)))

    let private kinds (path: DocumentPath) (entries: (string * YamlValue) list) : Result<string list, ContractError> =
        optional "type" path entries (fun path node ->
            match node with
            | Sequence _ -> stringValues path node
            | Scalar _ | Mapping _ -> scalar path node |> Result.map List.singleton) []

    let private primitive (path: DocumentPath) (entries: (string * YamlValue) list) (kind: string) : Result<Primitive, ContractError> =
        match kind with
        | "string" -> Ok Text
        | "integer" -> optional "format" path entries scalar "int32" |> Result.map (fun format -> if format = "int64" then Int64 else Int32)
        | "number" -> Ok Number
        | "boolean" -> Ok Boolean
        | _ -> Error(UnsupportedType(path, [kind]))

    let private nullable (node: YamlValue) =
        match node with
        | Mapping entries -> entries |> List.contains (Scalar "type", Scalar "null")
        | Scalar _ | Sequence _ -> false

    let private contextName (context: SchemaName) wire path = SchemaName.create path (SchemaName.text context + "." + wire)

    let private optional<'a> (key: string) (path: DocumentPath) (fields: (string * YamlValue) list) (read: DocumentPath -> YamlValue -> Result<'a, ContractError>) (fallback: 'a) : Result<'a, ContractError> =
        match find key fields with
        | None -> Ok fallback
        | Some value -> read (memberPath key path) value

    let private required (key: string) (path: DocumentPath) (fields: (string * YamlValue) list) : Result<YamlValue, ContractError> = find key fields |> Option.map Ok |> Option.defaultValue (Error(MissingField(path, key)))

    let private find (key: string) (fields: (string * YamlValue) list) : YamlValue option = fields |> List.tryPick (fun (name, value) -> if name = key then Some value else None)

    let private fields (path: DocumentPath) (node: YamlValue) : Result<(string * YamlValue) list, ContractError> =
        match node with
        | Mapping fields -> fields |> traverse (fun (key, value) -> scalar path key |> Result.map (fun key -> key, value))
        | Scalar _ | Sequence _ -> Error(ExpectedMapping path)

    let private stringValues (path: DocumentPath) (node: YamlValue) : Result<string list, ContractError> =
        sequence path node |> Result.bind (List.mapi (fun index value -> DocumentPath.itemAt index path, value) >> traverse (fun (path, value) -> scalar path value))

    let private sequence (path: DocumentPath) (node: YamlValue) : Result<YamlValue list, ContractError> =
        match node with
        | Sequence values -> Ok values
        | Scalar _ | Mapping _ -> Error(ExpectedSequence path)

    let private scalar (path: DocumentPath) (node: YamlValue) : Result<string, ContractError> =
        match node with
        | Scalar value -> Ok value
        | Sequence _ | Mapping _ -> Error(ExpectedScalar path)

    let private memberPath (key: string) (path: DocumentPath) = DocumentPath.memberAt key path

    let private traverse<'a, 'b, 'error> (read: 'a -> Result<'b, 'error>) (items: 'a list) : Result<'b list, 'error> =
        items |> List.fold (fun state item ->
            state |> Result.bind (fun values -> read item |> Result.map (fun value -> value :: values))) (Ok [])
        |> Result.map List.rev
