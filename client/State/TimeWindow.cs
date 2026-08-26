using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-1: the atlas's time-mode window (spec §4d,
/// <see cref="AtomNames.TimeWindow"/>), landing THIS batch in MINIMAL,
/// skeleton-conformant form -- solely so the follow-text link
/// (<see cref="FollowTextLink"/>) is a real, wired, testable object, per the
/// controller ruling ("TimeWindow atom lands THIS batch in minimal form
/// solely so the follow-text link is real; its full migration is ST-2").
/// FULL ownership of this atom -- slider drags, camera, arrangement all
/// writing through it instead of World.razor's own `_from`/`_to` fields --
/// is explicitly OUT of scope; those writers are disclosed, unmigrated, in
/// the batch report. This record exists to be Link(Locus -&gt; TimeWindow)'s
/// Target, nothing more, this batch.
/// </summary>
public sealed record TimeWindow(int From, int To)
{
    /// <summary>Mirrors World.razor's own DefaultFrom/DefaultTo (the Gospels
    /// era, -5..33) -- the bare-/world default every pre-ST-1 session already
    /// opened on.</summary>
    public static readonly TimeWindow Default = new(-5, 33);
}

/// <summary>Batch ST-1 seed intent -- idempotent the same way
/// <see cref="SetLocus"/> is (see that record's own doc comment): a fresh
/// value each call, record equality catches the repeat.</summary>
public sealed record SetTimeWindow(int From, int To, string? Origin = null) : IIntent<TimeWindow>
{
    public string Name => "set-time-window";

    public TimeWindow Apply(TimeWindow current) => new(From, To);
}

/// <summary>
/// Batch ST-1: "follow-text IS Link(Locus -&gt; TimeWindow)" (spec §4d), active
/// in split view -- the ONE link this batch wires. <see cref="Active"/> is
/// bound to the SAME bool the pre-existing follow chip already reads/writes
/// (World.razor's own `_follow`, unrenamed -- see the report's migration
/// notes), via the constructor-supplied delegate, so toggling the chip IS
/// toggling this link, no parallel flag.
///
/// DERIVE, disclosed: this app's own scripture-mode ("following") has never
/// computed a NUMERIC {From, To} window at all -- `EnterScriptureMode`
/// (World.razor) fetches a scene by REF, orthogonal to `_from`/`_to`, which
/// it deliberately never touches while scripture mode is active (confirmed
/// by reading that method: no assignment to either field). So the "scripture-
/// driven derivation" the brief asks this Derive to compute -- "exactly what
/// SyncFollowRef/EnterScriptureMode compute today" -- for the WINDOW VALUE
/// specifically is: nothing; the window is left exactly as it was. Derive is
/// therefore a pure identity pass-through -- and, precisely BECAUSE it is
/// identity, this link is INERT at runtime, not merely low-value (review
/// Adjudication 2): the runner's own dispatch of
/// <c>LinkDerivedIntent(name, current)</c> is rejected by
/// <see cref="StateAtom{T}.Dispatch"/>'s equality check before it ever
/// touches <see cref="TimeWindow"/>'s Value, fires its Changed event, or
/// stamps its LastOrigin. <see cref="StateLinkRunner{A,B}.SyncNow"/>, called
/// from World.razor's ToggleFollow, is likewise a guaranteed no-op here.
/// Still real, still wired, still tested (law 4/confluence, and the
/// inertness itself, both pinned in LocusAndTimeWindowTests.cs) -- the
/// scaffolding is correct and correctly scoped, it simply does no WORK yet.
/// The actual visible "follow" BEHAVIOR -- the atlas pane re-scening to the
/// reader's current chapter -- is driven separately, by World.razor
/// subscribing directly to the Locus atom's own Changed event (guarded on
/// this SAME Active flag) and calling EnterScriptureMode(locus.Ref), exactly
/// mirroring the pre-ST-1 SyncFollowRef mechanism, just keyed off the shared
/// Locus atom instead of a FollowRef parameter -- because the compiled
/// contract makes Derive synchronous and pure, and this behavior needs an
/// async scene fetch, no pure link could ever have carried it. FULL
/// TimeWindow ownership (ST-2) is expected to replace this identity Derive
/// with a real one once the atom itself, not just this link's scaffolding,
/// owns the window -- ST-2 should not treat the existing wiring as a working
/// reference for a link that carries real behavior; it never has.
/// </summary>
public sealed class FollowTextLink : IStateLink<Locus, TimeWindow>
{
    private readonly Func<bool> _active;

    public FollowTextLink(IStateAtom<Locus> source, IStateAtom<TimeWindow> target, Func<bool> active)
    {
        Source = source;
        Target = target;
        _active = active;
    }

    public string Name => "follow-text";

    public IStateAtom<Locus> Source { get; }
    public IStateAtom<TimeWindow> Target { get; }

    public TimeWindow Derive(Locus source, TimeWindow current) => current; // see this class's own header comment

    public bool Active => _active();
}
