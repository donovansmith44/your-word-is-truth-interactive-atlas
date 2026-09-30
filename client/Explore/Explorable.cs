using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public sealed class Explorable
{
    private readonly IExplorableClient _graph;

    internal Explorable(NodeCard card, IExplorableClient graph)
    {
        Card = card;
        _graph = graph;
        Groups = card.EdgeSummary.Select(entry => new FrontierGroup(entry.Kind, entry.Count)).ToList();
    }

    public NodeKind Kind => Card.Kind;

    public string Id => Card.Id;

    public string Label => Card.Label;

    public IReadOnlyList<FrontierGroup> Groups { get; }

    internal NodeCard Card { get; }

    public async Task<Page<Link>> Links(EdgeKind kind, int? cursor = null)
    {
        if (Groups.All(group => group.Kind != kind))
        {
            return new Page<Link>([], null);
        }

        var page = await _graph.Edges(Id, kind, cursor);
        return new Page<Link>(page.Entries.Select(entry => new Link(kind, entry.Node)).ToList(), page.Next);
    }

    public override bool Equals(object? obj) => obj is Explorable other && Kind == other.Kind && Id == other.Id;

    public override int GetHashCode() => HashCode.Combine(Kind, Id);

    public static bool operator ==(Explorable? left, Explorable? right) => left is null ? right is null : left.Equals(right);

    public static bool operator !=(Explorable? left, Explorable? right) => !(left == right);
}
