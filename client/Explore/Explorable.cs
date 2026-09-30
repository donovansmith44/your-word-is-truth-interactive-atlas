using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record Explorable(NodeKind Kind, string Id, string Label)
{
    public static Explorable From(NodeCard card) => new(card.Kind, card.Id, card.Label);

    public bool Equals(Explorable? other) => other is not null && Kind == other.Kind && Id == other.Id;

    public override int GetHashCode() => HashCode.Combine(Kind, Id);
}
