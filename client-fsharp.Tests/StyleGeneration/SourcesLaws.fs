module BibleAtlas.FSharp.Tests.StyleGeneration.SourcesLaws

open System.Net
open Microsoft.FSharp.Reflection
open FsCheck.Xunit
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Property>]
let ``the source presentation preserves the whole document and every card in served order`` (categories: byte) (cards: byte) (links: byte) reversed =
    let expected = SourcesFixtures.document categories cards links reversed
    Assert.Equal(Ok expected, SourcesPresenter.present expected.Document |> Result.map SourcesPresentation.view)

[<Property>]
let ``unlisted categories refuse the complete offending source records`` (cards: byte) (links: byte) =
    let expected = SourcesFixtures.document 0uy cards links false
    let document = { expected.Document with Categories = [] }
    match document.Sources with
    | first :: rest ->
        Assert.Equal(Error(SourcePresentationFailure.UnlistedCategories(NonEmpty.create first rest)), SourcesPresenter.present document)
    | [] -> Assert.Fail("the generated fixture must contain sources")

[<Property>]
let ``repeated category identities refuse their whole records before grouping`` (categories: byte) reversed =
    let fixture = SourcesFixtures.document categories 0uy 0uy reversed
    let categories = fixture.Document.Categories @ fixture.Document.Categories
    match categories with
    | first :: rest ->
        let document = { fixture.Document with Categories = categories }
        Assert.Equal(Error(SourcePresentationFailure.RepeatedCategories(NonEmpty.create first rest)), SourcesPresenter.present document)
    | [] -> Assert.Fail("the generated fixture must contain categories")

[<Property>]
let ``the source model has a whole-value transition for every message and reachable state`` (steps: byte) (cards: byte) (links: byte) =
    let request = SourcesFixtures.ticket steps
    let next = RequestId.next request
    let document = (SourcesFixtures.document 0uy cards links false).Document
    let refusal status = ReadFailure.HttpRejected { Status = status; Reason = Some "refused"; Body = Ok { Error = { Code = ErrorCode.NotFound; Message = "served refusal" } } }
    let failures = [ReadFailure.Unreachable "offline"; ReadFailure.Cancelled "cancelled"; refusal HttpStatusCode.NotFound; refusal HttpStatusCode.ServiceUnavailable; ReadFailure.InvalidAnswer NullPayload]
    let missing = { document with Categories = [] }
    let duplicate = { document with Categories = document.Categories @ document.Categories }
    let start, _ = Sources.init request
    match SourcesPresenter.present document, SourcesPresenter.present missing, SourcesPresenter.present duplicate with
    | Ok presented, Error unlisted, Error repeated ->
        let completions = [Ok document; Ok missing; Ok duplicate] @ (failures |> List.map Error)
        let models = start :: (completions |> List.map (fun completion -> Sources.update next (SourcesMessage.Loaded(request, completion)) start |> fst))
        let messages = SourcesMessage.Retry :: ([request; next] |> List.collect (fun ticket -> completions |> List.map (fun answer -> SourcesMessage.Loaded(ticket, answer))))
        let expectedTransition model message =
            let state = Sources.state model
            match message, state with
            | SourcesMessage.Retry, SourcesState.RetryableFailure _ -> SourcesState.Loading next, [SourcesEffect.Read next]
            | SourcesMessage.Retry, (SourcesState.Loading _ | SourcesState.Available _ | SourcesState.ReadRejected _ | SourcesState.PresentationRejected _) -> state, []
            | SourcesMessage.Loaded(ticket, answer), SourcesState.Loading pending when ticket = pending ->
                let expected =
                    match answer with
                    | Ok value when value = document -> SourcesState.Available presented
                    | Ok value when value = missing -> SourcesState.PresentationRejected unlisted
                    | Ok _ -> SourcesState.PresentationRejected repeated
                    | Error(ReadFailure.Unreachable _ as failure) | Error(ReadFailure.Cancelled _ as failure) -> SourcesState.RetryableFailure failure
                    | Error(ReadFailure.HttpRejected({ Status = HttpStatusCode.ServiceUnavailable } as rejection)) -> SourcesState.RetryableFailure(ReadFailure.HttpRejected rejection)
                    | Error(ReadFailure.HttpRejected _ as failure) | Error(ReadFailure.InvalidAnswer _ as failure) -> SourcesState.ReadRejected failure
                expected, []
            | SourcesMessage.Loaded _, (SourcesState.Loading _ | SourcesState.Available _ | SourcesState.RetryableFailure _ | SourcesState.ReadRejected _ | SourcesState.PresentationRejected _) -> state, []
        let actual = models |> List.collect (fun model -> messages |> List.map (fun message -> Sources.update next message model |> fun (model, effects) -> Sources.state model, effects))
        let expected = models |> List.collect (fun model -> messages |> List.map (expectedTransition model))
        Assert.Equal<(SourcesState * SourcesEffect list) list>(expected, actual)
        let reached = models |> List.map (fun model -> FSharpValue.GetUnionFields(Sources.state model, typeof<SourcesState>) |> fst |> _.Name) |> Set.ofList
        let all = FSharpType.GetUnionCases typeof<SourcesState> |> Array.map _.Name |> Set.ofArray
        Assert.Equal<Set<string>>(all, reached)
    | Error error, _, _ -> Assert.Fail($"the generated valid document was refused: {error}")
    | Ok _, Ok _, _ -> Assert.Fail("unlisted categories were accepted")
    | Ok _, Error _, Ok _ -> Assert.Fail("repeated categories were accepted")

[<Property>]
let ``HTTP failure retry policy covers every status in the rejection range`` (offset: byte) =
    let statuses = [400 .. 599] |> List.map (fun status -> 400 + (status - 400 + int offset) % 200)
    let expected = statuses |> List.map (fun status -> status >= 500)
    let actual = statuses |> List.map (fun status -> ReadFailure.canRetry (ReadFailure.HttpRejected { Status = enum status; Reason = None; Body = Error NullPayload }))
    Assert.Equal<bool list>(expected, actual)

[<Property>]
let ``Sources startup creates exactly one pending ticket and one read`` (steps: byte) =
    let request = SourcesFixtures.ticket steps
    let actual = Sources.init request |> fun (model, effects) -> Sources.state model, effects
    Assert.Equal((SourcesState.Loading request, [SourcesEffect.Read request]), actual)
