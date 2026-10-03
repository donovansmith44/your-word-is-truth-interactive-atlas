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
