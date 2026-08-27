using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-3 (R3, <see cref="AtomNames.FocusStack"/>): one popover
/// SESSION's whole state -- an immutable stack snapshot (<see cref="Stack"/>,
/// bottom-to-top order, <see cref="Current"/> = the last/top entry, of
/// Contracts' own <see cref="Focus"/>) PLUS the G2 trail (<see cref="Trail"/>,
/// "where I've been," append-only, consecutive-dedupe rule) as ONE atom
/// value -- R3, verbatim: "they mutate together." <see cref="ExplorerPopover"/>'s
/// own pre-atom <c>Stack{IExplorable} _stack</c> + <c>List{ExplorationDescriptor} _trail</c>
/// fields collapse into this.
///
/// EQUALITY (fix round 1, Q-3 -- review, "prefer descriptor-based equality;
/// it is also what makes law tests meaningful for this atom"): hand-rolled
/// (not the compiler-generated, per-property one a plain <c>record</c> would
/// get), comparing <see cref="Stack"/> by each entry's own
/// <see cref="Focus.Descriptor"/> ONLY -- <see cref="Focus.Node"/> is
/// PAYLOAD, excluded from equality. Why: <see cref="Contracts.Focus"/> is
/// Contracts/-sealed, extend-only, with its OWN (uneditable) generated
/// equality -- Descriptor value-equal, Node REFERENCE-equal (concrete
/// <see cref="IExplorable"/> implementations, e.g. VerseNode/PlaceNode, are
/// plain classes, never records). A naive <c>Stack.SequenceEqual</c> (the
/// original draft) therefore only recognized "no change" when every entry
/// was the literal SAME node object -- which <see cref="SeedFromTrail"/>
/// breaks: MainLayout's own "continue" reconstructs FRESH node instances
/// via <c>ExplorationDescriptor.Reconstruct</c> on every click, so two
/// re-seeds of an EQUAL trail would never compare equal, firing spurious
/// <c>Changed</c> events and breaking law 2 for exactly the intent whose
/// whole job is "the same session, seeded again." Descriptor is this
/// value's own OBSERVABLE identity everywhere else already --
/// <see cref="Visit"/>'s own idempotent-no-op guard, <see cref="Back"/>'s
/// own trail-append-dedupe guard, and the popover's own rendering (Title/
/// Kind/Key, never anything Node-specific) all key off it -- so this is the
/// identity the type ALREADY had, made honest in <c>Equals</c> too, not a
/// new rule invented for this fix.
///
/// CONSEQUENCE (fix round 2, N-7, trivia -- re-review): because equality is
/// descriptor-only, a law-2 no-op <see cref="SeedFromTrail"/>/<see cref="Reseed"/>
/// (one whose descriptors already match what the atom holds) KEEPS the
/// atom's own, already-stored <see cref="Focus.Node"/> instances rather than
/// swapping in the freshly-reconstructed ones the caller just supplied --
/// so <c>Current => FocusValue.Current!.Node</c> can hand a caller a node
/// object that is not literally the one it passed in. Benign today (every
/// real producer of a "fresh" node --
/// <c>ExplorationDescriptor.Reconstruct</c> for "continue," a live
/// <c>IExplorable</c> pushed from a click -- is behaviourally equivalent for
/// the SAME descriptor, and nothing in this app compares node references),
/// but worth knowing before a future node kind's own behavior depends on
/// object identity rather than descriptor equality.
/// </summary>
public sealed record FocusStack(IReadOnlyList<Focus> Stack, IReadOnlyList<ExplorationDescriptor> Trail)
{
    public static readonly FocusStack Empty = new(Array.Empty<Focus>(), Array.Empty<ExplorationDescriptor>());

    /// <summary>The top of the stack -- what the popover is CURRENTLY
    /// showing. Null only for <see cref="Empty"/> itself (every REAL popover
    /// session seeds at least one entry before ever being rendered from --
    /// see ExplorerPopover.razor's own OnInitializedAsync).</summary>
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

/// <summary>
/// R3: "visit appends trail + pushes." IDEMPOTENT (law 2, standard sense,
/// unlike ToggleSelection's own deliberate exception): re-visiting the node
/// ALREADY on top is a no-op for BOTH the stack and the trail -- mirrors
/// RecordTrailVisit's own pre-atom "consecutive duplicates collapsed" rule
/// (ExplorerPopover.razor's own header comment, G2 decision 1), extended
/// here to the STACK half too so Apply(Apply(v)) == Apply(v) genuinely
/// holds (a naive unconditional push would NOT be idempotent -- pushing the
/// same node twice in a row would grow the stack by two entries, not
/// converge). No current call site actually re-visits an already-current
/// node consecutively (Push/PushAsync/Back are all discrete, distinct-node
/// UI actions) -- this guard is a safety property law 2 requires, not a
/// behavior change any existing Playwright coverage can observe either way.
/// </summary>
public sealed record Visit(IExplorable Node, string? Origin = null) : IIntent<FocusStack>
{
    public string Name => "focus-visit";

