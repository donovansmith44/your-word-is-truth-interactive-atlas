using System.Text.Json;
using Microsoft.AspNetCore.Components;
using Microsoft.JSInterop;

namespace BibleAtlas.Client;

public interface IMapEvents
{
    void OnPlaceHover(string id, double x, double y);

    void OnPlaceHoverAmbiguous(string[] ids, double x, double y);

    void OnPlaceLeave();
    void OnPlaceClick(string id, double x, double y);

    // Fired instead of OnPlaceClick for Ctrl/Cmd-click -- never both for the same physical click.
    void OnPlaceToggleSelect(string id);

    void OnArrowHover(string key, double x, double y);
    void OnArrowLeave();
    void OnArrowClick(string key, double x, double y);

    void OnMapClick();

    void OnEscapePressed();

    void OnCameraChanged(double lat, double lon, double zoom);

    // kind is "transition" or "fall"; eventText/refNote are null and verses empty for an honestly
    // uneventful boundary (conditional presence, not a placeholder).
    void OnPolityDeltaClick(string polityId, string polityName, string kind, int titleFromYear, int titleToYear, string? eventText, string[] verses, string? refNote);
}

// Not thread-safe / not reentrant beyond normal Blazor WASM single-threaded use.
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

    // InvokeAsync serializes arguments with System.Text.Json's default (camelCase) options, never
    // Wire.Options -- passing s straight through would rename e.g. verse_groups to verseGroups,
    // silently diverging from the snake_case shape map.js is written against. Serializing with
    // Wire.Options first and sending the result as a string sidesteps that.
    // Each place's `name` is overwritten with its own DisplayName before serializing: map.js's
    // marker-label rendering reads p.name, so this is the only client-side change a period-
    // resolved display name needs.
    public async Task SetScene(Scene s)
    {
        var forMap = s with { Places = s.Places.Select(p => p with { Name = p.DisplayName }).ToList() };
        var json = JsonSerializer.Serialize(forMap, Wire.Options);
        await _module.InvokeVoidAsync("setScene", _id, json);
    }

    public async Task FitScene() => await _module.InvokeVoidAsync("fitScene", _id);

    public async Task SetPolities(List<PolityEraOut> polities, int from, int to)
    {
        var json = JsonSerializer.Serialize(polities, Wire.Options);
        await _module.InvokeVoidAsync("setPolities", _id, json, from, to);
    }

    public async Task SetPolitiesRoster(List<PolityEraOut> roster)
    {
        var json = JsonSerializer.Serialize(roster, Wire.Options);
        await _module.InvokeVoidAsync("setPolitiesRoster", _id, json);
    }

    public async Task BeginMorph() => await _module.InvokeVoidAsync("beginMorph", _id);

    // from/to must be the full, current live window (both handles' values), not just the dragged
    // handle's pre-drag value paired with the probe -- passing only the latter silently drops any
    // polity/era beyond the other, still-committed edge for the whole gesture. atYear is
    // specifically the dragged handle's own live value.
    public async Task MorphFrame(int from, int to, double atYear) => await _module.InvokeVoidAsync("morphFrame", _id, from, to, atYear);

    public async Task SettleMorph(int from, int to) => await _module.InvokeVoidAsync("settleMorph", _id, from, to);

    // Called at the start of every time-mode fetch (not after it resolves) so a transient fetch
    // failure still shows the last known-good polities instead of leaving them hidden.
    public async Task SetPolitiesVisible(bool visible) => await _module.InvokeVoidAsync("setPolitiesVisible", _id, visible);

    public async Task SetLandmarks(List<LandmarkDto> landmarks)
    {
        var json = JsonSerializer.Serialize(landmarks, Wire.Options);
        await _module.InvokeVoidAsync("setLandmarks", _id, json);
    }

    public async Task SetLandMask(JsonElement rings)
    {
        var json = JsonSerializer.Serialize(rings, Wire.Options);
        await _module.InvokeVoidAsync("setLandMask", _id, json);
    }

    // A bare string has no properties to rename, so (unlike SetScene) this skips the
    // Wire.Options serialize-to-string detour.
    public async Task SetIsolate(string? narrativeId) => await _module.InvokeVoidAsync("setIsolate", _id, narrativeId);

    // Instant, not animated: an animated pan would make the returned point unreliable (read
    // mid-animation) without an added moveend-await round trip.
    public async Task<(double X, double Y)> PanToPlace(double lat, double lon)
    {
        var point = await _module.InvokeAsync<ContainerPointDto>("panToPlace", _id, lat, lon);
        return (point.X, point.Y);
    }

    public async Task SetCamera(double lat, double lon, double zoom) =>
        await _module.InvokeVoidAsync("setCamera", _id, lat, lon, zoom);

    public static async Task BlinkPlace(IJSRuntime js, string placeId, bool active)
    {
        var module = await js.InvokeAsync<IJSObjectReference>("import", "./js/map.js");
        await module.InvokeVoidAsync("blinkPlace", placeId, active);
    }

    public static async Task SetNarrativeFocus(IJSRuntime js, IReadOnlyList<string> activeNarrativeIds, IReadOnlyList<string> currentEventIds)
    {
        var module = await js.InvokeAsync<IJSObjectReference>("import", "./js/map.js");
        await module.InvokeVoidAsync("setNarrativeFocus", activeNarrativeIds, currentEventIds);
    }

    // Deliberately deferred until both chapter-text and history fetches settle -- measuring
    // earlier would size against not-yet-loaded content and produce a wrong flip/clamp decision.
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
            // Circuit/module already gone during teardown (e.g. page unload racing this dispose).
        }

        _sinkRef.Dispose();
        await _module.DisposeAsync();
    }
}

