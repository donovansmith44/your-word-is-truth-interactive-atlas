using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public class YearNodeEventTimeTests
{
    [Fact]
    public void EventTimeConstructor_TitleIsTheFormattedYearRange()
    {
        var node = new YearNode(new YearSpan(fromYear: 31, toYear: 31));
        Assert.Equal("AD 31", node.Title);
        Assert.Equal("Year", node.Kind);
    }

    [Fact]
    public void EventTimeConstructor_GenuineRangeFormatsBothEndpoints()
    {
        var node = new YearNode(new YearSpan(fromYear: -1000, toYear: -960));
        Assert.Equal("1000 BC – 960 BC", node.Title);
    }

    [Fact]
    public void PlaceDateClaimConstructor_TitleFormatIsUnaffectedByTheNewConstructor()
    {
        var node = new YearNode("jerusalem_1", "Established", new YearSpan(fromYear: -1003, toYear: -1003), new List<string> { "2SA.5.6" }, "traditional");
        Assert.Equal("Established c. 1003 BC", node.Title);
        Assert.Equal("Year", node.Kind);
    }

    private static readonly Year Ad30 = new(label: "AD 30", value: 30);
    private static readonly Year Ad31 = new(label: "AD 31", value: 31);
    private static readonly TimeRange Ad31Only = new(from: Ad31, label: "AD 31", to: Ad31);
    private static readonly TimeRange Ad30To31 = new(from: Ad30, label: "AD 30 – 31", to: Ad31);

    private static SceneEvent Ev(string id, string label, TimeRange when) =>
        new(id: id, label: label, verseGroups: [], when: when);

    [Fact]
    public void DedupeAndOrder_SortsByFromYearFirst_EvenWhenAllShareTheQueriedWindow()
    {
        var events = new[]
        {
            Ev("e1", "Alpha event", Ad31Only),
            Ev("e2", "Beta event", Ad30To31),
        };

        var ordered = YearNode.DedupeAndOrder(events);

        Assert.Equal(new[] { "e2", "e1" }, ordered.Select(e => e.Id));
    }

    [Fact]
    public void DedupeAndOrder_TiesOnIdenticalWhenFallBackToLabelOrdinal()
    {
        var events = new[]
        {
            Ev("e2", "Zeta event", Ad31Only),
            Ev("e1", "Alpha event", Ad31Only),
        };

        var ordered = YearNode.DedupeAndOrder(events);

        Assert.Equal(new[] { "e1", "e2" }, ordered.Select(e => e.Id));
    }

    [Fact]
    public void DedupeAndOrder_DedupesByIdAcrossMultiplePlaces_FirstOccurrenceWins()
    {
        var events = new[]
        {
            Ev("e1", "Sojourn event", Ad31Only),
            Ev("e1", "Sojourn event", Ad31Only),
        };

        var ordered = YearNode.DedupeAndOrder(events);

        Assert.Single(ordered);
        Assert.Equal("e1", ordered[0].Id);
    }
}
