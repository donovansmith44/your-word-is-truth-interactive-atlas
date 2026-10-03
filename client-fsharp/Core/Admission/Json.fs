namespace BibleAtlas.FSharp

open System
open System.Text.Json
open System.Text.Json.Serialization
open BibleAtlas.FSharp.Admission

module rec Json =
    let encode value = JsonSerializer.Serialize(value, options.Value.Write)

    let decode<'a> (body: string) : Result<'a, Failure> =
        match parse body with
        | Error failure -> Error(Failures.wire failure)
        | Ok document ->
            use document = document
            read<'a> body document.RootElement |> Result.mapError Failures.wire

    let private parse (body: string) : Result<JsonDocument, WireFailure> =
        try Ok(JsonDocument.Parse body)
        with :? JsonException as error -> Error(WireFailure.NotJson(location error))

    let private read<'a> (body: string) (answer: JsonElement) : Result<'a, WireFailure> =
        if answer.ValueKind = JsonValueKind.Null then Error WireFailure.NullAnswer
        else
            try Ok(JsonSerializer.Deserialize<'a>(body, options.Value.Read))
            with :? JsonException as error -> Error(WireFailure.UnreadableAnswer(location error))

    let private location (error: JsonException) : JsonErrorLocation =
        { Path = Option.ofObj error.Path
          LineNumber = Option.ofNullable error.LineNumber
          BytePositionInLine = Option.ofNullable error.BytePositionInLine }

    let private options: Lazy<Options> = lazy (
        let read = JsonFSharpOptions.Default().WithAllowOverride().WithSkippableOptionFields(SkippableOptionFields.Always, deserializeNullAsNone = true).ToJsonSerializerOptions()
        let inner = read.Converters[0] :?> JsonConverterFactory
        read.Converters[0] <-
            { new JsonConverterFactory() with
                override _.CanConvert shape =
                    let ownsConverter =
                        shape.GetCustomAttributes(typeof<JsonConverterAttribute>, false)
                        |> Array.exists (fun attribute -> attribute.GetType() = typeof<JsonConverterAttribute>)
                    not ownsConverter && inner.CanConvert shape
                override _.CreateConverter(shape, options) = inner.CreateConverter(shape, options) }
        let write = JsonSerializerOptions(read)
        write.DefaultIgnoreCondition <- JsonIgnoreCondition.WhenWritingNull
        { Read = read; Write = write })

    type private Options = { Read: JsonSerializerOptions; Write: JsonSerializerOptions }
