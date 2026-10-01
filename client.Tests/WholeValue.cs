using System.Text.Json;

namespace BibleAtlas.Client.Tests;

internal static class WholeValue
{
    public static string Of<T>(T value) => JsonSerializer.Serialize(value);
}
