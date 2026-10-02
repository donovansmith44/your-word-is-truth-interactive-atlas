using System.Text.Json.Serialization;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record Link([property: JsonConverter(typeof(JsonStringEnumConverter<EdgeKind>))] EdgeKind Kind, PositionRef Target)
{
    public bool Equals(Link? other) => other is not null && Kind == other.Kind && Named(Target) == Named(other.Target);

    public override int GetHashCode() => HashCode.Combine(Kind, Named(Target));

    private static (ElementKind Kind, string Id) Named(PositionRef position)
    {
        var (kind, id, _) = Positions.Of(position);
        return (kind, id);
    }
}

public sealed record Entry(Link Neighbour, Link Edge);
