module BibleAtlas.FSharp.Tests.ContractTests

open System.IO
open System.Text.Json
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Fact>]
let ``text references are closed corpus cases and preserve the whole payload`` () =
    let body = """{"corpus":"bible","book":"JHN","chapter":3,"verse":16}"""
    let expected = TextRef.Bible { Book = BookId.JHN; Chapter = 3; Verse = 16 }
    Assert.Equal(Ok expected, Json.decode<TextRef> body)
    Assert.Equal(Ok expected, Json.decode<TextRef>(Json.encode expected))

[<Fact>]
let ``missing elements preserve their exact requested identity`` () =
    let expected = Element.Missing { Id = "Event:absent" }
    Assert.Equal(Ok expected, Json.decode<Element> """{"element":"missing","id":"Event:absent"}""")

[<Fact>]
let ``a null record cannot arrive as a successful contract answer`` () =
    Assert.True(Result.isError (Json.decode<NodeRef> "null"))

[<Fact>]
let ``unknown discriminators and vocabulary values are contract failures`` () =
    let bodies = ["""{"corpus":"unknown","book":"JHN","chapter":3,"verse":16}"""; """{"corpus":"bible","book":"NEW","chapter":3,"verse":16}"""]
    let refused = bodies |> List.map (Json.decode<TextRef> >> Result.isError)
    Assert.Equal<bool list>([true; true], refused)

[<Fact>]
let ``the committed text fixture roundtrips every published field`` () =
    let fixture = File.ReadAllText(Path.Combine(__SOURCE_DIRECTORY__, "../contracts/atlas-query-contract/fixtures/text-window-single.json"))
    let source = JsonSerializer.Deserialize<JsonElement>(fixture).GetProperty("body").GetRawText()
    match Json.decode<TextWindow> source with
    | Error failure -> Assert.Fail(string failure)
    | Ok text ->
        let normalized = Json.encode text
        let expected = JsonSerializer.Deserialize<JsonElement> source
        let actual = JsonSerializer.Deserialize<JsonElement> normalized
        Assert.True(JsonElement.DeepEquals(expected, actual), normalized)

[<Theory>]
[<InlineData("{\"id\":\"source\",\"category\":\"text\",\"title\":\"Title\",\"license\":\"PD\"}")>]
[<InlineData("{\"id\":\"source\",\"category\":\"text\",\"title\":null,\"license\":\"PD\",\"what_it_is\":\"Text\",\"what_we_built\":\"Reader\",\"licenses_row_key\":\"source\"}")>]
let ``required strings cannot disappear or arrive null`` body =
    Assert.True(Json.decode<SourceEntry> body |> Result.isError)
