using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public class YearNodeEventTimeTests
{
    [Fact]
    public void An_event_time_is_titled_with_the_served_label_of_its_years()
    {
        // Arrange
        var times = new[]
        {
            Ad31Only,
            new TimeRange(from: new Year(label: "1000 BC", value: -1000), label: "1000 – 960 BC", to: new Year(label: "960 BC", value: -960)),
        };

        // Act
        var titles = times.Select(when => (new YearNode(when, TheEvent).Title, new YearNode(when, TheEvent).Kind)).ToArray();

        // Assert
        Assert.Equal([("AD 31", "Year"), ("1000 – 960 BC", "Year")], titles);
    }

    private static readonly NodeRef TheEvent = new(id: "Event:ab_ur", kind: NodeKind.Event, label: "Terah's family leaves Ur");
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
