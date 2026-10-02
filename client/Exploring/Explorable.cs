using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public sealed class Explorable
{
    private readonly ServedPages _pages;

    internal Explorable(NodeRecord node, string root, ServedPages pages)
        : this(new NodePosition(new NodeRef(id: node.Id, kind: node.Kind, label: node.Label)), root, node, node.Provenance, node.EdgeSummary, [], pages)
    {
    }

    internal Explorable(EdgeRecord edge, string root, ServedPages pages)
        : this(
            new EdgePosition(new EdgeRef(id: edge.Id, kind: edge.Kind, label: edge.Label)),
            root,
            null,
            edge.Provenance,
            edge.EdgeSummary,
            [new Link(edge.Kind.Dual(), edge.Subject), new Link(edge.Kind, edge.Object)],
            pages)
    {
    }

    private Explorable(PositionRef identity, string root, NodeRecord? record, string? provenance, IEnumerable<EdgeSummaryEntry> summary, IReadOnlyList<Link> ends, ServedPages pages)
    {
        Identity = identity;
        Root = root;
        Record = record;
        (Kind, Id, Label) = Positions.Of(identity);
        Provenance = provenance;
        Groups = summary.Select(entry => new FrontierGroup(entry.Kind, entry.Count)).ToList();
        Ends = ends;
        _pages = pages;
    }

    public ElementKind Kind { get; }

    public string Id { get; }

    public string Label { get; }

    public PositionRef Identity { get; }

    public string Root { get; }

    public bool Moved => _pages.Serving is { } serving && serving != Root;

    public IReadOnlyList<FrontierGroup> Groups { get; }

    public IReadOnlyList<Link> Ends { get; }

    internal string? Provenance { get; }

    internal NodeRecord? Record { get; }

    public async Task<Page<Entry>> Entries(EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize)
    {
        if (Groups.All(group => group.Kind != kind))
        {
            return new Page<Entry>([], null);
        }

        var page = await _pages.Read(Root, Id, kind, cursor, limit);
        return new Page<Entry>(
            page.Entries.Select(entry => new Entry(new Link(kind, entry.Neighbour), new Link(kind, new EdgePosition(entry.Edge)))).ToList(),
            page.Next);
    }

    public override bool Equals(object? obj) => obj is Explorable other && PositionIdentity.Comparer.Equals(Identity, other.Identity);

    public override int GetHashCode() => PositionIdentity.Comparer.GetHashCode(Identity);

    public static bool operator ==(Explorable? left, Explorable? right) => left is null ? right is null : left.Equals(right);

    public static bool operator !=(Explorable? left, Explorable? right) => !(left == right);

    public static IEqualityComparer<Explorable> Served { get; } = new ServedComparer();

    private sealed class ServedComparer : IEqualityComparer<Explorable>
    {
        public bool Equals(Explorable? x, Explorable? y) => x == y && x?.Root == y?.Root;

        public int GetHashCode(Explorable element) => HashCode.Combine(element, element.Root);
    }
}
