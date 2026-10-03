module rec BibleAtlas.FSharp.Tests.ViewTests

open Bunit
open Xunit
open FsCheck.Xunit
open Microsoft.Extensions.DependencyInjection
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Client
open BibleAtlas.FSharp.Contract

[<Property>]
let ``the reader renders the whole served chapter with anchored red letter text`` (suffix: uint16) =
    let readingUnit = readingUnit suffix
    let model, _ = Model.init Route.Reader
    let model = ModelTests.withReading { Units = [readingUnit]; Next = None; Version = (WireFixtures.identity<ArtifactRoot> $"root-{suffix}") } model
    use context = new BunitContext()
    let view = render context model ignore
    let expected = $"""<article class="reader-column"><h1 class="chapter-head"><span class="chapter-head-book">Genesis</span><span class="chapter-head-num">1</span></h1><div class="verse-line explorable" data-testid="verse-line-1" data-focal="false" id="v1" tabindex="0" role="button" aria-label="Explore Genesis 1:1"><button type="button" class="verse-num" data-testid="verse-num-1">1</button><span class="verse-text">😀 <span class="words-of-christ"><span class="verse-mention" data-testid="verse-mention-1-Place:served-place-{suffix}" tabindex="0" role="button" aria-label="Explore ab">ab</span> cd</span>-{suffix}</span></div></article>"""
    (find view ".reader-column").MarkupMatches(expected)

[<Property>]
let ``a served reader anchor opens its typed position from the keyboard`` (suffix: uint16) (enter: bool) =
    let readingUnit = readingUnit suffix
    let readingAnchor = readingAnchor suffix
    let key = if enter then "Enter" else " "
    let model, _ = Model.init Route.Reader
    let model = ModelTests.withReading { Units = [readingUnit]; Next = None; Version = (WireFixtures.identity<ArtifactRoot> $"root-{suffix}") } model
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view ".verse-mention").KeyDown(Microsoft.AspNetCore.Components.Web.KeyboardEventArgs(Key = key))
    Assert.Equal<Message list>([OpenPosition(PositionRef.Node { Node = readingAnchor.Node })], messages)

[<Property>]
let ``the reader shows a failed contents read with an explicit Retry`` (suffix: uint16) =
    let model, _ = Model.init Route.Reader
    let model = { model with Surface = Surface.Reader(ModelTests.readerPage None (ReadingState.CouldNotLoadContents(Transport $"offline-{suffix}"))) }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- message :: messages)
    (find view "[data-testid='could-not-load-retry']").Click()
    Assert.Equal<Message list>([Page(SurfaceMessage.Reader ReadingMessage.RetryContents)], messages)

[<Property>]
let ``the Concord view renders the entire served paragraph and citation`` (suffix: uint16) =
    let readingUnit = readingUnit suffix
    let model, _ = Model.init (Route.Concord None)
    let unit = { readingUnit with Ref = (WireFixtures.identity<UnitReference> "BoC 1.1.1"); Body = { readingUnit.Body with Locus = TextRef.Concord { Part = 1; Article = 1; Paragraph = 1 }; WordsOfChrist = [] }; Node = { readingUnit.Node with Label = "BoC 1.1.1" }; EdgeSummary = [{ Kind = EdgeKind.Cites; Count = 1 }] }
    let model = ModelTests.withReading { Units = [unit]; Next = None; Version = (WireFixtures.identity<ArtifactRoot> $"root-{suffix}") } model
    use context = new BunitContext()
    let view = render context model ignore
    let expected = $"""<div class="concord-unit explorable" data-testid="concord-unit-TextUnit:served-verse-{suffix}" tabindex="0" role="button" aria-label="Explore BoC 1.1.1"><span class="concord-unit-ref">BoC 1.1.1</span>😀 <span class="concord-ref" data-testid="concord-ref-TextUnit:served-verse-{suffix}-2" tabindex="0" role="button" aria-label="Explore ab">ab</span> cd-{suffix}</div>"""
    (find view ".concord-unit").MarkupMatches(expected)

[<Property>]
let ``activating a served reader row dispatches exactly its typed position`` (suffix: uint16) =
    let readingUnit = readingUnit suffix
    let model, _ = Model.init Route.Reader
    let model = ModelTests.withReading { Units = [readingUnit]; Next = None; Version = (WireFixtures.identity<ArtifactRoot> $"root-{suffix}") } model
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view ".verse-line").Click()
    Assert.Equal<Message list>([OpenPosition(PositionRef.Node { Node = readingUnit.Node })], messages)

