module BibleAtlas.FSharp.Tests.StyleGeneration.SourcesAppLaws

open Bunit
open FsCheck.Xunit
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Client

[<Property>]
let ``the application refuses terminal source answers without offering a network retry`` (categories: byte) (cards: byte) =
    let document = (SourcesFixtures.document categories cards 0uy false).Document
    let start, _ = Model.init Route.Sources
    let answers = [Error(ReadFailure.InvalidAnswer NullPayload); Ok { document with Categories = [] }; Ok { document with Categories = document.Categories @ document.Categories }]
    let actual = answers |> List.map (fun answer ->
        let model, effects = Model.update (Page(SurfaceMessage.Sources(SourcesMessage.Loaded(start.Serial, answer)))) start
        use context = new BunitContext()
        let view = SourcesDom.renderApp context model ignore
        let retries = RenderedComponentExtensions.FindAll<AppView>(view, "[data-testid='could-not-load-retry']") |> Seq.map _.OuterHtml |> Seq.toList
        let rejected = RenderedComponentExtensions.FindAll<AppView>(view, "[data-testid='could-not-display']") |> Seq.map _.OuterHtml |> Seq.toList
        (effects, model.Serial, retries, rejected))
    let messages = ["The sources response cannot be displayed."; "The sources list contains inconsistent categories."; "The sources list contains inconsistent categories."]
    let expected = messages |> List.map (fun message -> ([], start.Serial, [], [$"<p class=\"popover-meta\" data-testid=\"could-not-display\">{message}</p>"]))
    Assert.Equal<(Effect list * RequestId * string list * string list) list>(expected, actual)
