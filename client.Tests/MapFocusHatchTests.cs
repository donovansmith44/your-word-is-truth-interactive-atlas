using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public class MapFocusHatchTests
{
    [Fact]
    public void Query_BuildsFromToAndPlaceParams()
    {
        var query = MapFocusHatch.Query("nazareth", Span(31, 31));
        Assert.Equal("from=31&to=31&place=nazareth", query);
    }

    [Fact]
    public void Query_MatchesTheExactShapeApplyExternalQueryAlreadyParses()
    {
        var query = MapFocusHatch.Query("jerusalem", Span(-1000, -960));
        var parts = query.Split('&').Select(p => p.Split('=', 2)).ToDictionary(kv => kv[0], kv => kv[1]);
        Assert.Equal("-1000", parts["from"]);
        Assert.Equal("-960", parts["to"]);
        Assert.Equal("jerusalem", parts["place"]);
    }

    [Fact]
    public void Query_EscapesThePlaceId()
    {
        var placeId = "some id/with?special&chars";
        var query = MapFocusHatch.Query(placeId, Span(-100, -50));
        Assert.Equal($"from=-100&to=-50&place={Uri.EscapeDataString(placeId)}", query);
    }

    private static TimeRange Span(int from, int to) =>
        new(from: new Year(label: "", value: from), label: "", to: new Year(label: "", value: to));
}
