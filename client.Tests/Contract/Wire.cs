using System.Text.Json;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Tests;

internal static class Wire
{
    public static T Read<T>(string wire) where T : class => JsonSerializer.Deserialize<T>(JsonSerializer.Serialize(wire))!;

    public static NodeId Node(string id) => Read<NodeId>(id);

    public static EdgeId Edge(string id) => Read<EdgeId>(id);

    public static ElementId Element(string id) => Read<ElementId>(id);

    public static ArtifactRoot Root(string root) => Read<ArtifactRoot>(root);

    public static EdgePageCursor EdgeCursor(int at) => JsonSerializer.Deserialize<EdgePageCursor>(JsonSerializer.Serialize(at))!;

    public static EdgePageCursor? EdgeCursor(int? at) => at is { } given ? EdgeCursor(given) : null;

    public static ElementPageCursor ElementCursor(int at) => JsonSerializer.Deserialize<ElementPageCursor>(JsonSerializer.Serialize(at))!;
}
