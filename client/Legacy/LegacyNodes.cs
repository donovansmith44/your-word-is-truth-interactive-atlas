using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public static class LegacyNodes
{
    public static IExplorable? For(Explorable node) => node.Kind.Match(node: kind => For(kind, node), edge: _ => null);

    public static string BookContainerId(string bookCode) =>
        NodeIds.Of(NodeKind.Container, $"{BookContainerPrefix}{bookCode}");

    public static string ChapterContainerId(string bookCode, int chapter) =>
        NodeIds.Of(NodeKind.Container, $"{ChapterContainerPrefix}{bookCode}{ChapterSeparator}{chapter}");

    private const string BookContainerPrefix = "bible-book-";
    private const string ChapterContainerPrefix = "bible-chapter-";
    private const char ChapterSeparator = '-';
    private const string ConcordCitationPrefix = "BoC ";

    private static IExplorable? For(NodeKind kind, Explorable node) => kind switch
    {
        NodeKind.TextUnit => TextUnit(NodeIds.LocalPart(node.Id)),
        NodeKind.Container => Container(NodeIds.LocalPart(node.Id)),
        NodeKind.Person => new PersonNode(node.Id, node.Label),
        NodeKind.Event => new EventNode(NodeIds.LocalPart(node.Id), node.Label),
        NodeKind.CatechismItem => new CatechismNode(NodeIds.LocalPart(node.Id), node.Label),
        NodeKind.CommentaryItem => new CommentaryItemNode(NodeIds.LocalPart(node.Id), node.Label),
        NodeKind.Place or NodeKind.Narrative or NodeKind.Anchor or NodeKind.Era or NodeKind.Polity or NodeKind.Source
            or NodeKind.Translation or NodeKind.PeopleGroup or NodeKind.LexiconEntry or NodeKind.Map => null,
    };

    private static IExplorable TextUnit(string citation) =>
        citation.StartsWith(ConcordCitationPrefix, StringComparison.Ordinal)
            ? new ConcordUnitNode(citation)
            : new VerseNode(citation);

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
