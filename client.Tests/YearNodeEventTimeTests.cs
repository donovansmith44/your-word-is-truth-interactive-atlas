using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

// EVT-3 Ticket 3: direct, isolated proof of YearNode's own NEW event-time
// constructor (reached from an event's own explorable Time: row,
// EventDateAndPlacesSection) -- Title/Kind identity only; the real
// chronological-layout fetch+render (ResolveFrontierAsync/ResolveChronologyAsync)
// is exercised end to end by tests/ux/event-timeplace.spec.ts instead, the
// same split every other component-adjacent static/constructor proof in
// this project already follows.
public class YearNodeEventTimeTests
{
    [Fact]
    public void EventTimeConstructor_TitleIsTheFormattedYearRange()
    {
        var node = new YearNode(new TimeRangeDto(31, 31));
        Assert.Equal("AD 31", node.Title);
        Assert.Equal("Year", node.Kind);
    }

    [Fact]
    public void EventTimeConstructor_GenuineRangeFormatsBothEndpoints()
    {
        var node = new YearNode(new TimeRangeDto(-1000, -960));
        Assert.Equal("1000 BC – 960 BC", node.Title);
    }

    // The pre-EVT-3 place-date-claim constructor is UNCHANGED -- both
    // constructors coexist on the SAME node kind (Kind == "Year" either
    // way); this pins that the new one didn't silently alter the old one's
    // own established Title format.
    [Fact]
    public void PlaceDateClaimConstructor_TitleFormatIsUnaffectedByTheNewConstructor()
    {
        var node = new YearNode("jerusalem_1", "Established", new TimeRangeDto(-1003, -1003), new List<string> { "2SA.5.6" }, "traditional");
        Assert.Equal("Established c. 1003 BC", node.Title);
        Assert.Equal("Year", node.Kind);
    }
}
