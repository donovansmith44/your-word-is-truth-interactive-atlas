namespace BibleAtlas.Client;

// Deliberately no implicit conversion to/from string: id and kind were both plain strings before,
// which let an argument-order swap at a call site pass the compiler silently. The only way to
// construct one is the explicit constructor, confined by convention to EdgeSectionRegistry.cs's
// literal-authoring sites; every other call site consumes an already-typed constant.
public readonly record struct EdgeKindId(string Value);

public interface IExplorableClient
{
    Task<NodeCardDto> Card(string id);

    Task<EdgePageDto> Edges(string id, EdgeKindId kind, int? cursor = null, int limit = 20);

    Task<TextWindowDto> Reading(string fromRef, int n, string dir = "onward", string corpus = "bible");
}
