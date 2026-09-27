using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.State;

public static class Selection
{
    public const string StorageKey = "selection-v1";

    public static readonly IReadOnlyList<ExplorationDescriptor> Empty = Array.Empty<ExplorationDescriptor>();
}

// Deliberately not idempotent: redispatching the same instance twice flips selection state back
// off. That's correct for Ctrl/Cmd-click toggle UX (clicking the same item again deselects it),
// not an oversight.
public sealed record ToggleSelection(ExplorationDescriptor Descriptor, string? Origin = null) : IIntent<IReadOnlyList<ExplorationDescriptor>>
{
    public string Name => "toggle-selection";

    public IReadOnlyList<ExplorationDescriptor> Apply(IReadOnlyList<ExplorationDescriptor> current)
    {
        var existing = current.FirstOrDefault(i => i.Kind == Descriptor.Kind && i.Key == Descriptor.Key);
        return existing is not null
            ? current.Where(i => i != existing).ToList()
            : current.Append(Descriptor).ToList();
    }
}

public sealed record RemoveSelection(ExplorationDescriptor Descriptor, string? Origin = null) : IIntent<IReadOnlyList<ExplorationDescriptor>>
{
    public string Name => "remove-selection";

    public IReadOnlyList<ExplorationDescriptor> Apply(IReadOnlyList<ExplorationDescriptor> current) =>
        current.Where(i => !(i.Kind == Descriptor.Kind && i.Key == Descriptor.Key)).ToList();
}

public sealed record ClearSelection(string? Origin = null) : IIntent<IReadOnlyList<ExplorationDescriptor>>
{
    public string Name => "clear-selection";

    public IReadOnlyList<ExplorationDescriptor> Apply(IReadOnlyList<ExplorationDescriptor> current) => Selection.Empty;
}
