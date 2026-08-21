using System.Text.Json;
using Microsoft.AspNetCore.Components;
using Microsoft.JSInterop;

namespace BibleAtlas.Client;

/// <summary>
/// Events map.js raises back into .NET (client/wwwroot/js/map.js's
/// <c>wireEvents</c> for place markers, <c>ArrowLayer</c> for narrative
/// arrows). Implemented by whichever page owns a <see cref="MapInterop"/>
/// (World.razor) -- a plain interface, not JS-invokable itself; see
/// <see cref="MapEventsSink"/> for the actual JS-callable bridge.
/// </summary>
public interface IMapEvents
{
    void OnPlaceHover(string id, double x, double y);
    void OnPlaceLeave();
    void OnPlaceClick(string id, double x, double y);
    void OnArrowHover(string key, double x, double y);
    void OnArrowLeave();
    void OnArrowClick(string key, double x, double y);

    /// <summary>
    /// Batch G1 requirement 3: fired on a genuine map-BACKGROUND click --
    /// tiles, borders, empty water -- never a marker or arrow click (both of
    /// those stop propagation in map.js before this can also fire for the
    /// same physical click). World.razor closes a pinned card on this, per
    /// the brief's "closes via Escape, an X, or clicking elsewhere on the
    /// map." Never fired for a mini instance (map.js's init() only wires
    /// this listener when !mini).
    /// </summary>
    void OnMapClick();

    /// <summary>
    /// Batch G1 requirement 3: fired on Escape, document-wide, while this
    /// (non-mini) map instance is mounted. A document-level listener (map.js's
    /// own watchEscape, wired inside init()) rather than a Blazor
    /// @onkeydown -- the same "must work regardless of which element
    /// currently has DOM focus" reason reader.js's watchShiftRelease exists,
    /// since a marker click never moves keyboard focus anywhere in
    /// particular. Never fired for a mini instance.
    /// </summary>
    void OnEscapePressed();

    /// <summary>
    /// Batch H fix round 2 (view-state round-trip, review Critical-1): fired
    /// on every Leaflet <c>moveend</c>/<c>zoomend</c> (map.js's own init()),
    /// continuously keeping <c>ViewStateService.Map</c>'s camera fields
    /// current -- replaces an earlier dispose-time <c>GetCamera()</c> read
    /// that raced a concurrently-mounting new <c>World</c> instance on
    /// transitions that dispose one page and mount another in the same
    /// navigation (see <c>World.razor</c>'s <c>OnCameraChanged</c> for the
    /// full account). Never fired for a mini instance.
    /// </summary>
    void OnCameraChanged(double lat, double lon, double zoom);

    /// <summary>
    /// Batch M requirement 4: a delta-bearing polity-era-boundary ring was
    /// clicked (or Enter-activated while keyboard-focused) -- map.js's own
    /// BorderLayer._makeDeltaHit. Every field this needs to build a
    /// <c>PolityDeltaNode</c> and open it in the popover is ALREADY
    /// resolved client-side (map.js's own in-memory roster) and handed
    /// through directly -- no second fetch, no server-side lookup by id.
    /// <paramref name="kind"/> is <c>"transition"</c> or <c>"fall"</c>;
    /// <paramref name="eventText"/>/<paramref name="refNote"/> are
    /// <c>null</c> and <paramref name="verses"/> empty for the MINIMAL-
    /// popover case (an honestly uneventful boundary -- conditional
    /// presence, not a placeholder).
    /// </summary>
    void OnPolityDeltaClick(string polityName, string kind, int titleFromYear, int titleToYear, string? eventText, string[] verses, string? refNote);
}

/// <summary>
/// Owns one Leaflet map instance living in <c>client/wwwroot/js/map.js</c>.
/// Every public member is a thin await over that module's exports, keyed by
/// the integer id <c>init</c> hands back. Not thread-safe / not reentrant
/// beyond normal Blazor WASM single-threaded use.
/// </summary>
public sealed class MapInterop : IAsyncDisposable
{
    private readonly IJSObjectReference _module;
    private readonly DotNetObjectReference<MapEventsSink> _sinkRef;
    private readonly int _id;

