using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-3 (R1): the ONE concrete <see cref="IEffectRegistry"/>
/// implementation -- realizes <see cref="IStateEffect{T}"/>'s own doc comment
/// (client/Contracts/State.cs) exactly: one owner per effect NAME; CLAIM on
/// mount, release on dispose (the returned <see cref="IDisposable"/>); LATEST
/// claim wins (a newer <see cref="Claim{T}"/> call for the SAME name always
/// supersedes an older one, synchronously, regardless of whether the older
/// claimant's own <see cref="IDisposable.Dispose"/> has been -- or ever will
/// be -- called); claiming RECONCILES (runs <see cref="IStateEffect{T}.Materialize"/>
/// against the atom's CURRENT value at claim time, even though a
/// law-2-converged atom raises no <see cref="IStateAtom{T}.Changed"/> for a
/// new claimant to react to); while claimed, every real change of the
/// effect's own <see cref="IStateEffect{T}.Source"/> re-runs the effect.
///
/// SUPERSESSION MECHANISM: each <see cref="Claim{T}"/> call mints a fresh,
/// private `object` TOKEN and stores it as the effect name's current owner.
/// The <see cref="IStateAtom{T}.Changed"/> handler this method wires closes
/// over that SAME token and checks -- ON EVERY INVOCATION, not just at
/// subscribe time -- whether it is STILL the name's current owner before
/// calling <see cref="IStateEffect{T}.Materialize"/> at all. This is what
/// makes "latest wins" true independent of Blazor's own undocumented
/// dispose-vs-mount ordering (the exact hazard ST-2's own review,
/// Adjudication A part 2, named as the reason a per-handler `SplitMode`-style
/// guard doesn't scale): a superseded claimant's handler stays WIRED
/// (nothing unsubscribes it until its own <see cref="IDisposable.Dispose"/>
/// eventually runs, whenever that is) but goes permanently INERT the moment
/// a newer claim exists, because the token comparison -- not subscription
/// membership -- is what gates materialization.
///
/// Blazor WebAssembly is single-threaded (this codebase's own established
/// fact -- see e.g. LocalStore.cs's own header, StateAtom{T}'s own lack of
/// locking) -- a plain <see cref="Dictionary{TKey,TValue}"/>, no locking, is
/// therefore the right level of ceremony here, consistent with every other
/// concrete State/ type.
/// </summary>
public sealed class EffectRegistry : IEffectRegistry
{
    private sealed class Slot
    {
        public object? Owner;

        // Fix round 1 (Q-7, trivia -- review, "cheap hardening"): the
        // CURRENT owner's own unsubscribe action, so a NEW claim can
        // proactively detach the prior claimant's handler immediately,
        // rather than leaving it wired (inert, via the token check alone --
        // correctness never depended on this) until ITS OWN, possibly-
        // never-called Dispose. Without this, a claimant that never
        // disposes leaks one live delegate on the atom for the app's
        // lifetime; with it, at most ONE superseded delegate is ever
        // wired at a time per name.
        public Action? Unsubscribe;
    }

    private readonly Dictionary<string, Slot> _slots = new();

