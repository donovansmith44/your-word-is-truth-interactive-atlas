using BibleAtlas.Client.Explore;
using Microsoft.AspNetCore.Components;

namespace BibleAtlas.Client.Contracts;

// Spec §4: the focus component -- the owner's named construct -- and its
// frontier/traversal/hatch/presentation vocabulary.
// See Views.cs header for the compiled-authority discipline binding this file.

/// <summary>
/// Spec §4: "descriptor + payload." SKELETON RESOLUTION (the spec names the
/// type, not its members): the descriptor is the G2 serialization seam
/// (<see cref="ExplorationDescriptor"/> -- saved explorations and the tray
/// already ride it); the payload is the live <see cref="IExplorable"/> node,
/// which every current surface already holds. Invariant:
/// <c>ExplorationDescriptor.Capture(Node)</c> equals <see cref="Descriptor"/>.
/// </summary>
public sealed record Focus(ExplorationDescriptor Descriptor, IExplorable Node);

/// <summary>Spec §4: a component that presents a focus, its frontier, and
/// its escape hatches. Composes onto <see cref="IViewComponent"/> -- never a
/// base class.</summary>
public interface IFocusComponent : IViewComponent
{
    Focus Focus { get; }
    IReadOnlyList<IFrontierAbstraction> Frontier { get; }
    IReadOnlyList<IEscapeHatch> EscapeHatches { get; }
}

/// <summary>
/// Spec §4: one frontier abstraction per typed edge family on the focus
/// node. Direction (spec §4, [ASSUMED -- §9 Q5]): these are GENERATED from
/// the crate's relations! manifest -- a new typed edge automatically
/// surfaces as a (default-presented) frontier abstraction; WHAT a popover
/// chooses to SHOW remains governed by POPOVER-LAW-1.
/// </summary>
public interface IFrontierAbstraction
{
    /// <summary>A relations! name: "comments-on", "spoken-by", ...</summary>
    string EdgeFamily { get; }

    /// <summary>Presentation label.</summary>
    string Label { get; }

    /// <summary>Known count where cheap, null where lazy.</summary>
    int? Cardinality { get; }

    ITraversal Traversal { get; }
}

/// <summary>
/// Spec §4: what Push/Back/expand MEAN. One implementation rules (the
/// popover); saved explorations and the selection tray consume the same
/// descriptors (<see cref="Describe"/> is the G2 seam).
/// </summary>
public interface ITraversal
{
    Task<IReadOnlyList<Focus>> Expand(int page);
    ExplorationDescriptor Describe(Focus target);
}

/// <summary>
/// Spec §4/§5: a typed exit that LEAVES graph traversal into a view context.
/// Kinds: "read-in-context" | "show-on-world" | "mini-map" | ... A hatch
/// that navigates nowhere is a contract violation (testable).
/// </summary>
public interface IEscapeHatch
{
    string Kind { get; }
    Task Invoke();
}

/// <summary>Spec §4: how (view, focus, frontier, hatches) LOOKS -- the ONE
/// render rule generalized. Components implement presentation via
/// composition (a presenter per surface), never by subclassing a base
/// component.</summary>
public interface IPresentation<TContext>
{
    RenderFragment Render(TContext ctx);
}
