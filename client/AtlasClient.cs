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
    // Capacity ~12: keyed by the requested "{from}:{to}" window -- a
    // low-traffic cache (one call per debounced time-mode window change,
    // not a hot per-hover path like scenes/chapters), so caching every
    // distinct requested window costs nothing meaningful in practice while
    // being a one-line, obviously-correct key.
    private readonly LruCache<string, PolitiesOut> _politiesCache = new(capacity: 12);
    // Batch E: PlaceCard's hover-lazy history fetch (place id + window),
    // same "instant on repeat hover" rationale and capacity ballpark as
    // _chapterCache above -- a scene's lit places cluster into a much
    // smaller set of distinct (place, window) pairs than 24 in practice.
    private readonly LruCache<string, PlaceDetail> _placeHistoryCache = new(capacity: 24);
    // Batch G1 requirement 2: PassageNode's own cross-references chip, keyed
    // by sref -- same "instant on repeat visits/re-opens" rationale as
    // _chapterCache/_placeHistoryCache above.
    private readonly LruCache<string, List<CrossRefOut>> _xrefsCache = new(capacity: 24);
    // Batch F: PassageNode's own catechism citation list, keyed by sref --
    // same rationale/capacity as _xrefsCache above (its own direct sibling:
    // CatechismSeamSection reads this exactly where CrossRefsSection reads
    // Xrefs, for the SAME PassageNode).
    private readonly LruCache<string, List<CatechismRefDto>> _catechismSpanCache = new(capacity: 24);
    private List<BookTocEntry>? _booksCache;
    private List<EraDto>? _erasCache;
    private List<LandmarkDto>? _landmarksCache;
    private LandMaskOut? _landMaskCache;

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

    /// <summary>
    /// Batch E: <c>GET /api/place/{id}?from=&amp;to=</c> -- the window-scoped
    /// history payload (resolved display name, the one blurb for this
    /// window, established/destroyed) PlaceCard fetches lazily on hover.
    /// <paramref name="from"/>/<paramref name="to"/> are optional (null in
    /// scripture mode, where there is no time window to resolve a blurb
    /// against -- see <c>atlas_core::history::resolve_display_name</c>'s own
    /// doc comment for why; established/destroyed still come back, being
    /// window-independent). LRU-cached like <see cref="Chapter"/>, keyed by
    /// (id, window) so a re-hover of the same place under the same window
    /// never re-fetches.
    /// </summary>
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

    /// <summary>
    /// Batch G1 requirement 2: <c>GET /api/xrefs/{sref}</c> -- aggregated
    /// cross-references for a verse or same-chapter passage span (union of
    /// member verses' own xrefs, votes summed, self-targets dropped, sorted
    /// desc, capped at 20). Reuses <see cref="CrossRefOut"/> -- the wire
    /// shape is identical to <see cref="VerseDetail.CrossRefs"/>'s own
    /// per-item shape. LRU-cached like <see cref="Chapter"/>/
    /// <see cref="PlaceHistory"/>: PassageNode's own popover reads this
    /// twice per open (once to decide the chip's conditional presence, once
    /// again if the chip is actually clicked) and this cache is what makes
    /// the second read free.
    /// </summary>
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

    /// <summary>
    /// Batch F ("the small catechism"): <c>GET /api/catechism/{sref}</c> --
    /// catechism items citing a verse or same-chapter passage span (union
    /// across member verses, no "votes" -- an item is cited or it isn't).
    /// Mirrors <see cref="Xrefs"/> exactly, same LRU-cached-by-sref
    /// treatment (PassageNode's own popover can read this twice per open --
    /// once to decide the section's conditional presence, once again if
    /// re-resolved -- the cache is what makes the second read free).
    /// </summary>
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

    /// <summary>
    /// Batch F: <c>GET /api/catechism/item/{id}</c> -- one catechism item's
    /// full content (text/explanation/where-written/proof verses). Uncached
    /// at this layer, same as <see cref="Verse"/>/<see cref="Place(string)"/>
    /// -- <see cref="Explore.CatechismNode"/> memoizes its own single fetch
    /// per node instance instead (mirrors <c>VerseNode.DetailAsync</c>'s own
    /// reasoning exactly).
    /// </summary>
    public Task<CatechismItemDetail> CatechismItem(string id) =>
        GetRequired<CatechismItemDetail>($"api/catechism/item/{id}");

    public Task<List<NarrativeOut>> Narratives() =>
        GetRequired<List<NarrativeOut>>("api/narratives");

    /// <summary>
    /// Batch N requirement 1's own event-id-keyed lookup half:
    /// <c>GET /api/narrative/event/{id}</c> -- every narrative position the
    /// given event id occupies (mirrors <see cref="CatechismItem"/>'s own
    /// id-keyed precedent). Uncached at this layer, same as
    /// <see cref="Verse"/>/<see cref="CatechismItem"/> --
    /// <see cref="Explore.NarrativeEventNode"/> memoizes its own single
    /// fetch per node instance instead (mirrors <c>VerseNode.DetailAsync</c>'s
    /// own reasoning exactly). Never called with a user-typed id -- always
    /// an id a PRIOR response (this same endpoint's own, or
    /// <see cref="VerseDetail.NarrativePositions"/>'s own <c>prior</c>/
    /// <c>following</c>) already handed back, per requirement 1's own
    /// "traversal steps resolve by event, not by re-searching verses."
    /// </summary>
    public Task<List<NarrativePositionDto>> NarrativeEventPositions(string eventId) =>
        GetRequired<List<NarrativePositionDto>>($"api/narrative/event/{Uri.EscapeDataString(eventId)}");

    /// <summary>
    /// Batch T requirement 4: <c>GET /api/event/{id}</c> -- an EVENT-kind
    /// PASSAGE's own rich content (title/date/places/witnesses/provenance).
    /// Uncached at this layer, same as <see cref="Verse"/>/
    /// <see cref="CatechismItem"/> -- <see cref="Explore.EventNode"/>
    /// memoizes its own single fetch per node instance instead (mirrors
    /// <c>VerseNode.DetailAsync</c>'s own reasoning exactly). Never called
    /// with a user-typed id -- always one a prior response (a reader
    /// heading, a verse's own EVENT membership row, a PRIOR/FOLLOWING
    /// traversal) already handed back.
    /// </summary>
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

    // Curated landmarks never change within a running session -- fetched
    // once and cached forever, same treatment as Books()/Eras() above.
    public async Task<List<LandmarkDto>> Landmarks()
    {
        _landmarksCache ??= await GetRequired<List<LandmarkDto>>("api/landmarks");
        return _landmarksCache;
    }

    // Batch R requirement 1: the curated land mask (clip geometry only, no
    // from/to) never changes within a running session either -- same
    // fetch-once-cache-forever treatment as Landmarks() above.
    public async Task<LandMaskOut> LandMask()
    {
        _landMaskCache ??= await GetRequired<LandMaskOut>("api/land-mask");
        return _landMaskCache;
    }

    private async Task<T> GetRequired<T>(string relativeUrl)
    {
        var result = await _http.GetFromJsonAsync<T>(relativeUrl, Wire.Options);
        return result ?? throw new InvalidOperationException($"empty response body from {relativeUrl}");
    }
}
