using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public sealed class ServedPages
{
    public const int Resident = 32;

    private readonly IExplorableClient _graph;
    private readonly PageStore<(string Root, string Id, EdgeKind Kind, int Cursor, int Limit), ServedPage> _pages = new(Resident);

    internal ServedPages(IExplorableClient graph) => _graph = graph;

    internal string? Serving { get; private set; }

    internal void Saw(string root) => Serving = root;

    internal Task<ServedPage> Read(string root, string id, EdgeKind kind, int? cursor, int limit) =>
        _pages.Read((root, id, kind, cursor ?? PagedReads.FirstPage, limit), async () =>
        {
            var page = Served(root, await _graph.Edges(id, kind, cursor, limit));
            return new ServedPage(page, await Words(root, TextUnits(page)));
        });

    private static IReadOnlyList<string> TextUnits(EdgePage page) =>
        page.Entries.Select(entry => entry.Neighbour).OfType<NodePosition>().Where(position => position.Node.Kind == NodeKind.TextUnit)
            .Select(position => position.Node.Id).Distinct().ToList();

    private async Task<IReadOnlyDictionary<string, UnitText>> Words(string root, IReadOnlyList<string> units)
    {
        if (units.Count == 0)
        {
            return new Dictionary<string, UnitText>();
        }

        var served = await _graph.Elements(units);
        Saw(served.Version);
        return served.Version == root
            ? units.Zip(served.Elements, (unit, element) => (Unit: unit, Words: WordsOf(element, unit))).ToDictionary(pair => pair.Unit, pair => pair.Words)
            : throw new ArtifactMoved(root, served.Version);
    }

    private static UnitText WordsOf(Element element, string unit) => element switch
    {
        NodeElement { Node.Text: { } text } => text,
        _ => throw new ContractBreach($"the element read answered {unit}, a text unit an edge page served, without its words"),
    };

    private EdgePage Served(string root, EdgePage page)
    {
        Saw(page.Version);
        return page.Version == root ? page : throw new ArtifactMoved(root, page.Version);
    }
}

internal sealed record ServedPage(EdgePage Edges, IReadOnlyDictionary<string, UnitText> Words);

public sealed class ArtifactMoved(string resolved, string serving)
    : Exception($"an element resolved from artifact {resolved} was paged from artifact {serving}");
