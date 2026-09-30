using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class RenderedLegacyNodesTests
{
    private static readonly Explorable Genesis1 = Resolved.Node(NodeKind.TextUnit, "text-unit:GEN.1.1", "GEN.1.1");
    private static readonly Explorable Egypt = Resolved.Node(NodeKind.Polity, "Polity:egypt", "Egypt");

    [Fact]
    public void A_node_asked_for_again_renders_the_legacy_instance_it_rendered_before()
    {
        // Arrange
        var rendered = new RenderedLegacyNodes();
        var first = rendered.For(Genesis1);

        // Act
        var again = rendered.For(Genesis1);

        // Assert
        Assert.Same(first, again);
    }

    [Fact]
    public void A_legacy_node_given_to_render_is_the_instance_that_renders_for_its_node()
    {
        // Arrange
        var rendered = new RenderedLegacyNodes();
        var given = new VerseNode("GEN.1.1");

        // Act
        rendered.Remember(Genesis1, given);

        // Assert
        Assert.Same(given, rendered.For(Genesis1));
    }

    [Fact]
    public void A_kind_with_no_legacy_body_renders_nothing()
    {
        // Arrange
        var rendered = new RenderedLegacyNodes();

        // Act
        var legacy = rendered.For(Egypt);

        // Assert
        Assert.Null(legacy);
    }
}
