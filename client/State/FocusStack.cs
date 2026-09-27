using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.State;

/// <summary>
/// Equality compares <see cref="Stack"/> by each entry's Descriptor only; Node is payload and
/// excluded. A reference-based comparison would never converge when a reseed reconstructs
/// fresh node instances for an already-current trail, so a no-op reseed keeps the atom's
/// existing Node instances rather than the ones just passed in -- <see cref="Current"/> can
/// hand back an object that isn't the literal instance a caller supplied for that descriptor.
/// </summary>
public sealed record FocusStack(IReadOnlyList<Focus> Stack, IReadOnlyList<ExplorationDescriptor> Trail)
{
    public static readonly FocusStack Empty = new(Array.Empty<Focus>(), Array.Empty<ExplorationDescriptor>());

    public Focus? Current => Stack.Count > 0 ? Stack[^1] : null;

    public bool Equals(FocusStack? other) =>
        other is not null
        && Stack.Select(f => f.Descriptor).SequenceEqual(other.Stack.Select(f => f.Descriptor))
        && Trail.SequenceEqual(other.Trail);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        foreach (var f in Stack)
        {
            hash.Add(f.Descriptor);
        }

        hash.Add(Trail.Count);
        return hash.ToHashCode();
    }
}

public sealed record Visit(IExplorable Node, string? Origin = null) : IIntent<FocusStack>
{
    public string Name => "focus-visit";

    public FocusStack Apply(FocusStack current)
    {
        var descriptor = ExplorationDescriptor.Capture(Node);
        if (current.Current is { } top && top.Descriptor == descriptor)
        {
            return current;
        }

        var stack = current.Stack.Append(new Focus(descriptor, Node)).ToList();
        var trail = current.Trail.Count > 0 && current.Trail[^1] == descriptor
            ? current.Trail
            : current.Trail.Append(descriptor).ToList();
        return new FocusStack(stack, trail);
    }
}

// A Back landing counts as a visit: append the landed-on node's descriptor to the trail
// (consecutively deduped), not just pop the stack.
public sealed record Back(string? Origin = null) : IIntent<FocusStack>
{
    public string Name => "focus-back";

    public FocusStack Apply(FocusStack current)
    {
        if (current.Stack.Count <= 1)
        {
            return current;
        }

        var stack = current.Stack.Take(current.Stack.Count - 1).ToList();
        var landed = stack[^1].Descriptor;
        var trail = current.Trail.Count > 0 && current.Trail[^1] == landed
            ? current.Trail
            : current.Trail.Append(landed).ToList();
        return new FocusStack(stack, trail);
    }
}

public sealed record Reseed(FocusStack Snapshot, string? Origin = null) : IIntent<FocusStack>
{
    public string Name => "focus-reseed";

    public FocusStack Apply(FocusStack current) => Snapshot;
}

public sealed record Reset(string? Origin = null) : IIntent<FocusStack>
{
    public string Name => "focus-reset";

    public FocusStack Apply(FocusStack current) => FocusStack.Empty;
}

public sealed record SeedFromTrail(IReadOnlyList<IExplorable> Nodes, string? Origin = null) : IIntent<FocusStack>
{
    public string Name => "focus-seed-from-trail";

    public FocusStack Apply(FocusStack current)
    {
        var result = FocusStack.Empty;
        foreach (var node in Nodes)
        {
            result = new Visit(node).Apply(result);
        }

        return result;
    }
}
