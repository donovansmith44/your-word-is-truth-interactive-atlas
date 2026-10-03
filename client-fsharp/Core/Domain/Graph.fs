namespace BibleAtlas.FSharp.Domain

open BibleAtlas.FSharp.Contract

[<RequireQualifiedAccess>]
type Direction = Forward | Reverse

type RelationDirection = private Directed of EdgeKind * Direction | Symmetric of EdgeKind

type Entity<'nodeId> = private { Id: 'nodeId; Kind: NodeKind; Label: string; Summary: Map<EdgeKind, int> }

[<RequireQualifiedAccess>]
type Node<'nodeId, 'partRole, 'level, 'mark> =
    | Entity of Entity<'nodeId>
    | TextUnit of TextUnit<'nodeId, 'partRole>
    | Container of Container<'nodeId, 'level>
    | Passage of Passage<'nodeId, 'mark>

type NodeEnds<'nodeId> = { Subject: 'nodeId; Object: 'nodeId }

type JustificationEnds<'nodeId, 'edgeId> = { Subject: 'nodeId; Evidence: 'edgeId }

[<RequireQualifiedAccess>]
type EdgeEnds<'nodeId, 'edgeId> =
    | Nodes of NodeEnds<'nodeId>
    | Justification of JustificationEnds<'nodeId, 'edgeId>

type Edge<'nodeId, 'edgeId> =
    private
        { Id: 'edgeId
          Kind: EdgeKind
          Ends: EdgeEnds<'nodeId, 'edgeId>
          Label: string
          Parentage: Parentage option
          Votes: int option
          Narrative: NarrativeId option
          Provenance: string option
          Summary: Map<EdgeKind, int> }

[<RequireQualifiedAccess>]
type Element<'nodeId, 'edgeId, 'partRole, 'level, 'mark> =
    | Node of Node<'nodeId, 'partRole, 'level, 'mark>
    | Edge of Edge<'nodeId, 'edgeId>

[<RequireQualifiedAccess>]
type EntityFailure = SpecializedNodeKind | NegativeEdgeCount

[<RequireQualifiedAccess>]
type EdgeFailure = IllegalEdgeEndpoint | NegativeEdgeCount

module Relations =
    let admit (kind: EdgeKind) (direction: Direction) : RelationDirection =
        DomainSkeleton.pending "Relations.admit"
    let dual (relation: RelationDirection) : RelationDirection =
        DomainSkeleton.pending "Relations.dual"
    let kind (relation: RelationDirection) : EdgeKind =
        match relation with Directed (kind, _) | Symmetric kind -> kind
    let direction (relation: RelationDirection) : Direction option =
        match relation with Directed (_, direction) -> Some direction | Symmetric _ -> None

module Nodes =
    let admitEntity (id: 'nodeId) (kind: NodeKind) (label: string) (summary: Map<EdgeKind, int>) : Result<Entity<'nodeId>, EntityFailure> =
        DomainSkeleton.pending "Nodes.admitEntity"
    let id (node: Node<'nodeId, 'partRole, 'level, 'mark>) : 'nodeId =
        DomainSkeleton.pending "Nodes.id"
    let label (node: Node<'nodeId, 'partRole, 'level, 'mark>) : string =
        DomainSkeleton.pending "Nodes.label"
    let entityKinds () : NonEmpty<NodeKind> =
        DomainSkeleton.pending "Nodes.entityKinds"
    let relations () : NonEmpty<EdgeKind> =
        DomainSkeleton.pending "Nodes.relations"
    let kind (entity: Entity<'nodeId>) : NodeKind = entity.Kind

module Edges =
    let admit (id: 'edgeId) (kind: EdgeKind) (ends: EdgeEnds<'nodeId, 'edgeId>) (label: string) (parentage: Parentage option) (votes: int option) (narrative: NarrativeId option) (provenance: string option) (summary: Map<EdgeKind, int>) : Result<Edge<'nodeId, 'edgeId>, EdgeFailure> =
        DomainSkeleton.pending "Edges.admit"
    let id (edge: Edge<'nodeId, 'edgeId>) : 'edgeId = edge.Id
    let kind (edge: Edge<'nodeId, 'edgeId>) : EdgeKind = edge.Kind
    let ends (edge: Edge<'nodeId, 'edgeId>) : EdgeEnds<'nodeId, 'edgeId> = edge.Ends
    let label (edge: Edge<'nodeId, 'edgeId>) : string = edge.Label
    let parentage (edge: Edge<'nodeId, 'edgeId>) : Parentage option = edge.Parentage
    let votes (edge: Edge<'nodeId, 'edgeId>) : int option = edge.Votes
    let narrative (edge: Edge<'nodeId, 'edgeId>) : NarrativeId option = edge.Narrative
    let provenance (edge: Edge<'nodeId, 'edgeId>) : string option = edge.Provenance
    let summary (edge: Edge<'nodeId, 'edgeId>) : Map<EdgeKind, int> = edge.Summary
