using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed class Explorable
{
    private readonly IExplorableClient _graph;

    internal Explorable(NodeRecord node, IExplorableClient graph)
        : this(new NodePosition(new NodeRef(id: node.Id, kind: node.Kind, label: node.Label)), node, node.Provenance, node.EdgeSummary, [], graph)
    {
    }

    internal Explorable(EdgeRecord edge, IExplorableClient graph)
        : this(
            new EdgePosition(new EdgeRef(id: edge.Id, kind: edge.Kind, label: edge.Label)),
            null,
            edge.Provenance,
            edge.EdgeSummary,
            [new Link(edge.Kind.Dual(), edge.Subject), new Link(edge.Kind, edge.Object)],
            graph)
    {
    }

    private Explorable(PositionRef identity, NodeRecord? record, string? provenance, IEnumerable<EdgeSummaryEntry> summary, IReadOnlyList<Link> ends, IExplorableClient graph)
    {
        Identity = identity;
        Record = record;
        (Kind, Id, Label) = Positions.Of(identity);
        Provenance = provenance;
        Groups = summary.Select(entry => new FrontierGroup(entry.Kind, entry.Count)).ToList();
        Ends = ends;
        _graph = graph;
    }

    public ElementKind Kind { get; }

    public string Id { get; }

    public string Label { get; }

    public PositionRef Identity { get; }

    public IReadOnlyList<FrontierGroup> Groups { get; }

    public IReadOnlyList<Link> Ends { get; }

    internal string? Provenance { get; }

    internal NodeRecord? Record { get; }

    public async Task<Page<Entry>> Entries(EdgeKind kind, int? cursor = null)
    {
        if (Groups.All(group => group.Kind != kind))
        {
            return new Page<Entry>([], null);
        }

        var page = await _graph.Edges(Id, kind, cursor);
        return new Page<Entry>(
            page.Entries.Select(entry => new Entry(new Link(kind, entry.Neighbour), new Link(kind, new EdgePosition(entry.Edge)))).ToList(),
            page.Next);
    }

    public override bool Equals(object? obj) => obj is Explorable other && Kind == other.Kind && Id == other.Id;

    public override int GetHashCode() => HashCode.Combine(Kind, Id);

    public static bool operator ==(Explorable? left, Explorable? right) => left is null ? right is null : left.Equals(right);

    public static bool operator !=(Explorable? left, Explorable? right) => !(left == right);
}
