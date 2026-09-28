using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests.Contract;

public sealed class EdgeSectionRegistryTests
{
    [Fact]
    public void The_registry_holds_exactly_the_four_section_policies_keyed_by_generated_kinds()
    {
        // Arrange
        var expected = new Dictionary<EdgeKind, EdgeSectionSpec>
        {
            [EdgeKind.Cites]         = new(EdgeKind.Cites,         SectionStyle.Quiet,    InitialClamp: 3,  SectionOrder.VotesRanked),
            [EdgeKind.Mentions]      = new(EdgeKind.Mentions,      SectionStyle.Standard, InitialClamp: 50, SectionOrder.Canonical),
            [EdgeKind.MentionedIn]   = new(EdgeKind.MentionedIn,   SectionStyle.Standard, InitialClamp: 12, SectionOrder.Canonical),
            [EdgeKind.CommentedOnBy] = new(EdgeKind.CommentedOnBy, SectionStyle.Standard, InitialClamp: 20, SectionOrder.Canonical),
        };
        // Act
        var actual = EdgeSectionRegistry.ByKind;
        // Assert
        Assert.Equal(expected, actual);
    }
}
