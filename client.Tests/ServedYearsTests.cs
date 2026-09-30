using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class ServedYearsTests
{
    private static readonly Year Year1406Bc = new(label: "1406 BC", value: -1406);
    private static readonly Year Year1400Bc = new(label: "1400 BC", value: -1400);
    private static readonly Year Year1571Bc = new(label: "1571 BC", value: -1571);
    private static readonly Year Year1451Bc = new(label: "1451 BC", value: -1451);
    private static readonly TimeRange ConquestOfHazor = new(from: Year1406Bc, label: "1406 – 1400 BC", to: Year1400Bc);
    private static readonly AtlasClient Unreached = new(new HttpClient());

    [Fact]
    public void A_time_and_place_is_titled_with_the_served_label_of_its_years()
    {
        // Arrange
        var node = new TimeAndPlaceNode("hazor", "Hazor", "conquest_of_hazor", ConquestOfHazor, "Conquest of Hazor", []);

        // Act
        var title = node.Title;

        // Assert
        Assert.Equal("Hazor, 1406 – 1400 BC", title);
    }

    [Fact]
    public async Task A_time_and_place_shows_its_years_on_the_map()
    {
        // Arrange
        var node = new TimeAndPlaceNode("hazor", "Hazor", "conquest_of_hazor", ConquestOfHazor, "Conquest of Hazor", []);

        // Act
        var explorations = await node.ExploreAsync(Unreached);

        // Assert
        Assert.Equal([new Exploration("Show on /world", "popover-chip-map", new ExplorationTarget.NavigateWorld("from=-1406&to=-1400"))], explorations);
    }

    [Fact]
    public async Task A_person_born_and_dead_offers_each_served_year_on_the_map()
    {
        // Arrange
        var node = PersonWith(Life(birth: Year1571Bc, death: Year1451Bc));

        // Act
        var explorations = await node.ExploreAsync(Unreached);

        // Assert
        Assert.Equal(
            [
                new Exploration("Born c. 1571 BC", "popover-chip-year-born", new ExplorationTarget.NavigateWorld("from=-1571&to=-1571")),
                new Exploration("Died c. 1451 BC", "popover-chip-year-died", new ExplorationTarget.NavigateWorld("from=-1451&to=-1451")),
            ],
            explorations);
    }

    [Fact]
    public async Task A_person_known_only_by_mention_offers_the_served_years_of_the_mentions()
    {
        // Arrange
        var node = PersonWith(Life(first: Year1406Bc, last: Year1400Bc));

        // Act
        var explorations = await node.ExploreAsync(Unreached);

        // Assert
        Assert.Equal([new Exploration("Mentioned across c. 1406 BC – 1400 BC", "popover-chip-year-span", new ExplorationTarget.NavigateWorld("from=-1406&to=-1400"))], explorations);
    }

    [Fact]
    public void A_persons_life_line_reads_the_served_years()
    {
        // Arrange
        var lives = new[]
        {
            Life(birth: Year1571Bc, death: Year1451Bc),
            Life(death: Year1451Bc),
            Life(first: Year1406Bc, last: Year1400Bc),
            Life(),
        };

        // Act
        var lines = lives.Select(PersonLifeSection.LineOf).ToArray();

        // Assert
        Assert.Equal(new string?[] { "Born c. 1571 BC · Died c. 1451 BC", "Died c. 1451 BC", "Mentioned across c. 1406 BC – 1400 BC", null }, lines);
    }

    [Fact]
    public void A_polity_change_is_titled_with_the_served_labels_of_its_years()
    {
        // Arrange
        var node = new PolityDeltaNode("canaan", "Canaan", "transition", Year1406Bc, Year1400Bc, null, [], null);

        // Act
        var title = node.Title;

        // Assert
        Assert.Equal("Canaan, 1406 BC → 1400 BC", title);
    }

    private static PersonLife Life(Year? birth = null, Year? death = null, Year? first = null, Year? last = null) =>
        new(alsoCalled: [], birth: birth, death: death, eternal: false, eternalGrounds: [], first: first, gender: null, last: last);

    private static PersonNode PersonWith(PersonLife life)
    {
        var node = new PersonNode("Person:moses", "Moses");
        node.CardAsync(() => Task.FromResult(new NodeCard(book: null, catechism: null, description: null, edgeSummary: [], @event: null, id: "Person:moses", kind: NodeKind.Person, label: "Moses", person: life, place: null, provenance: "", version: "")));
        return node;
    }
}
