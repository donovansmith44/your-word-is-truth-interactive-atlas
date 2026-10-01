using System.Diagnostics;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Exploring;

public interface IExplorer
{
    internal Task<IReadOnlyList<Explorable>> Resolve(IReadOnlyList<PositionRef> targets);
}

public sealed class GraphExplorer(IExplorableClient graph) : IExplorer
{
    async Task<IReadOnlyList<Explorable>> IExplorer.Resolve(IReadOnlyList<PositionRef> targets) =>
        (await graph.Elements(targets.Select(target => Positions.Of(target).Id).ToList())).Select(Of).ToList();

    private Explorable Of(Element element) => element switch
    {
        NodeElement { Node: var node } => new Explorable(node, graph),
        EdgeElement { Edge: var edge } => new Explorable(edge, graph),
        MissingElement { Id: var id } => throw new ContractBreach($"the element read names nothing for {id}, an id the graph served"),
        _ => throw new UnreachableException($"{element.GetType().Name} is an element the contract does not declare"),
    };
}

public sealed class ContractBreach(string message) : Exception(message);