    /// <summary>
    /// Public, more-specifically-typed overload (the explicit
    /// <see cref="IEffectRegistry.Claim{T}"/> below forwards to this one) --
    /// callers that hold a concrete <see cref="EffectRegistry"/> reference
    /// (every real call site in this app; the interface exists for
    /// Contracts/-facing code, not because any concrete caller needs to be
    /// decoupled from this type) can read <see cref="EffectClaim.ReconcileTask"/>
    /// without a downcast -- the ONE piece of information a caller
    /// legitimately needs back from a claim that the interface's own
    /// `IDisposable` return type cannot carry (the compiled contract's
    /// signature is extend-only; this is our own concrete type's own,
    /// additive capability, not a Contracts/ edit).
    /// </summary>
    public EffectClaim Claim<T>(IStateEffect<T> effect)
    {
        var slot = GetOrAddSlot(effect.Name);

        // Fix round 1 (Q-7, trivia): proactively detach the PRIOR owner's
        // handler -- cheap hardening on top of the token check below, which
        // already made this safe on its own; this just stops a
        // never-disposed prior claimant from leaking one live delegate on
        // its own Source atom for the app's lifetime.
        slot.Unsubscribe?.Invoke();
        slot.Unsubscribe = null;

        var token = new object();
        slot.Owner = token; // latest claim wins -- supersedes any prior claimant immediately, synchronously

        void OnSourceChanged()
        {
            if (!ReferenceEquals(slot.Owner, token))
            {
                return; // superseded -- see this class's own header comment
            }

            var value = effect.Source.Value;
            if (effect.AppliesTo(value))
            {
                _ = effect.Materialize(value); // fire-and-forget: ongoing changes aren't awaited by any caller, matching the pre-registry OnTimeWindowChanged handler this replaces
            }
        }

        effect.Source.Changed += OnSourceChanged;
        slot.Unsubscribe = () => effect.Source.Changed -= OnSourceChanged;

        // Reconcile-on-claim: run against the CURRENT value now, since a
        // law-2-converged atom (the exact S-1 blank-atlas-pane hazard ST-2's
        // own review found and fixed by hand, via TriggerFollowSync) raises
        // no Changed for this brand-new claim to react to.
        Task? reconcileTask = null;
        if (effect.AppliesTo(effect.Source.Value))
        {
            reconcileTask = effect.Materialize(effect.Source.Value);
        }

        return new EffectClaim(reconcileTask, () =>
        {
            effect.Source.Changed -= OnSourceChanged; // safe even if already detached by a later claim's own proactive unsubscribe above -- removing an already-removed delegate is a no-op
            if (ReferenceEquals(slot.Owner, token))
            {
                slot.Owner = null;
                slot.Unsubscribe = null;
            }
        });
    }

    IDisposable IEffectRegistry.Claim<T>(IStateEffect<T> effect) => Claim(effect);

    private Slot GetOrAddSlot(string name)
    {
        if (!_slots.TryGetValue(name, out var slot))
        {
            slot = new Slot();
            _slots[name] = slot;
        }

        return slot;
    }
}

/// <summary>
/// The handle <see cref="EffectRegistry.Claim{T}"/> returns. Disposing it is
/// the RELEASE half of the claim/release contract (<see cref="IStateEffect{T}"/>'s
/// own doc comment) -- idempotent (a second Dispose is a harmless no-op, the
/// same discipline every other IDisposable in client/State/ already follows,
/// e.g. <see cref="StateLinkRunner{A,B}"/>).
/// </summary>
public sealed class EffectClaim : IDisposable
{
    private Action? _release;

    internal EffectClaim(Task? reconcileTask, Action release)
    {
        ReconcileTask = reconcileTask;
        _release = release;
    }

    /// <summary>
    /// The <see cref="Task"/> returned by the reconcile-on-claim
    /// <see cref="IStateEffect{T}.Materialize"/> call, if <see cref="IStateEffect{T}.AppliesTo"/>
    /// gated it to run at all; null otherwise. A caller that needs to AWAIT
    /// "did the claim's own initial materialization finish" (mirroring the
    /// pre-registry `_pendingLinkEffect` await contract World.razor's own
    /// ToggleFollow/InitializeSplitModeIfNeeded depended on, so a caller's
    /// own async method still only completes once the actual fetch does)
    /// reads this ONCE, immediately after <see cref="EffectRegistry.Claim{T}"/>
    /// returns -- it is not re-populated by any LATER, Changed-triggered
    /// materialization (those are genuinely fire-and-forget, matching the
    /// mechanism they replace).
    /// </summary>
    public Task? ReconcileTask { get; }

    public void Dispose()
    {
        _release?.Invoke();
        _release = null;
    }
}

/// <summary>
/// Batch ST-3 (R1, "a concrete effect base if useful"): a small adapter so a
/// call site can declare an <see cref="IStateEffect{T}"/> as three values
/// (a name, a source atom, a pure gate, an async materializer) rather than
/// hand-writing a bespoke class per effect -- the SAME "small records/
/// adapters over bespoke types" economy this app already applies to intents
/// (<see cref="LinkDerivedIntent{T}"/> plays the identical role for links).
/// </summary>
public sealed class DelegateEffect<T> : IStateEffect<T> where T : notnull
{
    private readonly Func<T, bool> _appliesTo;
    private readonly Func<T, Task> _materialize;

    public DelegateEffect(string name, IStateAtom<T> source, Func<T, bool> appliesTo, Func<T, Task> materialize)
    {
        Name = name;
        Source = source;
        _appliesTo = appliesTo;
        _materialize = materialize;
    }

    public string Name { get; }
    public IStateAtom<T> Source { get; }
    public bool AppliesTo(T value) => _appliesTo(value);
    public Task Materialize(T value) => _materialize(value);
}
