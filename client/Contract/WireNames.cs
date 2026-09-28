using System.Reflection;
using System.Text.Json.Serialization;

namespace BibleAtlas.Client.Contract;

public static class WireNames
{
    public static string WireName<T>(this T value) where T : struct, Enum => Cache<T>.Names[value];

    public static T Parse<T>(string name) where T : struct, Enum =>
        Cache<T>.Values.TryGetValue(name, out var value) ? value : throw new FormatException($"'{name}' is not a declared {typeof(T).Name}");

    private static class Cache<T> where T : struct, Enum
    {
        public static readonly IReadOnlyDictionary<T, string> Names =
            Enum.GetValues<T>().ToDictionary(v => v, v =>
                typeof(T).GetField(v.ToString())!.GetCustomAttribute<JsonStringEnumMemberNameAttribute>()!.Name);

        public static readonly IReadOnlyDictionary<string, T> Values = Names.ToDictionary(p => p.Value, p => p.Key);
    }
}
