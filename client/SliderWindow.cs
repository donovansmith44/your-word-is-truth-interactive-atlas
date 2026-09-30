using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public static class SliderWindow
{
    // The slider shows the server's label for its window; a window no time scene has served yet
    // (a first visit straight into scripture mode) is asked for through the same cached scene
    // request time mode makes, so returning to time costs nothing more.
    public static async Task<TimeRange?> ServedFor(AtlasClient atlas, TimeRange? known, int from, int to) =>
        known is { } served && served.From.Value == from && served.To.Value == to
            ? served
            : (await atlas.SceneTime(from, to)).Window;
}
