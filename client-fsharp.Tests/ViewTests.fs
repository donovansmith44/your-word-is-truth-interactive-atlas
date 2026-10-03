module rec BibleAtlas.FSharp.Tests.ViewTests

open Bunit
open Xunit
open Microsoft.Extensions.DependencyInjection
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Client
open BibleAtlas.FSharp.Contract

[<Fact>]
let ``the reader renders the whole served chapter with anchored red letter text`` () =
    let model, _ = Model.init Route.Reader
    let model = ModelTests.withReading { Units = [readingUnit]; Next = None; Version = "root" } { model with Contents = Map.ofList [Corpus.Bible, Ready ModelTests.contents] }
    use context = new BunitContext()
    let view = render context model ignore
    let expected = """<article class="reader-column"><h1 class="chapter-head"><span class="chapter-head-book">Genesis</span><span class="chapter-head-num">1</span></h1><div class="verse-line explorable" data-testid="verse-line-1" data-focal="false" id="v1" tabindex="0" role="button" aria-label="Explore Genesis 1:1"><button type="button" class="verse-num" data-testid="verse-num-1">1</button><span class="verse-text">😀 <span class="words-of-christ"><span class="verse-mention" data-testid="verse-mention-1-Place:served-place" tabindex="0" role="button" aria-label="Explore ab">ab</span> cd</span></span></div></article>"""
    (find view ".reader-column").MarkupMatches(expected)

[<Theory>]
[<InlineData("Enter")>]
[<InlineData(" ")>]
let ``a served reader anchor opens its typed position from the keyboard`` key =
    let model, _ = Model.init Route.Reader
    let model = ModelTests.withReading { Units = [readingUnit]; Next = None; Version = "root" } model
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view ".verse-mention").KeyDown(Microsoft.AspNetCore.Components.Web.KeyboardEventArgs(Key = key))
    Assert.Equal<Message list>([OpenPosition(PositionRef.Node { Node = readingAnchor.Node })], messages)

[<Fact>]
let ``the reader shows a failed contents read with an explicit Retry`` () =
    let model, _ = Model.init Route.Reader
    let model = { model with Contents = Map.ofList [Corpus.Bible, Failed(model.Serial, Transport "offline", None)] }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- message :: messages)
    (find view "[data-testid='could-not-load-retry']").Click()
    Assert.Equal<Message list>([Retry], messages)

[<Fact>]
let ``the Concord view renders the entire served paragraph and citation`` () =
    let model, _ = Model.init (Route.Concord None)
    let unit = { readingUnit with Ref = "BoC 1.1.1"; Body = { readingUnit.Body with Locus = TextRef.Concord { Part = 1; Article = 1; Paragraph = 1 }; WordsOfChrist = [] }; Node = { readingUnit.Node with Label = "BoC 1.1.1" }; EdgeSummary = [{ Kind = EdgeKind.Cites; Count = 1 }] }
    let model = ModelTests.withReading { Units = [unit]; Next = None; Version = "root" } model
    use context = new BunitContext()
    let view = render context model ignore
    let expected = """<div class="concord-unit explorable" data-testid="concord-unit-TextUnit:served-verse" tabindex="0" role="button" aria-label="Explore BoC 1.1.1"><span class="concord-unit-ref">BoC 1.1.1</span>😀 <span class="concord-ref" data-testid="concord-ref-TextUnit:served-verse-2" tabindex="0" role="button" aria-label="Explore ab">ab</span> cd</div>"""
    (find view ".concord-unit").MarkupMatches(expected)

[<Fact>]
let ``activating a served reader row dispatches exactly its typed position`` () =
    let model, _ = Model.init Route.Reader
    let model = ModelTests.withReading { Units = [readingUnit]; Next = None; Version = "root" } model
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view ".verse-line").Click()
    Assert.Equal<Message list>([OpenPosition(PositionRef.Node { Node = readingUnit.Node })], messages)