    private MapInterop(IJSObjectReference module, DotNetObjectReference<MapEventsSink> sinkRef, int id)
    {
        _module = module;
        _sinkRef = sinkRef;
        _id = id;
    }

    public static async Task<MapInterop> Create(IJSRuntime js, ElementReference el, bool mini, IMapEvents sink)
    {
        var module = await js.InvokeAsync<IJSObjectReference>("import", "./js/map.js");
        var sinkRef = DotNetObjectReference.Create(new MapEventsSink(sink));
        var id = await module.InvokeAsync<int>("init", el, sinkRef, new { mini });
        return new MapInterop(module, sinkRef, id);
    }

    /// <summary>
    /// Pushes a freshly-fetched scene into the map. <see cref="IJSObjectReference.InvokeAsync{TValue}"/>
    /// serializes its arguments with System.Text.Json's default options
    /// (roughly camelCase), never <see cref="Wire.Options"/> -- passing
    /// <paramref name="s"/> straight through would rename e.g.
    /// <c>verse_groups</c> to <c>verseGroups</c> on arrival in JS, silently
    /// diverging from the snake_case wire shape atlas-server actually
    /// sends (and that map.js, and later the Task 12 arrow layer, are
    /// written against). Serializing here with Wire.Options first and
    /// sending the result as a plain string sidesteps that: map.js
    /// JSON.parses it back into an object whose keys are byte-for-byte the
    /// same as the original API response.
    ///
    /// Batch E: each place's <c>name</c> key is overwritten with its own
    /// <see cref="ScenePlace.DisplayName"/> before serializing -- map.js's
    /// marker-label rendering already reads <c>p.name</c> (see
    /// wwwroot/js/map.js), so feeding the period-resolved name through that
    /// SAME key is the Luz/Bethel swap's entire client-side wiring; map.js
    /// itself needs no change. The wire-shape <c>display_name</c> key is
    /// still present alongside it (harmless -- map.js reads only the keys
    /// it knows about), so this stays a faithful re-serialization of `s`,
    /// not a lossy transform.
    /// </summary>
    public async Task SetScene(Scene s)
    {
        var forMap = s with
        {
            Places = s.Places.Select(p => p with { Name = p.DisplayName }).ToList(),
            QuietPlaces = s.QuietPlaces ?? [],
            Arrows = s.Arrows ?? [],
            Narratives = s.Narratives ?? []
        };
        var json = JsonSerializer.Serialize(forMap, Wire.Options);
        await _module.InvokeVoidAsync("setScene", _id, json);
    }

    public async Task FitScene() => await _module.InvokeVoidAsync("fitScene", _id);

    /// <summary>
    /// Pushes a freshly-fetched polity list (Batch B2, "borders v2, the
    /// cartographer's edition": hand-authored per-polity timerange borders,
    /// swapped wholesale on every window change). <paramref name="polities"/>
    /// is the flat <c>polities</c> array <c>GET /api/polities</c> returns
    /// (already deterministically ordered by id then <c>from</c>); each
    /// element's own <see cref="PolityEraOut.Rings"/> is already a raw
    /// <see cref="JsonElement"/> -- a parsed-JSON tree with no C# property
    /// names for a naming policy to rename, so there is no snake_case/
    /// camelCase risk there the way there is for the REST of a POCO like
    /// <see cref="PolityEraOut"/> itself (still routed through
    /// <see cref="Wire.Options"/> and sent as a string, same "serialize
    /// with Wire.Options, then InvokeVoidAsync a string" shape every
    /// <c>SetX</c> method on this class follows). Batch M requirement 4:
    /// <paramref name="from"/>/<paramref name="to"/> are the CURRENTLY
    /// APPLIED window -- map.js's own <c>BorderLayer</c> needs it to decide
    /// which era boundary (if any) each ring is an explorable delta target
    /// for ("every boundary whose transition falls INSIDE the window").
    /// </summary>
    public async Task SetPolities(List<PolityEraOut> polities, int from, int to)
    {
        var json = JsonSerializer.Serialize(polities, Wire.Options);
        await _module.InvokeVoidAsync("setPolities", _id, json, from, to);
    }

