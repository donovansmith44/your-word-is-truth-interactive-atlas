using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public interface IExplorableClient
{
    const int DefaultPageSize = 20;

    Task<NodeCard> Card(string id);

    Task<EdgeCard> EdgeCard(string id);

    Task<EdgePage> Edges(string id, EdgeKind kind, int? cursor = null, int limit = DefaultPageSize);

    Task<EdgePage> EdgeEdges(string edgeId, EdgeKind kind, int? cursor = null, int limit = DefaultPageSize);

    Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible);
}
