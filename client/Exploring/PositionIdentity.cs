using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public sealed class PositionIdentity : IEqualityComparer<PositionRef>, IEqualityComparer<NodeRef>
{
    public static readonly PositionIdentity Comparer = new();

    private PositionIdentity()
    {
    }

    public bool Equals(PositionRef? x, PositionRef? y) =>
        x is null ? y is null : y is not null && Named(x) == Named(y);

    public int GetHashCode(PositionRef position) => Named(position).GetHashCode();

    public bool Equals(NodeRef? x, NodeRef? y) => Equals(At(x), At(y));

    public int GetHashCode(NodeRef node) => GetHashCode(new NodePosition(node));

    private static NodePosition? At(NodeRef? node) => node is null ? null : new NodePosition(node);

    private static (ElementKind Kind, ElementId Id) Named(PositionRef position)
    {
        var (kind, id, _) = Positions.Of(position);
        return (kind, id);
    }
}
