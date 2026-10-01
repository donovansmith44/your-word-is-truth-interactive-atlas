using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed class Explorable
{
    private const string UnservedEdgeLabel = "";

    private readonly Func<EdgeKind, int?, Task<EdgePage>> _page;

    internal Explorable(NodeCard card, IExplorableClient graph)
        : this(new NodePosition(new NodeRef(id: card.Id, kind: card.Kind, label: card.Label)), card.Provenance, card.EdgeSummary, (kind, cursor) => graph.Edges(card.Id, kind, cursor))
    {
    }

    internal Explorable(EdgeCard card, IExplorableClient graph)
        : this(new EdgePosition(new EdgeRef(id: card.Id, kind: card.Kind, label: card.Label)), card.Provenance, card.EdgeSummary, (kind, cursor) => graph.EdgeEdges(card.Id, kind, cursor))
    {
    }

    private Explorable(PositionRef identity, string provenance, IEnumerable<EdgeSummaryEntry> summary, Func<EdgeKind, int?, Task<EdgePage>> page)
    {
        Identity = identity;
        (Kind, Id, Label) = Positions.Of(identity);
        Provenance = provenance;
        Groups = summary.Select(entry => new FrontierGroup(entry.Kind, entry.Count)).ToList();
        _page = page;
    }

    public ElementKind Kind { get; }

    public string Id { get; }

    public string Label { get; }

    public PositionRef Identity { get; }

    public IReadOnlyList<FrontierGroup> Groups { get; }

    internal string Provenance { get; }

    public async Task<Page<Link>> Links(EdgeKind kind, int? cursor = null)
    {
        var entries = await Entries(kind, cursor);
        return new Page<Link>(entries.Items.Select(entry => entry.Neighbour).ToList(), entries.Next);
    }

    public async Task<Page<Entry>> Entries(EdgeKind kind, int? cursor = null)
    {
        if (Groups.All(group => group.Kind != kind))
        {
            return new Page<Entry>([], null);
        }

        var page = await _page(kind, cursor);
        return new Page<Entry>(page.Entries.Select(entry => new Entry(new Link(kind, entry.Neighbour), Connection(kind, entry))).ToList(), page.Next);
    }

    private static Link Connection(EdgeKind kind, EdgeEntry entry) => entry.End switch
    {
        EdgeEnd.From => new Link(EdgeKind.SourceOf, new EdgePosition(new EdgeRef(id: entry.Edge, kind: kind, label: UnservedEdgeLabel))),
        EdgeEnd.To => new Link(EdgeKind.TargetOf, new EdgePosition(new EdgeRef(id: entry.Edge, kind: kind.Dual(), label: UnservedEdgeLabel))),
    };

    public override bool Equals(object? obj) => obj is Explorable other && Kind == other.Kind && Id == other.Id;

    public override int GetHashCode() => HashCode.Combine(Kind, Id);

    public static bool operator ==(Explorable? left, Explorable? right) => left is null ? right is null : left.Equals(right);

    public static bool operator !=(Explorable? left, Explorable? right) => !(left == right);
}
