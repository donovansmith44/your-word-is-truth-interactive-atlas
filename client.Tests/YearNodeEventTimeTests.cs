using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

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
        var titles = times.Select(when => (new YearNode(when).Title, new YearNode(when).Kind)).ToArray();

        // Assert
        Assert.Equal([("AD 31", "Year"), ("1000 – 960 BC", "Year")], titles);
    }

    [Fact]
    public void A_place_date_is_titled_with_what_it_dates_and_the_served_label_of_its_claim()
    {
        // Arrange
        var established = new Year(label: "1003 BC", value: -1003);
        var claim = new DateClaim(@event: null, label: "c. 1003 BC", note: "traditional", verses: [], when: new TimeRange(from: established, label: "1003 BC", to: established));
        var node = new YearNode("jerusalem", new PlaceDate("Established", claim));

        // Act
        var title = (node.Title, node.Kind, node.PlaceId, node.Label);

        // Assert
        Assert.Equal(("Established c. 1003 BC", "Year", "jerusalem", "Established"), title);
    }

    [Fact]
    public async Task A_place_dates_chips_are_the_verses_its_claim_rests_on_then_the_map()
    {
        // Arrange
        var established = new Year(label: "1003 BC", value: -1003);
        var zion = new TextSpan(from: new TextPoint(unit: new BibleRef(BookId._2SA, 5, 7), word: null), to: new TextPoint(unit: new BibleRef(BookId._2SA, 5, 7), word: null));
        var claim = new DateClaim(@event: null, label: "c. 1003 BC", note: "traditional", verses: [zion], when: new TimeRange(from: established, label: "1003 BC", to: established));
        var node = new YearNode("jerusalem", new PlaceDate("Established", claim));

        // Act
        var chips = (await node.ExploreAsync(new StubbedAtlas("").Client())).Select(chip => (chip.Label, chip.ChipTestId, TargetOf(chip.Target))).ToList();

        // Assert
        Assert.Equal([("2SA.5.7", "popover-chip-verse-2SA.5.7", "Verse 2SA.5.7"), ("Show this time on the map", "popover-chip-map", "World from=-1003&to=-1003")], chips);
    }

    private static string TargetOf(ExplorationTarget target) => target switch
    {
        ExplorationTarget.Push { Next: VerseNode verse } => $"Verse {verse.Title}",
        ExplorationTarget.NavigateWorld world => $"World {world.Query}",
        _ => target.ToString(),
    };

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
