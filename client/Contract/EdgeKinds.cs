using System.Text.Json;

namespace BibleAtlas.Client.Contract;

public static class EdgeKinds
{
    public static string Label(this EdgeKind kind) => kind.WireName();

    public static EdgeKind Parse(string label) =>
        ByLabel.TryGetValue(label, out var kind) ? kind : throw new FormatException($"'{label}' is not a declared edge kind");

    public static EdgeKind Dual(this EdgeKind kind) => Duals[kind];

    public static bool IsSymmetric(this EdgeKind kind) => Duals[kind] == kind;

    private const string VocabularyResource = "graph-vocabulary.json";

    private static readonly IReadOnlyDictionary<string, EdgeKind> ByLabel =
        Enum.GetValues<EdgeKind>().ToDictionary(k => k.Label());

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
