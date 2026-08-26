using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-2 (controller ruling R1): the atlas's time-mode window is now a
/// ONE-OF UNION -- <see cref="TimeMode"/> | <see cref="ScriptureMode"/> --
/// mirroring the "one-or-the-other" discipline World.razor already kept by
/// hand pre-ST-2 (<c>_scriptureRef</c> non-null meant window params passed
/// null; <c>EnterScriptureMode</c> never touched <c>_from</c>/<c>_to</c>).
/// This is a NEW value type superseding ST-1's minimal
/// <c>TimeWindow(int From, int To)</c> record (extend-only compliant -- it
/// is OUR OWN type, not a Contracts/ member; ST-1's consumers are updated in
/// this same commit, per the controller ruling).
///
/// A sealed record HIERARCHY (not a struct enum + payload, not a single
/// record with nullable fields for both arms) -- chosen so illegal states
/// (a window that is somehow BOTH a numeric range AND a scripture ref, or
/// NEITHER) are unrepresentable, the same "structure it so it cannot occur"
/// philosophy the owner's own ruling names for this whole migration.
/// C# record equality is used for law 2 (idempotence) and the union-equality
/// tests this batch adds (client.Tests/State/LocusAndTimeWindowTests.cs --
/// fix round 1, Q-4: corrected from an earlier draft's own stale
/// "TimeWindowTests.cs" citation, no such file exists) -- deliverable 2 asks
/// this to be PROVEN, not assumed, since a hand-written Equals bug here
/// would poison every law test built on top of it.
/// </summary>
public abstract record TimeWindow
{
    /// <summary>Mirrors World.razor's own DefaultFrom/DefaultTo (the Gospels
    /// era, -5..33) -- the bare-/world default every pre-ST-2 session already
    /// opened on.</summary>
    public static readonly TimeWindow Default = new TimeMode(-5, 33);
}

/// <summary>The atlas is showing a plain numeric time-window scene.</summary>
public sealed record TimeMode(int From, int To) : TimeWindow;

/// <summary>
/// The atlas is following a scripture reference -- Batch ST-2's
/// <see cref="FollowTextLink"/> derives exactly this arm (R2). Carries only
/// the ref string, deliberately no numeric window: pre-ST-2 scripture mode
/// never computed one either (see FollowTextLink's own header comment for
/// the full history of that discipline).
/// </summary>
public sealed record ScriptureMode(string Ref) : TimeWindow;

/// <summary>
/// Enters (or stays in) time mode with an explicit numeric window. Idempotent
/// by construction -- a fresh <see cref="TimeMode"/> each call, record
/// equality catches a repeat dispatch (law 2).
/// </summary>
public sealed record SetTimeWindow(int From, int To, string? Origin = null) : IIntent<TimeWindow>
{
    public string Name => "set-time-window";

    public TimeWindow Apply(TimeWindow current) => new TimeMode(From, To);
}

/// <summary>
/// Enters (or stays in) scripture mode against an explicit ref. Idempotent
/// the same way <see cref="SetTimeWindow"/> is (law 2).
/// </summary>
public sealed record SetScriptureWindow(string Ref, string? Origin = null) : IIntent<TimeWindow>
{
    public string Name => "set-scripture-window";

    public TimeWindow Apply(TimeWindow current) => new ScriptureMode(Ref);
}

/// <summary>
/// Batch ST-2 (controller ruling R2): "follow-text IS Link(Locus -&gt;
/// TimeWindow)" (spec §4d), now REAL. <see cref="Derive"/> computes exactly
/// <c>ScriptureMode(locus.Ref)</c> -- synchronous, pure, and genuinely
/// idempotent (the SAME locus derives the SAME <see cref="ScriptureMode"/>
/// value every time; record equality makes a repeat derive-and-dispatch a
/// true no-op via <see cref="StateAtom{T}.Dispatch"/>'s own law-2 guard, not
/// merely "returns current unchanged" the way ST-1's identity Derive did).
///
/// <see cref="Active"/> is bound to the SAME "is this pane genuinely
/// following" bool World.razor's follow chip reads/writes -- unchanged
/// plumbing shape from ST-1 (a constructor-supplied delegate), just now
/// resolving to the ViewArrangement projection (controller ruling R4:
/// <c>Split &amp;&amp; Follow</c>) instead of the old <c>SplitMode &amp;&amp;
/// _follow</c> field closure -- see World.razor's own
/// <c>_followLink</c> construction site.
///
/// THE ASYNC SCENE FETCH DOES NOT LIVE HERE (R2, explicit): Derive is pure
/// and synchronous by the compiled contract's own declaration
/// (<see cref="IStateLink{A,B}.Derive"/>) -- it cannot perform I/O. The
/// actual "atlas pane re-scenes to the followed chapter" BEHAVIOR is an
/// EFFECT, realized by World.razor's own subscription to
/// <c>TimeWindowAtom.Changed</c> (<c>OnTimeWindowChanged</c>) -- the TARGET
/// atom's own Changed event, not a side-channel subscription to the SOURCE
/// (Locus.Changed) the way ST-1's now-RETIRED mechanism worked (ST-1's
/// <c>OnLocusChanged</c> used to call <c>EnterScriptureMode</c> directly,
/// gated on this exact <see cref="Active"/> flag; that call is gone --
/// <c>OnLocusChanged</c> now only re-renders the picker boxes). This closes
/// ST-1 review's own "inert" disclosure for good: the link is no longer a
/// scaffolding-only object that happens to do no work -- it is the thing
/// that ACTUALLY carries "follow" now, load-bearing behavior included.
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

    public TimeWindow Derive(Locus source, TimeWindow current) => new ScriptureMode(source.Ref);

    public bool Active => _active();
}
