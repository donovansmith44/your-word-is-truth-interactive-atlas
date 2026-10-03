namespace BibleAtlas.FSharp.Domain

type OfficeTerm<'nodeId, 'office> =
    private { Person: 'nodeId; Office: 'office; Span: YearSpan<'nodeId>; Grounds: 'nodeId list }

type YearContext<'nodeId, 'facet when 'facet: comparison> =
    private { Year: Year<'nodeId>; Counts: Map<'facet, int> }

type SpanContext<'nodeId, 'facet when 'facet: comparison> =
    private { Span: YearSpan<'nodeId>; Years: YearContext<'nodeId, 'facet> list }

[<RequireQualifiedAccess>]
type FacetCountFailure = NegativeFacetCount

[<RequireQualifiedAccess>]
type SpanContextFailure = YearOutsideSpan | UnorderedYears | RepeatedYear

module YearReadings =
    let admitYear (year: Year<'nodeId>) (counts: Map<'facet, int>) : Result<YearContext<'nodeId, 'facet>, FacetCountFailure> =
        DomainSkeleton.pending "YearReadings.admitYear"
    let admitSpan (span: YearSpan<'nodeId>) (years: YearContext<'nodeId, 'facet> list) : Result<SpanContext<'nodeId, 'facet>, SpanContextFailure> =
        DomainSkeleton.pending "YearReadings.admitSpan"
    let officeTerm (person: 'nodeId) (office: 'office) (span: YearSpan<'nodeId>) (grounds: 'nodeId list) : OfficeTerm<'nodeId, 'office> =
        { Person = person; Office = office; Span = span; Grounds = grounds }
    let year (context: YearContext<'nodeId, 'facet>) : Year<'nodeId> = context.Year
    let counts (context: YearContext<'nodeId, 'facet>) : Map<'facet, int> = context.Counts
    let span (context: SpanContext<'nodeId, 'facet>) : YearSpan<'nodeId> = context.Span
    let years (context: SpanContext<'nodeId, 'facet>) : YearContext<'nodeId, 'facet> list = context.Years
    let person (term: OfficeTerm<'nodeId, 'office>) : 'nodeId = term.Person
    let office (term: OfficeTerm<'nodeId, 'office>) : 'office = term.Office