    /// <summary>
    /// Batch M requirement 3a: pushes the FULL, unfiltered polity roster
    /// (every era of every polity -- fetched once, World.razor's own
    /// PushPolitiesRosterIfReady, mirroring PushLandMaskIfReady/
    /// PushLandmarksIfReady's "fetch once, push once" shape) -- the one
    /// thing the morph engine's own <c>lookup</c> (border-morph.js) needs
    /// to answer an ARBITRARY drag-time window with zero network latency.
    /// Same serialize-with-Wire.Options-then-string shape as
    /// <see cref="SetPolities"/> above.
    /// </summary>
    public async Task SetPolitiesRoster(List<PolityEraOut> roster)
    {
        var json = JsonSerializer.Serialize(roster, Wire.Options);
        await _module.InvokeVoidAsync("setPolitiesRoster", _id, json);
    }

    /// <summary>
    /// Batch M requirement 3a: begins a morph gesture (TimeSlider.razor's
    /// own OnWindowDrag, fired on the FIRST drag-move update since the last
    /// settle -- World.razor's own HandleWindowDrag decides when this is a
    /// "first" call). Fix round 1 (C1): no anchor-year parameter -- every
    /// <see cref="MorphFrame"/> call now carries the full live window on its
    /// own, so there is nothing left for this call itself to remember.
    /// </summary>
    public async Task BeginMorph() => await _module.InvokeVoidAsync("beginMorph", _id);

    /// <summary>
    /// Batch M requirement 3a: the scrub itself -- called on every
    /// drag-move update; map.js's own BorderLayer.requestMorphFrame is what
    /// coalesces however many of these land within one animation frame into
    /// a single actual evaluate+paint (the CPU evaluator, behind its own
    /// documented GPU/WebGL swap seam -- see map.js's own comment).
    /// Fix round 1 (Critical C1): <paramref name="from"/>/<paramref
    /// name="to"/> are the FULL, CURRENT live window (both handles' current
    /// values -- World.razor's own HandleWindowDrag passes its own
    /// `window.From`/`window.To` straight through, unchanged, the same pair
    /// <see cref="SettleMorph"/> already correctly receives on release) --
    /// NOT just the dragged handle's own pre-drag value paired with the
    /// probe, which silently dropped any polity/era beyond the OTHER,
    /// still-committed edge for the whole gesture (six of twelve real
    /// polities in the review's own live-verified replay -- see the batch
    /// report's own "Fix round 1" section). <paramref name="atYear"/> is
    /// still specifically the DRAGGED handle's own live value, the one
    /// `animate` sweeps against.
    /// </summary>
    public async Task MorphFrame(int from, int to, double atYear) => await _module.InvokeVoidAsync("morphFrame", _id, from, to, atYear);

    /// <summary>
    /// Batch M requirement 3a: "on release, settle into the static
    /// layered-era presentation" -- INSTANTLY and LOCALLY from the
    /// already-loaded roster (zero network wait; World.razor's own
    /// separate, debounced network fetch -- LoadPolitiesFor, unchanged --
    /// still lands a little later and repaints from the server's own
    /// answer, byte-identical by construction, so this is a pure UX win
    /// with no correctness risk from skipping ahead of it).
    /// </summary>
    public async Task SettleMorph(int from, int to) => await _module.InvokeVoidAsync("settleMorph", _id, from, to);

    /// <summary>
    /// Toggles the polity-border layer's visibility without touching its
    /// data -- used only to hide it on entering scripture mode (scripture
    /// mode has no time window, so no polities are shown there) and to
    /// unconditionally restore it at the start of every time-mode polities
    /// fetch (World.razor's LoadPolitiesFor), so a transient fetch failure
    /// right after returning from scripture mode still shows the last
    /// known-good polities instead of staying hidden.
    /// </summary>
    public async Task SetPolitiesVisible(bool visible) => await _module.InvokeVoidAsync("setPolitiesVisible", _id, visible);

