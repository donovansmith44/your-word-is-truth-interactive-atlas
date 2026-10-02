using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public abstract record ElementKind
{
    private ElementKind()
    {
    }

    public abstract T Match<T>(Func<NodeKind, T> node, Func<EdgeKind, T> edge);

    public static ElementKind Of(PositionRef position) => Positions.Of(position).Kind;

    public sealed record Node(NodeKind Kind) : ElementKind
    {
        public override T Match<T>(Func<NodeKind, T> node, Func<EdgeKind, T> edge) => node(Kind);
    }

    public sealed record Edge(EdgeKind Kind) : ElementKind
    {
        public override T Match<T>(Func<NodeKind, T> node, Func<EdgeKind, T> edge) => edge(Kind);
    }
}
