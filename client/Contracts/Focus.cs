using BibleAtlas.Client.Explore;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Contracts;

// Invariant: ExplorationDescriptor.Capture(Node) must equal Descriptor.
public sealed record Focus(ExplorationDescriptor Descriptor, IExplorable Node);

public interface IFocusComponent : IViewComponent
{
    Focus Focus { get; }
    IReadOnlyList<IFrontierAbstraction> Frontier { get; }
    IReadOnlyList<IEscapeHatch> EscapeHatches { get; }
}

public interface IFrontierAbstraction
{
    string EdgeFamily { get; }

    string Label { get; }

    int? Cardinality { get; }

    ITraversal Traversal { get; }
}

public interface ITraversal
{
    Task<IReadOnlyList<Focus>> Expand(int page);
    ExplorationDescriptor Describe(Focus target);
}

public interface IEscapeHatch
{
    string Kind { get; }
    Task Invoke();
}

public interface IPresentation<TContext>
{
    RenderFragment Render(TContext ctx);
}
