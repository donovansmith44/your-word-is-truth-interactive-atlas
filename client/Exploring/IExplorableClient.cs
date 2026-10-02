using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public interface IExplorableClient
{
    Task<NodeRecord> Card(string id);

    Task<ElementPage> Elements(IReadOnlyList<string> ids);

    Task<EdgePage> Edges(string positionId, EdgeKind kind, int? cursor = null, int limit = Exploring.Affordances.PageSize);

    Task<TextWindow> Reading(string fromRef, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible);
}
