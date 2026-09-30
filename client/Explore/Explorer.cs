using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public interface IExplorer
{
    Task<Explorable> Resolve(NodeRef target);

    Task<Explorable> Follow(Link link);
}

public sealed class GraphExplorer(IExplorableClient graph) : IExplorer
{
    public async Task<Explorable> Resolve(NodeRef target) => new Explorable(await graph.Card(target.Id), graph);

    public Task<Explorable> Follow(Link link) => Resolve(link.Target);
}