    /// <summary>
    /// Pushes the curated landmark list (fetched once, see
    /// <see cref="AtlasClient.Landmarks"/>) -- always-visible, non-interactive
    /// labels rendered once and never diffed/updated again for the life of
    /// the map instance.
    /// </summary>
    public async Task SetLandmarks(List<LandmarkDto> landmarks)
    {
        var json = JsonSerializer.Serialize(landmarks, Wire.Options);
        await _module.InvokeVoidAsync("setLandmarks", _id, json);
    }

    /// <summary>
    /// Batch R requirement 1 ("borders become part of the plate"): pushes the
    /// curated land mask once (fetched once, see <see cref="AtlasClient.LandMask"/>)
    /// -- the clip geometry <c>BorderLayer</c> builds its SVG
    /// <c>&lt;clipPath&gt;</c> from, so polity washes never spill into open
    /// sea. <paramref name="rings"/> is the flat <c>[[[lat,lon],...],...]</c>
    /// array <c>LandMaskOut.Rings</c> carries, forwarded verbatim (a raw
    /// <see cref="JsonElement"/>, same "the client never inspects ring
    /// coordinates" reasoning <see cref="SetPolities"/> already documents for
    /// its own <c>Rings</c>). Never diffed/updated again for the life of the
    /// map instance -- the mask is static.
    /// </summary>
    public async Task SetLandMask(JsonElement rings)
    {
        var json = JsonSerializer.Serialize(rings, Wire.Options);
        await _module.InvokeVoidAsync("setLandMask", _id, json);
    }

    /// <summary>
    /// WORLD-4 legend isolate: <paramref name="narrativeId"/> null clears
    /// every arrow back to unfaded; any other value fades every arrow whose
    /// narrative doesn't match it. A bare string argument has no properties
    /// to rename, so (unlike <see cref="SetScene"/>) this can go straight
    /// through <see cref="IJSObjectReference.InvokeVoidAsync"/> without a
    /// Wire.Options detour.
    /// </summary>
    public async Task SetIsolate(string? narrativeId) => await _module.InvokeVoidAsync("setIsolate", _id, narrativeId);

    /// <summary>
    /// Batch G1 requirement 3 (narrative traversal): recenters the map on
    /// (lat, lon) -- the adjacent place's own marker -- WITHOUT changing
    /// zoom, and returns its new on-screen container point so the caller
    /// can re-anchor the pinned place-card there (the same coordinate space
    /// map.js already reports via containerPoint on hover/click). Instant,
    /// not animated (map.js's own panToPlace uses `{animate:false}`) --
    /// deliberately, so the returned point is always the FINAL settled
    /// position, never one read mid-animation; see map.js's own comment for
    /// why an animated pan would make that point unreliable without a much
    /// more complex moveend-await round trip for no real user-facing
    /// benefit here (the brief allows "fly/pan" either way).
    /// </summary>
    public async Task<(double X, double Y)> PanToPlace(double lat, double lon)
    {
        var point = await _module.InvokeAsync<ContainerPointDto>("panToPlace", _id, lat, lon);
        return (point.X, point.Y);
    }

    /// <summary>
    /// Batch H: sets the map's view directly to a previously-captured
    /// camera (<see cref="ViewStateService"/>'s own <c>MapViewState</c>,
    /// kept current by <see cref="IMapEvents.OnCameraChanged"/> rather than
    /// a dispose-time read -- see that member's own comment) -- instant,
    /// not animated, same reasoning <see cref="FitScene"/>/<see cref="PanToPlace"/>
    /// already document for their own `{animate:false}`.
    /// </summary>
    public async Task SetCamera(double lat, double lon, double zoom) =>
        await _module.InvokeVoidAsync("setCamera", _id, lat, lon, zoom);

