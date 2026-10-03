module BibleAtlas.FSharp.Tests.ViewTests

open Bunit
open Xunit
open Microsoft.Extensions.DependencyInjection
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Client
open BibleAtlas.FSharp.Contract

let render (context: BunitContext) (model: Model) (dispatch: Message -> unit) =
    context.Render<AppView>(Microsoft.AspNetCore.Components.RenderFragment(fun builder ->
        builder.OpenComponent<AppView>(0)
        builder.AddAttribute(1, "Model", model)
        builder.AddAttribute(2, "Dispatch", dispatch)
        builder.CloseComponent()))

let find<'a when 'a :> Microsoft.AspNetCore.Components.IComponent> (view: IRenderedComponent<'a>) selector =
    RenderedComponentExtensions.Find<'a>(view, selector)

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
