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

[<Theory>]
[<InlineData("contents-bible")>]
[<InlineData("contents-concord")>]
let ``each committed contents document roundtrips its complete served vocabulary`` name =
    let fixture = File.ReadAllText(Path.Combine(__SOURCE_DIRECTORY__, $"../contracts/atlas-query-contract/fixtures/{name}.json"))
    let source = JsonSerializer.Deserialize<JsonElement>(fixture).GetProperty("body").GetRawText()
    match Json.decode<Contents> source with
    | Error failure -> Assert.Fail(string failure)
    | Ok contents ->
        let actual = JsonSerializer.Deserialize<JsonElement>(Json.encode contents)
        Assert.True(JsonElement.DeepEquals(JsonSerializer.Deserialize<JsonElement>(source), actual), actual.GetRawText())

[<Fact>]
let ``a Concord text window accepts an explicitly null optional heading`` () =
    let body = """{"units":[{"ref":"BoC 1.1.1","node":{"id":"TextUnit:served-concord","kind":"TextUnit","label":"BoC 1.1.1"},"heading":null,"edge_summary":[{"kind":"cites","count":1}],"body":{"text":"Read John 3:16.","locus":{"article":1,"corpus":"concord","paragraph":1,"part":1},"anchors":[{"start":5,"end":14,"kind":"cites","node":{"id":"TextUnit:served-citation","kind":"TextUnit","label":"John 3:16"}}],"words_of_christ":[]}}],"next":null,"version":"root"}"""
    let expected: TextWindow = { Version = "root"; Next = None; Units = [{ Ref = "BoC 1.1.1"; Node = { Id = "TextUnit:served-concord"; Kind = NodeKind.TextUnit; Label = "BoC 1.1.1" }; Heading = None; EdgeSummary = [{ Kind = EdgeKind.Cites; Count = 1 }]; Body = { Text = "Read John 3:16."; Locus = TextRef.Concord { Article = 1; Paragraph = 1; Part = 1 }; Anchors = [{ Start = 5; End = 14; Kind = EdgeKind.Cites; Node = { Id = "TextUnit:served-citation"; Kind = NodeKind.TextUnit; Label = "John 3:16" } }]; WordsOfChrist = [] } }] }
    match Json.decode<TextWindow> body with
    | Ok actual -> Assert.Equal(expected, actual)
    | Error failure -> Assert.Fail(sprintf "%A" failure)
