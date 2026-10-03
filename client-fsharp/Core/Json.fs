namespace BibleAtlas.FSharp

open System.Text.Json
open System.Text.Json.Serialization

module Json =
    let private readOptions =
        JsonFSharpOptions.Default().WithAllowOverride().WithSkippableOptionFields(SkippableOptionFields.Always).ToJsonSerializerOptions()

    let private writeOptions =
        let options = JsonSerializerOptions(readOptions)
        options.DefaultIgnoreCondition <- JsonIgnoreCondition.WhenWritingNull
        options

    let encode value = JsonSerializer.Serialize(value, writeOptions)

    let decode<'a> (body: string) : Result<'a, Failure> =
        try Ok(JsonSerializer.Deserialize<'a>(body, readOptions))
        with :? JsonException as error -> Error(Contract error.Message)
