using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-3 (R2, <see cref="AtomNames.Selection"/>): value = an ORDERED,
/// duplicate-free list of <see cref="ExplorationDescriptor"/> -- "record
/// equality = the membership law" (R2, verbatim): identity is Kind+Key
/// (never Title, which is cached display text). The atom's own T is the
/// bare <c>IReadOnlyList{ExplorationDescriptor}</c> -- no wrapper record --
/// since R2 states the "value" IS the list; <see cref="StateAtom{T}"/> is
/// constructed with <see cref="SequenceEqualityComparer{T}"/> (Program.cs)
/// so law 2/law 5 hold despite List{T}'s own lack of structural equality
/// (see that comparer's own header for why).
///
/// NO <c>Add</c> intent: R2 allows one "if a non-toggle path exists" --
/// grepped every call site (Reader.razor's OnLineToggleSelect, World.razor's
/// marker/label toggle) and both are Ctrl/Cmd-click TOGGLES. Disclosed, not
/// silently omitted.
///
/// Fix round 1 (Q-2 -- IMPORTANT, review): <c>SelectionTrayService</c> is
/// RETIRED entirely -- its only remaining job (write "selection-v1" to
/// localStorage on every atom change) was a side effect living in a
/// constructor kept alive ONLY by a force-resolve nothing else in the app
/// needed ("no load-bearing constructors," the ruling's own words), and its
/// own <c>Available</c> property had zero real readers (the SAVE-affordance
/// UI it was documented as gating reads a DIFFERENT service,
/// <c>SavedExplorationsService.Available</c>). The persistence-write
/// subscription moves directly into THIS atom's own factory (Program.cs,
/// the SAME lambda that already has the JS runtime in hand to seed the
/// atom's initial value) -- an EXPLICIT initialization call in Program.cs's
/// own startup path, guaranteed to run because the atom itself has REAL,
/// load-bearing consumers (SelectionTray.razor/Reader.razor/World.razor all
/// inject it directly), not because anything was force-resolved to trigger
/// it.
/// </summary>
public static class Selection
{
    /// The "selection-v1" localStorage key -- read at the atom's own
    /// construction (Program.cs) to seed its initial value, written on every
    /// subsequent atom Changed (same factory). One name, one home, referenced
    /// by Program.cs directly; no separate service class carries it anymore.
    public const string StorageKey = "selection-v1";

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
