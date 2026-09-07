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

    // Q-2 fix round 1 (review finding, Low): direct, isolated proof of
    // YearNode.DedupeAndOrder -- the pure logic extracted from
    // ResolveChronologyAsync so the ordering fix (When.FromYear, then
    // ToYear, then label -- was label-only) is testable without a fetch.
    private static SceneEvent Ev(string id, string label, int fromYear, int toYear) =>
        new(id, label, new TimeRangeDto(fromYear, toYear), new List<VerseGroup>());

    [Fact]
    public void DedupeAndOrder_SortsByFromYearFirst_EvenWhenAllShareTheQueriedWindow()
    {
        // Both events legitimately appear in a "query year 31" window, but
        // the SECOND genuinely started a year earlier -- must sort first.
        // Fix round 2 (review N-4): the fixture now DELIBERATELY opposes
        // the date signal to the label signal ("Alpha" is alphabetically
        // first but chronologically second) -- the round-1 fixture's own
        // date order happened to agree with its label order, so the OLD
        // label-only sort passed it too and the test discriminated
        // nothing. This one goes RED under any label-first revert.
        var events = new[]
        {
            Ev("e1", "Alpha event", 31, 31),
            Ev("e2", "Beta event", 30, 31),
        };

        var ordered = YearNode.DedupeAndOrder(events);

        Assert.Equal(new[] { "e2", "e1" }, ordered.Select(e => e.Id));
    }

    [Fact]
    public void DedupeAndOrder_TiesOnIdenticalWhenFallBackToLabelOrdinal()
    {
        // The honest limit, disclosed: two events dated to the EXACT same
        // window carry no wire signal finer than the year itself -- the
        // label tiebreak is what remains, unchanged from before this fix.
        var events = new[]
        {
            Ev("e2", "Zeta event", 31, 31),
            Ev("e1", "Alpha event", 31, 31),
        };

        var ordered = YearNode.DedupeAndOrder(events);

        Assert.Equal(new[] { "e1", "e2" }, ordered.Select(e => e.Id));
    }

    [Fact]
    public void DedupeAndOrder_DedupesByIdAcrossMultiplePlaces_FirstOccurrenceWins()
    {
        // An event attested at more than one place in the same window
        // (rare, real) is ONE row, never one per place -- server order
        // (first occurrence) wins.
        var events = new[]
        {
            Ev("e1", "Sojourn event", 31, 31),
            Ev("e1", "Sojourn event", 31, 31), // same event, a second located place
        };

        var ordered = YearNode.DedupeAndOrder(events);

        Assert.Single(ordered);
        Assert.Equal("e1", ordered[0].Id);
    }
}