[<Property>]
let ``the reader retains a continued quiet heading and its served target`` (suffix: uint16) =
    let readingUnit = readingUnit suffix
    let event = { Id = (WireFixtures.identity<NodeId> $"Event:served-heading-{suffix}"); Kind = NodeKind.Event; Label = $"Served heading {suffix}" }
    let unit = { readingUnit with Heading = Some { Event = event; IsContinuation = true; Kind = EventKind.General } }
    let model, _ = Model.init Route.Reader
    let model = ModelTests.withReading { Units = [unit]; Next = None; Version = (WireFixtures.identity<ArtifactRoot> $"root-{suffix}") } model
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    let heading = find view ".pericope-heading"
    heading.MarkupMatches($"""<h2 class="pericope-heading pericope-heading-continuation explorable-quiet" data-testid="pericope-heading-Event:served-heading-{suffix}" data-continuation="true" tabindex="0" role="button" aria-label="Explore Served heading {suffix} (continued)"><span class="pericope-heading-continuation-marker" data-testid="pericope-heading-continuation-marker-Event:served-heading-{suffix}">continued</span>Served heading {suffix}</h2>""")
    heading.Click()
    Assert.Equal<Message list>([OpenPosition(PositionRef.Node { Node = event })], messages)

[<Property>]
let ``the reader renders an event heading without an invented continuation`` (suffix: uint16) =
    let readingUnit = readingUnit suffix
    let event = { Id = (WireFixtures.identity<NodeId> $"Event:served-heading-{suffix}"); Kind = NodeKind.Event; Label = $"Served heading {suffix}" }
    let unit = { readingUnit with Heading = Some { Event = event; IsContinuation = false; Kind = EventKind.Event } }
    let model, _ = Model.init Route.Reader
    let model = ModelTests.withReading { Units = [unit]; Next = None; Version = (WireFixtures.identity<ArtifactRoot> $"root-{suffix}") } model
    use context = new BunitContext()
    let view = render context model ignore
    (find view ".pericope-heading").MarkupMatches($"""<h2 class="pericope-heading explorable" data-testid="pericope-heading-Event:served-heading-{suffix}" data-continuation="false" tabindex="0" role="button" aria-label="Explore Served heading {suffix}">Served heading {suffix}</h2>""")

[<Property>]
let ``unrelated typing over a reader anchor does not open a focus`` (suffix: uint16) =
    let readingUnit = readingUnit suffix
    let model, _ = Model.init Route.Reader
    let model = ModelTests.withReading { Units = [readingUnit]; Next = None; Version = (WireFixtures.identity<ArtifactRoot> $"root-{suffix}") } model
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    let key = string (char (int 'a' + int suffix % 26))
    (find view ".verse-mention").KeyDown(Microsoft.AspNetCore.Components.Web.KeyboardEventArgs(Key = key))
    Assert.Equal<Message list>([], messages)

[<Property>]
let ``the Concord view keeps a paragraph with no served edges as plain text`` (suffix: uint16) =
    let readingUnit = readingUnit suffix
    let model, _ = Model.init (Route.Concord None)
    let unit = { readingUnit with Ref = (WireFixtures.identity<UnitReference> "BoC 1.1.1"); Body = { readingUnit.Body with Locus = TextRef.Concord { Part = 1; Article = 1; Paragraph = 1 }; Anchors = []; WordsOfChrist = [] } }
    let model = ModelTests.withReading { Units = [unit]; Next = None; Version = (WireFixtures.identity<ArtifactRoot> $"root-{suffix}") } model
    use context = new BunitContext()
    let view = render context model ignore
    (find view ".concord-unit").MarkupMatches($"""<div class="concord-unit" data-testid="concord-unit-TextUnit:served-verse-{suffix}"><span class="concord-unit-ref">BoC 1.1.1</span>😀 ab cd-{suffix}</div>""")

[<Property>]
let ``the served Concord continuation exposes Next through an Elmish message`` (suffix: uint16) =
    let model, _ = Model.init (Route.Concord None)
    let model = ModelTests.withReading { Units = []; Next = Some (WireFixtures.identity<UnitReference> $"BoC 1.1.{int suffix + 1}"); Version = (WireFixtures.identity<ArtifactRoot> $"root-{suffix}") } model
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    let button = find view "[data-testid='concord-next']"
    button.MarkupMatches($"""<button type="button" class="concord-nav-button" data-testid="concord-next">Next ›</button>""")
    button.Click()
    Assert.Equal<Message list>([Page(SurfaceMessage.Concord ConcordMessage.Next)], messages)

