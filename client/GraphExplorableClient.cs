using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public sealed class GraphExplorableClient : IExplorableClient
{
    private const char IdSeparator = ',';

    private readonly HttpClient _http;

    public GraphExplorableClient(HttpClient http) => _http = http;

    public Task<NodeRecord> Card(string id) =>
        _http.GetRequired<NodeRecord>($"api/node/{Uri.EscapeDataString(id)}");

    public async Task<ElementPage> Elements(IReadOnlyList<string> ids)
    {
        var asked = string.Join(IdSeparator, ids.Select(Uri.EscapeDataString));
        var first = await ElementsAt(asked, null);
        var elements = new List<Element>(first.Elements);
        for (var page = first; page.Next is int cursor;)
        {
            page = await ElementsAt(asked, cursor);
            if (page.Version != first.Version)
            {
                return await Elements(ids);
            }

            elements.AddRange(page.Elements);
        }

        return new ElementPage(elements: elements, next: null, previous: null, version: first.Version);
    }

    private Task<ElementPage> ElementsAt(string asked, int? cursor) =>
        _http.GetRequired<ElementPage>($"api/elements?ids={asked}" + (cursor is int c ? $"&cursor={c}" : ""));

    public Task<EdgePage> Edges(string positionId, EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize) =>
        _http.GetRequired<EdgePage>(
            $"api/node/{Uri.EscapeDataString(positionId)}/edges?kind={Uri.EscapeDataString(kind.WireName())}&limit={limit}"
            + (cursor is int c ? $"&cursor={c}" : ""));

    public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
        _http.GetRequired<TextWindow>(
            $"api/text?ref={Uri.EscapeDataString(fromRef)}&n={n}&dir={Uri.EscapeDataString(dir.WireName())}&corpus={Uri.EscapeDataString(corpus.WireName())}");
}
