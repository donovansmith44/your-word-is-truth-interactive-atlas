using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed record V1Node(string Kind, string Key, string Title);

public sealed record V1Exploration(string Id, string Name, DateTimeOffset CreatedUtc, List<V1Node> Nodes);

public sealed record V2Link(EdgeKind Kind, NodeRef Target)
{
    public bool Equals(V2Link? other) => other is not null && Kind == other.Kind && PositionIdentity.Comparer.Equals(Target, other.Target);

    public override int GetHashCode() => HashCode.Combine(Kind, PositionIdentity.Comparer.GetHashCode(Target));
}

public sealed record V2Exploration(string Id, string Name, DateTimeOffset CreatedUtc, NodeRef Start, List<V2Link> Steps)
{
    public bool Equals(V2Exploration? other) =>
        other is not null && (Id, Name, CreatedUtc) == (other.Id, other.Name, other.CreatedUtc) && PositionIdentity.Comparer.Equals(Start, other.Start) && Steps.SequenceEqual(other.Steps);

    public override int GetHashCode() => Steps.Aggregate(HashCode.Combine(Id, Name, CreatedUtc, PositionIdentity.Comparer.GetHashCode(Start)), HashCode.Combine);
}

public sealed record Translated<T>(IReadOnlyList<T> Kept, int Dropped);

public static class LegacySaves
{
    public const string ExplorationsKey = "explorations-v1";
    public const string SelectionKey = "selection-v1";
    public const string V2ExplorationsKey = "explorations-v2";

    private const char KeySeparator = '|';
    private const char ChapterSeparator = '.';

    public static Translated<SavedExploration> Explorations(IReadOnlyList<V1Exploration> v1) =>
        v1.Select(Exploration).Aggregate(
            new Translated<SavedExploration>([], 0),
            (sum, one) => new([.. sum.Kept, .. one.Kept], sum.Dropped + one.Dropped));

    public static IReadOnlyList<SavedExploration> Explorations(IReadOnlyList<V2Exploration> v2) => v2.Select(Exploration).ToList();

    public static SavedExploration Exploration(V2Exploration v2) =>
        new(v2.Id, v2.Name, v2.CreatedUtc, new NodePosition(v2.Start), v2.Steps.Select(step => new Link(step.Kind, new NodePosition(step.Target))).ToList());

    public static Translated<SavedExploration> Exploration(V1Exploration v1)
    {
        var nodes = Nodes(v1.Nodes);
        var kept = nodes.Kept;
        if (kept.Count == 0)
        {
            return new([], nodes.Dropped);
        }

        var steps = kept.Skip(1).Select((node, before) => new Link(Via(kept[before].Kind, node.Kind), new NodePosition(node))).ToList();
        return new([new SavedExploration(v1.Id, v1.Name, v1.CreatedUtc, new NodePosition(kept[0]), steps)], nodes.Dropped);
    }

    public static Translated<NodeRef> Nodes(IReadOnlyList<V1Node> v1)
    {
        var kept = v1.Select(Node).OfType<NodeRef>().ToList();
        return new(kept, v1.Count - kept.Count);
    }

    public static NodeRef? Node(V1Node node) => node.Kind switch
    {
        "Verse" or "ConcordUnit" => Ref(NodeKind.TextUnit, NodeIds.Of(NodeKind.TextUnit, node.Key), node.Title),
        "Passage" => Ref(NodeKind.TextUnit, NodeIds.Of(NodeKind.TextUnit, CanonRef.FirstVerseOf(node.Key)), node.Title),
        "Chapter" => Chapter(node),
        "Book" => Ref(NodeKind.Container, LegacyNodes.BookContainerId(node.Key), node.Title),
        "Place" => Ref(NodeKind.Place, NodeIds.Of(NodeKind.Place, node.Key), node.Title),
        "Person" => Ref(NodeKind.Person, node.Key, node.Title),
        "Event" => Ref(NodeKind.Event, NodeIds.Of(NodeKind.Event, node.Key), node.Title),
        "TimeAndPlace" => AttestedEvent(node),
        "Catechism" => Ref(NodeKind.CatechismItem, NodeIds.Of(NodeKind.CatechismItem, node.Key), node.Title),
        "CommentaryItem" => Ref(NodeKind.CommentaryItem, NodeIds.Of(NodeKind.CommentaryItem, node.Key), node.Title),
        _ => null,
    };

    private static EdgeKind Via(NodeKind from, NodeKind to) => (from, to) switch
    {
        (_, NodeKind.Container) => EdgeKind.MemberOf,
        (NodeKind.Container, NodeKind.TextUnit) => EdgeKind.Contains,
        (NodeKind.TextUnit, NodeKind.Place or NodeKind.Person) => EdgeKind.Mentions,
        (NodeKind.Place, NodeKind.Event) => EdgeKind.SiteOf,
        _ => EdgeKind.MemberOf,
    };

    private static NodeRef? Chapter(V1Node node)
    {
        var bookAndChapter = node.Key.Split(ChapterSeparator);
        return bookAndChapter.Length == 2 && int.TryParse(bookAndChapter[1], out var chapter)
            ? Ref(NodeKind.Container, LegacyNodes.ChapterContainerId(bookAndChapter[0], chapter), node.Title)
            : null;
    }

    private static NodeRef? AttestedEvent(V1Node node)
    {
        var placeAndEvent = node.Key.Split(KeySeparator, 2);
        return placeAndEvent.Length == 2
            ? Ref(NodeKind.Event, NodeIds.Of(NodeKind.Event, placeAndEvent[1]), node.Title)
            : null;
    }

    private static NodeRef Ref(NodeKind kind, string id, string label) => new(id: id, kind: kind, label: label);
}
