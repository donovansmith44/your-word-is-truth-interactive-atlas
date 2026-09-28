using System.Reflection;
using System.Text.Json.Serialization;

namespace BibleAtlas.Client.Contract;

public static class WireNames
{
    public static string WireName<T>(this T value) where T : struct, Enum => Cache<T>.Names.Value[value];

    public static T Parse<T>(string name) where T : struct, Enum =>
        Cache<T>.Values.Value.TryGetValue(name, out var value) ? value : throw new FormatException($"'{name}' is not a declared {typeof(T).Name}");

    // Lazy rather than a static initialiser, so a caller sees NoWireNameException itself and not a TypeInitializationException around it.
    private static class Cache<T> where T : struct, Enum
    {
        public static readonly Lazy<IReadOnlyDictionary<T, string>> Names =
            new(() => Enum.GetValues<T>().ToDictionary(v => v, DeclaredWireName));

        public static readonly Lazy<IReadOnlyDictionary<string, T>> Values =
            new(() => Names.Value.ToDictionary(p => p.Value, p => p.Key));
    }

    private static string DeclaredWireName<T>(T value) where T : struct, Enum =>
        typeof(T).GetField(value.ToString())!.GetCustomAttribute<JsonStringEnumMemberNameAttribute>()?.Name
        ?? throw new NoWireNameException(typeof(T), value);
}

public sealed class NoWireNameException(Type enumType, Enum member)
    : InvalidOperationException($"{enumType.Name}.{member} has no wire name; WireNames serves only generated contract enums");
