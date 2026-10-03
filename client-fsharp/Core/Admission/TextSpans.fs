namespace BibleAtlas.FSharp.Admission

open BibleAtlas.FSharp.Domain

type ScalarOffset = private ScalarOffset of int

type Utf16Offset = private Utf16Offset of int

type ScalarSpan = private ScalarSpan of Endpoints<ScalarOffset>

type Utf16Span = private Utf16Span of Endpoints<Utf16Offset>

[<RequireQualifiedAccess>]
type ScalarOffsetFailure = NegativeScalarOffset

[<RequireQualifiedAccess>]
type Utf16OffsetFailure = NegativeUtf16Offset

[<RequireQualifiedAccess>]
type ScalarSpanFailure = ReversedScalarSpan

[<RequireQualifiedAccess>]
type Utf16SpanFailure = ReversedUtf16Span

[<RequireQualifiedAccess>]
type TextSpanFailure = SpanOutsideText

module TextSpans =
    let scalarOffset (offset: int) : Result<ScalarOffset, ScalarOffsetFailure> =
        DomainSkeleton.pending "TextSpans.scalarOffset"
    let utf16Offset (offset: int) : Result<Utf16Offset, Utf16OffsetFailure> =
        DomainSkeleton.pending "TextSpans.utf16Offset"
    let scalarSpan (ends: Endpoints<ScalarOffset>) : Result<ScalarSpan, ScalarSpanFailure> =
        DomainSkeleton.pending "TextSpans.scalarSpan"
    let utf16Span (ends: Endpoints<Utf16Offset>) : Result<Utf16Span, Utf16SpanFailure> =
        DomainSkeleton.pending "TextSpans.utf16Span"
    let inText (text: string) (span: ScalarSpan) : Result<Utf16Span, TextSpanFailure> =
        DomainSkeleton.pending "TextSpans.inText"
    let scalarValue (ScalarOffset offset) : int = offset
    let utf16Value (Utf16Offset offset) : int = offset
    let scalarEnds (ScalarSpan ends) : Endpoints<ScalarOffset> = ends
    let utf16Ends (Utf16Span ends) : Endpoints<Utf16Offset> = ends

namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract

type TextPiece = { Text: string; IsWordsOfChrist: bool; Anchor: Anchor option }

module TextRuns =
    let runs (unit: UnitText) =
        let offsets = unit.Text.EnumerateRunes() |> Seq.scan (fun offset rune -> offset + rune.Utf16SequenceLength) 0 |> Seq.toArray
        let utf16 scalar = offsets[max 0 (min scalar (offsets.Length - 1))]
        let red = unit.WordsOfChrist |> List.map (fun span -> utf16 span.Start, utf16 span.End)
        let anchored = unit.Anchors |> List.map (fun anchor -> utf16 anchor.Start, utf16 anchor.End, anchor)
        let cuts =
            [0; unit.Text.Length]
            @ List.collect (fun (start, finish) -> [start; finish]) red
            @ List.collect (fun (start, finish, _) -> [start; finish]) anchored
            |> List.distinct |> List.sort
        let pieces =
            cuts |> List.pairwise |> List.map (fun (start, finish) ->
                { Text = unit.Text.Substring(start, finish - start)
                  IsWordsOfChrist = red |> List.exists (fun (left, right) -> left <= start && finish <= right)
                  Anchor = anchored |> List.tryPick (fun (left, right, anchor) -> if left <= start && finish <= right then Some anchor else None) })
        pieces |> List.fold (fun runs piece ->
            match runs with
            | (first :: earlier) :: rest when first.IsWordsOfChrist = piece.IsWordsOfChrist -> (piece :: first :: earlier) :: rest
            | _ -> [piece] :: runs) []
        |> List.rev |> List.map List.rev
