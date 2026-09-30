namespace BibleAtlas.Client.Contract;

public static class NodeIds
{
    // The server writes every node id as `Kind:local`, save a text unit, whose kind it writes as `text-unit`.
    private const string TextUnitKind = "text-unit";

    private const char KindSeparator = ':';

    public static string Of(NodeKind kind, string localPart) =>
        $"{(kind == NodeKind.TextUnit ? TextUnitKind : kind.WireName())}{KindSeparator}{localPart}";

    public static string LocalPart(NodeRef node) => LocalPart(node.Id);

    public static string LocalPart(string id)
    {
        var separator = id.IndexOf(KindSeparator);
        return separator < 0
            ? throw new FormatException($"'{id}' is not a node id: it has no 'Kind{KindSeparator}' before its local part")
            : id[(separator + 1)..];
    }
}
