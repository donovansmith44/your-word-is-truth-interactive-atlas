using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public interface IExplorer
{
    Task<Explorable> Resolve(PositionRef target);

    Task<Explorable> Follow(Link link);

    Task<Presentation?> Present(Explorable node, Surface surface);
}

public sealed class GraphExplorer(IExplorableClient graph) : IExplorer
{
    private const string ProvenanceField = "Provenance";

    public Task<Explorable> Resolve(PositionRef target)
    {
        var (kind, id, _) = Positions.Of(target);
        return kind.Match(node: _ => Node(id), edge: _ => Edge(id));
    }

    public Task<Explorable> Follow(Link link) => Resolve(link.Target);

    public Task<Presentation?> Present(Explorable node, Surface surface) =>
        Task.FromResult<Presentation?>(
            Presentation.Of(node.Kind, surface) is null
                ? null
                : new Presentation.Card(node.Label, [new Presentation.Field(ProvenanceField, node.Provenance)]));

    private async Task<Explorable> Node(string id) => new Explorable(await graph.Card(id), graph);

    private async Task<Explorable> Edge(string id) => new Explorable(await graph.EdgeCard(id), graph);
}
