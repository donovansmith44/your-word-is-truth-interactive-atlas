using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Contracts;

// Invariant: ExplorationDescriptor.Capture(Node) must equal Descriptor.
public sealed record Focus(ExplorationDescriptor Descriptor, IExplorable Node);

public interface IEscapeHatch
{
    string Kind { get; }
    Task Invoke();
}
