namespace BibleAtlas.FSharp

open System
open System.Text.Json
open System.Text.Json.Serialization
open BibleAtlas.FSharp.Contract

type internal JsonAdapter private () =
    static let readOptions =
        let options = JsonSerializerOptions()
        IdentityConverters.all |> List.iter options.Converters.Add
        let fsharp = JsonFSharpOptions.Default().WithAllowOverride().WithSkippableOptionFields(SkippableOptionFields.Always, deserializeNullAsNone = true)
        options.Converters.Add(JsonFSharpConverter(fsharp))
        options

    static let writeOptions =
        let options = JsonSerializerOptions(readOptions)
        options.DefaultIgnoreCondition <- JsonIgnoreCondition.WhenWritingNull
        options

    static member Decode<'a>(body: string) : Result<'a, WireFailure> =
        let deserialize (element: JsonElement) =
            if element.ValueKind = JsonValueKind.Null then Error NullPayload
            else
                try Ok(JsonSerializer.Deserialize<'a>(element, readOptions))
                with :? JsonException as error -> Error(InvalidValue(JsonAdapter.Diagnostic error))
        try
            use document = JsonDocument.Parse body
            deserialize document.RootElement
        with :? JsonException as error -> Error(InvalidSyntax(JsonAdapter.Diagnostic error))

    static member Encode<'a>(value: 'a) = JsonSerializer.Serialize(value, writeOptions)

    static member private Diagnostic(error: JsonException) =
        let optional (value: Nullable<int64>) = if value.HasValue then Some(value.GetValueOrDefault()) else None
        { Path = JsonPath.ofLibraryPath (if isNull error.Path then "$" else error.Path)
          Line = optional error.LineNumber; Byte = optional error.BytePositionInLine
          Message = error.Message }