[<Fact>]
let ``the reader retains a continued quiet heading and its served target`` () =
    let event = { Id = "Event:served-heading"; Kind = NodeKind.Event; Label = "Served heading" }
    let unit = { readingUnit with Heading = Some { Event = event; IsContinuation = true; Kind = EventKind.General } }
    let model, _ = Model.init Route.Reader
    let model = ModelTests.withReading { Units = [unit]; Next = None; Version = "root" } model
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    let heading = find view ".pericope-heading"
    heading.MarkupMatches("""<h2 class="pericope-heading pericope-heading-continuation explorable-quiet" data-testid="pericope-heading-Event:served-heading" data-continuation="true" tabindex="0" role="button" aria-label="Explore Served heading (continued)"><span class="pericope-heading-continuation-marker" data-testid="pericope-heading-continuation-marker-Event:served-heading">continued</span>Served heading</h2>""")
    heading.Click()
    Assert.Equal<Message list>([OpenPosition(PositionRef.Node { Node = event })], messages)

[<Fact>]
let ``the reader renders an event heading without an invented continuation`` () =
    let event = { Id = "Event:served-heading"; Kind = NodeKind.Event; Label = "Served heading" }
    let unit = { readingUnit with Heading = Some { Event = event; IsContinuation = false; Kind = EventKind.Event } }
    let model, _ = Model.init Route.Reader
    let model = ModelTests.withReading { Units = [unit]; Next = None; Version = "root" } model
    use context = new BunitContext()
    let view = render context model ignore
    (find view ".pericope-heading").MarkupMatches("""<h2 class="pericope-heading explorable" data-testid="pericope-heading-Event:served-heading" data-continuation="false" tabindex="0" role="button" aria-label="Explore Served heading">Served heading</h2>""")

[<Fact>]
let ``unrelated typing over a reader anchor does not open a focus`` () =
    let model, _ = Model.init Route.Reader
    let model = ModelTests.withReading { Units = [readingUnit]; Next = None; Version = "root" } model
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view ".verse-mention").KeyDown(Microsoft.AspNetCore.Components.Web.KeyboardEventArgs(Key = "a"))
    Assert.Equal<Message list>([], messages)

[<Fact>]
let ``the Concord view keeps a paragraph with no served edges as plain text`` () =
    let model, _ = Model.init (Route.Concord None)
    let unit = { readingUnit with Ref = "BoC 1.1.1"; Body = { readingUnit.Body with Locus = TextRef.Concord { Part = 1; Article = 1; Paragraph = 1 }; Anchors = []; WordsOfChrist = [] } }
    let model = ModelTests.withReading { Units = [unit]; Next = None; Version = "root" } model
    use context = new BunitContext()
    let view = render context model ignore
    (find view ".concord-unit").MarkupMatches("""<div class="concord-unit" data-testid="concord-unit-TextUnit:served-verse"><span class="concord-unit-ref">BoC 1.1.1</span>😀 ab cd</div>""")

[<Fact>]
let ``the served Concord continuation exposes Next through an Elmish message`` () =
    let model, _ = Model.init (Route.Concord None)
    let model = ModelTests.withReading { Units = []; Next = Some "BoC 1.1.21"; Version = "root" } model
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    let button = find view "[data-testid='concord-next']"
    button.MarkupMatches("""<button type="button" class="concord-nav-button" data-testid="concord-next">Next ›</button>""")
    button.Click()
    Assert.Equal<Message list>([ReadNext], messages)

[<Fact>]
let ``a terminal Concord page offers no invented continuation`` () =
    let model, _ = Model.init (Route.Concord None)
    let model = ModelTests.withReading { Units = []; Next = None; Version = "root" } model
    use context = new BunitContext()
    let view = render context model ignore
    Assert.Empty(RenderedComponentExtensions.FindAll<AppView>(view, "[data-testid='concord-next']"))

