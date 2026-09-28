using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public sealed class GraphExplorableClient : IExplorableClient
{
    private readonly HttpClient _http;

    public GraphExplorableClient(HttpClient http) => _http = http;

    public Task<NodeCard> Card(string id) =>
        _http.GetRequired<NodeCard>($"api/node/{Uri.EscapeDataString(id)}");

    public Task<EdgePage> Edges(string id, EdgeKind kind, int? cursor = null, int limit = 20) =>
        _http.GetRequired<EdgePage>(
            $"api/node/{Uri.EscapeDataString(id)}/edges?kind={Uri.EscapeDataString(kind.WireName())}&limit={limit}"
            + (cursor is int c ? $"&cursor={c}" : ""));

    public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
        _http.GetRequired<TextWindow>(
            $"api/text?ref={Uri.EscapeDataString(fromRef)}&n={n}&dir={Uri.EscapeDataString(dir.WireName())}&corpus={Uri.EscapeDataString(corpus.WireName())}");
}
