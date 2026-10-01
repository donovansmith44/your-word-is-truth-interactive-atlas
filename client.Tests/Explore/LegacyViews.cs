using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;
using BibleAtlas.Client.Legacy;

namespace BibleAtlas.Client.Tests;

internal static class LegacyViews
{
    public static readonly Year Year2000Bc = new(label: "2000 BC", value: -2000);
    public static readonly TimeRange TwoThousandBc = new(from: Year2000Bc, label: "2000 BC", to: Year2000Bc);
    public static readonly NodeRef TerahLeavesUr = new(id: "Event:ab_ur", kind: NodeKind.Event, label: "Terah's family leaves Ur");

    public static IReadOnlyList<IExplorable> Every() =>
    [
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
    ];
}
