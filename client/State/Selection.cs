using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.State;

public static class Selection
{
    public const string StorageKey = "selection-v2";

    public static readonly IReadOnlyList<NodeRef> Empty = Array.Empty<NodeRef>();
}

public sealed record ToggleSelection(NodeRef Node, string? Origin = null) : IIntent<IReadOnlyList<NodeRef>>
{
    public bool Equals(ToggleSelection? other) => other is not null && PositionIdentity.Comparer.Equals(Node, other.Node) && Origin == other.Origin;

    public override int GetHashCode() => HashCode.Combine(PositionIdentity.Comparer.GetHashCode(Node), Origin);

    public string Name => "toggle-selection";

    public IReadOnlyList<NodeRef> Apply(IReadOnlyList<NodeRef> current) =>
        current.Contains(Node, PositionIdentity.Comparer)
            ? current.Where(selected => !PositionIdentity.Comparer.Equals(selected, Node)).ToList()
            : current.Append(Node).ToList();
}

public sealed record RemoveSelection(NodeRef Node, string? Origin = null) : IIntent<IReadOnlyList<NodeRef>>
{
    public bool Equals(RemoveSelection? other) => other is not null && PositionIdentity.Comparer.Equals(Node, other.Node) && Origin == other.Origin;

    public override int GetHashCode() => HashCode.Combine(PositionIdentity.Comparer.GetHashCode(Node), Origin);

    public string Name => "remove-selection";

    public IReadOnlyList<NodeRef> Apply(IReadOnlyList<NodeRef> current) =>
        current.Where(selected => !PositionIdentity.Comparer.Equals(selected, Node)).ToList();
}

public sealed record ClearSelection(string? Origin = null) : IIntent<IReadOnlyList<NodeRef>>
{
    public string Name => "clear-selection";

    public IReadOnlyList<NodeRef> Apply(IReadOnlyList<NodeRef> current) => Selection.Empty;
}
