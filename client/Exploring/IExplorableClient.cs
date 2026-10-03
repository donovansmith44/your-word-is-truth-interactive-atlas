using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client;

public interface IExplorableClient
{
    Task<NodeRecord> Card(NodeId id);

    Task<ElementPage> Elements(IReadOnlyList<ElementId> ids);

    Task<EdgePage> Edges(ElementId position, EdgeKind kind, EdgePageCursor? cursor = null, int limit = Exploring.Affordances.PageSize);

    Task<TextWindow> Reading(TextWindowReference from, int n, WindowDir dir = WindowDir.Onward, Corpus corpus = Corpus.Bible);
}
