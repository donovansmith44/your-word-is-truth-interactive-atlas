module BibleAtlas.FSharp.Tests.AnchoredTextTests

open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

let locus = TextRef.Bible { Book = BookId.JHN; Chapter = 3; Verse = 16 }

[<Fact>]
let ``scalar offsets compose Unicode anchors within one red letter run`` () =
    let anchor = { Start = 2; End = 4; Kind = EdgeKind.Cites; Node = { Id = "text-unit:JHN.3.16"; Kind = NodeKind.TextUnit; Label = "John 3:16" } }
    let unit = { Text = "😀 ab cd"; Locus = locus; Anchors = [anchor]; WordsOfChrist = [{ Start = 2; End = 7 }] }
    let expected = [[{ Text = "😀 "; IsWordsOfChrist = false; Anchor = None }]; [{ Text = "ab"; IsWordsOfChrist = true; Anchor = Some anchor }; { Text = " cd"; IsWordsOfChrist = true; Anchor = None }]]
    Assert.Equal<TextPiece list list>(expected, AnchoredText.runs unit)

[<Fact>]
let ``an empty served text renders no invented run`` () =
    let unit = { Text = ""; Locus = locus; Anchors = []; WordsOfChrist = [] }
    Assert.Equal<TextPiece list list>([], AnchoredText.runs unit)

[<Fact>]
let ``two anchors and a crossing red letter boundary retain every served piece`` () =
    let first = { Start = 0; End = 3; Kind = EdgeKind.Mentions; Node = { Id = "Person:first"; Kind = NodeKind.Person; Label = "First" } }
    let second = { Start = 4; End = 7; Kind = EdgeKind.Mentions; Node = { Id = "Person:second"; Kind = NodeKind.Person; Label = "Second" } }
    let unit = { Text = "abc def"; Locus = locus; Anchors = [first; second]; WordsOfChrist = [{ Start = 2; End = 5 }] }
    let expected = [[{ Text = "ab"; IsWordsOfChrist = false; Anchor = Some first }]; [{ Text = "c"; IsWordsOfChrist = true; Anchor = Some first }; { Text = " "; IsWordsOfChrist = true; Anchor = None }; { Text = "d"; IsWordsOfChrist = true; Anchor = Some second }]; [{ Text = "ef"; IsWordsOfChrist = false; Anchor = Some second }]]
    Assert.Equal<TextPiece list list>(expected, AnchoredText.runs unit)
