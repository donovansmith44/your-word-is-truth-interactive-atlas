using System.Diagnostics;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

public interface IExplorer
{
    Task<Explorable> Resolve(PositionRef target);

    Task<IReadOnlyList<Explorable>> Resolve(IReadOnlyList<PositionRef> targets);

    Task<Explorable> Follow(Link link);

    Task<Presentation?> Present(Explorable element, Surface surface);
}

public sealed class GraphExplorer(IExplorableClient graph) : IExplorer
{
    private const string ProvenanceField = "Provenance";

    public async Task<Explorable> Resolve(PositionRef target) => (await Resolve([target]))[0];

    public async Task<IReadOnlyList<Explorable>> Resolve(IReadOnlyList<PositionRef> targets) =>
        (await graph.Elements(targets.Select(target => Positions.Of(target).Id).ToList())).Select(Of).ToList();

    public Task<Explorable> Follow(Link link) => Resolve(link.Target);

    public Task<Presentation?> Present(Explorable element, Surface surface) =>
        Task.FromResult<Presentation?>(
            Presentation.Of(element.Kind, surface) is null
                ? null
                : new Presentation.Card(element.Label, element.Provenance is { } provenance ? [new Presentation.Field(ProvenanceField, provenance)] : []));

    private Explorable Of(Element element) => element switch
    {
        NodeElement { Node: var node } => new Explorable(node, graph),
        EdgeElement { Edge: var edge } => new Explorable(edge, graph),
        MissingElement { Id: var id } => throw new ContractBreach($"the element read names nothing for {id}, an id the graph served"),
        _ => throw new UnreachableException($"{element.GetType().Name} is an element the contract does not declare"),
    };
}

public sealed class ContractBreach(string message) : Exception(message);
