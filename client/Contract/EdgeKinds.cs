using System.Text.Json;

namespace BibleAtlas.Client.Contract;

public static class EdgeKinds
{
    public static EdgeKind Dual(this EdgeKind kind) => Duals[kind];

    public static bool IsSymmetric(this EdgeKind kind) => Duals[kind] == kind;

    private const string VocabularyResource = "graph-vocabulary.json";

    private static readonly IReadOnlyDictionary<EdgeKind, EdgeKind> Duals = LoadDuals();

    private static IReadOnlyDictionary<EdgeKind, EdgeKind> LoadDuals()
    {
        using var stream = typeof(EdgeKinds).Assembly.GetManifestResourceStream(VocabularyResource)!;
        using var doc = JsonDocument.Parse(stream);
        var duals = new Dictionary<EdgeKind, EdgeKind>();
        foreach (var r in doc.RootElement.GetProperty("relations").EnumerateArray())
        {
            var forward = WireNames.Parse<EdgeKind>(r.GetProperty("forward").GetString()!);
            var inverse = WireNames.Parse<EdgeKind>(r.GetProperty("inverse").GetString()!);
            duals[forward] = inverse;
            duals[inverse] = forward;
        }
        foreach (var s in doc.RootElement.GetProperty("symmetric").EnumerateArray())
        {
            var kind = WireNames.Parse<EdgeKind>(s.GetProperty("label").GetString()!);
            duals[kind] = kind;
        }
        return duals;
    }
}
