using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public interface IExplorer
{
    Task<Explorable> Resolve(NodeRef target);

    Task<Explorable> Follow(Link link);

    Task<Presentation?> Present(Explorable node, Surface surface);
}

public sealed class GraphExplorer(IExplorableClient graph) : IExplorer
{
    private const string ProvenanceField = "Provenance";

    public async Task<Explorable> Resolve(NodeRef target) => new Explorable(await graph.Card(target.Id), graph);

    public Task<Explorable> Follow(Link link) => Resolve(link.Target);

    public Task<Presentation?> Present(Explorable node, Surface surface) =>
        Task.FromResult<Presentation?>(
            Presentation.Of(node.Kind, surface) is null
                ? null
                : new Presentation.Card(node.Label, [new Presentation.Field(ProvenanceField, node.Card.Provenance)]));
}
