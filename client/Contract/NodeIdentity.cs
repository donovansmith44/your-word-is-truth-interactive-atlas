namespace BibleAtlas.Client.Contract;

public sealed class NodeIdentity : IEqualityComparer<NodeRef>
{
    public static readonly NodeIdentity Comparer = new();

    private NodeIdentity()
    {
    }

    public bool Equals(NodeRef? x, NodeRef? y) =>
        x is null ? y is null : y is not null && x.Kind == y.Kind && x.Id == y.Id;

    public int GetHashCode(NodeRef node) => HashCode.Combine(node.Kind, node.Id);
}
