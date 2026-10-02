using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

public sealed class IdentityTests
{
    [Fact]
    public void Every_legacy_node_names_the_served_node_it_stands_for()
    {
        // Arrange
        var nodes = new IExplorable[]
        {
            new VerseNode("GEN.1.1"),
            new ConcordUnitNode("BoC 7.2.1"),
            new ChapterNode("GEN", 1, 50),
            new BookNode("GEN"),
            new PassageNode("GEN.1.1-5", "In the beginning"),
            new PersonNode("Person:moses_2108", "Moses"),
            new EventNode("ab_ur", TerahLeavesUr.Label),
            new CatechismNode("commandment-1", "The First Commandment"),
            new CommentaryItemNode("kretzmann/0.1.0", "The Creation of the World.: The Creation of Chaos and Light"),
            new AuthorNode("GEN"),
            new YearNode(TwoThousandBc, TerahLeavesUr),
            new PolityDeltaNode("egypt", "Egypt", "fall", TwoThousandBc.From, TwoThousandBc.To, null, [], null),
        };
        // Act
        var identities = nodes.Select(n => (n.Identity.Kind, n.Identity.Id, n.Identity.Label)).ToList();
        // Assert
        Assert.Equal(
            [
                (NodeKind.TextUnit, "text-unit:GEN.1.1", "GEN.1.1"),
                (NodeKind.TextUnit, "text-unit:BoC 7.2.1", "BoC 7.2.1"),
                (NodeKind.Container, "Container:bible-chapter-GEN-1", "GEN.1"),
                (NodeKind.Container, "Container:bible-book-GEN", "GEN"),
                (NodeKind.TextUnit, "text-unit:GEN.1.1", "GEN.1.1-5"),
                (NodeKind.Person, "Person:moses_2108", "Moses"),
                (NodeKind.Event, "Event:ab_ur", "Terah's family leaves Ur"),
                (NodeKind.CatechismItem, "CatechismItem:commandment-1", "The First Commandment"),
                (NodeKind.CommentaryItem, "CommentaryItem:kretzmann/0.1.0", "The Creation of the World.: The Creation of Chaos and Light"),
                (NodeKind.Container, "Container:bible-book-GEN", "GEN"),
                (NodeKind.Event, "Event:ab_ur", "Terah's family leaves Ur"),
                (NodeKind.Polity, "Polity:egypt", "Egypt"),
            ],
            identities);
    }

    private static readonly Year Year2000Bc = new(label: "2000 BC", value: -2000);
    private static readonly TimeRange TwoThousandBc = new(from: Year2000Bc, label: "2000 BC", to: Year2000Bc);
    private static readonly NodeRef TerahLeavesUr = new(id: "Event:ab_ur", kind: NodeKind.Event, label: "Terah's family leaves Ur");
}
