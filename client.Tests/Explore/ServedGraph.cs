using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

internal sealed class ServedGraph : IExplorableClient
{
    public const string Provenance = "kjv";
    private const string Version = "v";
    private const string EdgeId = "e";

    private readonly Dictionary<string, NodeCard> _cards = [];
    private readonly Dictionary<(string Id, EdgeKind Kind, int? Cursor), EdgePage> _pages = [];

    public int? LimitAsked { get; private set; }

    public ServedGraph Serving(NodeCard card)
    {
        _cards[card.Id] = card;
        return this;
    }

    public ServedGraph Serving(string id, EdgeKind kind, int? cursor, EdgePage page)
    {
        _pages[(id, kind, cursor)] = page;
        return this;
    }

    public Task<NodeCard> Card(string id) => Task.FromResult(_cards[id]);

    public Task<EdgePage> Edges(string id, EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize)
    {
        LimitAsked = limit;
        return Task.FromResult(_pages[(id, kind, cursor)]);
    }

    public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
        throw new NotSupportedException();

    public static NodeCard Card(NodeKind kind, string id, string label, params FrontierGroup[] groups) =>
        new(
            book: null, catechism: null, description: null,
            edgeSummary: groups.Select(group => new EdgeSummaryEntry(count: group.Count, kind: group.Kind)).ToList(),
            @event: null, id: id, kind: kind, label: label, person: null, place: null,
            provenance: Provenance, version: Version);

    public static NodeRef Ref(NodeKind kind, string id, string label) =>
        new(id: id, kind: WireNames.Parse<PositionKind>(kind.WireName()), label: label);

    public static EdgePage Page(EdgeKind kind, int? next, params NodeRef[] nodes) =>
        new(
            entries: nodes.Select(node => new EdgeEntry(edge: EdgeId, loci: null, narrative: null, node: node, note: null, parentage: null, votes: null)).ToList(),
            kind: kind, next: next, version: Version);
}

internal static class Resolved
{
    public static Explorable Node(NodeKind kind, string id, string label, params FrontierGroup[] groups) =>
        Node(new ServedGraph().Serving(ServedGraph.Card(kind, id, label, groups)), ServedGraph.Ref(kind, id, label));

    public static Explorable Node(NodeRef identity) =>
        Node(WireNames.Parse<NodeKind>(identity.Kind.WireName()), identity.Id, identity.Label);

    public static Explorable Node(ServedGraph graph, NodeRef target) =>
        new GraphExplorer(graph).Resolve(target).GetAwaiter().GetResult();
}
