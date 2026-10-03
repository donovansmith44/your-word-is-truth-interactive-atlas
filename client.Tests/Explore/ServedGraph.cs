using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

internal sealed class ServedGraph : IExplorableClient
{
    public const string Provenance = "kjv";
    public static readonly BibleAtlas.Client.Contract.Provenance ServedProvenance = new(id: Provenance, title: "The King James Version");
    public const string Version = "v";
    public const string EdgeId = "e";

    private readonly Dictionary<string, NodeRecord> _cards = [];
    private readonly Dictionary<string, EdgeRecord> _edges = [];
    private readonly Dictionary<(string Id, EdgeKind Kind, int? Cursor), EdgePage> _pages = [];
    private string _root = Version;
    private string? _elementsRoot;

    public int? LimitAsked { get; private set; }

    public int ElementReads { get; private set; }

    public int NeighbourReads { get; private set; }

    public ServedGraph AtRoot(string root)
    {
        _root = root;
        return this;
    }

    public ServedGraph AtRoot(ArtifactRoot root) => AtRoot(root.ToString());

    public ServedGraph ElementsAt(ArtifactRoot root) => ElementsAt(root.ToString());

    public ServedGraph ElementsAt(string root)
    {
        _elementsRoot = root;
        return this;
    }

    public ServedGraph Serving(NodeRecord card)
    {
        _cards[card.Id.ToString()] = card;
        return this;
    }

    public ServedGraph Serving(EdgeRecord edge)
    {
        _edges[edge.Id.ToString()] = edge;
        return this;
    }

    public ServedGraph Serving(string id, EdgeKind kind, int? cursor, EdgePage page)
    {
        _pages[(id, kind, cursor)] = page;
        foreach (var unit in page.Entries.Select(entry => entry.Neighbour).OfType<NodePosition>().Select(position => position.Node).Where(node => node.Kind == NodeKind.TextUnit))
        {
            _cards.TryAdd(unit.Id.ToString(), TextCard(unit, WordsOf(unit.Id.ToString())));
        }

        return this;
    }

    public ServedGraph Serving(ElementId id, EdgeKind kind, int? cursor, EdgePage page) => Serving(id.ToString(), kind, cursor, page);

    public ServedGraph Failing(ElementId id, EdgeKind kind) => Failing(id.ToString(), kind);

    public ServedGraph Failing(string id, EdgeKind kind)
    {
        _pages.Remove((id, kind, null));
        return this;
    }

    public Task<NodeRecord> Card(NodeId id) => Task.FromResult(_cards[id.ToString()]);

    public Task<ElementPage> Elements(IReadOnlyList<ElementId> ids)
    {
        ElementReads++;
        return Task.FromResult(new ElementPage(elements: ids.Select(Element).ToList(), next: null, previous: null, version: Wire.Root(_elementsRoot ?? _root)));
    }

    public Task<EdgePage> Edges(ElementId position, EdgeKind kind, EdgePageCursor? cursor = null, int limit = BibleAtlas.Client.Exploring.Affordances.PageSize)
    {
        LimitAsked = limit;
        NeighbourReads++;
        return Task.FromResult(_pages[(position.ToString(), kind, cursor?.Value)] with { Version = Wire.Root(_root) });
    }

