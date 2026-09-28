using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public static class MapFocusHatch
{
    public static string Query(string placeId, TimeRange window) =>
        $"from={window.FromYear}&to={window.ToYear}&place={Uri.EscapeDataString(placeId)}";
}
