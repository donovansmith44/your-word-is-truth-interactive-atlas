module BibleAtlas.FSharp.Tests.ModelTests

open System
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

let firstChapter = { Id = "Container:bible-chapter-GEN-1"; Title = "1"; Kind = ContentsChildKind.Chapter; Ref = "GEN.1"; Locus = TextRef.Bible { Book = BookId.GEN; Chapter = 1; Verse = 1 }; Count = 31 }
let contents = { Corpus = Corpus.Bible; Version = "root"; Roots = [{ Id = "Container:bible-book-GEN"; Title = "Genesis"; Kind = ContentsRootKind.Book; Group = Some Testament.OT; Ref = "GEN.1.1"; Locus = firstChapter.Locus; Children = [firstChapter] }] }

let withReading window (model: Model) =
    let read =
        match model.Route with
        | Route.Concord reference -> Reads.textWindow (reference |> Option.defaultValue "BoC 1.1.1") (Some 20) None None (Some Corpus.Concord)
        | _ -> Reads.textWindow firstChapter.Ref None None (Some TextScope.Chapter) (Some Corpus.Bible)
    let session = ReadSession.beginRead model.Serial read Empty |> ReadSession.complete model.Serial (Ok window)
    { model with ReadingSession = ReadingState.Active session }

[<Fact>]
let ``reader startup requests the served contents before choosing its first reading`` () =
    let model, effects = Model.init Route.Reader
    Assert.Equal<Effect list>([ReadContents(Corpus.Bible, model.Serial)], effects)
    let actual, effects = Model.update (ContentsLoaded(Corpus.Bible, model.Serial, Ok contents)) model
    let expected = { model with Contents = Map.ofList [Corpus.Bible, Ready contents]; ReadingSession = ReadingState.Active(ReadSession.beginRead model.Serial (Reads.textWindow firstChapter.Ref None None (Some TextScope.Chapter) (Some Corpus.Bible)) Empty) }
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
    let expected = { model with Contents = Map.ofList [Corpus.Concord, Ready opening]; ReadingSession = ReadingState.Active(ReadSession.beginRead model.Serial (Reads.textWindow "BoC 1.1.1" (Some 20) None None (Some Corpus.Concord)) Empty) }
    Assert.Equal(expected, actual)
    Assert.Equal<Effect list>([ReadText(model.Serial, Reads.textWindow "BoC 1.1.1" (Some 20) None None (Some Corpus.Concord))], effects)

[<Fact>]
let ``opening a focus has a new identity and resolves only its requested position`` () =
    let model, _ = Model.init Route.Sources
    let position = Resolved.position ExplorationTests.start
    let actual, effects = Model.update (OpenPosition position) model
    let request = RequestId.next model.Serial
    Assert.Equal({ model with Serial = request; Focus = FocusState.Opening(request, position) }, actual)
    Assert.Equal<Effect list>([ReadOpening(request, position)], effects)

[<Fact>]
let ``closing focus refuses a late successful opening`` () =
    let model, _ = Model.init Route.Sources
    let opening, _ = Model.update (OpenPosition(Resolved.position ExplorationTests.start)) model
    let closed, _ = Model.update CloseFocus opening
    let actual, effects = Model.update (FocusLoaded(opening.Serial, Ok ExplorationTests.trail)) closed
    Assert.Equal({ opening with Serial = RequestId.next opening.Serial; Focus = FocusState.Closed }, actual)
    Assert.Equal<Effect list>([], effects)

[<Fact>]
let ``following preserves the current trail while its replacement is in flight`` () =
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened ExplorationTests.trail }
    let link: Link = { Kind = EdgeKind.Contains; Target = Resolved.position (ExplorationTests.node "Person:target" "root") }
    let actual, effects = Model.update (Traverse(Traversal.Follow link)) model
    let request = RequestId.next model.Serial
    Assert.Equal({ model with Serial = request; Focus = FocusState.Walking(request, ExplorationTests.trail, Traversal.Follow link) }, actual)
    Assert.Equal<Effect list>([WalkFocus(request, ExplorationTests.trail, Traversal.Follow link)], effects)

[<Fact>]
let ``a superseded focus completion cannot replace a later traversal`` () =
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened ExplorationTests.trail }
    let first, _ = Model.update (Traverse Traversal.Back) model
    let next, _ = Model.update (Traverse Traversal.Renew) first
    let actual, effects = Model.update (FocusLoaded(first.Serial, Error(Transport "old"))) next
    Assert.Equal(next, actual)
    Assert.Equal<Effect list>([], effects)

[<Fact>]
let ``a failed opening retains exactly the position needed by Retry`` () =
    let model, _ = Model.init Route.Sources
    let position = Resolved.position ExplorationTests.start
    let opening, _ = Model.update (OpenPosition position) model
    let failed, _ = Model.update (FocusLoaded(opening.Serial, Error(Transport "offline"))) opening
    Assert.Equal({ opening with Focus = FocusState.CouldNotOpen(position, Transport "offline") }, failed)
    let retry, effects = Model.update RetryFocus failed
    let request = RequestId.next failed.Serial
    Assert.Equal({ failed with Serial = request; Focus = FocusState.Opening(request, position) }, retry)
    Assert.Equal<Effect list>([ReadOpening(request, position)], effects)

