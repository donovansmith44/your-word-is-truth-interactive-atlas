using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public sealed class GraphExplorableClient : IExplorableClient
{
    private const string NodeResource = "api/node";
    private const string EdgeResource = "api/edge";

    private readonly HttpClient _http;

    public GraphExplorableClient(HttpClient http) => _http = http;

    public Task<NodeCard> Card(string id) =>
        _http.GetRequired<NodeCard>($"{NodeResource}/{Uri.EscapeDataString(id)}");

    public Task<EdgeCard> EdgeCard(string id) =>
        _http.GetRequired<EdgeCard>($"{EdgeResource}/{Uri.EscapeDataString(id)}");

    public Task<EdgePage> Edges(string id, EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize) =>
        Frontier(NodeResource, id, kind, cursor, limit);

    public Task<EdgePage> EdgeEdges(string edgeId, EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize) =>
        Frontier(EdgeResource, edgeId, kind, cursor, limit);

    public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
        _http.GetRequired<TextWindow>(
            $"api/text?ref={Uri.EscapeDataString(fromRef)}&n={n}&dir={Uri.EscapeDataString(dir.WireName())}&corpus={Uri.EscapeDataString(corpus.WireName())}");

    private Task<EdgePage> Frontier(string resource, string id, EdgeKind kind, int? cursor, int limit) =>
        _http.GetRequired<EdgePage>(
            $"{resource}/{Uri.EscapeDataString(id)}/edges?kind={Uri.EscapeDataString(kind.WireName())}&limit={limit}"
            + (cursor is int c ? $"&cursor={c}" : ""));
}
