using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

// EVT-3 Ticket 3 (the §5-declared map-focus-at-time hatch): direct,
// isolated proof of MapFocusHatch.Query's own pure query-building logic --
// the piece that decides the EXACT {place, window} query string
// EventDateAndPlacesSection's own Place: row hands to
// IPopoverSectionContext.NavigateWorldAsync. The real end-to-end
// window+focus effect (World.razor reading `place=` back off this exact
// shape and panning the camera) is exercised by
// tests/ux/event-timeplace.spec.ts instead, the same "pure logic here,
// real render/interaction there" split this app's other component-adjacent
// static helpers already follow.
public class MapFocusHatchTests
{
    [Fact]
    public void Query_BuildsFromToAndPlaceParams()
    {
        var query = MapFocusHatch.Query("nazareth", new TimeRangeDto(31, 31));
        Assert.Equal("from=31&to=31&place=nazareth", query);
    }

    [Fact]
    public void Query_MatchesTheExactShapeApplyExternalQueryAlreadyParses()
    {
        // World.razor's own ApplyExternalQuery splits on '&' then '=' --
        // this pins that MapFocusHatch.Query never emits anything that
        // shape can't round-trip (no stray '&'/'=' inside an unescaped
        // segment).
        var query = MapFocusHatch.Query("jerusalem", new TimeRangeDto(-1000, -960));
        var parts = query.Split('&').Select(p => p.Split('=', 2)).ToDictionary(kv => kv[0], kv => kv[1]);
        Assert.Equal("-1000", parts["from"]);
        Assert.Equal("-960", parts["to"]);
        Assert.Equal("jerusalem", parts["place"]);
    }

    [Fact]
    public void Query_EscapesThePlaceId()
    {
        var placeId = "some id/with?special&chars";
        var query = MapFocusHatch.Query(placeId, new TimeRangeDto(-100, -50));
        Assert.Equal($"from=-100&to=-50&place={Uri.EscapeDataString(placeId)}", query);
    }
}