[<Fact>]
let ``an opened focus renders its entire card presentation`` () =
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened ExplorationTests.trail }
    use context = new BunitContext()
    let view = render context model ignore
    let expected = """<div class="popover-body" data-testid="popover-body"><div class="popover-section" data-testid="popover-section-card"><p class="focus-title" data-testid="popover-card-title">Person:start</p><dl class="focus-fields"><div class="focus-field" data-testid="popover-field-Provenance"><dt>Provenance</dt><dd>test</dd></div></dl></div></div>"""
    (find view "[data-testid='popover-body']").MarkupMatches(expected)

[<Fact>]
let ``closing a visible focus dispatches CloseFocus`` () =
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened ExplorationTests.trail }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view "[data-testid='popover-close']").Click()
    Assert.Equal<Message list>([CloseFocus], messages)

[<Fact>]
let ``a failed focus retries its own request rather than the underlying page`` () =
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.CouldNotOpen(Resolved.position ExplorationTests.start, Transport "offline") }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view "[data-testid='popover-body'] [data-testid='could-not-load-retry']").Click()
    Assert.Equal<Message list>([RetryFocus], messages)

[<Fact>]
let ``Back in the popover dispatches the exploration operation`` () =
    let target = ExplorationTests.node "Person:target" "root"
    let trail = Trail.follow { Kind = EdgeKind.Contains; Target = target } ExplorationTests.trail
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened trail }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view "[data-testid='popover-breadcrumb-back']").Click()
    Assert.Equal<Message list>([Traverse Traversal.Back], messages)

[<Fact>]
let ``a text presentation follows its served anchor within the current exploration`` () =
    let record = { PresentationTests.node with Kind = NodeKind.TextUnit; Text = Some readingUnit.Body }
    let trail = Trail.beginAt (PresentationTests.resolved record)
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened trail }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    let expected = """<p class="focus-text" data-testid="popover-text">😀 <span class="words-of-christ"><button type="button" class="focus-anchor explorable" data-testid="popover-anchor-Place:served-place-2">ab</button> cd</span></p>"""
    (find view "[data-testid='popover-text']").MarkupMatches(expected)
    (find view ".focus-anchor").Click()
    Assert.Equal<Message list>([Traverse(Traversal.Follow { Kind = readingAnchor.Kind; Target = PositionRef.Node { Node = readingAnchor.Node } })], messages)

[<Fact>]
let ``Escape closes the focus without changing the route`` () =
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened ExplorationTests.trail }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view "[data-testid='popover']").KeyDown(Microsoft.AspNetCore.Components.Web.KeyboardEventArgs(Key = "Escape"))
    Assert.Equal<Message list>([CloseFocus], messages)

[<Fact>]
let ``an invalid text presentation retries by renewing the exploration`` () =
    let trail = Trail.beginAt (PresentationTests.resolved { PresentationTests.node with Kind = NodeKind.TextUnit })
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened trail }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view "[data-testid='popover-body'] [data-testid='could-not-load-retry']").Click()
    Assert.Equal<Message list>([Traverse Traversal.Renew], messages)

[<Fact>]
let ``the source view composes the complete served source under its category`` () =
    let source = { Id = "test"; Category = "text"; Title = "Served source"; WhatItIs = "What it is"; WhatWeBuilt = "What we built"; License = "CC0"; LicensesRowKey = "test"; Link = Some "https://example.test/" }
    let model, _ = Model.init Route.Sources
    let model = { model with Sources = Ready { Categories = [{ Id = "text"; Label = "Texts" }]; Sources = [source]; Provenances = None } }
    use context = new BunitContext()
    let view = render context model (fun (_: Message) -> ())
    let expected = """<article class="source-card" data-testid="source-test"><h3 class="source-title">Served source</h3><p class="source-what">What it is</p><p class="source-built"><span class="source-label">What we built:</span> What we built</p><p class="source-license"><span class="source-label">License:</span> CC0</p><a class="source-link" data-testid="source-link-test" href="https://example.test/" target="_blank" rel="noopener noreferrer">Visit source</a></article>"""
    Assert.Equal(expected, (find view "[data-testid='source-test']").OuterHtml)

