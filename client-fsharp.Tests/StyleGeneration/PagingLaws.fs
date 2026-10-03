module BibleAtlas.FSharp.Tests.StyleGeneration.PagingLaws

open FsCheck.Xunit
open Xunit
open BibleAtlas.FSharp
open BibleAtlas.FSharp.Contract

[<Property(MaxTest = 4)>]
let ``future sized Concord journeys request capped pages and retain only the complete current window`` (extra: byte) (size: byte) =
    let pageCount = PagingFixtures.futurePageCount + int extra
    let pageSize = 1 + int size % PagingFixtures.pageLimit
    let initialReference = "opaque-opening"
    let start, _ = Model.init (Route.Concord(Some initialReference))
    let opened, _ = Model.update (Page(SurfaceMessage.Concord(ConcordMessage.Reading(ReadingMessage.ContentsLoaded(start.Serial, Ok PagingFixtures.contents))))) start
    let first = PagingFixtures.window 0 pageSize
    let firstModel, _ = Model.update (Page(SurfaceMessage.Concord(ConcordMessage.Reading(ReadingMessage.TextLoaded(opened.Serial, Ok first))))) opened
    let actual =
        [1 .. pageCount] |> List.fold (fun model index ->
            let request = RequestId.next model.Serial
            let previous = PagingFixtures.window (index - 1) pageSize
            let read = Reads.textWindow $"opaque-next-{index - 1}" (Some PagingFixtures.pageLimit) (Some WindowDir.Onward) None (Some Corpus.Concord)
            let pending, effects = Model.update (Page(SurfaceMessage.Concord ConcordMessage.Next)) model
            let pendingSession = ReadSession.beginRead request read (Ready previous)
            let expectedPending = Ok(Some initialReference, ReadingState.Active(PagingFixtures.contents, pendingSession), request, FocusState.Closed), [ReadText(Corpus.Concord, request, read)]
            Assert.Equal(expectedPending, (PagingFixtures.project pending, effects))
            let window = PagingFixtures.window index pageSize
            let ready, effects = Model.update (Page(SurfaceMessage.Concord(ConcordMessage.Reading(ReadingMessage.TextLoaded(request, Ok window))))) pending
            let readySession = ReadSession.complete request (Ok window) pendingSession
            let expectedReady = Ok(Some initialReference, ReadingState.Active(PagingFixtures.contents, readySession), request, FocusState.Closed), []
            Assert.Equal(expectedReady, (PagingFixtures.project ready, effects))
            ready) firstModel
    let request = [1 .. pageCount] |> List.fold (fun ticket _ -> RequestId.next ticket) opened.Serial
    let read = Reads.textWindow $"opaque-next-{pageCount - 1}" (Some PagingFixtures.pageLimit) (Some WindowDir.Onward) None (Some Corpus.Concord)
    let session = ReadSession.beginRead request read Empty |> ReadSession.complete request (Ok(PagingFixtures.window pageCount pageSize))
    let expected = Ok(Some initialReference, ReadingState.Active(PagingFixtures.contents, session), request, FocusState.Closed)
    Assert.Equal(expected, PagingFixtures.project actual)
