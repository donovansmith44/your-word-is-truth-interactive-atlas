using System.Net.Http.Json;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public sealed class GraphExplorableClient : IExplorableClient
{
    private readonly HttpClient _http;

    public GraphExplorableClient(HttpClient http) => _http = http;

    public async Task<NodeCard> Card(string id)
    {
        var url = $"api/node/{Uri.EscapeDataString(id)}";
        var result = await _http.GetFromJsonAsync<NodeCard>(url);
        return result ?? throw new InvalidOperationException($"empty response body from {url}");
    }

    public async Task<EdgePage> Edges(string id, EdgeKind kind, int? cursor = null, int limit = 20)
    {
        var url = $"api/node/{Uri.EscapeDataString(id)}/edges?kind={Uri.EscapeDataString(kind.Label())}&limit={limit}";
        if (cursor is int c)
        {
            url += $"&cursor={c}";
        }

        var result = await _http.GetFromJsonAsync<EdgePage>(url);
        return result ?? throw new InvalidOperationException($"empty response body from {url}");
    }

    public async Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible)
    {
        var url = $"api/text?ref={Uri.EscapeDataString(fromRef)}&n={n}&dir={Uri.EscapeDataString(dir.WireName())}&corpus={Uri.EscapeDataString(corpus.WireName())}";
        var result = await _http.GetFromJsonAsync<TextWindow>(url);
        return result ?? throw new InvalidOperationException($"empty response body from {url}");
    }
}
