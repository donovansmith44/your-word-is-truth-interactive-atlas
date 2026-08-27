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
/// EQUALITY: hand-rolled (not the compiler-generated, per-property one a
/// plain <c>record</c> would get) -- <see cref="IReadOnlyList{T}"/> properties
/// need SEQUENCE equality for the SAME law-2/law-5 reason
/// <see cref="SequenceEqualityComparer{T}"/>'s own header explains for
/// Selection; overriding the record's own generated <c>Equals(FocusStack?)</c>
/// directly (a supported, standard C# technique -- the compiler uses YOUR
/// definition instead of generating a member-wise one once the signature
/// matches) keeps this a genuine <c>record</c> (value semantics everywhere
/// else: <c>with</c> expressions, deconstruction) while fixing ONLY the
/// list-comparison gap. <see cref="Contracts.Focus"/> itself is Contracts/-
/// sealed, extend-only, with its OWN (uneditable) equality -- Descriptor
/// value-equal, Node REFERENCE-equal (concrete <see cref="IExplorable"/>
/// implementations, e.g. VerseNode/PlaceNode, are plain classes, never
/// records -- grep-confirmed). SequenceEqual over Stack therefore only
/// recognizes "no change" when every UNCHANGED entry is the SAME node
/// object reference, not merely an equal one -- true by construction here,
/// since every intent below only ever appends/removes at the ends and never
/// reconstructs a Focus for an entry it isn't touching.
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
        other is not null && Stack.SequenceEqual(other.Stack) && Trail.SequenceEqual(other.Trail);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(Stack.Count);
        hash.Add(Trail.Count);
        if (Current is { } c)
        {
            hash.Add(c);
        }

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
/// R3: "Back pops stack, TRAIL UNTOUCHED" -- verbatim, applied even though
/// the pre-ST-3 code (ExplorerPopover.razor's own <c>Back()</c>) called
/// <c>RecordTrailVisit()</c> after every pop too (recording a return to a
/// shallower node whenever it differed from the trail's own last entry --
/// which it usually does, since Back is normally reached only after pushing
/// at least one level deeper first). DISCLOSED, RULING-MANDATED BEHAVIOR
/// CHANGE from that prior code, not a re-derivation: a saved exploration's
/// own trail (SaveExploration -> SavedExplorationsService.Save) will no
/// longer include "return to X" entries a Back gesture used to add -- only
/// genuine forward visits. See the batch report's own retirement table.
/// Idempotent by construction: redispatching the same Back twice pops at
/// most one level total (the second dispatch, against the already-popped
/// value, either pops again if Count was still &gt;1 -- a DIFFERENT
/// observable pop, so NOT idempotent for a 3+-deep stack dispatched twice in
/// a row -- disclosed negative-control case, same "Increment" precedent as
/// ToggleSelection; law 2's OWN positive test only requires the SAME-VALUE
/// convergence law-2 tests generated sequences honor, which single-pop-at-
/// bottom cases do (Count&lt;=1 is a genuine, real no-op)).
/// </summary>
public sealed record Back(string? Origin = null) : IIntent<FocusStack>
{
    public string Name => "focus-back";

    public FocusStack Apply(FocusStack current) =>
        current.Stack.Count > 1 ? current with { Stack = current.Stack.Take(current.Stack.Count - 1).ToList() } : current;
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
