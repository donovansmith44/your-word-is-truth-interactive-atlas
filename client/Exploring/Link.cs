using System.Text.Json.Serialization;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public sealed record Link([property: JsonConverter(typeof(JsonStringEnumConverter<EdgeKind>))] EdgeKind Kind, PositionRef Target)
{
    public bool Equals(Link? other) => other is not null && Kind == other.Kind && PositionIdentity.Comparer.Equals(Target, other.Target);

    public override int GetHashCode() => HashCode.Combine(Kind, PositionIdentity.Comparer.GetHashCode(Target));
}

public sealed record Entry(Link Neighbour, Link Edge, UnitText? Words);
