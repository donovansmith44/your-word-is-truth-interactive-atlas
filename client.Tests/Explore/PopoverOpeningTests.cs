using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class PopoverOpeningTests
{
    private static readonly PositionRef Exodus = ServedGraph.At(NodeKind.Narrative, "Narrative:exodus", "The Exodus");
    private static readonly SavedExploration Saved = new("seed", "Seed", DateTimeOffset.UnixEpoch, Exodus, []);

    private static string Described(PopoverOpening opening) =>
        opening.Match(explore: target => $"explore {Positions.Of(target).Id}", resume: saved => $"resume {saved.Id}", legacy: node => $"legacy {node.Title}");

    [Fact]
    public void An_opening_on_a_position_is_matched_as_explore()
    {
        // Arrange
        PopoverOpening opening = new PopoverOpening.Explore(Exodus);

        // Act
        var described = Described(opening);

        // Assert
        Assert.Equal("explore Narrative:exodus", described);
    }

    [Fact]
    public void An_opening_on_a_save_is_matched_as_resume()
    {
        // Arrange
        PopoverOpening opening = new PopoverOpening.Resume(Saved);

        // Act
        var described = Described(opening);

        // Assert
        Assert.Equal("resume seed", described);
    }

    [Fact]
    public void An_opening_on_a_legacy_node_is_matched_as_legacy()
    {
        // Arrange
        PopoverOpening opening = new PopoverOpening.Legacy(new VerseNode("GEN.1.1"));

        // Act
        var described = Described(opening);

        // Assert
        Assert.Equal("legacy GEN.1.1", described);
    }
}
