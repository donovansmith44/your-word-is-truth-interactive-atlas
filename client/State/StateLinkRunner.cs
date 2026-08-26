using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-1: the link RUNNER -- subscribes a declared <see cref="IStateLink{A,B}"/>
/// to its own <see cref="IStateLink{A,B}.Source"/> and dispatches derived
/// intents into its own <see cref="IStateLink{A,B}.Target"/> while
/// <see cref="IStateLink{A,B}.Active"/>, per the contract's own ALGEBRA
/// section. The <c>IStateLink&lt;A,B&gt;</c> passed in is the pure
/// declaration (Derive/Active); this class is the IMPERATIVE half (the
/// subscription, the dispatch) -- kept separate so a link's own Derive stays
/// trivially unit-testable with no event wiring at all.
///
/// LAW 3 (no-echo): the mechanism a bidirectional link PAIR needs to avoid
/// oscillating. Concretely: this runner refuses to derive at all when its
/// own Source atom's <see cref="StateAtom{T}.LastOrigin"/> is non-null --
/// i.e., when the change that just fired <see cref="IStateAtom{T}.Changed"/>
/// on Source was ITSELF a link-derived update (dispatched via
/// <see cref="LinkDerivedIntent{T}"/>, which always stamps Origin), not a
/// genuine user gesture. Trace why this is sufficient for a real
/// bidirectional pair -- Link("ab", Source=A, Target=B) and
/// Link("ba", Source=B, Target=A), both Active:
///   1. A user dispatches into A with Origin=null. A.Changed fires.
///   2. Runner("ab") sees A.LastOrigin == null -> derives, dispatches into
///      B via LinkDerivedIntent(Origin: "ab"). B.Changed fires (assuming the
///      derived value actually differs -- law 2's own no-op guard).
///   3. Runner("ba") (subscribed to B.Changed, since B is ITS Source) checks
///      B.LastOrigin -- it's "ab" (non-null) -- and refuses to derive.
///   The chain stops after exactly one hop; A is never written to again for
///   this gesture. Without this guard, step 3 would derive from B back into
///   A, potentially re-triggering Runner("ab") in an unbounded loop whenever
///   Derive isn't a true fixed point.
/// SCOPE NOTE (disclosed in the batch report): this is a coarse, SAFE rule
/// -- "never re-derive across ANY link-derived change," not just a change
/// traceable to one's own reverse counterpart -- which is more than
/// sufficient to prevent oscillation but would also suppress a genuine,
/// intentional multi-hop derivation CHAIN (A -&gt; B -&gt; C, all real links) after
/// its first hop. No such chain exists in this app as of ST-1 (the only
/// link landing this batch, Locus -&gt; TimeWindow, is one hop); a future batch
/// that needs real chaining should revisit this rule specifically, not the
/// atom/projection layer around it.
/// </summary>
public sealed class StateLinkRunner<A, B> : IDisposable
{
    private readonly string _name;
    private readonly IStateLink<A, B> _link;
    private readonly StateAtom<A> _source;
    private readonly StateAtom<B> _target;
    private bool _disposed;

    /// <summary>
    /// <paramref name="name"/> is taken explicitly, NOT read off
    /// <paramref name="link"/> -- the compiled <see cref="IStateLink{A,B}"/>
    /// contract declares Source/Target/Derive/Active only, no Name member
    /// (its own header prose describes "the link's name," but that member
    /// isn't in the interface itself, and Contracts/ types are extend-only:
    /// this runner can't add one). Callers that want a link's name
    /// discoverable on the link object itself (as <see cref="FollowTextLink"/>
    /// does, for its own test/telemetry vocabulary -- the same role
    /// <see cref="IIntent{T}.Name"/> plays for intents) are free to add a
    /// `Name` property to their OWN concrete type and pass it through here
    /// explicitly; the runner itself stays correct for any
    /// <see cref="IStateLink{A,B}"/> implementation, named or not.
    /// </summary>
    public StateLinkRunner(string name, IStateLink<A, B> link, StateAtom<A> source, StateAtom<B> target)
    {
        if (!ReferenceEquals(link.Source, source))
        {
            throw new ArgumentException("link.Source must be the same instance as the supplied source atom", nameof(source));
        }

        if (!ReferenceEquals(link.Target, target))
        {
            throw new ArgumentException("link.Target must be the same instance as the supplied target atom", nameof(target));
        }

        _name = name;
        _link = link;
        _source = source;
        _target = target;
        _source.Changed += OnSourceChanged;
    }

    private void OnSourceChanged()
    {
        if (_disposed || !_link.Active)
        {
            return;
        }

        if (_source.LastOrigin is not null)
        {
            return; // law 3 (no-echo) -- see this class's own header comment
        }

        var derived = _link.Derive(_source.Value, _target.Value);
        _target.Dispatch(new LinkDerivedIntent<B>(_name, derived));
    }

    /// <summary>
    /// Forces one derive-and-dispatch pass right now, regardless of whether
    /// Source just changed -- for the one real UX case a pure Changed
    /// subscription can't cover: a link's OWN <see cref="IStateLink{A,B}.Active"/>
    /// flag flipping from false to true (e.g. the follow chip being clicked
    /// back on) must re-sync the Target to the CURRENT Source value
    /// immediately, not wait for the next unrelated Source mutation. Mirrors
    /// FOLLOW-1's own pre-existing "toggling follow back on re-syncs to the
    /// current chapter's scene" behavior -- see World.razor's ToggleFollow.
    /// Still routes through the SAME no-echo guard (skipped here -- a
    /// caller-invoked SyncNow is by definition not a reaction to a Source
    /// echo) and the SAME Dispatch, so it stays law-2/law-3 consistent with
    /// every other path into Target.
    /// </summary>
    public void SyncNow()
    {
        if (_disposed || !_link.Active)
        {
            return;
        }

        var derived = _link.Derive(_source.Value, _target.Value);
        _target.Dispatch(new LinkDerivedIntent<B>(_name, derived));
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        _source.Changed -= OnSourceChanged;
    }
}
