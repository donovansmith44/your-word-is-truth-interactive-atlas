using System.Net.Http.Json;

namespace BibleAtlas.Client;

public sealed class GraphExplorableClient : IExplorableClient
{
    private readonly HttpClient _http;

    public GraphExplorableClient(HttpClient http) => _http = http;

    public async Task<NodeCardDto> Card(string id)
    {
        var url = $"api/node/{Uri.EscapeDataString(id)}";
        var result = await _http.GetFromJsonAsync<NodeCardDto>(url, Wire.Options);
        return result ?? throw new InvalidOperationException($"empty response body from {url}");
    }

    public async Task<EdgePageDto> Edges(string id, EdgeKindId kind, int? cursor = null, int limit = 20)
    {
        // kind.Value, never kind.ToString(): EdgeKindId's auto-generated record ToString() would
        // print "EdgeKindId { Value = mentions }", not the bare wire label.
        var url = $"api/node/{Uri.EscapeDataString(id)}/edges?kind={Uri.EscapeDataString(kind.Value)}&limit={limit}";
        if (cursor is int c)
        {
            url += $"&cursor={c}";
        }

        var result = await _http.GetFromJsonAsync<EdgePageDto>(url, Wire.Options);
        return result ?? throw new InvalidOperationException($"empty response body from {url}");
    }

    public async Task<TextWindowDto> Reading(string fromRef, int n, string dir = "onward", string corpus = "bible")
    {
        var url = $"api/text?ref={Uri.EscapeDataString(fromRef)}&n={n}&dir={Uri.EscapeDataString(dir)}&corpus={Uri.EscapeDataString(corpus)}";
        var result = await _http.GetFromJsonAsync<TextWindowDto>(url, Wire.Options);
        return result ?? throw new InvalidOperationException($"empty response body from {url}");
    }
}
