using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public static class LegacyNodes
{
    public static IExplorable? For(Explorable node) => node.Kind.Match(node: kind => For(kind, node), edge: _ => null);

    public static NodeId BookContainerId(string bookCode) =>
        LegacyNodeIds.Of(NodeKind.Container, $"{BookContainerPrefix}{bookCode}");

    public static NodeId ChapterContainerId(string bookCode, int chapter) =>
        LegacyNodeIds.Of(NodeKind.Container, $"{ChapterContainerPrefix}{bookCode}{ChapterSeparator}{chapter}");

    private const string BookContainerPrefix = "bible-book-";
    private const string ChapterContainerPrefix = "bible-chapter-";
    private const char ChapterSeparator = '-';

    private static NodeRef NodeOf(Explorable node) =>
        node.Identity is NodePosition { Node: var reference } ? reference : throw new ContractBreach($"{node.Label} is an edge, not a node");

    private static IExplorable? For(NodeKind kind, Explorable node) => kind switch
    {
        NodeKind.Container => Container(LegacyNodeIds.LocalPart(NodeOf(node))),
        NodeKind.Person => new PersonNode(NodeOf(node).Id, node.Label),
        NodeKind.Event => new EventNode(LegacyNodeIds.LocalPart(NodeOf(node)), node.Label),
        NodeKind.CatechismItem => new CatechismNode(LegacyNodeIds.LocalPart(NodeOf(node)), node.Label),
        NodeKind.CommentaryItem => new CommentaryItemNode(LegacyNodeIds.LocalPart(NodeOf(node)), node.Label),
        NodeKind.TextUnit or NodeKind.Place or NodeKind.Narrative or NodeKind.Anchor or NodeKind.Era or NodeKind.Polity or NodeKind.Source
            or NodeKind.Translation or NodeKind.PeopleGroup or NodeKind.LexiconEntry or NodeKind.Map => null,
    };

    private static IExplorable? Container(string local)
    {
        if (local.StartsWith(ChapterContainerPrefix, StringComparison.Ordinal))
        {
            var bookAndChapter = local[ChapterContainerPrefix.Length..].Split(ChapterSeparator);
            return bookAndChapter.Length == 2 && int.TryParse(bookAndChapter[1], out var chapter)
                ? new ChapterNode(bookAndChapter[0], chapter)
                : null;
        }

        return local.StartsWith(BookContainerPrefix, StringComparison.Ordinal)
            ? new BookNode(local[BookContainerPrefix.Length..])
            : null;
    }
}
