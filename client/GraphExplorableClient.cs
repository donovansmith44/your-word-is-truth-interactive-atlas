using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public sealed class GraphExplorableClient : IExplorableClient
{
    private const char IdSeparator = ',';

    private readonly HttpClient _http;

    public GraphExplorableClient(HttpClient http) => _http = http;

    public Task<NodeRecord> Card(string id) =>
        _http.GetRequired<NodeRecord>($"api/node/{Uri.EscapeDataString(id)}");

    public async Task<IReadOnlyList<Element>> Elements(IReadOnlyList<string> ids) =>
        (await _http.GetRequired<ElementPage>($"api/elements?ids={string.Join(IdSeparator, ids.Select(Uri.EscapeDataString))}")).Elements;

    public Task<EdgePage> Edges(string positionId, EdgeKind kind, int? cursor = null, int limit = IExplorableClient.DefaultPageSize) =>
        _http.GetRequired<EdgePage>(
            $"api/node/{Uri.EscapeDataString(positionId)}/edges?kind={Uri.EscapeDataString(kind.WireName())}&limit={limit}"
            + (cursor is int c ? $"&cursor={c}" : ""));

    public Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible) =>
        _http.GetRequired<TextWindow>(
            $"api/text?ref={Uri.EscapeDataString(fromRef)}&n={n}&dir={Uri.EscapeDataString(dir.WireName())}&corpus={Uri.EscapeDataString(corpus.WireName())}");
}
