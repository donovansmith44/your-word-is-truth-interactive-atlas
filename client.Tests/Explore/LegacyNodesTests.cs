using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public sealed class LegacyNodesTests
{
    [Fact]
    public void A_served_node_reopens_as_the_legacy_node_that_stands_for_it_or_takes_the_generic_path()
    {
        // Arrange
        var served = new[]
        {
            Resolved.Node(NodeKind.TextUnit, "text-unit:GEN.1.1", "GEN.1.1"),
            Resolved.Node(NodeKind.TextUnit, "text-unit:BoC 7.2.1", "BoC 7.2.1"),
            Resolved.Node(NodeKind.Container, "Container:bible-chapter-GEN-1", "Genesis 1"),
            Resolved.Node(NodeKind.Container, "Container:bible-book-GEN", "Genesis"),
            Resolved.Node(NodeKind.Container, "Container:bible", "The Holy Bible"),
            Resolved.Node(NodeKind.Container, "Container:concord-art-preface-1", "Preface"),
            Resolved.Node(NodeKind.Event, "Event:ab_ur", "Terah's family leaves Ur"),
            Resolved.Node(NodeKind.Narrative, "Narrative:exodus", "The Exodus"),
            Resolved.Node(NodeKind.Place, "Place:hazor-1", "Hazor"),
            Resolved.Node(NodeKind.Person, "Person:moses_2108", "Moses"),
            Resolved.Node(NodeKind.Anchor, "Anchor:a", "1000 BC"),
            Resolved.Node(NodeKind.Era, "Era:e", "Judges"),
            Resolved.Node(NodeKind.Polity, "Polity:egypt", "Ptolemaic Egypt"),
            Resolved.Node(NodeKind.CatechismItem, "CatechismItem:commandment-1", "The First Commandment"),
            Resolved.Node(NodeKind.Source, "Source:kjv", "KJV"),
            Resolved.Node(NodeKind.Translation, "Translation:kjv", "KJV"),
            Resolved.Node(NodeKind.PeopleGroup, "PeopleGroup:p", "Philistines"),
            Resolved.Node(NodeKind.CommentaryItem, "CommentaryItem:kretzmann/0.1.0", "The Creation of Chaos and Light"),
            Resolved.Node(NodeKind.LexiconEntry, "LexiconEntry:H7225", "H7225"),
            Resolved.Node(NodeKind.Map, "Map:m", "Canaan"),
        };
        // Act
        var reopened = served.Select(node => Shape(LegacyNodes.For(node))).ToList();
        // Assert
        Assert.Equal(
            [
                "VerseNode GEN.1.1",
                "ConcordUnitNode BoC 7.2.1",
                "ChapterNode GEN.1",
                "BookNode GEN",
                GenericPath,
                GenericPath,
                "EventNode Terah's family leaves Ur",
                GenericPath,
                GenericPath,
                "PersonNode Moses",
                GenericPath,
                GenericPath,
                GenericPath,
                "CatechismNode The First Commandment",
                GenericPath,
                GenericPath,
                GenericPath,
                "CommentaryItemNode The Creation of Chaos and Light",
                GenericPath,
                GenericPath,
            ],
            reopened);
    }

    [Fact]
    public void Every_node_kind_has_a_bridge_answer()
    {
        // Arrange
        var everyKind = Enum.GetValues<NodeKind>();
        // Act
        var answered = everyKind.Select(kind => Record.Exception(() => LegacyNodes.For(Resolved.Node(kind, "x:y", "y")))).ToList();
        // Assert
        Assert.All(answered, exception => Assert.Null(exception));
    }

    [Fact]
    public void A_legacy_node_reopened_from_its_identity_keeps_that_identity()
    {
        // Arrange
        var legacy = new IExplorable[]
        {
            new VerseNode("GEN.1.1"),
            new ConcordUnitNode("BoC 7.2.1"),
            new ChapterNode("GEN", 1),
            new BookNode("GEN"),
            new PassageNode("GEN.1.1-5", "In the beginning"),
            new PersonNode("Person:moses_2108", "Moses"),
            new EventNode("ab_ur", "Terah's family leaves Ur"),
            new CatechismNode("commandment-1", "The First Commandment"),
            new CommentaryItemNode("kretzmann/0.1.0", "The Creation of Chaos and Light"),
            new AuthorNode("GEN"),
            new YearNode(TwoThousandBc, ServedGraph.Ref(NodeKind.Event, "Event:ab_ur", "Terah's family leaves Ur")),
            new PolityDeltaNode("egypt", "Egypt", "fall", TwoThousandBc.From, TwoThousandBc.To, null, [], null),
        };
        // Act
        var reopened = legacy.Select(node => Reopened(Resolved.Node(node.Identity))).ToList();
        // Assert
        Assert.Equal(
            [
                ("VerseNode GEN.1.1", true),
                ("ConcordUnitNode BoC 7.2.1", true),
                ("ChapterNode GEN.1", true),
                ("BookNode GEN", true),
                ("VerseNode GEN.1.1", true),
                ("PersonNode Moses", true),
                ("EventNode Terah's family leaves Ur", true),
                ("CatechismNode The First Commandment", true),
                ("CommentaryItemNode The Creation of Chaos and Light", true),
                ("BookNode GEN", true),
                ("EventNode Terah's family leaves Ur", true),
                (GenericPath, false),
            ],
            reopened);
    }

    private const string GenericPath = "generic";

    private static string Shape(IExplorable? node) => node is null ? GenericPath : $"{node.GetType().Name} {node.Title}";

    private static (string Shape, bool KeepsItsIdentity) Reopened(Explorable identity)
    {
        var again = LegacyNodes.For(identity);
        return (Shape(again), again is not null && Resolved.Node(again.Identity) == identity);
    }

    private static readonly Year Year2000Bc = new(label: "2000 BC", value: -2000);
    private static readonly TimeRange TwoThousandBc = new(from: Year2000Bc, label: "2000 BC", to: Year2000Bc);
}
