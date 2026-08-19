using System.Net.Http;
using System.Net.Http.Json;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.Extensions.Configuration;

namespace BibleAtlas.Client;

/// <summary>
/// Typed HTTP client for atlas-server's <c>/api/*</c> endpoints. Registered
/// as a DI singleton (see Program.cs). Scene responses (the hot path -- one
/// call per slider/dropdown change) are cached in a capacity-48 LRU keyed by
/// request shape; the canon table of contents and eras list are small,
/// stable, and needed by nearly every page, so they are fetched once and
/// cached for the lifetime of the app. Chapters (fetched by PlaceCard's
/// hover-verse-text block -- design-direction.md's "Hover place card --
/// REVISED": "Verse text loads via the chapter endpoint, LRU-cached, so
/// hovers stay instant after first touch") get their own smaller capacity-24
/// LRU: a hover's whole point is instant repeat-visits, and a scene's places
/// tend to cluster into a much smaller set of distinct chapters than the
/// 48-scene cache is sized for.
/// </summary>
public sealed class AtlasClient
{
    private readonly HttpClient _http;
    private readonly LruCache<string, Scene> _sceneCache = new(capacity: 48);
    private readonly LruCache<string, ChapterOut> _chapterCache = new(capacity: 24);
    private List<BookTocEntry>? _booksCache;
    private List<EraDto>? _erasCache;

    public AtlasClient(HttpClient http)
    {
        _http = http;
    }

    /// <summary>
    /// Resolves the API origin from configuration's "ApiBase" key (set in
    /// wwwroot/appsettings.Development.json for local dev, where the Blazor
    /// dev server and atlas-server run on different ports), falling back to
    /// the host's own base address -- the single-binary release deployment,
    /// where atlas-server serves both the API and the published client from
    /// the same origin, needs no override.
    /// </summary>
    public static Uri ResolveBaseAddress(IConfiguration configuration, IWebAssemblyHostEnvironment hostEnvironment)
    {
        var apiBase = configuration["ApiBase"];
        var raw = string.IsNullOrWhiteSpace(apiBase) ? hostEnvironment.BaseAddress : apiBase;
        // Uri's relative-combination rules replace the last path segment of a
        // base that doesn't end in '/'; force the trailing slash so every
        // relative request path below (e.g. "api/scene") appends cleanly
        // instead of clobbering part of a configured ApiBase.
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

    public async Task<List<BookTocEntry>> Books()
    {
        _booksCache ??= await GetRequired<List<BookTocEntry>>("api/books");
        return _booksCache;
    }

    public async Task<List<EraDto>> Eras()
    {
        _erasCache ??= await GetRequired<List<EraDto>>("api/eras");
        return _erasCache;
    }

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

    public Task<PlaceDetail> Place(string id) =>
        GetRequired<PlaceDetail>($"api/place/{id}");

    public Task<List<NarrativeOut>> Narratives() =>
        GetRequired<List<NarrativeOut>>("api/narratives");

    private async Task<T> GetRequired<T>(string relativeUrl)
    {
        var result = await _http.GetFromJsonAsync<T>(relativeUrl, Wire.Options);
        return result ?? throw new InvalidOperationException($"empty response body from {relativeUrl}");
    }
}
