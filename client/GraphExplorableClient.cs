using System.Net.Http.Json;

namespace BibleAtlas.Client;

/// <summary>
/// The ONE concrete implementation of <see cref="IExplorableClient"/> this
/// batch ships (Batch M-D2, P7 closure) -- talks to exactly the two generic
/// graph endpoints plus the text-window endpoint, nothing else. No caching,
/// no retry, no derived state: every method is a direct GET + deserialize
/// ("client stays thin: no business logic; the server's summaries/pages are
/// the truth," the batch brief's own controller decision 1) -- contrast with
/// <see cref="AtlasClient"/>, whose own per-endpoint LRU caches exist
/// because ITS callers (scene/chapter/place-history hovers) are genuinely
/// hot, repeat-visit paths; this seam's own real call sites this batch
/// (superscript-adjacent single-node lookups) are not, so adding caching
/// here preemptively would be undemonstrated complexity, not thinness.
///
/// Registered as its own DI singleton (Program.cs), reusing
/// <see cref="AtlasClient.ResolveBaseAddress"/> for the identical
/// dev-vs-release origin resolution <see cref="AtlasClient"/> already
/// established -- a SEPARATE <see cref="HttpClient"/> instance, not a
/// shared one, so this seam's own request lifecycle is never entangled with
/// the bespoke client's.
/// </summary>
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

    public async Task<EdgePageDto> Edges(string id, string kind, int? cursor = null, int limit = 20)
    {
        var url = $"api/node/{Uri.EscapeDataString(id)}/edges?kind={Uri.EscapeDataString(kind)}&limit={limit}";
        if (cursor is int c)
        {
            url += $"&cursor={c}";
        }

        var result = await _http.GetFromJsonAsync<EdgePageDto>(url, Wire.Options);
        return result ?? throw new InvalidOperationException($"empty response body from {url}");
    }

    public async Task<TextWindowDto> Reading(string fromRef, int n, string dir = "onward")
    {
        var url = $"api/text?ref={Uri.EscapeDataString(fromRef)}&n={n}&dir={Uri.EscapeDataString(dir)}";
        var result = await _http.GetFromJsonAsync<TextWindowDto>(url, Wire.Options);
        return result ?? throw new InvalidOperationException($"empty response body from {url}");
    }
}
