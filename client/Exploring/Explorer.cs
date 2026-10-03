using System.Diagnostics;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public interface IExplorer
{
    internal Task<IReadOnlyList<Explorable>> Resolve(IReadOnlyList<PositionRef> targets);
}

public sealed class GraphExplorer(IExplorableClient graph) : IExplorer
{
    private readonly ServedPages _pages = new(graph);

    async Task<IReadOnlyList<Explorable>> IExplorer.Resolve(IReadOnlyList<PositionRef> targets)
    {
        var served = await graph.Elements(targets.Select(target => Positions.Of(target).Id).ToList());
        _pages.Saw(served.Version);
        return served.Elements.Select(element => Of(element, served.Version)).ToList();
    }

    private Explorable Of(Element element, ArtifactRoot root) => element switch
    {
        NodeElement { Node: var node } => new Explorable(node, root, _pages),
        EdgeElement { Edge: var edge } => new Explorable(edge, root, _pages),
        MissingElement { Id: var id } => throw new ContractBreach($"the element read names nothing for {id}, an id the graph served"),
        _ => throw new UnreachableException($"{element.GetType().Name} is an element the contract does not declare"),
    };
}

public sealed class ContractBreach(string message) : Exception(message);
