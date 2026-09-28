using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public interface IExplorableClient
{
    Task<NodeCard> Card(string id);

    Task<EdgePage> Edges(string id, EdgeKind kind, int? cursor = null, int limit = 20);

    Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible);
}
