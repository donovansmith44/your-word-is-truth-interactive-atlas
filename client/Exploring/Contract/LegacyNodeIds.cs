using System.Text.Json;

namespace BibleAtlas.Client.Contract;

public static class LegacyNodeIds
{
    private const string TextUnitKind = "text-unit";

    private const char KindSeparator = ':';

    public static NodeId Of(NodeKind kind, string localPart)
    {
        var prefix = $"{(kind == NodeKind.TextUnit ? TextUnitKind : kind.WireName())}{KindSeparator}";
        return localPart.StartsWith(prefix, StringComparison.Ordinal)
            ? throw new FormatException($"'{localPart}' is already a node id; Of wants its local part")
            : Read<NodeId>(prefix + localPart);
    }

    public static T Read<T>(string wire) where T : class =>
        JsonSerializer.Deserialize<T>(JsonSerializer.Serialize(wire)) ?? throw new FormatException($"'{wire}' does not read as a {typeof(T).Name}");

    public static string LocalPart(NodeRef node) => LocalPart(node.Id.ToString());

    public static string LocalPart(NodeId id) => LocalPart(id.ToString());

    public static string LocalPart(string text)
    {
        var separator = text.IndexOf(KindSeparator);
        return separator < 0
            ? throw new FormatException($"'{text}' is not a node id: it has no 'Kind{KindSeparator}' before its local part")
            : text[(separator + 1)..];
    }
}
