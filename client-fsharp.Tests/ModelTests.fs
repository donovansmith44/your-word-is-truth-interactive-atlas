module BibleAtlas.FSharp.Tests.ModelTests

open System
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

let firstChapter = { Id = "Container:bible-chapter-GEN-1"; Title = "1"; Kind = ContentsChildKind.Chapter; Ref = "GEN.1"; Locus = TextRef.Bible { Book = BookId.GEN; Chapter = 1; Verse = 1 }; Count = 31 }
let contents = { Corpus = Corpus.Bible; Version = "root"; Roots = [{ Id = "Container:bible-book-GEN"; Title = "Genesis"; Kind = ContentsRootKind.Book; Group = Some Testament.OT; Ref = "GEN.1.1"; Locus = firstChapter.Locus; Children = [firstChapter] }] }

[<Fact>]
let ``reader startup requests the served contents before choosing its first reading`` () =
    let model, effects = Model.init Route.Reader
    Assert.Equal<Effect list>([ReadContents(Corpus.Bible, model.Serial)], effects)
    let actual, effects = Model.update (ContentsLoaded(Corpus.Bible, model.Serial, Ok contents)) model
    let expected = { model with Contents = Map.ofList [Corpus.Bible, Ready contents]; Reading = Loading(model.Serial, None) }
    Assert.Equal(expected, actual)
    Assert.Equal<Effect list>([ReadText(model.Serial, Reads.textWindow firstChapter.Ref None None (Some TextScope.Chapter) (Some Corpus.Bible))], effects)

[<Fact>]
let ``navigating away while text loads prevents its completion from reopening the old reading`` () =
    let model, _ = Model.init Route.Reader
    let model, _ = Model.update (ContentsLoaded(Corpus.Bible, model.Serial, Ok contents)) model
    let old = model.Serial
    let next, _ = Model.update (Navigate Route.Sources) model
    let stale, effects = Model.update (TextLoaded(old, Error(Transport "old failure"))) next
    Assert.Equal(next, stale)
    Assert.Equal<Effect list>([], effects)

[<Fact>]
let ``retry gives a failed source read a new request identity`` () =
    let model, _ = Model.init Route.Sources
    let model, _ = Model.update (SourcesLoaded(model.Serial, Error(Transport "offline"))) model
    let retry, effects = Model.update Retry model
    let expected = { model with Serial = RequestId.next model.Serial; Sources = Loading(RequestId.next model.Serial, None) }
    Assert.Equal(expected, retry)
    Assert.Equal<Effect list>([ReadSources retry.Serial], effects)

[<Theory>]
[<InlineData("/read/JHN/3", "JHN", 3)>]
[<InlineData("/read/1SA/2", "1SA", 2)>]
let ``reader routes roundtrip the generated book vocabulary`` (url: string) book chapter =
    let code = Json.decode<BookId>(Json.encode book) |> Result.toOption |> Option.get
    let expected = Route.Read { Book = code; Chapter = chapter }
    Assert.Equal(expected, Routes.parse (Uri("http://example.test" + url)))
    Assert.Equal(url, Routes.url expected)

[<Theory>]
[<InlineData("/read/NEW/3")>]
[<InlineData("/read/JHN/0")>]
[<InlineData("/read/JHN/nope")>]
[<InlineData("/unknown")>]
let ``unknown routes and invalid chapter input are refused without a partial match`` path =
    Assert.Equal(Route.NotFound, Routes.parse (Uri("http://example.test" + path)))

[<Fact>]
let ``Concord opens a bounded reading from the served corpus beginning`` () =
    let opening = { contents with Corpus = Corpus.Concord; Roots = [{ contents.Roots.Head with Ref = "BoC 1.1.1"; Locus = TextRef.Concord { Part = 1; Article = 1; Paragraph = 1 }; Children = [] }] }
    let model, _ = Model.init (Route.Concord None)
    let actual, effects = Model.update (ContentsLoaded(Corpus.Concord, model.Serial, Ok opening)) model
    let expected = { model with Contents = Map.ofList [Corpus.Concord, Ready opening]; Reading = Loading(model.Serial, None) }
    Assert.Equal(expected, actual)
    Assert.Equal<Effect list>([ReadText(model.Serial, Reads.textWindow "BoC 1.1.1" (Some 20) None None (Some Corpus.Concord))], effects)
