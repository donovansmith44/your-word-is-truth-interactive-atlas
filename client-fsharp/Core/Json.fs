namespace BibleAtlas.FSharp

open System
open System.Text.Json
open System.Text.Json.Serialization

module rec Json =
    let encode value = JsonSerializer.Serialize(value, options.Value.Write)

    let decode<'a> (body: string) : Result<'a, Failure> =
        try Ok(JsonSerializer.Deserialize<'a>(body, options.Value.Read))
        with :? JsonException as error -> Error(Contract error.Message)

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
