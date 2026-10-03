using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Geography;

public sealed class AtlasMapSource(AtlasClient atlas) : IMapSource
{
    private const int TimelineStart = -4004;
    private const int TimelineEnd = 100;

    public async Task<MapLayers> During(int from, int to)
    {
        var scene = atlas.SceneTime(from, to);
        var polities = atlas.Polities(from, to);

        return new MapLayers(await scene, (await polities).All);
    }

    public async Task<MapLayers> For(string scriptureRef) =>
        new(await atlas.SceneScripture(LegacyNodeIds.Read<BibleReference>(scriptureRef)), []);

    public async Task<IReadOnlyList<Polity>> Roster() =>
        (await atlas.Polities(TimelineStart, TimelineEnd)).All;
}
