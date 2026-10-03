using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public sealed class ServedPages
{
    public const int Resident = 32;

    private readonly IExplorableClient _graph;
    private readonly PageStore<(ArtifactRoot Root, ElementId Id, EdgeKind Kind, EdgePageCursor Cursor, int Limit), ServedPage> _pages = new(Resident);

    internal ServedPages(IExplorableClient graph) => _graph = graph;

    internal ArtifactRoot? Serving { get; private set; }

    internal void Saw(ArtifactRoot root) => Serving = root;

    internal Task<ServedPage> Read(ArtifactRoot root, ElementId id, EdgeKind kind, EdgePageCursor? cursor, int limit) =>
        _pages.Read((root, id, kind, cursor ?? EdgePageCursor.First, limit), async () =>
        {
            var page = Served(root, await _graph.Edges(id, kind, cursor, limit));
            return new ServedPage(page, await Words(root, TextUnits(page)));
        });

    private static IReadOnlyList<ElementId> TextUnits(EdgePage page) =>
        page.Entries.Select(entry => entry.Neighbour).OfType<NodePosition>().Where(position => position.Node.Kind == NodeKind.TextUnit)
            .Select(position => (ElementId)position.Node.Id).Distinct().ToList();

    private async Task<IReadOnlyDictionary<ElementId, UnitText>> Words(ArtifactRoot root, IReadOnlyList<ElementId> units)
    {
        if (units.Count == 0)
        {
            return new Dictionary<ElementId, UnitText>();
        }

        var served = await _graph.Elements(units);
        Saw(served.Version);
        return served.Version == root
            ? units.Zip(served.Elements, (unit, element) => (Unit: unit, Words: WordsOf(element, unit))).ToDictionary(pair => pair.Unit, pair => pair.Words)
            : throw new ArtifactMoved(root, served.Version);
    }

    private static UnitText WordsOf(Element element, ElementId unit) => element switch
    {
        NodeElement { Node.Text: { } text } => text,
        _ => throw new ContractBreach($"the element read answered {unit}, a text unit an edge page served, without its words"),
    };

    private EdgePage Served(ArtifactRoot root, EdgePage page)
    {
        Saw(page.Version);
        return page.Version == root ? page : throw new ArtifactMoved(root, page.Version);
    }
}

internal sealed record ServedPage(EdgePage Edges, IReadOnlyDictionary<ElementId, UnitText> Words);

public sealed class ArtifactMoved(ArtifactRoot resolved, ArtifactRoot serving)
    : Exception($"an element resolved from artifact {resolved} was paged from artifact {serving}");
