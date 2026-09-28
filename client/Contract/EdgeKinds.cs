using System.Reflection;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace BibleAtlas.Client.Contract;

public static class EdgeKinds
{
    public static string Label(this EdgeKind kind) => Labels[kind];

    public static EdgeKind Parse(string label) =>
        ByLabel.TryGetValue(label, out var kind) ? kind : throw new FormatException($"'{label}' is not a declared edge kind");

    public static EdgeKind Dual(this EdgeKind kind) => Duals[kind];

    public static bool IsSymmetric(this EdgeKind kind) => Duals[kind] == kind;

    private const string VocabularyResource = "graph-vocabulary.json";

    // JsonStringEnumConverter<EdgeKind> (on every EdgeKind property in Wire.g.cs) reads
    // JsonStringEnumMemberName, not EnumMember -- that is the attribute System.Text.Json
    // actually consults when it serializes an EdgeKind, so it is the one source of truth
    // for the wire label, even though NSwag also emits the DataContract EnumMember pair.
    private static readonly IReadOnlyDictionary<EdgeKind, string> Labels =
        Enum.GetValues<EdgeKind>().ToDictionary(k => k, k =>
            typeof(EdgeKind).GetField(k.ToString())!.GetCustomAttribute<JsonStringEnumMemberNameAttribute>()!.Name);

    private static readonly IReadOnlyDictionary<string, EdgeKind> ByLabel =
        Labels.ToDictionary(p => p.Value, p => p.Key);

    private static readonly IReadOnlyDictionary<EdgeKind, EdgeKind> Duals = LoadDuals();

    private static IReadOnlyDictionary<EdgeKind, EdgeKind> LoadDuals()
    {
        using var stream = typeof(EdgeKinds).Assembly.GetManifestResourceStream(VocabularyResource)!;
        using var doc = JsonDocument.Parse(stream);
        var duals = new Dictionary<EdgeKind, EdgeKind>();
        foreach (var r in doc.RootElement.GetProperty("relations").EnumerateArray())
        {
            var forward = Parse(r.GetProperty("forward").GetString()!);
            var inverse = Parse(r.GetProperty("inverse").GetString()!);
            duals[forward] = inverse;
            duals[inverse] = forward;
        }
        foreach (var s in doc.RootElement.GetProperty("symmetric").EnumerateArray())
        {
            var kind = Parse(s.GetProperty("label").GetString()!);
            duals[kind] = kind;
        }
        return duals;
    }
}
