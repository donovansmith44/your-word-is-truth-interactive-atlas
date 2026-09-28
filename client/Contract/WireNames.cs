using System.Reflection;
using System.Text.Json.Serialization;

namespace BibleAtlas.Client.Contract;

public static class WireNames
{
    public static string WireName<T>(this T value) where T : struct, Enum => Cache<T>.Names[value];

    private static class Cache<T> where T : struct, Enum
    {
        public static readonly IReadOnlyDictionary<T, string> Names =
            Enum.GetValues<T>().ToDictionary(v => v, v =>
                typeof(T).GetField(v.ToString())!.GetCustomAttribute<JsonStringEnumMemberNameAttribute>()!.Name);
    }
}
