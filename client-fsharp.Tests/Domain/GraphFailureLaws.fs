module BibleAtlas.FSharp.Tests.Domain.GraphFailureLaws

open System
open FsCheck.Xunit
open Xunit
open Microsoft.FSharp.Reflection
open BibleAtlas.FSharp.Admission
open BibleAtlas.FSharp.Contract

[<Property(MaxTest = 1)>]
let ``every graph answer failure carries its complete typed evidence and no message string`` () =
    let actual = FSharpType.GetUnionCases(typeof<GraphFailure>) |> Array.map (fun case -> case.Name, case.GetFields() |> Array.map _.PropertyType |> Array.toList) |> Array.toList
    let expected =
        [ "MissingElement", [typeof<ElementId>]
          "ElementCountMismatch", [typeof<ElementCounts>]
          "UnexpectedElement", [typeof<PositionAnswer>]
          "EmptyContinuation", [typeof<ElementPageCursor>]
          "ExcessContinuation", [typeof<ElementPageCursor>]
          "TextMissing", [typeof<NodeId>]
          "ContentsCorpusMismatch", [typeof<CorpusAnswer>]
          "TextCorpusMismatch", [typeof<CorpusAnswer>]
          "MissingOpening", [typeof<Corpus>] ]
    Assert.Equal<(string * Type list) list>(expected, actual)

[<Property>]
let ``every graph answer category is terminal and remains unaltered inside the read failure`` (requested: uint16) (received: uint16) (corpus: Corpus) =
    let id = BibleAtlas.FSharp.Tests.WireFixtures.identity<NodeId> $"served-{requested}"
    let element = ElementId.ofNodeId id
    let position = PositionRef.Node { Node = { Id = id; Kind = NodeKind.Person; Label = $"served-{received}" } }
    let answered = PositionRef.Node { Node = { Id = BibleAtlas.FSharp.Tests.WireFixtures.identity<NodeId> $"answered-{received}"; Kind = NodeKind.Event; Label = $"answered-{received}" } }
    let extraElement = 1UL
    let cursor = BibleAtlas.FSharp.Tests.WireFixtures.identity<ElementPageCursor> (int received)
    let other = match corpus with Corpus.Bible -> Corpus.Concord | Corpus.Concord -> Corpus.Bible
    let failures =
        [ GraphFailure.MissingElement element
          GraphFailure.ElementCountMismatch { Requested = uint64 requested; Received = uint64 requested + uint64 received + extraElement }
          GraphFailure.UnexpectedElement { Requested = position; Received = answered }
          GraphFailure.EmptyContinuation cursor
          GraphFailure.ExcessContinuation cursor
          GraphFailure.TextMissing id
          GraphFailure.ContentsCorpusMismatch { Requested = corpus; Received = other }
          GraphFailure.TextCorpusMismatch { Requested = corpus; Received = other }
          GraphFailure.MissingOpening corpus ]
    let actual = failures |> List.map (fun failure -> ReadFailure.Terminal(TerminalFailure.InvalidGraph failure))
    let expected = failures |> List.map (fun failure -> false, Some failure)
    let evidence = actual |> List.map (fun failure ->
        let graph =
            match failure with
            | ReadFailure.Terminal(TerminalFailure.InvalidGraph failure) -> Some failure
            | ReadFailure.Terminal _ | ReadFailure.Transient _ | ReadFailure.Cancelled -> None
        ReadFailures.retryable failure, graph)
    Assert.Equal<(bool * GraphFailure option) list>(expected, evidence)