// DotNetObjectReference.Create needs a concrete class whose methods carry [JSInvokable] directly,
// so this wraps an IMapEvents sink rather than requiring the page component itself to be
// attributed -- map.js only ever sees this bridge, never the page.
public sealed class MapEventsSink
{
    private readonly IMapEvents _sink;

    public MapEventsSink(IMapEvents sink) => _sink = sink;

    [JSInvokable] public void OnPlaceHover(string id, double x, double y) => _sink.OnPlaceHover(id, x, y);
    [JSInvokable] public void OnPlaceHoverAmbiguous(string[] ids, double x, double y) => _sink.OnPlaceHoverAmbiguous(ids, x, y);
    [JSInvokable] public void OnPlaceLeave() => _sink.OnPlaceLeave();
    [JSInvokable] public void OnPlaceClick(string id, double x, double y) => _sink.OnPlaceClick(id, x, y);
    [JSInvokable] public void OnPlaceToggleSelect(string id) => _sink.OnPlaceToggleSelect(id);
    [JSInvokable] public void OnArrowHover(string key, double x, double y) => _sink.OnArrowHover(key, x, y);
    [JSInvokable] public void OnArrowLeave() => _sink.OnArrowLeave();
    [JSInvokable] public void OnArrowClick(string key, double x, double y) => _sink.OnArrowClick(key, x, y);
    [JSInvokable] public void OnMapClick() => _sink.OnMapClick();
    [JSInvokable] public void OnEscapePressed() => _sink.OnEscapePressed();
    [JSInvokable] public void OnCameraChanged(double lat, double lon, double zoom) => _sink.OnCameraChanged(lat, lon, zoom);
    [JSInvokable] public void OnPolityDeltaClick(string polityId, string polityName, string kind, int titleFromYear, int titleToYear, string? eventText, string[] verses, string? refNote) =>
        _sink.OnPolityDeltaClick(polityId, polityName, kind, titleFromYear, titleToYear, eventText, verses, refNote);
}

// These JS-interop return shapes deserialize via Blazor's default (camelCase) JSON options, not
// Wire.Options -- they never cross the real HTTP API, so there's no snake_case wire shape to match.
public sealed record ContainerPointDto(double X, double Y);

public sealed record CardMeasurementDto(double Width, double Height, double ContainerWidth, double ContainerHeight);
