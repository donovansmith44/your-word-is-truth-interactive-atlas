module BibleAtlas.FSharp.Tests.StyleGeneration.SourcesViewLaws

open System.Threading.Tasks
open Bunit
open FsCheck.Xunit
open Xunit
open BibleAtlas.FSharp

[<Property>]
let ``the Sources view renders every prepared category and complete card without regrouping`` (categories: byte) (cards: byte) (links: byte) reversed =
    let fixture = SourcesFixtures.document categories cards links reversed
    match SourcesPresenter.present fixture.Document with
    | Error error -> Assert.Fail($"fixture refused: {error}")
    | Ok presentation ->
        use context = new BunitContext()
        let view = SourcesDom.render context (SourcesState.Available presentation) ignore
        let actual = RenderedComponentExtensions.FindAll<SourcesDom.Component>(view, ".sources-category") |> Seq.map _.OuterHtml |> Seq.toList
        let expected = fixture.Sections |> List.map (fun section ->
            let cards = section.Cards |> List.map (fun card ->
                let source = card.Entry
                let link =
                    match card.Visit with
                    | None -> ""
                    | Some link -> $"<a class=\"source-link\" data-testid=\"source-link-{source.Id}\" href=\"{link}\" target=\"_blank\" rel=\"noopener noreferrer\">Visit source</a>"
                $"<article class=\"source-card\" data-testid=\"source-{source.Id}\"><h3 class=\"source-title\">{source.Title}</h3><p class=\"source-what\">{source.WhatItIs}</p><p class=\"source-built\"><span class=\"source-label\">What we built:</span> {source.WhatWeBuilt}</p><p class=\"source-license\"><span class=\"source-label\">License:</span> {source.License}</p>{link}</article>") |> String.concat ""
            $"<section class=\"sources-category\" data-testid=\"sources-category-{section.Category.Id}\"><h2 class=\"sources-category-title\">{section.Category.Label}</h2><div class=\"sources-grid\">{cards}</div></section>")
        Assert.Equal<string list>(expected, actual)

[<Property>]
let ``only a prepared retryable source failure offers Retry`` (steps: byte) (categories: byte) =
    let request = SourcesFixtures.ticket steps
    let fixture = SourcesFixtures.document categories 0uy 0uy false
    let start, _ = Sources.init request
    let unlisted = { fixture.Document with Categories = [] }
    let answers = [Error(ReadFailure.Unreachable "offline"); Error(ReadFailure.Cancelled "cancelled"); Error(ReadFailure.InvalidAnswer NullPayload); Ok unlisted; Ok fixture.Document]
    let states = Sources.state start :: (answers |> List.map (fun answer -> Sources.update (RequestId.next request) (SourcesMessage.Loaded(request, answer)) start |> fst |> Sources.state))
    let actual = states |> List.map (fun state ->
        use context = new BunitContext()
        let messages = TaskCompletionSource<SourcesMessage>()
        let view = SourcesDom.render context state messages.SetResult
        let buttons = RenderedComponentExtensions.FindAll<SourcesDom.Component>(view, "[data-testid='could-not-load-retry']")
        match Seq.toList buttons with
        | [] -> None
        | [button] -> button.Click(); Some(messages.Task.GetAwaiter().GetResult())
        | _ -> Assert.Fail("the page offered duplicate Retry buttons"); None)
    Assert.Equal<SourcesMessage option list>([None; Some SourcesMessage.Retry; Some SourcesMessage.Retry; None; None; None], actual)
