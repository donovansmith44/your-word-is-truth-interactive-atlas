using System.Net.Http;
using System.Net.Http.Json;
using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.Extensions.Configuration;

namespace BibleAtlas.Client;

public sealed class AtlasClient
{
    private readonly HttpClient _http;
    private readonly LruCache<string, Scene> _sceneCache = new(capacity: 48);
    private readonly LruCache<string, ChapterOut> _chapterCache = new(capacity: 24);
    private readonly LruCache<string, PolitiesOut> _politiesCache = new(capacity: 12);
    private readonly LruCache<string, PlaceDetail> _placeHistoryCache = new(capacity: 24);
    private readonly LruCache<string, List<CrossRefOut>> _xrefsCache = new(capacity: 24);
    private readonly LruCache<string, List<CatechismRefDto>> _catechismSpanCache = new(capacity: 24);
    // AsyncMemo, not a plain cache: several callers can independently invoke Books()/Eras()/etc.
    // concurrently before any has resolved (e.g. around app startup), so a plain cache would
    // double-fetch without in-flight dedup.
    private readonly Explore.AsyncMemo<List<BookTocEntry>> _booksCache = new();
    private readonly Explore.AsyncMemo<List<EraDto>> _erasCache = new();
    private readonly Explore.AsyncMemo<List<LandmarkDto>> _landmarksCache = new();
    private readonly Explore.AsyncMemo<LandMaskOut> _landMaskCache = new();
    private readonly Explore.AsyncMemo<SourcesDocumentOut> _sourcesCache = new();

    public AtlasClient(HttpClient http)
    {
        _http = http;
    }

    // ApiBase config overrides for local dev (Blazor dev server + atlas-server on different
    // ports); falls back to the host's own base address for the single-binary release deployment
    // (API and client served from the same origin).
    public static Uri ResolveBaseAddress(IConfiguration configuration, IWebAssemblyHostEnvironment hostEnvironment)
    {
        var apiBase = configuration["ApiBase"];
        var raw = string.IsNullOrWhiteSpace(apiBase) ? hostEnvironment.BaseAddress : apiBase;
        // Uri's relative-combination rules replace the last path segment of a base that doesn't
        // end in '/'; force the trailing slash so a relative request path (e.g. "api/scene")
        // appends cleanly instead of clobbering part of a configured ApiBase.
        return new Uri(raw.EndsWith('/') ? raw : raw + "/");
    }

    public async Task<Scene> SceneTime(int from, int to)
    {
        var key = $"time:{from}:{to}";
        if (_sceneCache.TryGet(key, out var cached))
        {
            return cached;
        }

        var scene = await GetRequired<Scene>($"api/scene?from={from}&to={to}");
        _sceneCache.Put(key, scene);
        return scene;
    }

    public async Task<Scene> SceneScripture(string sref)
    {
        var key = $"scripture:{sref}";
        if (_sceneCache.TryGet(key, out var cached))
        {
            return cached;
        }

        var scene = await GetRequired<Scene>($"api/scene/scripture?ref={Uri.EscapeDataString(sref)}");
        _sceneCache.Put(key, scene);
        return scene;
    }

    public Task<List<BookTocEntry>> Books() => _booksCache.Get(() => GetRequired<List<BookTocEntry>>("api/books"));

    public Task<List<EraDto>> Eras() => _erasCache.Get(() => GetRequired<List<EraDto>>("api/eras"));

    public async Task<ChapterOut> Chapter(string book, int chapter)
    {
        var key = $"{book}.{chapter}";
        if (_chapterCache.TryGet(key, out var cached))
        {
            return cached;
        }

        var result = await GetRequired<ChapterOut>($"api/chapter/{key}");
        _chapterCache.Put(key, result);
        return result;
    }

    public Task<VerseDetail> Verse(string vref) =>
        GetRequired<VerseDetail>($"api/verse/{vref}");

    // No cache here (unlike Chapter): Kretzmann re-fetches fresh on every locus change by design
    // (LoadCommentaryAsync's own request-id guard discards stale in-flight responses), so a
    // curator-added commentary unit is visible on the very next chapter visit.
    public Task<KretzmannChapterOut> KretzmannChapter(string book, int chapter) =>
        GetRequired<KretzmannChapterOut>($"api/kretzmann/chapter/{book}.{chapter}");

    public Task<PlaceDetail> Place(string id) =>
        GetRequired<PlaceDetail>($"api/place/{id}");

