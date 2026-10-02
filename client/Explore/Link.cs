using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record Link(EdgeKind Kind, NodeRef Target)
{
    public bool Equals(Link? other) =>
        other is not null && Kind == other.Kind && Target.Kind == other.Target.Kind && Target.Id == other.Target.Id;

    public override int GetHashCode() => HashCode.Combine(Kind, Target.Kind, Target.Id);
}
