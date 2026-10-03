namespace BibleAtlas.FSharp

type JsonPath = private JsonPath of string

type JsonDiagnostic = { Path: JsonPath; Line: int64 option; Byte: int64 option; Message: string }

type WireFailure = InvalidSyntax of JsonDiagnostic | InvalidValue of JsonDiagnostic | NullPayload

module JsonPath =
    let ofLibraryPath path = JsonPath path
    let text (JsonPath path) = path

module WireFailure =
    let render failure =
        match failure with
        | InvalidSyntax diagnostic -> JsonPath.text diagnostic.Path + ": invalid JSON: " + diagnostic.Message
        | InvalidValue diagnostic -> JsonPath.text diagnostic.Path + ": invalid contract value: " + diagnostic.Message
        | NullPayload -> "$: the contract answer is null"
