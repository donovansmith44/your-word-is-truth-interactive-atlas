using System.Diagnostics;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public static class Positions
{
    public static (ElementKind Kind, ElementId Id, string Label) Of(PositionRef position) => position switch
    {
        NodePosition { Node: var node } => (new ElementKind.Node(node.Kind), (ElementId)node.Id, node.Label),
        EdgePosition { Edge: var edge } => (new ElementKind.Edge(edge.Kind), (ElementId)edge.Id, edge.Label),
        _ => throw new UnreachableException($"{position.GetType().Name} is a position the contract does not declare"),
    };
}
