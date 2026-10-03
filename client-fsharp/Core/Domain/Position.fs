namespace BibleAtlas.FSharp

open BibleAtlas.FSharp.Contract
open BibleAtlas.FSharp.Admission

module Positions =
    let id position =
        match position with
        | PositionRef.Node position -> ElementId.ofNodeId position.Node.Id
        | PositionRef.Edge position -> ElementId.ofEdgeId position.Edge.Id

    let sameIdentity left right =
        match left, right with
        | PositionRef.Node left, PositionRef.Node right -> left.Node.Kind = right.Node.Kind && left.Node.Id = right.Node.Id
        | PositionRef.Edge left, PositionRef.Edge right -> left.Edge.Kind = right.Edge.Kind && left.Edge.Id = right.Edge.Id
        | PositionRef.Node _, PositionRef.Edge _ | PositionRef.Edge _, PositionRef.Node _ -> false

type Explorable =
    private
    | NodeExplorable of root: ArtifactRoot * node: NodeRecord
    | EdgeExplorable of root: ArtifactRoot * edge: EdgeRecord

module rec Explorable =
    let fold node edge resolved =
        match resolved with
        | NodeExplorable(_, record) -> node record
        | EdgeExplorable(_, record) -> edge record

    let ofElement root element =
        match element with
        | Element.Node node ->
            if node.Node.Version = root then Ok(NodeExplorable(root, node.Node))
            else Error(ArtifactMoved(root, node.Node.Version))
        | Element.Edge edge -> Ok(EdgeExplorable(root, edge.Edge))
        | Element.Missing missing -> Error(Failures.graph(GraphFailure.MissingElement missing.Id))

    let root element =
        match element with
        | NodeExplorable(root, _) | EdgeExplorable(root, _) -> root

    let element resolved =
        match resolved with
        | NodeExplorable(_, node) -> Element.Node { Node = node }
        | EdgeExplorable(_, edge) -> Element.Edge { Edge = edge }

    let internal sameIdentity left right =
        Positions.sameIdentity (position left) (position right)

    let position element =
        match element with
        | NodeExplorable(_, node) -> PositionRef.Node { Node = { Id = node.Id; Kind = node.Kind; Label = node.Label } }
        | EdgeExplorable(_, edge) -> PositionRef.Edge { Edge = { Id = edge.Id; Kind = edge.Kind; Label = edge.Label } }
