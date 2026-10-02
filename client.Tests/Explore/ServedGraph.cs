using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

internal sealed class ServedGraph : IExplorableClient
{
    public const string Provenance = "kjv";
    private const string Version = "v";
    private const string EdgeId = "e";

    private readonly Dictionary<string, NodeRecord> _cards = [];
    private readonly Dictionary<string, EdgeRecord> _edges = [];
    private readonly Dictionary<(string Id, EdgeKind Kind, int? Cursor), EdgePage> _pages = [];

    public int? LimitAsked { get; private set; }

    public int ElementReads { get; private set; }

    public ServedGraph Serving(NodeRecord card)
    {
        _cards[card.Id] = card;
        return this;
    }

    public ServedGraph Serving(EdgeRecord edge)
    {
        _edges[edge.Id] = edge;
        return this;
    }

    public ServedGraph Serving(string id, EdgeKind kind, int? cursor, EdgePage page)
    {
        _pages[(id, kind, cursor)] = page;
        return this;
    }

    public Task<NodeRecord> Card(string id) => Task.FromResult(_cards[id]);

    public Task<IReadOnlyList<Element>> Elements(IReadOnlyList<string> ids)
    {
        ElementReads++;
        return Task.FromResult<IReadOnlyList<Element>>(ids.Select(Element).ToList());
    }

    public Task<EdgePage> Edges(string positionId, EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize)
    {
        LimitAsked = limit;
        return Task.FromResult(_pages[(positionId, kind, cursor)]);
    }

    public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
        throw new NotSupportedException();

    private Element Element(string id) =>
        _cards.TryGetValue(id, out var node) ? new NodeElement(node)
        : _edges.TryGetValue(id, out var edge) ? new EdgeElement(edge)
        : new MissingElement(id);

    public static NodeRecord Card(NodeKind kind, string id, string label, params FrontierGroup[] groups) =>
        new(
            book: null, catechism: null, description: null,
            edgeSummary: Summary(groups),
            @event: null, id: id, kind: kind, label: label, person: null, place: null, era: null, map: null, polity: null,
            provenance: Provenance, version: Version);

    public static EdgeRecord EdgeRecordOf(EdgeRef edge, NodeRef subject, NodeRef @object, params FrontierGroup[] groups) =>
        EdgeRecordOf(edge, subject, @object, Provenance, groups);

    public static EdgeRecord EdgeRecordOf(EdgeRef edge, NodeRef subject, NodeRef @object, string? provenance, params FrontierGroup[] groups) =>
        new(
            edgeSummary: Summary(groups), id: edge.Id, kind: edge.Kind, label: edge.Label, narrative: null,
            @object: At(@object), parentage: null, provenance: provenance, subject: At(subject), votes: null);

    public static MapDetail MapWindow(TimeRange window) => new(window: window);

    public static EraDetail EraWindow(TimeRange window) => new(window: window);

    public static PolityDetail Reign(TimeRange reign) => new(reign: reign);

    public static PlaceDetail PlaceAt(string displayName, double lat, double lon) =>
        new(blurb: null, canonicalName: null, destroyed: null, displayName: displayName, established: null, lat: lat, lon: lon);

    public static TimeRange Range(Year from, Year to, string label) => new(from: from, label: label, to: to);

    public static NodeRef Ref(NodeKind kind, string id, string label) => new(id: id, kind: kind, label: label);

    public static EdgeRef EdgeRef(EdgeKind kind, string id, string label) => new(id: id, kind: kind, label: label);

    public static PositionRef At(NodeRef node) => new NodePosition(node);

    public static PositionRef At(NodeKind kind, string id, string label) => At(Ref(kind, id, label));

    public static PositionRef AtEdge(EdgeRef edge) => new EdgePosition(edge);

    public static EdgeRef EdgeTo(EdgeKind kind, PositionRef neighbour) =>
        EdgeRef(kind, $"{EdgeId}:{Positions.Of(neighbour).Id}", $"{EdgeId} {Positions.Of(neighbour).Label}");

    public static Entry EntryTo(EdgeKind kind, PositionRef neighbour) =>
        new(new Link(kind, neighbour), new Link(kind, AtEdge(EdgeTo(kind, neighbour))));

    public static EdgePage Page(EdgeKind kind, int? next, params NodeRef[] nodes) =>
        Page(kind, next, nodes.Select(At).ToArray());

    public static EdgePage Page(EdgeKind kind, int? next, params PositionRef[] neighbours) =>
        Page(kind, next, neighbours.Select(neighbour => (EdgeTo(kind, neighbour), neighbour)).ToArray());

    public static EdgePage Page(EdgeKind kind, int? next, params (EdgeRef Edge, PositionRef Neighbour)[] entries) =>
        new(
            entries: entries.Select(entry => new EdgeEntry(edge: entry.Edge, loci: null, narrative: null, neighbour: entry.Neighbour, note: null, parentage: null, votes: null)).ToList(),
            kind: kind, next: next, version: Version);

    private static List<EdgeSummaryEntry> Summary(IEnumerable<FrontierGroup> groups) =>
        groups.Select(group => new EdgeSummaryEntry(count: group.Count, kind: group.Kind)).ToList();
}

internal static class Resolved
{
    public static Explorable Node(NodeKind kind, string id, string label, params FrontierGroup[] groups) =>
        Node(new ServedGraph().Serving(ServedGraph.Card(kind, id, label, groups)), ServedGraph.Ref(kind, id, label));

    public static Explorable Node(NodeRef identity) => Node(identity.Kind, identity.Id, identity.Label);

    public static Explorable Node(ServedGraph graph, NodeRef target) => At(graph, ServedGraph.At(target));

    public static Explorable At(ServedGraph graph, PositionRef target) =>
        new GraphExplorer(graph).Resolve(target).GetAwaiter().GetResult();
}
