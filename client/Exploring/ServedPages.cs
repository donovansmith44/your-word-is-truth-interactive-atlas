using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public sealed class ServedPages
{
    public const int Resident = 32;

    private readonly IExplorableClient _graph;
    private readonly LruCache<(string Root, string Id, EdgeKind Kind, int? Cursor, int Limit), AsyncMemo<EdgePage>> _pages = new(Resident);

    internal ServedPages(IExplorableClient graph) => _graph = graph;

    internal Task<EdgePage> Read(string root, string id, EdgeKind kind, int? cursor, int limit)
    {
        var key = (root, id, kind, cursor, limit);
        if (!_pages.TryGet(key, out var memo))
        {
            memo = new AsyncMemo<EdgePage>();
            _pages.Put(key, memo);
        }

        return memo.Get(async () => Served(root, await _graph.Edges(id, kind, cursor, limit)));
    }

    private static EdgePage Served(string root, EdgePage page) =>
        page.Version == root ? page : throw new ArtifactMoved(root, page.Version);
}

public sealed class ArtifactMoved(string resolved, string serving)
    : Exception($"an element resolved from artifact {resolved} was paged from artifact {serving}");
