using System.Text.Json;

namespace BibleAtlas.Client.Tests;

internal static class WholeValue
{
    private static readonly JsonSerializerOptions Visible = new() { IncludeFields = true };

    public static string Of<T>(T value)
    {
        var whole = JsonSerializer.Serialize(value, Visible);
        return Blind(JsonDocument.Parse(whole).RootElement)
            ? throw new ArgumentException($"{typeof(T).Name} holds a value with nothing visible to compare: {whole}", nameof(value))
            : whole;
    }

    private static bool Blind(JsonElement element) => element.ValueKind switch
    {
        JsonValueKind.Object => !element.EnumerateObject().Any() || element.EnumerateObject().Any(property => Blind(property.Value)),
        JsonValueKind.Array => element.EnumerateArray().Any(Blind),
        _ => false,
    };
}
