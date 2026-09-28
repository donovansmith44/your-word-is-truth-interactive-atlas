using System.Net.Http;
using BibleAtlas.Client.Contract;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.Extensions.Configuration;

namespace BibleAtlas.Client;

public sealed class AtlasClient
{
    private readonly HttpClient _http;
    private readonly LruCache<string, Scene> _sceneCache = new(capacity: 48);
    private readonly LruCache<string, Chapter> _chapterCache = new(capacity: 24);
    private readonly LruCache<string, Polities> _politiesCache = new(capacity: 12);
    private readonly LruCache<string, PlaceDetail> _placeHistoryCache = new(capacity: 24);
    private readonly LruCache<string, IReadOnlyList<CrossRef>> _xrefsCache = new(capacity: 24);
    private readonly LruCache<string, IReadOnlyList<CatechismRef>> _catechismSpanCache = new(capacity: 24);
    // AsyncMemo, not a plain cache: several callers can independently invoke Books()/Eras()/etc.
    // concurrently before any has resolved (e.g. around app startup), so a plain cache would
    // double-fetch without in-flight dedup.
    private readonly Explore.AsyncMemo<List<CanonBook>> _booksCache = new();
    private readonly Explore.AsyncMemo<List<Era>> _erasCache = new();
    private readonly Explore.AsyncMemo<IReadOnlyList<Landmark>> _landmarksCache = new();
    private readonly Explore.AsyncMemo<LandMask> _landMaskCache = new();
    private readonly Explore.AsyncMemo<SourcesDocument> _sourcesCache = new();

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

        var scene = await _http.GetRequired<Scene>($"api/scene?from={from}&to={to}");
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

        var scene = await _http.GetRequired<Scene>($"api/scene/scripture?ref={Uri.EscapeDataString(sref)}");
        _sceneCache.Put(key, scene);
        return scene;
    }

    public Task<List<CanonBook>> Books() => _booksCache.Get(() => _http.GetRequired<List<CanonBook>>("api/books"));

    public Task<List<Era>> Eras() => _erasCache.Get(() => _http.GetRequired<List<Era>>("api/eras"));

    public async Task<Chapter> Chapter(string book, int chapter)
    {
        var key = $"{book}.{chapter}";
        if (_chapterCache.TryGet(key, out var cached))
        {
            return cached;
        }

        var result = await _http.GetRequired<Chapter>($"api/chapter/{key}");
        _chapterCache.Put(key, result);
        return result;
    }

    public Task<VerseDetail> Verse(string vref) =>
        _http.GetRequired<VerseDetail>($"api/verse/{vref}");

    // No cache here (unlike Chapter): Kretzmann re-fetches fresh on every locus change by design
    // (LoadCommentaryAsync's own request-id guard discards stale in-flight responses), so a
    // curator-added commentary unit is visible on the very next chapter visit.
    public Task<KretzmannChapter> KretzmannChapter(string book, int chapter) =>
        _http.GetRequired<KretzmannChapter>($"api/kretzmann/chapter/{book}.{chapter}");

    public Task<PlaceDetail> Place(string id) =>
        _http.GetRequired<PlaceDetail>($"api/place/{id}");

    public async Task<PlaceDetail> PlaceHistory(string id, int? from, int? to)
    {
        var key = from is int f && to is int t ? $"{id}:{f}:{t}" : id;
        if (_placeHistoryCache.TryGet(key, out var cached))
        {
            return cached;
        }

        var url = from is int f2 && to is int t2 ? $"api/place/{id}?from={f2}&to={t2}" : $"api/place/{id}";
        var result = await _http.GetRequired<PlaceDetail>(url);
        _placeHistoryCache.Put(key, result);
        return result;
    }

    public async Task<IReadOnlyList<CrossRef>> Xrefs(string sref)
    {
        if (_xrefsCache.TryGet(sref, out var cached))
        {
            return cached;
        }

        var result = await _http.GetRequired<IReadOnlyList<CrossRef>>($"api/xrefs/{sref}");
        _xrefsCache.Put(sref, result);
        return result;
    }

    public async Task<IReadOnlyList<CatechismRef>> Catechism(string sref)
    {
        if (_catechismSpanCache.TryGet(sref, out var cached))
        {
            return cached;
        }

        var result = await _http.GetRequired<IReadOnlyList<CatechismRef>>($"api/catechism/{sref}");
        _catechismSpanCache.Put(sref, result);
        return result;
    }

    public Task<CatechismItem> CatechismItem(string id) =>
        _http.GetRequired<CatechismItem>($"api/catechism/item/{id}");

    public Task<List<Narrative>> Narratives() =>
        _http.GetRequired<List<Narrative>>("api/narratives");

    public Task<NarrativeEventPositions> NarrativeEventPositions(string eventId) =>
        _http.GetRequired<NarrativeEventPositions>($"api/narrative/event/{Uri.EscapeDataString(eventId)}");

    public Task<EventDetail> Event(string id) =>
        _http.GetRequired<EventDetail>($"api/event/{Uri.EscapeDataString(id)}");

    public async Task<Polities> Polities(int from, int to)
    {
        var key = $"polities:{from}:{to}";
        if (_politiesCache.TryGet(key, out var cached))
        {
            return cached;
        }

        var result = await _http.GetRequired<Polities>($"api/polities?from={from}&to={to}");
        _politiesCache.Put(key, result);
        return result;
    }

    public Task<IReadOnlyList<Landmark>> Landmarks() => _landmarksCache.Get(() => _http.GetRequired<IReadOnlyList<Landmark>>("api/landmarks"));

    public Task<LandMask> LandMask() => _landMaskCache.Get(() => _http.GetRequired<LandMask>("api/land-mask"));

    public Task<SourcesDocument> Sources() => _sourcesCache.Get(() => _http.GetRequired<SourcesDocument>("api/sources"));

    public Task<NodeCard> NodeCard(string nodeId) =>
        _http.GetRequired<NodeCard>($"api/node/{Uri.EscapeDataString(nodeId)}");

    public Task<Contract.Contents> Contents(Corpus corpus)
    {
        if (!_contentsCache.TryGetValue(corpus, out var memo))
        {
            memo = new Explore.AsyncMemo<Contract.Contents>();
            _contentsCache[corpus] = memo;
        }

        return memo.Get(() => _http.GetRequired<Contract.Contents>($"api/contents/{corpus.WireName()}"));
    }

    private readonly Dictionary<Corpus, Explore.AsyncMemo<Contract.Contents>> _contentsCache = new();
}
