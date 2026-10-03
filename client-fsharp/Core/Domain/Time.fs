namespace BibleAtlas.FSharp.Domain

type Endpoints<'value> = { First: 'value; Last: 'value }

type Year<'nodeId> = private { Node: 'nodeId; Value: int; Label: string }

type YearSpan<'nodeId> = private YearSpan of Endpoints<Year<'nodeId>>

[<RequireQualifiedAccess>]
type Precision = Exact | Circa

type Dating<'nodeId> = private { Span: YearSpan<'nodeId>; Precision: Precision }

[<RequireQualifiedAccess>]
type YearFailure = YearZero | OutsideAtlas

[<RequireQualifiedAccess>]
type YearSpanFailure = ReversedYears

module Years =
    let admit (node: 'nodeId) (label: string) (value: int) (atlas: Endpoints<int>) : Result<Year<'nodeId>, YearFailure> =
        DomainSkeleton.pending "Years.admit"
    let compare (left: Year<'nodeId>) (right: Year<'nodeId>) : int =
        DomainSkeleton.pending "Years.compare"
    let singleton (year: Year<'nodeId>) : YearSpan<'nodeId> =
        DomainSkeleton.pending "Years.singleton"
    let between (ends: Endpoints<Year<'nodeId>>) : Result<YearSpan<'nodeId>, YearSpanFailure> =
        DomainSkeleton.pending "Years.between"
    let contains (span: YearSpan<'nodeId>) (year: Year<'nodeId>) : bool =
        DomainSkeleton.pending "Years.contains"
    let cover (left: YearSpan<'nodeId>) (right: YearSpan<'nodeId>) : YearSpan<'nodeId> =
        DomainSkeleton.pending "Years.cover"
    let overlaps (left: YearSpan<'nodeId>) (right: YearSpan<'nodeId>) : bool =
        DomainSkeleton.pending "Years.overlaps"
    let meet (left: YearSpan<'nodeId>) (right: YearSpan<'nodeId>) : YearSpan<'nodeId> option =
        DomainSkeleton.pending "Years.meet"
    let within (inner: YearSpan<'nodeId>) (outer: YearSpan<'nodeId>) : bool =
        DomainSkeleton.pending "Years.within"
    let adjacent (left: YearSpan<'nodeId>) (right: YearSpan<'nodeId>) : bool =
        DomainSkeleton.pending "Years.adjacent"
    let years (span: YearSpan<'nodeId>) (served: Year<'nodeId> list) : Year<'nodeId> list =
        DomainSkeleton.pending "Years.years"
    let endpoints (YearSpan ends) : Endpoints<Year<'nodeId>> = ends
    let node (year: Year<'nodeId>) : 'nodeId = year.Node
    let value (year: Year<'nodeId>) : int = year.Value
    let label (year: Year<'nodeId>) : string = year.Label

module Dates =
    let dated (span: YearSpan<'nodeId>) (precision: Precision) : Dating<'nodeId> =
        { Span = span; Precision = precision }
    let span (dating: Dating<'nodeId>) : YearSpan<'nodeId> = dating.Span
    let precision (dating: Dating<'nodeId>) : Precision = dating.Precision
