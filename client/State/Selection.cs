using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-3 (R2, <see cref="AtomNames.Selection"/>): value = an ORDERED,
/// duplicate-free list of <see cref="ExplorationDescriptor"/> -- "record
/// equality = the membership law" (R2, verbatim): identity is Kind+Key,
/// matching <see cref="SelectionTrayService"/>'s own pre-atom identity rule
/// exactly (never Title, which is cached display text). The atom's own T is
/// the bare <c>IReadOnlyList{ExplorationDescriptor}</c> -- no wrapper record
/// -- since R2 states the "value" IS the list; <see cref="StateAtom{T}"/> is
/// constructed with <see cref="SequenceEqualityComparer{T}"/> (Program.cs)
/// so law 2/law 5 hold despite List{T}'s own lack of structural equality
/// (see that comparer's own header for why).
///
/// NO <c>Add</c> intent: R2 allows one "if a non-toggle path exists" --
/// grepped every call site (Reader.razor's OnLineToggleSelect, World.razor's
/// marker/label toggle) and both are Ctrl/Cmd-click TOGGLES, matching
/// SelectionTrayService's own pre-atom "NO ACTION VERBS... exactly add/
/// remove/clear/list" scoping. Disclosed, not silently omitted.
/// </summary>
public static class Selection
{
    public static readonly IReadOnlyList<ExplorationDescriptor> Empty = Array.Empty<ExplorationDescriptor>();
}

/// <summary>
/// The Ctrl/Cmd-click gesture itself -- not currently selected -> add it;
/// already selected -> remove it. DELIBERATELY, DISCLOSED-ly NOT idempotent
/// in the per-intent sense law 2 otherwise asks every intent to honor
/// (Apply(Apply(v)) != Apply(v) whenever this descriptor is already
/// selected -- redispatching the SAME ToggleSelection instance twice
/// genuinely flips selection state twice, back to the original, exactly
/// mirroring client.Tests/State/Generators.cs's own <c>Increment</c>
/// negative-control precedent for "an intent that is deliberately NOT
/// idempotent, by its own nature, not by oversight"). This is the CORRECT,
/// required behavior -- SelectionTrayService.Toggle already worked this way
/// pre-atom, and Ctrl/Cmd-click's whole UX contract (click = select, click
/// AGAIN on the SAME item = deselect) is exactly a flip, not a no-op-on-
/// redispatch. R5's own "idempotent TOGGLE-PAIRS" phrasing (not "idempotent
/// Toggle") is read as blessing this shape: the property that DOES hold is
/// pair-idempotence (two toggles of the SAME descriptor return to the
/// ORIGINAL list), tested in SelectionTests.cs, not literal law 2.
/// </summary>
public sealed record ToggleSelection(ExplorationDescriptor Descriptor, string? Origin = null) : IIntent<IReadOnlyList<ExplorationDescriptor>>
{
    public string Name => "toggle-selection";

    public IReadOnlyList<ExplorationDescriptor> Apply(IReadOnlyList<ExplorationDescriptor> current)
    {
        var existing = current.FirstOrDefault(i => i.Kind == Descriptor.Kind && i.Key == Descriptor.Key);
        return existing is not null
            ? current.Where(i => i != existing).ToList() // record equality, mirrors SelectionTrayService.Toggle's own pre-atom logic exactly
            : current.Append(Descriptor).ToList();
    }
}

/// <summary>selection-chip-{i}'s own remove control. Genuinely idempotent
/// (law 2, standard sense): removing an already-absent descriptor is a
/// structural no-op, so redispatching the same instance twice produces the
/// same list both times.</summary>
public sealed record RemoveSelection(ExplorationDescriptor Descriptor, string? Origin = null) : IIntent<IReadOnlyList<ExplorationDescriptor>>
{
    public string Name => "remove-selection";

    public IReadOnlyList<ExplorationDescriptor> Apply(IReadOnlyList<ExplorationDescriptor> current) =>
        current.Where(i => !(i.Kind == Descriptor.Kind && i.Key == Descriptor.Key)).ToList();
}

/// <summary>selection-clear. Idempotent by construction (ignores <c>current</c>
/// entirely, same shape as <see cref="SetLocus"/>/<see cref="ResetCounter"/>-style
/// intents elsewhere in this app).</summary>
public sealed record ClearSelection(string? Origin = null) : IIntent<IReadOnlyList<ExplorationDescriptor>>
{
    public string Name => "clear-selection";

    public IReadOnlyList<ExplorationDescriptor> Apply(IReadOnlyList<ExplorationDescriptor> current) => Selection.Empty;
}
