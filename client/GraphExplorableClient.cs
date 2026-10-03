#pragma warning disable ATLASWIRE
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public sealed class GraphExplorableClient : IExplorableClient
{
    private const char IdSeparator = ',';

    private readonly HttpClient _http;

    public GraphExplorableClient(HttpClient http) => _http = http;

    public Task<NodeRecord> Card(NodeId id) =>
        _http.GetRequired<NodeRecord>($"api/node/{Uri.EscapeDataString(id.Value)}");

    public async Task<ElementPage> Elements(IReadOnlyList<ElementId> ids)
    {
        var asked = string.Join(IdSeparator, ids.Select(id => Uri.EscapeDataString(id.Value)));
        var first = await ElementsAt(asked, null);
        var elements = new List<Element>(first.Elements);
        for (var page = first; page.Next is { } cursor;)
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

    private Task<ElementPage> ElementsAt(string asked, ElementPageCursor? cursor) =>
        _http.GetRequired<ElementPage>($"api/elements?ids={asked}" + (cursor is { } c ? $"&cursor={c.Value}" : ""));

    public Task<EdgePage> Edges(ElementId position, EdgeKind kind, EdgePageCursor? cursor = null, int limit = Exploring.Affordances.PageSize) =>
        _http.GetRequired<EdgePage>(
            $"api/node/{Uri.EscapeDataString(position.Value)}/edges?kind={Uri.EscapeDataString(kind.WireName())}&limit={limit}"
            + (cursor is { } c ? $"&cursor={c.Value}" : ""));

    public Task<TextWindow> Reading(TextWindowReference from, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
        _http.GetRequired<TextWindow>(
            $"api/text?ref={Uri.EscapeDataString(from.Value)}&n={n}&dir={Uri.EscapeDataString(dir.WireName())}&corpus={Uri.EscapeDataString(corpus.WireName())}");
}
