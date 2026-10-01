using System.Text.Json;

namespace BibleAtlas.Client.Contract;

public static class EdgeKinds
{
    public static EdgeKind Dual(this EdgeKind kind) => Duals[kind];

    public static bool IsSymmetric(this EdgeKind kind) => Duals[kind] == kind;

    public static string DisplayLabel(this EdgeKind kind) => Displays[kind];

    private const string VocabularyResource = "graph-vocabulary.json";

    private static readonly (IReadOnlyDictionary<EdgeKind, EdgeKind> Duals, IReadOnlyDictionary<EdgeKind, string> Displays) Vocabulary = LoadVocabulary();

    private static IReadOnlyDictionary<EdgeKind, EdgeKind> Duals => Vocabulary.Duals;

    private static IReadOnlyDictionary<EdgeKind, string> Displays => Vocabulary.Displays;

    private static (IReadOnlyDictionary<EdgeKind, EdgeKind>, IReadOnlyDictionary<EdgeKind, string>) LoadVocabulary()
    {
        using var stream = typeof(EdgeKinds).Assembly.GetManifestResourceStream(VocabularyResource)!;
        using var doc = JsonDocument.Parse(stream);
        var duals = new Dictionary<EdgeKind, EdgeKind>();
        var displays = new Dictionary<EdgeKind, string>();
        foreach (var r in doc.RootElement.GetProperty("relations").EnumerateArray())
        {
            var forward = WireNames.Parse<EdgeKind>(r.GetProperty("forward").GetString()!);
            var inverse = WireNames.Parse<EdgeKind>(r.GetProperty("inverse").GetString()!);
            duals[forward] = inverse;
            duals[inverse] = forward;
            displays[forward] = r.GetProperty("forward_display").GetString()!;
            displays[inverse] = r.GetProperty("inverse_display").GetString()!;
        }
        foreach (var s in doc.RootElement.GetProperty("symmetric").EnumerateArray())
        {
            var kind = WireNames.Parse<EdgeKind>(s.GetProperty("label").GetString()!);
            duals[kind] = kind;
            displays[kind] = s.GetProperty("display").GetString()!;
        }
        return (duals, displays);
    }
}
