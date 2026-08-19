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
    /// </summary>
    public async Task SetScene(Scene s)
    {
        var json = JsonSerializer.Serialize(s, Wire.Options);
        await _module.InvokeVoidAsync("setScene", _id, json);
    }

    public async Task FitScene() => await _module.InvokeVoidAsync("fitScene", _id);

    /// <summary>
    /// Pushes a freshly-fetched border snapshot (design-direction.md's
    /// Atlas plate detail: "historical borders render as a vector
    /// overlay... swapped to the snapshot nearest the selected window").
    /// Unlike <see cref="SetScene"/>, <paramref name="geojson"/> is already
    /// a <see cref="JsonElement"/> -- a raw parsed-JSON tree with no C#
    /// property names for a naming policy to rename in the first place, so
    /// there is no snake_case/camelCase risk here the way there is for a
    /// POCO like <see cref="Scene"/>. Still routed through
    /// <see cref="Wire.Options"/> and sent as a string (rather than passed
    /// as a bare interop argument) purely so every <c>SetX</c> method on
    /// this class follows the exact same "serialize with Wire.Options,
    /// then InvokeVoidAsync a string" shape -- one pattern to read, not two.
    /// </summary>
    public async Task SetBorders(JsonElement geojson)
    {
        var json = JsonSerializer.Serialize(geojson, Wire.Options);
        await _module.InvokeVoidAsync("setBorders", _id, json);
    }

    /// <summary>
    /// Toggles the border layer's visibility without touching its data --
    /// used only to hide it on entering scripture mode (design-direction.md:
    /// "Scripture mode: hide the border layer AND the tag... restore on
    /// return to time mode") and to unconditionally restore it at the start
    /// of every time-mode border fetch (World.razor's LoadBordersFor), so a
    /// transient fetch failure right after returning from scripture mode
    /// still shows the last known-good snapshot instead of staying hidden.
    /// </summary>
    public async Task SetBordersVisible(bool visible) => await _module.InvokeVoidAsync("setBordersVisible", _id, visible);

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
    /// WORLD-4 legend isolate: <paramref name="narrativeId"/> null clears
    /// every arrow back to unfaded; any other value fades every arrow whose
    /// narrative doesn't match it. A bare string argument has no properties
    /// to rename, so (unlike <see cref="SetScene"/>) this can go straight
    /// through <see cref="IJSObjectReference.InvokeVoidAsync"/> without a
    /// Wire.Options detour.
    /// </summary>
    public async Task SetIsolate(string? narrativeId) => await _module.InvokeVoidAsync("setIsolate", _id, narrativeId);

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
}
