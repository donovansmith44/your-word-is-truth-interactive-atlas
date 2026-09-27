using System.Text.Json;
using Microsoft.JSInterop;

namespace BibleAtlas.Client;

internal static class LocalStore
{
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
    };

    // Never throws: returns fallback whenever localStorage is unavailable (private browsing,
    // blocked storage), empty, or holds unparseable JSON.
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

    // Never throws: silently no-ops on failure (quota exceeded, storage blocked, private-browsing
    // SecurityError) -- only durability across a reload is lost, never a crash.
    public static void Write<T>(IJSInProcessRuntime js, string key, T value)
    {
        try
        {
            js.InvokeVoid("localStorage.setItem", key, JsonSerializer.Serialize(value, Options));
        }
        catch (Exception)
        {
        }
    }

    // A real round-trip probe (write, read back, remove), not just a try/catch existence check:
    // some browsers expose the localStorage object fine but throw on actual use (Safari's
    // private-browsing mode is the classic case).
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