    /// <summary>
    /// Batch R requirement 5 (place-in-verse hover -> marker blink): a
    /// STATIC helper, not an instance method -- unlike every other member on
    /// this class, the caller (Explore/VerseTextSection's own mini-reader)
    /// owns no <see cref="MapInterop"/>/map instance of its own; it just
    /// needs to reach WHICHEVER live, non-mini map instance(s) currently
    /// exist (the full <c>/world</c> page's own map, and/or a split view's
    /// embedded atlas pane's own map -- map.js's own module-level
    /// <c>instances</c> registry already tracks every one of them, keyed by
    /// id, independent of which C# component created it). map.js's own
    /// <c>blinkPlace</c> loops that registry directly, so this is a single,
    /// ID-less JS interop call -- "works against the live map in split view
    /// and full /world+popover contexts" falls out of that for free, with no
    /// Blazor-side component wiring at all. A no-op, harmlessly, when no
    /// live non-mini map currently has this place on it (e.g. Reader.razor
    /// standalone, or a place off the current scene's own window).
    /// </summary>
    public static async Task BlinkPlace(IJSRuntime js, string placeId, bool active)
    {
        var module = await js.InvokeAsync<IJSObjectReference>("import", "./js/map.js");
        await module.InvokeVoidAsync("blinkPlace", placeId, active);
    }

    /// <summary>
    /// Batch N requirement 3 (map focus sync -- "the same graph, seen
    /// twice"): a STATIC helper, same shape as <see cref="BlinkPlace"/>
    /// directly above for the SAME reason -- the caller
    /// (<see cref="Components.ExplorerPopover"/>) owns no live map instance
    /// of its own; it just needs to reach WHICHEVER live, non-mini map
    /// instance(s) currently exist (map.js's own module-level
    /// <c>instances</c> registry), so this works identically whether the
    /// live map is the full <c>/world</c> page's own or a split view's
    /// embedded atlas pane's own, with no page-specific wiring -- "works in
    /// full reader, split view, and map-side popovers (one component,
    /// every context)," per the brief verbatim. <paramref name="activeNarrativeIds"/>
    /// empty clears every arrow back to normal (popover closed, or Current
    /// touches no narrative); non-empty amplifies every arrow of those
    /// narratives and recedes (never vanishes) every other narrative's own,
    /// with <paramref name="currentEventIds"/> picking out the specific
    /// leg(s) touching the CURRENTLY open event for the strongest emphasis
    /// (map.js's own <c>setNarrativeFocus</c>/<c>ArrowLayer.setFocus</c>).
    /// A no-op, harmlessly, when no live non-mini map currently exists
    /// (e.g. a plain, unsplit Reader.razor) -- same precedent as
    /// <see cref="BlinkPlace"/>.
    /// </summary>
    public static async Task SetNarrativeFocus(IJSRuntime js, IReadOnlyList<string> activeNarrativeIds, IReadOnlyList<string> currentEventIds)
    {
        var module = await js.InvokeAsync<IJSObjectReference>("import", "./js/map.js");
        await module.InvokeVoidAsync("setNarrativeFocus", activeNarrativeIds, currentEventIds);
    }

    /// <summary>
    /// HOTFIX batch (batch-hotfix-brief.md requirement 1): measures a
    /// PlaceCard's own just-rendered box plus its DOM parent's clientWidth
    /// (the map container this card is currently inside -- see map.js's own
    /// <c>measureCardPlacement</c> for why that parent is always exactly the
    /// right box), so <see cref="CardPlacement.Compute"/> can decide whether
    /// to flip below the marker and/or clamp horizontally. A static helper,
    /// same shape as <see cref="BlinkPlace"/> above -- the card has no live
    /// <see cref="MapInterop"/> of its own to hang this off, it just needs
    /// the one already-loaded module. Deliberately deferred until BOTH
    /// PlaceCard.razor's own chapter-text and history fetches have settled
    /// -- see its OnAfterRenderAsync's own comment for why measuring any
    /// earlier would size against not-yet-loaded content (the blurb/dates
    /// sections specifically, conditionally absent from the DOM until then)
    /// and produce a wrong decision.
    /// </summary>
    public static async Task<CardMeasurementDto> MeasureCardPlacement(IJSRuntime js, ElementReference cardEl)
    {
        var module = await js.InvokeAsync<IJSObjectReference>("import", "./js/map.js");
        return await module.InvokeAsync<CardMeasurementDto>("measureCardPlacement", cardEl);
    }

