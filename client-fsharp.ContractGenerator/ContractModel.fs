namespace BibleAtlas.FSharp.ContractGenerator

type ContractDocument = private ContractDocument of string

type GeneratedSource = private GeneratedSource of string

type SchemaName = private SchemaName of string

type PathSegment = Member of string | Item of int

type DocumentPath = private DocumentPath of PathSegment list

type YamlDiagnostic = { Line: int; Column: int; Message: string }

type ContractError =
    | InvalidYaml of YamlDiagnostic
    | NoSchemas of DocumentPath
    | ExpectedScalar of DocumentPath
    | ExpectedSequence of DocumentPath
    | ExpectedMapping of DocumentPath
    | MissingField of DocumentPath * string
    | UnsupportedType of DocumentPath * string list
    | UnsupportedUnion of DocumentPath
    | InvalidReference of DocumentPath * string
    | InvalidName of DocumentPath * string
    | EmptyVocabulary of DocumentPath
    | UnsupportedParameterLocation of DocumentPath * string

type YamlValue = Scalar of string | Sequence of YamlValue list | Mapping of (YamlValue * YamlValue) list

type Primitive = Text | Int32 | Int64 | Number | Boolean

type TypeShape = Primitive of Primitive | Named of SchemaName | Optional of TypeShape | Many of TypeShape | StringMap of TypeShape

type Field = { WireName: string; Name: SchemaName; Shape: TypeShape }

type EnumCase = { WireName: string; Name: SchemaName }

type TaggedCase = { Tag: EnumCase; Payload: SchemaName }

type SchemaDefinition =
    | Identity of Primitive
    | Alias of TypeShape
    | Record of Field list
    | Enumeration of EnumCase list
    | Tagged of discriminator: string * cases: TaggedCase list

type Schema = { Name: SchemaName; Definition: SchemaDefinition }

type ParameterLocation = Path | Query

type Parameter = { WireName: string; Shape: TypeShape; Required: bool; Location: ParameterLocation }

type Operation = { Name: SchemaName; Path: string; Response: TypeShape; Parameters: Parameter list }

type ContractModel = { Schemas: Schema list; Operations: Operation list }

module ContractDocument =
    let ofText text = ContractDocument text
    let text (ContractDocument text) = text

module GeneratedSource =
    let ofText text = GeneratedSource text
    let text (GeneratedSource text) = text

module DocumentPath =
    let root = DocumentPath []
    let memberAt key (DocumentPath segments) = DocumentPath(segments @ [Member key])
    let itemAt index (DocumentPath segments) = DocumentPath(segments @ [Item index])
    let render (DocumentPath segments) =
        segments
        |> List.fold (fun path segment ->
            match segment with
            | Member key -> path + "." + key
            | Item index -> path + $"[{index}]") "$"

module SchemaName =
    let create path wire =
        let words =
            wire |> Seq.fold (fun (finished, current) letter ->
                if System.Char.IsAsciiLetterOrDigit letter then finished, current + string letter
                else if current = "" then finished, current
                else current :: finished, "") ([], "")
        let finished, current = words
        let words = (if current = "" then finished else current :: finished) |> List.rev
        let capitalize (word: string) =
            match Seq.tryHead word with
            | None -> ""
            | Some first -> string (System.Char.ToUpperInvariant first) + word.Substring(1)
        let joined = words |> List.map capitalize |> String.concat ""
        match Seq.tryHead joined with
        | None -> Error(InvalidName(path, wire))
        | Some first -> Ok(SchemaName(if System.Char.IsDigit first then "N" + joined else joined))

    let text (SchemaName name) = name

module ContractError =
    let render error =
        match error with
        | InvalidYaml diagnostic -> $"$ at {diagnostic.Line}:{diagnostic.Column}: invalid YAML: {diagnostic.Message}"
        | NoSchemas path -> $"{DocumentPath.render path}: no schemas"
        | ExpectedScalar path -> $"{DocumentPath.render path}: expected scalar"
        | ExpectedSequence path -> $"{DocumentPath.render path}: expected sequence"
        | ExpectedMapping path -> $"{DocumentPath.render path}: expected mapping"
        | MissingField(path, key) -> $"{DocumentPath.render path}: missing {key}"
        | UnsupportedType(path, kinds) -> DocumentPath.render path + ": unsupported type " + String.concat "," kinds
        | UnsupportedUnion path -> $"{DocumentPath.render path}: unsupported union"
        | InvalidReference(path, reference) -> $"{DocumentPath.render path}: invalid reference {reference}"
        | InvalidName(path, wire) -> $"{DocumentPath.render path}: invalid name {wire}"
        | EmptyVocabulary path -> $"{DocumentPath.render path}: empty vocabulary"
        | UnsupportedParameterLocation(path, location) -> $"{DocumentPath.render path}: unsupported parameter location {location}"
