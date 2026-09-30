using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record FrontierGroup(EdgeKind Kind, int Count);

public sealed record Page<T>(IReadOnlyList<T> Items, int? Next)
{
    public bool Equals(Page<T>? other) => other is not null && Next == other.Next && Items.SequenceEqual(other.Items);

    public override int GetHashCode() => Items.Aggregate(Next.GetHashCode(), HashCode.Combine);
}
