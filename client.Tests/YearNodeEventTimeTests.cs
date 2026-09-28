using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public class YearNodeEventTimeTests
{
    [Fact]
    public void EventTimeConstructor_TitleIsTheFormattedYearRange()
    {
        var node = new YearNode(new TimeRange(fromYear: 31, toYear: 31));
        Assert.Equal("AD 31", node.Title);
        Assert.Equal("Year", node.Kind);
    }

    [Fact]
    public void EventTimeConstructor_GenuineRangeFormatsBothEndpoints()
    {
        var node = new YearNode(new TimeRange(fromYear: -1000, toYear: -960));
        Assert.Equal("1000 BC – 960 BC", node.Title);
    }

    [Fact]
    public void PlaceDateClaimConstructor_TitleFormatIsUnaffectedByTheNewConstructor()
    {
        var node = new YearNode("jerusalem_1", "Established", new TimeRange(fromYear: -1003, toYear: -1003), new List<string> { "2SA.5.6" }, "traditional");
        Assert.Equal("Established c. 1003 BC", node.Title);
        Assert.Equal("Year", node.Kind);
    }

    private static SceneEvent Ev(string id, string label, int fromYear, int toYear) =>
        new(id: id, label: label, verseGroups: [], when: new TimeRange(fromYear: fromYear, toYear: toYear));

    [Fact]
    public void DedupeAndOrder_SortsByFromYearFirst_EvenWhenAllShareTheQueriedWindow()
    {
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
        var events = new[]
        {
            Ev("e1", "Sojourn event", 31, 31),
            Ev("e1", "Sojourn event", 31, 31),
        };

        var ordered = YearNode.DedupeAndOrder(events);

        Assert.Single(ordered);
        Assert.Equal("e1", ordered[0].Id);
    }
}