    public async ValueTask DisposeAsync()
    {
        try
        {
            await _module.InvokeVoidAsync("destroy", _id);
        }
        catch (JSDisconnectedException)
        {
            // The circuit/module is already gone during teardown (e.g. page
            // unload racing this dispose) -- nothing left to clean up JS-side.
        }

        _sinkRef.Dispose();
        await _module.DisposeAsync();
    }
}

/// <summary>
/// The actual JS-callable target: <see cref="DotNetObjectReference.Create{TValue}"/>
/// needs a concrete class whose methods carry <see cref="JSInvokableAttribute"/>
/// directly, so this wraps an <see cref="IMapEvents"/> sink rather than
/// requiring the page component itself (World.razor) to be attributed --
/// map.js only ever sees this bridge, never the page.
/// </summary>
public sealed class MapEventsSink
{
    private readonly IMapEvents _sink;

    public MapEventsSink(IMapEvents sink) => _sink = sink;

    [JSInvokable] public void OnPlaceHover(string id, double x, double y) => _sink.OnPlaceHover(id, x, y);
    [JSInvokable] public void OnPlaceLeave() => _sink.OnPlaceLeave();
    [JSInvokable] public void OnPlaceClick(string id, double x, double y) => _sink.OnPlaceClick(id, x, y);
    [JSInvokable] public void OnArrowHover(string key, double x, double y) => _sink.OnArrowHover(key, x, y);
    [JSInvokable] public void OnArrowLeave() => _sink.OnArrowLeave();
    [JSInvokable] public void OnArrowClick(string key, double x, double y) => _sink.OnArrowClick(key, x, y);
    [JSInvokable] public void OnMapClick() => _sink.OnMapClick();
    [JSInvokable] public void OnEscapePressed() => _sink.OnEscapePressed();
    [JSInvokable] public void OnCameraChanged(double lat, double lon, double zoom) => _sink.OnCameraChanged(lat, lon, zoom);
    [JSInvokable] public void OnPolityDeltaClick(string polityName, string kind, int titleFromYear, int titleToYear, string? eventText, string[] verses, string? refNote) =>
        _sink.OnPolityDeltaClick(polityName, kind, titleFromYear, titleToYear, eventText, verses, refNote);
}

/// <summary>
/// Batch G1: <see cref="MapInterop.PanToPlace"/>'s own return shape --
/// deserialized via Blazor's DEFAULT (camelCase) JS-interop JSON options,
/// not <see cref="Wire.Options"/> (this never crosses the real HTTP API, so
/// there is no snake_case wire shape to match here -- see map.js's own
/// panToPlace, which returns a plain <c>{x, y}</c> object).
/// </summary>
public sealed record ContainerPointDto(double X, double Y);

/// <summary>
/// HOTFIX batch: <see cref="MapInterop.MeasureCardPlacement"/>'s own return
/// shape -- deserialized via Blazor's DEFAULT (camelCase) JS-interop JSON
/// options, not <see cref="Wire.Options"/> (this never crosses the real
/// HTTP API either, same reasoning as <see cref="ContainerPointDto"/> above
/// -- see map.js's own <c>measureCardPlacement</c>, which returns a plain
/// <c>{width, height, containerWidth, containerHeight}</c> object).
/// <c>ContainerHeight</c> added fix round 1 (review finding, Important) --
/// see <see cref="CardPlacement.Compute"/>'s own header for why it's now
/// required, not optional.
/// </summary>
public sealed record CardMeasurementDto(double Width, double Height, double ContainerWidth, double ContainerHeight);
