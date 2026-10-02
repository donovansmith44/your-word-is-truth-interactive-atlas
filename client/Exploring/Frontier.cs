using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public sealed record FrontierGroup(EdgeKind Kind, int Count);

public sealed record Page<T>(IReadOnlyList<T> Items, int? Previous, int? Next)
{
    public bool Equals(Page<T>? other) => other is not null && Previous == other.Previous && Next == other.Next && Items.SequenceEqual(other.Items);

    public override int GetHashCode() => Items.Aggregate(HashCode.Combine(Previous, Next), HashCode.Combine);
}
