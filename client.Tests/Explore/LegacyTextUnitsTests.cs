using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Legacy;

namespace BibleAtlas.Client.Tests;

public sealed class LegacyTextUnitsTests
{
    private const string ServedReference = "EXO.14.21";

    [Fact]
    public void An_opening_on_a_served_reference_explores_that_text_unit()
    {
        // Arrange
        var expected = (Opens: "explore", Position: ((ElementKind)new ElementKind.Node(NodeKind.TextUnit), Wire.Element("text-unit:EXO.14.21"), ServedReference));

        // Act
        var opening = LegacyTextUnits.Opening(ServedReference);

        // Assert
        Assert.Equal(expected, opening.Match(explore: target => ("explore", Positions.Of(target)), resume: saved => ("resume", Positions.Of(saved.Start)), legacy: node => ("legacy", Positions.Of(new NodePosition(node.Identity)))));
    }
}