    public Task<TextWindow> Reading(TextWindowReference fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
        throw new NotSupportedException();

    private Element Element(ElementId id) =>
        _cards.TryGetValue(id.ToString(), out var node) ? new NodeElement(node)
        : _edges.TryGetValue(id.ToString(), out var edge) ? new EdgeElement(edge)
        : new MissingElement(id);

    public static NodeRecord Card(NodeKind kind, string id, string label, params FrontierGroup[] groups) =>
        new(
            book: null, catechism: null, description: null,
            edgeSummary: Summary(groups),
            @event: null, id: Wire.Node(id), kind: kind, label: label, person: null, place: null, era: null, map: null, polity: null,
            provenance: ServedProvenance, text: null, version: Wire.Root(Version));

    public static NodeRecord Card(NodeKind kind, NodeId id, string label, params FrontierGroup[] groups) => Card(kind, id.ToString(), label, groups);

    public static NodeRecord Card(NodeKind kind, ElementId id, string label, params FrontierGroup[] groups) => Card(kind, id.ToString(), label, groups);

    public static NodeRecord TextCard(NodeRef unit, UnitText text, params FrontierGroup[] groups) =>
        Card(unit.Kind, unit.Id.ToString(), unit.Label, groups) with { Text = text };

    public static UnitText UnitTextOf(TextRef locus, string text, IReadOnlyList<Anchor> anchors, IReadOnlyList<WordsOfChristSpan> wordsOfChrist) =>
        new(anchors: anchors, locus: locus, text: text, wordsOfChrist: wordsOfChrist);

    public static UnitText WordsOf(string words) => UnitTextOf(new BibleRef(book: BookId.GEN, chapter: 1, verse: 1), words, [], []);

    public static Element ElementOf(ElementId id, NodeRecord subject) => ElementOf(id.ToString(), subject);

    public static Element ElementOf(string id, NodeRecord subject) =>
        new NodeElement(id == subject.Id.ToString() ? subject : TextCard(Ref(NodeKind.TextUnit, id, id), WordsOf(id)));

    public static Anchor AnchorOf(EdgeKind kind, NodeRef node, int start, int end) => new(end: end, kind: kind, node: node, start: start);

    public static EdgeRecord EdgeRecordOf(EdgeRef edge, NodeRef subject, NodeRef @object, params FrontierGroup[] groups) =>
        EdgeRecordOf(edge, subject, @object, ServedProvenance, groups);

    public static EdgeRecord EdgeRecordOf(EdgeRef edge, NodeRef subject, NodeRef @object, BibleAtlas.Client.Contract.Provenance? provenance, params FrontierGroup[] groups) =>
        new(
            edgeSummary: Summary(groups), id: edge.Id, kind: edge.Kind, label: edge.Label, narrative: null,
            @object: At(@object), parentage: null, provenance: provenance, subject: At(subject), votes: null);

    public static MapDetail MapWindow(TimeRange window) => new(window: window);

    public static EraDetail EraWindow(TimeRange window) => new(window: window);

    public static PolityDetail Reign(TimeRange reign) => new(reign: reign);

    public static PlaceDetail PlaceAt(string displayName, double lat, double lon) =>
        new(canonicalName: null, destroyed: null, displayName: displayName, established: null, lat: lat, lon: lon);

    public static TimeRange Range(Year from, Year to, string label) => new(from: from, label: label, to: to);

    public static EdgePageCursor? PageBefore(int from, int limit) => from > 0 ? Wire.EdgeCursor(Math.Max(0, from - limit)) : null;

    public static NodeRef Ref(NodeKind kind, string id, string label) => new(id: Wire.Node(id), kind: kind, label: label);

    public static EdgeRef EdgeRef(EdgeKind kind, string id, string label) => new(id: Wire.Edge(id), kind: kind, label: label);

    public static EdgeRef EdgeRef(EdgeKind kind, EdgeId id, string label) => new(id: id, kind: kind, label: label);

    public static PositionRef At(NodeRef node) => new NodePosition(node);

    public static PositionRef At(NodeKind kind, string id, string label) => At(Ref(kind, id, label));

    public static PositionRef At(NodeKind kind, NodeId id, string label) => At(new NodeRef(id: id, kind: kind, label: label));

    public static PositionRef AtEdge(EdgeRef edge) => new EdgePosition(edge);

    public static EdgeRef EdgeTo(EdgeKind kind, PositionRef neighbour) =>
        EdgeRef(kind, $"{EdgeId}:{Positions.Of(neighbour).Id}", $"{EdgeId} {Positions.Of(neighbour).Label}");

    public static Entry EntryTo(EdgeKind kind, PositionRef neighbour) =>
        new(new Link(kind, neighbour), new Link(kind, AtEdge(EdgeTo(kind, neighbour))), null);

    public static EdgePage Page(EdgeKind kind, int? next, params NodeRef[] nodes) =>
        Page(kind, next, nodes.Select(At).ToArray());

    public static EdgePage Page(EdgeKind kind, int? next, params PositionRef[] neighbours) =>
        Page(kind, next, neighbours.Select(neighbour => (EdgeTo(kind, neighbour), neighbour)).ToArray());

    public static EdgePage Page(EdgeKind kind, int? next, params (EdgeRef Edge, PositionRef Neighbour)[] entries) =>
        new(
            entries: entries.Select(entry => new EdgeEntry(edge: entry.Edge, loci: null, narrative: null, neighbour: entry.Neighbour, note: null, parentage: null, votes: null)).ToList(),
            kind: kind, next: Wire.EdgeCursor(next), previous: null, version: Wire.Root(Version));

    private static List<EdgeSummaryEntry> Summary(IEnumerable<FrontierGroup> groups) =>
        groups.Select(group => new EdgeSummaryEntry(count: group.Count, kind: group.Kind)).ToList();
}

internal static class Resolved
{
    public static Explorable Node(NodeKind kind, string id, string label, params FrontierGroup[] groups) =>
        Node(new ServedGraph().Serving(ServedGraph.Card(kind, id, label, groups)), ServedGraph.Ref(kind, id, label));

    public static Explorable Node(NodeRef identity) => Node(identity.Kind, identity.Id.ToString(), identity.Label);

    public static Explorable Node(NodeKind kind, NodeId id, string label, params FrontierGroup[] groups) => Node(kind, id.ToString(), label, groups);

    public static Explorable Node(ServedGraph graph, NodeRef target) => At(graph, ServedGraph.At(target));

    public static Explorable At(IExplorableClient graph, PositionRef target) =>
        new GraphExplorer(graph).BeginAt(target).GetAwaiter().GetResult();

    public static async Task<Explorable> BeginAt(this IExplorer explorer, PositionRef target) =>
        await Explore.Begin(explorer, target) is Outcome<Exploration>.Arrived { Value: var begun }
            ? begun.Current
            : throw new InvalidOperationException($"beginning at {Positions.Of(target).Label} did not arrive");
}

internal static class Walked
{
    public static Exploration WalkedBack(this Exploration trail) =>
        Explore.Back.Run(new GraphExplorer(new ServedGraph()), trail).GetAwaiter().GetResult() is Outcome<(Explorable Value, Exploration Trail)>.Arrived { Value.Trail: var back }
            ? back
            : throw new InvalidOperationException($"going back from {trail.Current.Label} asked the graph");
}
