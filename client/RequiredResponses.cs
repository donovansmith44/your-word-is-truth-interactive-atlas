using System.Net.Http.Json;

namespace BibleAtlas.Client;

public static class RequiredResponses
{
    public static async Task<T> GetRequired<T>(this HttpClient http, string relativeUrl) =>
        await http.GetFromJsonAsync<T>(relativeUrl) ?? throw new InvalidOperationException($"empty response body from {relativeUrl}");
}
