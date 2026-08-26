using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-1: the ONE concrete <see cref="IStateAtom{T}"/> implementation --
/// every named atom in this app (Locus, TimeWindow, and whatever ST-2/ST-3
/// add: FocusStack, Selection, ViewArrangement) is an instance of this class,
/// never a bespoke per-atom type. Registered singleton in Program.cs (the
/// ViewStateService precedent -- Blazor WASM has exactly one DI scope for
/// the app's whole life, so "scoped" and "singleton" coincide here).
///
/// LAW 1 (single-writer): <see cref="Value"/> has a PRIVATE setter --
/// <see cref="Dispatch"/> is structurally the only code path in this whole
/// assembly that can ever change it. (client.Tests/State/StateAtomLawTests.cs
/// asserts this via reflection -- "no public setter exists" -- so a future
/// edit that widens it fails loudly, not silently.)
///
/// LAW 2 (idempotence): <see cref="Dispatch"/> compares the intent's result
/// against the CURRENT value (via <paramref name="comparer"/> or
/// <see cref="EqualityComparer{T}.Default"/>) and no-ops -- Value unchanged,
/// <see cref="Changed"/> not raised -- when they're equal. Combined with an
/// intent whose own <see cref="IIntent{T}.Apply"/> is itself idempotent
/// (Apply(Apply(v)) == Apply(v), the per-intent obligation -- see each
/// intent record's own doc comment), dispatching the SAME intent twice always
/// produces the same observable effect as dispatching it once: the second
/// call's Apply(current) equals current (since current already equals
/// Apply(the value before either dispatch)), so it hits this no-op branch.
/// </summary>
public sealed class StateAtom<T> : IStateAtom<T>
{
    private readonly IEqualityComparer<T> _comparer;

    public StateAtom(string name, T initial, IEqualityComparer<T>? comparer = null)
    {
        Name = name;
        Value = initial;
        _comparer = comparer ?? EqualityComparer<T>.Default;
    }

    public string Name { get; }

    public T Value { get; private set; }

    /// <summary>
    /// LAW 3 (no-echo) support: the <see cref="IIntent{T}.Origin"/> of the
    /// most recent intent that actually changed <see cref="Value"/> (null =
    /// that change was user-originated; a link's own <see cref="IStateLink{A,B}.Name"/>
    /// = it was link-derived). NOT part of <see cref="IStateAtom{T}"/> (the
    /// compiled contract's own seam) -- a new member on OUR OWN concrete
    /// type, read only by <see cref="StateLinkRunner{A,B}"/> to decide
    /// whether ITS OWN source just echoed a derivation that already passed
    /// through it (see that class's own header comment for the full
    /// mechanism a bidirectional link pair needs to avoid oscillating).
    /// Stays at its prior value across a no-op Dispatch (law 2) -- there is
    /// no "new" origin to record when nothing changed.
    /// </summary>
    public string? LastOrigin { get; private set; }

    public event Action? Changed;

    public void Dispatch(IIntent<T> intent)
    {
        var next = intent.Apply(Value);
        if (_comparer.Equals(Value, next))
        {
            return; // law 2: same-effect dispatch is silent -- no Changed, no LastOrigin update
        }

        Value = next;
        LastOrigin = intent.Origin;
        Changed?.Invoke();
    }
}
