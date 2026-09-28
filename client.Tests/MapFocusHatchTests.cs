using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public class MapFocusHatchTests
{
    [Fact]
    public void Query_BuildsFromToAndPlaceParams()
    {
        var query = MapFocusHatch.Query("nazareth", new TimeRange(fromYear: 31, toYear: 31));
        Assert.Equal("from=31&to=31&place=nazareth", query);
    }

    [Fact]
    public void Query_MatchesTheExactShapeApplyExternalQueryAlreadyParses()
    {
        var query = MapFocusHatch.Query("jerusalem", new TimeRange(fromYear: -1000, toYear: -960));
        var parts = query.Split('&').Select(p => p.Split('=', 2)).ToDictionary(kv => kv[0], kv => kv[1]);
        Assert.Equal("-1000", parts["from"]);
        Assert.Equal("-960", parts["to"]);
        Assert.Equal("jerusalem", parts["place"]);
    }

    [Fact]
    public void Query_EscapesThePlaceId()
    {
        var placeId = "some id/with?special&chars";
        var query = MapFocusHatch.Query(placeId, new TimeRange(fromYear: -100, toYear: -50));
        Assert.Equal($"from=-100&to=-50&place={Uri.EscapeDataString(placeId)}", query);
    }
}
