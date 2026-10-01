using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Geography;

public interface IMapSource
{
    Task<MapLayers> During(int from, int to);

    Task<MapLayers> For(string scriptureRef);

    Task<IReadOnlyList<Polity>> Roster();
}

public sealed record MapLayers(Scene Scene, IReadOnlyList<Polity> Polities);