    public async Task<PlaceDetail> PlaceHistory(string id, int? from, int? to)
    {
        var key = from is int f && to is int t ? $"{id}:{f}:{t}" : id;
        if (_placeHistoryCache.TryGet(key, out var cached))
        {
            return cached;
        }

        var url = from is int f2 && to is int t2 ? $"api/place/{id}?from={f2}&to={t2}" : $"api/place/{id}";
        var result = await GetRequired<PlaceDetail>(url);
        _placeHistoryCache.Put(key, result);
        return result;
    }

    public async Task<List<CrossRefOut>> Xrefs(string sref)
    {
        if (_xrefsCache.TryGet(sref, out var cached))
        {
            return cached;
        }

        var result = await GetRequired<List<CrossRefOut>>($"api/xrefs/{sref}");
        _xrefsCache.Put(sref, result);
        return result;
    }

    public async Task<List<CatechismRefDto>> Catechism(string sref)
    {
        if (_catechismSpanCache.TryGet(sref, out var cached))
        {
            return cached;
        }

        var result = await GetRequired<List<CatechismRefDto>>($"api/catechism/{sref}");
        _catechismSpanCache.Put(sref, result);
        return result;
    }

    public Task<CatechismItemDetail> CatechismItem(string id) =>
        GetRequired<CatechismItemDetail>($"api/catechism/item/{id}");

    public Task<List<NarrativeOut>> Narratives() =>
        GetRequired<List<NarrativeOut>>("api/narratives");

    public Task<NarrativeEventPositionsResult> NarrativeEventPositions(string eventId) =>
        GetRequired<NarrativeEventPositionsResult>($"api/narrative/event/{Uri.EscapeDataString(eventId)}");

    public Task<EventDetail> Event(string id) =>
        GetRequired<EventDetail>($"api/event/{Uri.EscapeDataString(id)}");

    public async Task<PolitiesOut> Polities(int from, int to)
    {
        var key = $"polities:{from}:{to}";
        if (_politiesCache.TryGet(key, out var cached))
        {
            return cached;
        }

        var result = await GetRequired<PolitiesOut>($"api/polities?from={from}&to={to}");
        _politiesCache.Put(key, result);
        return result;
    }

    public Task<List<LandmarkDto>> Landmarks() => _landmarksCache.Get(() => GetRequired<List<LandmarkDto>>("api/landmarks"));

    public Task<LandMaskOut> LandMask() => _landMaskCache.Get(() => GetRequired<LandMaskOut>("api/land-mask"));

    public Task<SourcesDocumentOut> Sources() => _sourcesCache.Get(() => GetRequired<SourcesDocumentOut>("api/sources"));

    public async Task<ContractDto> Contract(CancellationToken cancellationToken = default)
    {
        return await GetRequired<ContractDto>("api/contract", cancellationToken);
    }

    public Task<EdgePage> NodeEdges(string nodeId, EdgeKind kind, int? cursor = null, int limit = 200) =>
        GetRequired<EdgePage>($"api/node/{Uri.EscapeDataString(nodeId)}/edges?kind={Uri.EscapeDataString(kind.WireName())}&limit={limit}" + (cursor is int c ? $"&cursor={c}" : ""));

    public Task<NodeCard> NodeCard(string nodeId) =>
        GetRequired<NodeCard>($"api/node/{Uri.EscapeDataString(nodeId)}");

    public Task<TextWindow> ConcordUnit(string citation) =>
        GetRequired<TextWindow>($"api/text?ref={Uri.EscapeDataString(citation)}&n=1&corpus=concord");

    public Task<Contract.Contents> Contents(Corpus corpus)
    {
        if (!_contentsCache.TryGetValue(corpus, out var memo))
        {
            memo = new Explore.AsyncMemo<Contract.Contents>();
            _contentsCache[corpus] = memo;
        }

        return memo.Get(() => GetRequired<Contract.Contents>($"api/contents/{corpus.WireName()}"));
    }

    private readonly Dictionary<Corpus, Explore.AsyncMemo<Contract.Contents>> _contentsCache = new();

    private async Task<T> GetRequired<T>(string relativeUrl, CancellationToken cancellationToken = default)
    {
        var result = await _http.GetFromJsonAsync<T>(relativeUrl, Wire.Options, cancellationToken);
        return result ?? throw new InvalidOperationException($"empty response body from {relativeUrl}");
    }
}