[<Property>]
let ``a terminal Concord page offers no invented continuation`` (suffix: uint16) =
    let model, _ = Model.init (Route.Concord None)
    let model = ModelTests.withReading { Units = []; Next = None; Version = (WireFixtures.identity<ArtifactRoot> $"root-{suffix}") } model
    use context = new BunitContext()
    let view = render context model ignore
    Assert.Empty(RenderedComponentExtensions.FindAll<AppView>(view, "[data-testid='concord-next']"))

[<Property>]
let ``an opened focus renders its entire card presentation`` (suffix: uint16) =
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened (focusTrail suffix) }
    use context = new BunitContext()
    let view = render context model ignore
    let expected = $"""<div class="popover-body" data-testid="popover-body"><div class="popover-section" data-testid="popover-section-card"><p class="focus-title" data-testid="popover-card-title">Person:start-{suffix}</p><dl class="focus-fields"><div class="focus-field" data-testid="popover-field-Provenance"><dt>Provenance</dt><dd>test</dd></div></dl></div></div>"""
    (find view "[data-testid='popover-body']").MarkupMatches(expected)

[<Property>]
let ``closing a visible focus dispatches CloseFocus`` (suffix: uint16) =
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened (focusTrail suffix) }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view "[data-testid='popover-close']").Click()
    Assert.Equal<Message list>([CloseFocus], messages)

[<Property>]
let ``a failed focus retries its own request rather than the underlying page`` (suffix: uint16) =
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.CouldNotOpen(Explorable.position (Trail.current (focusTrail suffix)), Transport $"offline-{suffix}") }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view "[data-testid='popover-body'] [data-testid='could-not-load-retry']").Click()
    Assert.Equal<Message list>([RetryFocus], messages)

[<Property>]
let ``Back in the popover dispatches the exploration operation`` (suffix: uint16) =
    let target = ExplorationTests.node $"Person:target-{suffix}" $"root-{suffix}"
    let trail = Trail.follow { Kind = EdgeKind.Contains; Target = target } (focusTrail suffix)
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened trail }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view "[data-testid='popover-breadcrumb-back']").Click()
    Assert.Equal<Message list>([Traverse Traversal.Back], messages)

[<Property>]
let ``a text presentation follows its served anchor within the current exploration`` (suffix: uint16) =
    let readingUnit = readingUnit suffix
    let readingAnchor = readingAnchor suffix
    let record = { PresentationTests.node suffix with Kind = NodeKind.TextUnit; Text = Some readingUnit.Body }
    let trail = Trail.beginAt (PresentationTests.resolved record)
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened trail }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    let expected = $"""<p class="focus-text" data-testid="popover-text">😀 <span class="words-of-christ"><button type="button" class="focus-anchor explorable" data-testid="popover-anchor-Place:served-place-{suffix}-2">ab</button> cd</span>-{suffix}</p>"""
    (find view "[data-testid='popover-text']").MarkupMatches(expected)
    (find view ".focus-anchor").Click()
    Assert.Equal<Message list>([Traverse(Traversal.Follow { Kind = readingAnchor.Kind; Target = PositionRef.Node { Node = readingAnchor.Node } })], messages)

[<Property>]
let ``Escape dispatches CloseFocus for every generated current node`` (suffix: uint16) =
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened (focusTrail suffix) }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view "[data-testid='popover']").KeyDown(Microsoft.AspNetCore.Components.Web.KeyboardEventArgs(Key = "Escape"))
    Assert.Equal<Message list>([CloseFocus], messages)

[<Property>]
let ``an invalid text presentation retries by renewing the exploration`` (suffix: uint16) =
    let trail = Trail.beginAt (PresentationTests.resolved { PresentationTests.node suffix with Kind = NodeKind.TextUnit })
    let model, _ = Model.init Route.Sources
    let model = { model with Focus = FocusState.Opened trail }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- messages @ [message])
    (find view "[data-testid='popover-body'] [data-testid='could-not-load-retry']").Click()
    Assert.Equal<Message list>([Traverse Traversal.Renew], messages)

[<Property>]
let ``the source view composes the complete served source under its category`` (suffix: uint16) =
    let source = { Id = $"test-{suffix}"; Category = "text"; Title = $"Served source {suffix}"; WhatItIs = $"What it is {suffix}"; WhatWeBuilt = $"What we built {suffix}"; License = "CC0"; LicensesRowKey = $"test-{suffix}"; Link = Some $"https://example.test/{suffix}" }
    let model, _ = Model.init Route.Sources
    let model = { model with Surface = Surface.Sources(Ready { Categories = [{ Id = "text"; Label = "Texts" }]; Sources = [source]; Provenances = None }) }
    use context = new BunitContext()
    let view = render context model (fun (_: Message) -> ())
    let expected = $"""<article class="source-card" data-testid="source-test-{suffix}"><h3 class="source-title">Served source {suffix}</h3><p class="source-what">What it is {suffix}</p><p class="source-built"><span class="source-label">What we built:</span> What we built {suffix}</p><p class="source-license"><span class="source-label">License:</span> CC0</p><a class="source-link" data-testid="source-link-test-{suffix}" href="https://example.test/{suffix}" target="_blank" rel="noopener noreferrer">Visit source</a></article>"""
    Assert.Equal(expected, (find view $"[data-testid='source-test-{suffix}']").OuterHtml)