[<Fact>]
let ``an explicit failure offers Retry and dispatches its message`` () =
    let model, _ = Model.init Route.Sources
    let model = { model with Sources = Failed(model.Serial, Transport "offline", None) }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- message :: messages)
    (find view "[data-testid='could-not-load-retry']").Click()
    Assert.Equal<Message list>([Retry], messages)

[<Fact>]
let ``the shared header retains primary navigation translation and credits`` () =
    let model, _ = Model.init Route.Sources
    use context = new BunitContext()
    let view = render context model (fun (_: Message) -> ())
    let expected = """<header class="app-header header-parchment"><a class="wordmark" href="/">Bible Explorer <span class="wordmark-glyph" aria-hidden="true">∴</span></a><nav class="app-nav" aria-label="Primary"><a class="nav-link" href="/" data-testid="nav-reader">Reader</a><a class="nav-link" href="/world" data-testid="nav-world">World</a><a class="nav-link" href="/kretzmann" data-testid="nav-kretzmann">Kretzmann</a><a class="nav-link" href="/concord" data-testid="nav-concord">Concord</a></nav><div class="header-tools"><label class="translation-label" for="translation-select">Translation</label><select id="translation-select" class="translation-select" data-testid="translation-select"><option value="kjv">KJV</option></select><a class="attribution" data-testid="attribution" href="/sources">Credits</a></div></header>"""
    Assert.Equal(expected, (find view "header").OuterHtml)

[<Fact>]
let ``the WebAssembly entry component runs the sources command and renders its response`` () =
    use context = new BunitContext()
    context.JSInterop.Mode <- JSRuntimeMode.Loose
    let document = { Categories = [{ Id = "text"; Label = "Texts" }]; Sources = []; Provenances = None }
    use response = new System.Net.Http.HttpResponseMessage(System.Net.HttpStatusCode.OK, Content = new System.Net.Http.StringContent(Json.encode document))
    use handler = new TransportTests.Handler(response)
    use http = new System.Net.Http.HttpClient(handler, BaseAddress = System.Uri "http://example.test/")
    context.Services.AddSingleton<System.Net.Http.HttpClient>(http) |> ignore
    context.Renderer.SetRendererInfo(Microsoft.AspNetCore.Components.RendererInfo("WebAssembly", true))
    let navigation = context.Services.GetRequiredService<Microsoft.AspNetCore.Components.NavigationManager>()
    navigation.NavigateTo("/sources")
    let view = context.Render<App>()
    view.WaitForAssertion(fun () -> Assert.Equal("Texts", (find view ".sources-category-title").TextContent))
    navigation.NavigateTo("/not-found")
    view.WaitForAssertion(fun () -> Assert.Empty(RenderedComponentExtensions.FindAll<App>(view, ".sources-category-title")))

let render (context: BunitContext) (model: Model) (dispatch: Message -> unit) : IRenderedComponent<AppView> =
    context.Render<AppView>(Microsoft.AspNetCore.Components.RenderFragment(fun builder ->
        builder.OpenComponent<AppView>(0)
        builder.AddAttribute(1, "Model", model)
        builder.AddAttribute(2, "Dispatch", dispatch)
        builder.CloseComponent()))

let find<'a when 'a :> Microsoft.AspNetCore.Components.IComponent> (view: IRenderedComponent<'a>) (selector: string) : AngleSharp.Dom.IElement =
    RenderedComponentExtensions.Find<'a>(view, selector)
let readingAnchor: Anchor = { Start = 2; End = 4; Kind = EdgeKind.Mentions; Node = { Id = "Place:served-place"; Kind = NodeKind.Place; Label = "Served place" } }
let readingUnit: TextUnit = { Ref = "GEN.1.1"; Node = { Id = "TextUnit:served-verse"; Kind = NodeKind.TextUnit; Label = "Genesis 1:1" }; Heading = None; EdgeSummary = []; Body = { Text = "😀 ab cd"; Locus = ModelTests.firstChapter.Locus; Anchors = [readingAnchor]; WordsOfChrist = [{ Start = 2; End = 7 }] } }
