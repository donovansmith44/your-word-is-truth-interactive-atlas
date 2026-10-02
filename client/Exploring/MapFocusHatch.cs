using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public static class MapFocusHatch
{
    public static string Query(string placeId, TimeRange window) =>
        $"from={window.From.Value}&to={window.To.Value}&place={Uri.EscapeDataString(placeId)}";
}