[<Property>]
let ``an explicit failure offers Retry and dispatches its message`` (suffix: uint16) =
    let model, _ = Model.init Route.Sources
    let model = { model with Surface = Surface.Sources(Failed(model.Serial, Transport $"offline-{suffix}", None)) }
    let mutable messages = []
    use context = new BunitContext()
    let view = render context model (fun message -> messages <- message :: messages)
    (find view "[data-testid='could-not-load-retry']").Click()
    Assert.Equal<Message list>([Page(SurfaceMessage.Sources SourcesMessage.Retry)], messages)

[<Property>]
let ``the shared header retains primary navigation translation and credits`` (suffix: uint16) =
    let route =
        match suffix % 6us with
        | 0us -> Route.Reader
        | 1us -> Route.World
        | 2us -> Route.Kretzmann
        | 3us -> Route.Concord None
        | 4us -> Route.Sources
        | _ -> Route.NotFound
    let model, _ = Model.init route
    use context = new BunitContext()
    let view = render context model (fun (_: Message) -> ())
    let expected = """<header class="app-header header-parchment"><a class="wordmark" href="/">Bible Explorer <span class="wordmark-glyph" aria-hidden="true">∴</span></a><nav class="app-nav" aria-label="Primary"><a class="nav-link" href="/" data-testid="nav-reader">Reader</a><a class="nav-link" href="/world" data-testid="nav-world">World</a><a class="nav-link" href="/kretzmann" data-testid="nav-kretzmann">Kretzmann</a><a class="nav-link" href="/concord" data-testid="nav-concord">Concord</a></nav><div class="header-tools"><label class="translation-label" for="translation-select">Translation</label><select id="translation-select" class="translation-select" data-testid="translation-select"><option value="kjv">KJV</option></select><a class="attribution" data-testid="attribution" href="/sources">Credits</a></div></header>"""
    Assert.Equal(expected, (find view "header").OuterHtml)

[<Property>]
let ``the WebAssembly entry component runs the sources command and renders its response`` (suffix: uint16) =
    use context = new BunitContext()
    context.JSInterop.Mode <- JSRuntimeMode.Loose
    let document = { Categories = [{ Id = $"text-{suffix}"; Label = $"Texts {suffix}" }]; Sources = []; Provenances = None }
    use response = new System.Net.Http.HttpResponseMessage(System.Net.HttpStatusCode.OK, Content = new System.Net.Http.StringContent(Json.encode document))
    use handler = new TransportTests.Handler(response)
    use http = new System.Net.Http.HttpClient(handler, BaseAddress = System.Uri "http://example.test/")
    context.Services.AddSingleton<System.Net.Http.HttpClient>(http) |> ignore
    context.Renderer.SetRendererInfo(Microsoft.AspNetCore.Components.RendererInfo("WebAssembly", true))
    let navigation = context.Services.GetRequiredService<Microsoft.AspNetCore.Components.NavigationManager>()
    navigation.NavigateTo("/sources")
    let view = context.Render<App>()
    view.WaitForAssertion(fun () -> Assert.Equal($"Texts {suffix}", (find view ".sources-category-title").TextContent))
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
let private focusTrail (suffix: uint16) = Trail.beginAt (ExplorationTests.node $"Person:start-{suffix}" $"root-{suffix}")
let private readingUnit (suffix: uint16) : TextUnit = { Ref = WireFixtures.identity<UnitReference> "GEN.1.1"; Node = { Id = WireFixtures.identity<NodeId> $"TextUnit:served-verse-{suffix}"; Kind = NodeKind.TextUnit; Label = "Genesis 1:1" }; Heading = None; EdgeSummary = []; Body = { Text = $"😀 ab cd-{suffix}"; Locus = ModelTests.firstChapter.Locus; Anchors = [readingAnchor suffix]; WordsOfChrist = [{ Start = 2; End = 7 }] } }
let private readingAnchor (suffix: uint16) : Anchor = { Start = 2; End = 4; Kind = EdgeKind.Mentions; Node = { Id = WireFixtures.identity<NodeId> $"Place:served-place-{suffix}"; Kind = NodeKind.Place; Label = $"Served place {suffix}" } }
