module rec BibleAtlas.FSharp.Tests.ContractTests

open System.IO
open System.Text.Json
open System.Text.Json.Nodes
open Xunit
open FsCheck
open FsCheck.Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Property>]
let ``text references retain every field of either closed corpus case`` (book: BookId) (first: uint16) (second: uint16) (third: uint16) (bible: bool) =
    let first, second, third = int first + 1, int second + 1, int third + 1
    let body, expected =
        if bible then
            $"{{\"corpus\":\"bible\",\"book\":{Json.encode book},\"chapter\":{second},\"verse\":{third}}}", TextRef.Bible { Book = book; Chapter = second; Verse = third }
        else
            $"{{\"corpus\":\"concord\",\"part\":{first},\"article\":{second},\"paragraph\":{third}}}", TextRef.Concord { Part = first; Article = second; Paragraph = third }
    Assert.Equal((Ok expected, Ok expected), (Json.decode<TextRef> body, Json.decode<TextRef>(Json.encode expected)))

[<Property>]
let ``missing elements preserve the whole generated requested identity`` (suffix: uint16) =
    let id = $"Event:absent-{suffix}"
    let expected = Element.Missing { Id = WireFixtures.identity<ElementId> id }
    Assert.Equal(Ok expected, Json.decode<Element> $"{{\"element\":\"missing\",\"id\":{Json.encode id}}}")

[<Property>]
let ``a null contract record stays refused through every generated JSON whitespace envelope`` (left: byte) (right: byte) =
    let body = String.replicate (int left) " \t" + "null" + String.replicate (int right) "\r\n"
    Assert.Equal(true, Json.decode<NodeRef> body |> Result.isError)

[<Property>]
let ``unknown corpus discriminators and book vocabulary values stay contract failures`` (suffix: uint16) (book: BookId) =
    let unknown = $"unknown-{suffix}"
    let bodies = [$"{{\"corpus\":{Json.encode unknown},\"book\":{Json.encode book},\"chapter\":3,\"verse\":16}}"; $"{{\"corpus\":\"bible\",\"book\":{Json.encode unknown},\"chapter\":3,\"verse\":16}}"]
    Assert.Equal<bool list>([true; true], bodies |> List.map (Json.decode<TextRef> >> Result.isError))

[<Property>]
let ``the committed text fixture retains its complete served payload under generated artifact roots`` (NonNull root: NonNull<string>) =
    let source = fixture "text-window-single"
    source["version"] <- JsonValue.Create root
    match Json.decode<TextWindow> (source.ToJsonString()) with
    | Error failure -> Assert.Fail(string failure)
    | Ok text -> Assert.True(JsonElement.DeepEquals(JsonSerializer.Deserialize<JsonElement>(source.ToJsonString()), JsonSerializer.Deserialize<JsonElement>(Json.encode text)), Json.encode text)

[<Property>]
let ``both committed contents documents retain every served field under generated artifact roots`` (NonNull root: NonNull<string>) =
    for name in ["contents-bible"; "contents-concord"] do
        let source = fixture name
        source["version"] <- JsonValue.Create root
        match Json.decode<Contents> (source.ToJsonString()) with
        | Error failure -> Assert.Fail(string failure)
        | Ok contents -> Assert.True(JsonElement.DeepEquals(JsonSerializer.Deserialize<JsonElement>(source.ToJsonString()), JsonSerializer.Deserialize<JsonElement>(Json.encode contents)), Json.encode contents)

[<Property>]
let ``a Concord text window preserves its complete generated unit when the optional heading is null`` (suffix: uint16) (NonNull text: NonNull<string>) =
    let root = $"root-{suffix}"
    let id = $"TextUnit:served-concord-{suffix}"
    let citation = $"TextUnit:served-citation-{suffix}"
    let content = "Read John 3:16. " + text
    let body = $"{{\"units\":[{{\"ref\":\"BoC 1.1.1\",\"node\":{{\"id\":{Json.encode id},\"kind\":\"TextUnit\",\"label\":\"BoC 1.1.1\"}},\"heading\":null,\"edge_summary\":[{{\"kind\":\"cites\",\"count\":1}}],\"body\":{{\"text\":{Json.encode content},\"locus\":{{\"article\":1,\"corpus\":\"concord\",\"paragraph\":1,\"part\":1}},\"anchors\":[{{\"start\":5,\"end\":14,\"kind\":\"cites\",\"node\":{{\"id\":{Json.encode citation},\"kind\":\"TextUnit\",\"label\":\"John 3:16\"}}}}],\"words_of_christ\":[]}}}}],\"next\":null,\"version\":{Json.encode root}}}"
    let expected: TextWindow = { Version = WireFixtures.identity<ArtifactRoot> root; Next = None; Units = [{ Ref = WireFixtures.identity<UnitReference> "BoC 1.1.1"; Node = { Id = WireFixtures.identity<NodeId> id; Kind = NodeKind.TextUnit; Label = "BoC 1.1.1" }; Heading = None; EdgeSummary = [{ Kind = EdgeKind.Cites; Count = 1 }]; Body = { Text = content; Locus = TextRef.Concord { Article = 1; Paragraph = 1; Part = 1 }; Anchors = [{ Start = 5; End = 14; Kind = EdgeKind.Cites; Node = { Id = WireFixtures.identity<NodeId> citation; Kind = NodeKind.TextUnit; Label = "John 3:16" } }]; WordsOfChrist = [] } }] }
    Assert.Equal(Ok expected, Json.decode<TextWindow> body)

let private fixture (name: string) : JsonObject =
    let source = File.ReadAllText(Path.Combine(__SOURCE_DIRECTORY__, $"../contracts/atlas-query-contract/fixtures/{name}.json"))
    let parsed = JsonNode.Parse source
    parsed["body"].AsObject()
