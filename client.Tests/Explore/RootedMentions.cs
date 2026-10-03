using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

internal sealed class RootedMentions(int size) : IExplorableClient
{
    public static readonly PositionRef Person = ServedGraph.At(NodeKind.Person, "Person:abraham", "Abraham");

    private int _failing;

    public string Root { get; set; } = "root-a";

    private ArtifactRoot Served => Wire.Root(Root);

    public int Asked { get; private set; }

    public RootedMentions Failing(int reads)
    {
        _failing = reads;
        return this;
    }

    public static string Label(string root, int verse) => $"{root}:GEN.1.{verse}";

    public Task<NodeRecord> Card(NodeId id) => throw new NotSupportedException();

    public Task<ElementPage> Elements(IReadOnlyList<ElementId> ids) =>
        Task.FromResult(new ElementPage(
            elements: ids.Select(id => ServedGraph.ElementOf(id, ServedGraph.Card(NodeKind.Person, Positions.Of(Person).Id, Positions.Of(Person).Label, new FrontierGroup(EdgeKind.MentionedIn, size)) with { Version = Served })).ToList(),
            next: null, previous: null, version: Served));

    public Task<EdgePage> Edges(ElementId positionId, EdgeKind kind, EdgePageCursor? cursor = null, int limit = BibleAtlas.Client.Exploring.Affordances.PageSize)
    {
        Asked++;
        if (_failing > 0)
        {
            _failing--;
            return Task.FromException<EdgePage>(new HttpRequestException("offline"));
        }

        var from = cursor?.Value ?? 0;
        var to = Math.Min(from + limit, size);
        var verses = Enumerable.Range(from, to - from).Select(n => ServedGraph.Ref(NodeKind.TextUnit, $"text-unit:{Label(Root, n)}", Label(Root, n))).ToArray();
        return Task.FromResult(ServedGraph.Page(kind, to < size ? to : null, verses) with { Previous = ServedGraph.PageBefore(from, limit), Version = Served });
    }

    public Task<TextWindow> Reading(TextWindowReference fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
        throw new NotSupportedException();

    public async Task<PageWindow<Entry>> Opened() => await Paging.Window(await Presenting(), EdgeKind.MentionedIn);

    public async Task<PresentationRequest> Presenting() => new(await new GraphExplorer(this).BeginAt(Person), Surface.Popover);
}