    public FocusStack Apply(FocusStack current)
    {
        var descriptor = ExplorationDescriptor.Capture(Node);
        if (current.Current is { } top && top.Descriptor == descriptor)
        {
            return current; // already here -- idempotent no-op, see this record's own header
        }

        var stack = current.Stack.Append(new Focus(descriptor, Node)).ToList();
        var trail = current.Trail.Count > 0 && current.Trail[^1] == descriptor
            ? current.Trail
            : current.Trail.Append(descriptor).ToList();
        return new FocusStack(stack, trail);
    }
}

/// <summary>
/// Fix round 1 (Adjudication D, S-1 -- CRITICAL, review): G2's OWN
/// record-of-nodes-traversed rule, restored -- **a Back LANDING is a
/// visit.** The pre-atom code (`ExplorerPopover.razor`'s own `Back()`)
/// called `RecordTrailVisit()` after every pop, appending the LANDED-ON
/// node's own descriptor to the trail with the SAME consecutive-dedupe
/// guard `Visit` uses (never double-appending when the landed node already
/// equals the trail's own last entry) -- `tests/ux/CONTRACT.md`'s own
/// shipped EXPLORE-TRAIL-1 row states this explicitly and is the winning
/// authority: "Recorded at every point Current actually changes (initial
/// open, every Push, **every Back landing**), collapsing consecutive
/// duplicates." An earlier draft of this record read "Back pops stack,
/// trail untouched" into R3's own wording and shipped it as a deliberate,
/// disclosed deviation -- WRONG: R3's wording was the controller's own
/// transcription error, not a ruling that ever meant to retire G2's shipped
/// behavior. Fixed here, not re-litigated further.
///
/// Idempotent by construction over the SAME-VALUE convergence law-2 tests
/// actually exercise: at the bottom of the stack (Count&lt;=1) this is a
/// genuine, real no-op (`current` returned unchanged, by reference). A
/// 3+-deep stack dispatched twice in a row is NOT pointwise-idempotent
/// (each dispatch pops --and appends-- once more) -- the same disclosed
/// negative-control shape `ToggleSelection` already established via the
/// `Increment` precedent, not a defect.
/// </summary>
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

/// <summary>
/// Fix round 1 (S-2/Adjudication E -- CRITICAL, review): the ownership
/// hand-off primitive -- restores a FULL session snapshot verbatim,
/// ignoring <c>current</c> entirely (the same overwrite shape
/// <see cref="Reset"/>/<see cref="SeedFromTrail"/> already use). The ONE
/// caller is a SUPERSEDED <c>ExplorerPopover</c> instance that reclaims
/// "focus-stack" ownership after the previously-active session closes
/// (its own <c>Dispose</c> having Reset the atom to <see cref="FocusStack.Empty"/>)
/// while THIS instance is still mounted and showing something real --
/// "claim = reconcile," mirroring <c>EffectRegistry.Claim</c>'s own
/// reconcile-on-claim shape (client/State/EffectRegistry.cs), just
/// reconciling from the reclaiming instance's OWN last-known session
/// (its frozen local snapshot) rather than from the atom's stale/reset
/// value. See ExplorerPopover.razor's own reclaim wiring for the full
/// mechanism.
/// </summary>
public sealed record Reseed(FocusStack Snapshot, string? Origin = null) : IIntent<FocusStack>
{
    public string Name => "focus-reseed";

    public FocusStack Apply(FocusStack current) => Snapshot;
}

/// <summary>
/// R3: "close resets both" -- stack AND trail. Idempotent by construction
/// (ignores <c>current</c> entirely, same overwrite shape as
/// <see cref="SetLocus"/>). Dispatched once per popover close (ANY close
/// path -- see ExplorerPopover.razor's own <c>Dispose</c>), replacing the
/// pre-atom "a fresh component instance naturally starts blank" mechanism
/// that stopped being sufficient the moment this state moved into a
/// DI-singleton, cross-instance-shared atom (mirrors ST-2's own
/// SyncTimeWindowProjection-at-mount lesson: a shared atom outlives any one
/// page visit, so "fresh" now has to be dispatched, not assumed).
/// </summary>
public sealed record Reset(string? Origin = null) : IIntent<FocusStack>
{
    public string Name => "focus-reset";

    public FocusStack Apply(FocusStack current) => FocusStack.Empty;
}

/// <summary>
/// R3 (G2 decision 5, "continue" -- MainLayout.razor's own hamburger trail
/// click): seeds the WHOLE session from an ordered node list in one shot --
/// index 0 pushed first, ..., the LAST entry pushed last and therefore
/// <see cref="FocusStack.Current"/>, mirroring ExplorerPopover's own pre-atom
/// SeedStack loop (push + record-trail-visit per node) exactly. Idempotent
/// by construction (ignores <c>current</c> entirely -- a fresh seed always
/// replaces whatever was there, the same "claim reconciles by overwriting"
/// shape <see cref="Reset"/> uses, just seeded rather than empty).
/// </summary>
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
