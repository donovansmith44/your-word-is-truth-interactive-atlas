namespace BibleAtlas.FSharp.Domain

[<RequireQualifiedAccess>]
type Position<'elementId, 'nodeId> = Element of 'elementId | Years of YearSpan<'nodeId>

[<RequireQualifiedAccess>]
type ResolvedValue<'nodeId, 'edgeId, 'reference, 'partRole, 'level, 'mark, 'provenance, 'timeEvidence, 'facet when 'facet: comparison> =
    | GraphElement of Element<'nodeId, 'edgeId, 'reference, 'partRole, 'level, 'mark, 'provenance>
    | Event of EventContext<'nodeId, 'reference, 'partRole, 'mark, 'timeEvidence>
    | Year of YearContext<'nodeId, 'facet>
    | Years of SpanContext<'nodeId, 'facet>

type Resolved<'root, 'position, 'value> = private { Position: 'position; Value: Rooted<'root, 'value> }

[<RequireQualifiedAccess>]
type ResolutionFailure = DifferentPosition | MissingPosition

module Resolution =
    let admit (requested: 'position) (observed: 'position) (value: Rooted<'root, 'value>) : Result<Resolved<'root, 'position, 'value>, ResolutionFailure> =
        DomainSkeleton.pending "Resolution.admit"
    let position (resolved: Resolved<'root, 'position, 'value>) : 'position = resolved.Position
    let root (resolved: Resolved<'root, 'position, 'value>) : 'root = Rooted.root resolved.Value
    let value (resolved: Resolved<'root, 'position, 'value>) : Rooted<'root, 'value> = resolved.Value