[<Fact>]
let ``Concord Next reads only the served next reference with a bounded page`` () =
    let model, _ = Model.init (Route.Concord(Some "BoC 1.1.1"))
    let window: TextWindow = { Units = []; Next = Some "BoC 1.1.21"; Version = "root" }
    let model = withReading window model
    let actual, effects = Model.update ReadNext model
    let request = RequestId.next model.Serial
    Assert.Equal({ model with Serial = request; ReadingSession = ReadingState.Active(ReadSession.beginRead request (Reads.textWindow "BoC 1.1.21" (Some 20) (Some WindowDir.Onward) None (Some Corpus.Concord)) (Ready window)); Focus = FocusState.Closed }, actual)
    Assert.Equal<Effect list>([ReadText(request, Reads.textWindow "BoC 1.1.21" (Some 20) (Some WindowDir.Onward) None (Some Corpus.Concord))], effects)

[<Fact>]
let ``retry after a failed Concord page retains that page instead of reopening the route`` () =
    let model, _ = Model.init (Route.Concord(Some "BoC 1.1.1"))
    let window: TextWindow = { Units = []; Next = Some "BoC 1.1.21"; Version = "root" }
    let model = withReading window model
    let next, _ = Model.update ReadNext model
    let failed, _ = Model.update (TextLoaded(next.Serial, Error(Transport "offline"))) next
    let actual, effects = Model.update Retry failed
    let request = RequestId.next failed.Serial
    Assert.Equal({ failed with Serial = request; ReadingSession = ReadingState.Active(ReadSession.beginRead request (Reads.textWindow "BoC 1.1.21" (Some 20) (Some WindowDir.Onward) None (Some Corpus.Concord)) (Ready window)) }, actual)
    Assert.Equal<Effect list>([ReadText(request, Reads.textWindow "BoC 1.1.21" (Some 20) (Some WindowDir.Onward) None (Some Corpus.Concord))], effects)

[<Fact>]
let ``a chapter absent from the served contents is an explicit unavailable reading`` () =
    let model, _ = Model.init (Route.Read { Book = BookId.GEN; Chapter = 2 })
    let actual, effects = Model.update (ContentsLoaded(Corpus.Bible, model.Serial, Ok contents)) model
    let expected = { model with Contents = Map.ofList [Corpus.Bible, Ready contents]; ReadingSession = ReadingState.Unavailable(model.Serial, Contract "the requested reading has no opening in the served contents") }
    Assert.Equal(expected, actual)
    Assert.Equal<Effect list>([], effects)

[<Fact>]
let ``a duplicate Next while a page is pending leaves the whole reading unchanged`` () =
    let model, _ = Model.init (Route.Concord None)
    let model = withReading { Units = []; Next = Some "BoC 1.1.21"; Version = "root" } model
    let pending, _ = Model.update ReadNext model
    let actual, effects = Model.update ReadNext pending
    Assert.Equal(pending, actual)
    Assert.Equal<Effect list>([], effects)

[<Fact>]
let ``a stale page completion cannot replace a newer retry`` () =
    let model, _ = Model.init (Route.Concord None)
    let model = withReading { Units = []; Next = Some "BoC 1.1.21"; Version = "root" } model
    let pending, _ = Model.update ReadNext model
    let failed, _ = Model.update (TextLoaded(pending.Serial, Error(Transport "offline"))) pending
    let retry, _ = Model.update Retry failed
    let actual, effects = Model.update (TextLoaded(pending.Serial, Ok { Units = []; Next = None; Version = "stale" })) retry
    Assert.Equal(retry, actual)
    Assert.Equal<Effect list>([], effects)

[<Fact>]
let ``ten thousand Concord turns retain exactly the last page`` () =
    let pageCount = 10_000
    let read = Reads.textWindow "opening" (Some 20) None None (Some Corpus.Concord)
    let page index : TextWindow =
        { Units = [{ Ref = $"page-{index}"; Node = { Id = $"TextUnit:page-{index}"; Kind = NodeKind.TextUnit; Label = $"Page {index}" }; Heading = None; EdgeSummary = []; Body = { Locus = TextRef.Concord { Part = 1; Article = 1; Paragraph = index }; Text = $"Page {index} body"; Anchors = []; WordsOfChrist = [] } }]
          Next = Some $"next-{index}"; Version = "root" }
    let first = page 0
    let model, _ = Model.init (Route.Concord None)
    let initial = ReadSession.beginRead model.Serial read Empty |> ReadSession.complete model.Serial (Ok first)
    let model = { model with ReadingSession = ReadingState.Active initial }
    let actual =
        [1..pageCount] |> List.fold (fun model index ->
            let pending, _ = Model.update ReadNext model
            let window = page index
            Model.update (TextLoaded(pending.Serial, Ok window)) pending |> fst) model
    let expectedRequest = [1..pageCount] |> List.fold (fun request _ -> RequestId.next request) model.Serial
    let lastRead = Reads.textWindow $"next-{pageCount - 1}" (Some 20) (Some WindowDir.Onward) None (Some Corpus.Concord)
    let lastWindow = page pageCount
    let lastSession = ReadSession.beginRead expectedRequest lastRead Empty |> ReadSession.complete expectedRequest (Ok lastWindow)
    Assert.Equal({ model with Serial = expectedRequest; ReadingSession = ReadingState.Active lastSession }, actual)
