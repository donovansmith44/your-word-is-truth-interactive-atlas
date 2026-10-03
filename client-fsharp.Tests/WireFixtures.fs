namespace BibleAtlas.FSharp.Tests

open System.Text.Json

module WireFixtures =
    let identity<'a> (value: obj) : 'a = JsonSerializer.Deserialize<'a>(JsonSerializer.Serialize value)
