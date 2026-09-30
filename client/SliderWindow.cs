using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public static class SliderWindow
{
    public static async Task<TimeRange?> ServedFor(Request request, AtlasClient atlas, TimeRange? known, int from, int to) =>
        known is { } served && served.From.Value == from && served.To.Value == to
            ? served
            : (await request.Fetch(() => atlas.SceneTime(from, to)))?.Window;
}
