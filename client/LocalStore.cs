using System.Text.Json;
using Microsoft.JSInterop;

namespace BibleAtlas.Client;

/// <summary>
/// Batch G2 decisions 4/6: the shared localStorage-JSON-document primitive
/// both new persisted stores (<see cref="SavedExplorationsService"/>,
/// <see cref="SelectionTrayService"/>) build on -- ViewStateService's own
/// "restore on init, synced continuously at the point state actually
/// changes, never captured lazily" idiom (see that file's own header
/// comment), now backed by REAL persistence: ViewStateService's own Map/
/// Reader halves are deliberately in-memory only (that file's own doc
/// comment, "NOT persisted to localStorage -- explicitly out of scope"),
/// this batch's own disclosed extension of the SAME restore/sync shape to a
/// genuinely durable store. camelCase property names throughout -- a plain
/// client-local JSON document, never a wire DTO (<see cref="Wire"/>'s own
/// snake_case convention is for talking to atlas-server specifically, per
/// that file's own header comment, and does not apply here).
///
/// Blazor WebAssembly's own DI-registered <see cref="IJSRuntime"/> instance
/// always implements <see cref="IJSInProcessRuntime"/> in a WASM host (there
/// is no network hop to a separate browser process the way Blazor Server
/// has) -- the standard, documented cast Program.cs's own service
/// registration performs once, letting every read/write below be a plain
/// synchronous call, no Task/await ceremony for a same-process localStorage
/// round trip.
/// </summary>
internal static class LocalStore
{
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
    };

    /// Best-effort read: returns <paramref name="fallback"/> (never throws)
    /// whenever localStorage is unavailable (private browsing, blocked
    /// storage), empty, or holds unparseable JSON (a future format change,
    /// corruption) -- "storage failures degrade gracefully... never a
    /// crash," per the brief, verbatim.
    public static T Read<T>(IJSInProcessRuntime js, string key, T fallback)
    {
        try
        {
            var json = js.Invoke<string?>("localStorage.getItem", key);
            if (string.IsNullOrEmpty(json))
            {
                return fallback;
            }

            return JsonSerializer.Deserialize<T>(json, Options) ?? fallback;
        }
        catch (Exception)
        {
            return fallback;
        }
    }

    /// Best-effort write: silently no-ops on failure (quota exceeded,
    /// storage blocked, private-browsing SecurityError) -- the in-memory
    /// state the caller already mutated stays correct for the rest of THIS
    /// session regardless; only durability across a reload is lost, never a
    /// crash.
    public static void Write<T>(IJSInProcessRuntime js, string key, T value)
    {
        try
        {
            js.InvokeVoid("localStorage.setItem", key, JsonSerializer.Serialize(value, Options));
        }
        catch (Exception)
        {
            // Storage full/unavailable -- non-fatal, see this class's own header comment.
        }
    }

    /// A one-time, real round-trip probe (write, read back, remove) --
    /// distinct from a plain `try { localStorage } catch` existence check,
    /// since some browsers expose the localStorage OBJECT fine but throw on
    /// actual use (Safari's own private-browsing mode is the classic case).
    /// <see cref="SavedExplorationsService"/>/<see cref="SelectionTrayService"/>
    /// each call this ONCE, at construction, and cache the result as their
    /// own `Available` -- "feature hides if storage unavailable," per the
    /// brief, verbatim: consuming markup (the popover's own save button, the
    /// hamburger button) reads that cached flag to decide whether to render
    /// itself at all, rather than re-probing on every render.
    public static bool Probe(IJSInProcessRuntime js)
    {
        const string probeKey = "explorations-v1-probe";
        try
        {
            js.InvokeVoid("localStorage.setItem", probeKey, "1");
            var ok = js.Invoke<string?>("localStorage.getItem", probeKey) == "1";
            js.InvokeVoid("localStorage.removeItem", probeKey);
            return ok;
        }
        catch (Exception)
        {
            return false;
        }
    }
}
