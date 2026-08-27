using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Views;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch VC-1 (owner order, verbatim: "your reader world split interface is
/// too specific we should support split for any view. No privileged split
/// host, no privileged guest."). SUPERSEDES ST-2's Reader-only-host
/// three-arm union (<c>ReaderOnly</c>/<c>WorldOnly</c>/<c>Split</c>, deleted
/// wholesale) with the controller's own R2 shape: an ORDERED list of view
/// NAMES (<see cref="Views.ViewNames"/>, extend-only vocabulary like
/// <see cref="AtomNames"/>) plus a LAYOUT KIND (<see cref="LayoutKinds"/>)
/// plus the two fields that are only ever meaningful on a two-member
/// split-h pair (<see cref="DividerFraction"/>, <see cref="Follow"/>).
///
/// NAMING (disclosed, deliberate): the C# type keeps the name
/// <c>ViewArrangement</c> -- matching <see cref="AtomNames.ViewArrangement"/>
/// and every existing <c>StateAtom&lt;ViewArrangement&gt;</c> injection site
/// verbatim -- even though R2's own prose calls the reshaped value
/// "Arrangement(...)". R2 is illustrative record-constructor shorthand in
/// prose, not a mandated C# identifier; keeping the existing type name is
/// what let every <c>@inject StateAtom&lt;ViewArrangement&gt;</c> line in
/// Reader.razor/World.razor/Sources.razor go UNTOUCHED, a real risk
/// reduction in the highest-blast-radius files this batch touches.
///
/// EQUALITY: hand-rolled (not the compiler-generated per-property one a
/// plain positional record would get) -- <see cref="Members"/> is an
/// <c>IReadOnlyList&lt;string&gt;</c>, and reference/list identity is not
/// the right notion of "same arrangement" (the same house discipline
/// Selection/FocusStack already established for a list-valued atom). Two
/// arrangements are equal iff Members compares by SEQUENCE (order matters
/// -- host vs. guest is positional, R2's own "ordered Members"), plus
/// LayoutKind/DividerFraction/Follow by value.
/// </summary>
public sealed record ViewArrangement(IReadOnlyList<string> Members, string LayoutKind, double? DividerFraction, bool Follow)
{
    /// <summary>A fresh session starts single-reader -- byte-identical
    /// starting state to ST-2's own <c>ReaderOnly</c> default.</summary>
    public static readonly ViewArrangement Default = new(new[] { ViewNames.Reader }, LayoutKinds.Single, null, false);

    /// <summary>Batch CORPREAD-1a, DELIVERABLE 0b / SPLIT-5050 (owner order,
    /// verbatim: "by default whatever opens up to the right when you split
    /// screen is way bigger. make it so that it's 50/50 by default"). The
    /// ONE named constant a no-stored-<see cref="DividerFraction"/> initial
    /// width computation reads -- <c>CompositionSplit.razor</c>'s own
    /// <c>OnAfterRenderAsync</c> is the sole production site (grep-able;
    /// conformance-tripwired, <c>ConformanceTests.cs</c>'s own
    /// <c>DefaultDividerFraction_ReferencedFromExactlyOneProductionSite</c>)
    /// -- never a bare <c>0.5</c> literal, and applies through
    /// <c>CompositionSplit</c> to EVERY pairing (no per-pair tuning, per the
    /// ticket's own text) since that component is the ONE place any host's
    /// divider ever restores its width from.</summary>
    public const double DefaultDividerFraction = 0.5;

    public bool Equals(ViewArrangement? other) =>
        other is not null
        && Members.SequenceEqual(other.Members)
        && LayoutKind == other.LayoutKind
        && DividerFraction == other.DividerFraction
        && Follow == other.Follow;

    public override int GetHashCode()
    {
        var hash = new HashCode();
        foreach (var m in Members)
        {
            hash.Add(m);
        }

        hash.Add(LayoutKind);
        hash.Add(DividerFraction);
        hash.Add(Follow);
        return hash.ToHashCode();
    }
}

/// <summary>R2: the closed, extend-only layout-kind vocabulary. Fix round 2
/// (N-6, trivia -- re-review, doc rot): <c>CompositionHost</c> is RETIRED
/// (fix round 1) -- <c>CompositionSplit</c> is the component that meets an
/// unrecognized value and fails loud (R6) rather than silently rendering
/// nothing; see that component's own header.</summary>
public static class LayoutKinds
{
    public const string Single = "single";
    public const string SplitH = "split-h";

    /// <summary>The closed vocabulary, as data -- <c>CompositionSplit.razor</c>'s
    /// own "unknown kind fails loud" branch (R6) reads THIS, not a hand-rolled
    /// `!= Single &amp;&amp; != SplitH` inline check, so client.Tests can prove
    /// the vocabulary check directly (IsKnown("single")/IsKnown("split-h") are
    /// true, an arbitrary string is false) without rendering Razor at all.</summary>
    public static readonly IReadOnlyList<string> All = new[] { Single, SplitH };

    public static bool IsKnown(string kind) => All.Contains(kind);
}

/// <summary>
/// R2: "single member full page." Idempotent by construction: a redundant
/// re-entry into the SAME single member (already <see cref="LayoutKinds.Single"/>
/// with this exact name as the one Member) returns <c>current</c> unchanged
/// -- both for law 2 and so a redundant dispatch never clobbers state a
/// richer arrangement had no reason to lose (mirrors ST-2's own
/// <c>EnterSplit</c> no-op-when-already-there reasoning).
/// </summary>
public sealed record EnterSingle(string ViewName, string? Origin = null) : IIntent<ViewArrangement>
{
    // Explicit interface implementation, deliberately -- ViewName (the
    // PAYLOAD: which view becomes the single member) and the intent's own
    // stable vocabulary name ("enter-single", the test/telemetry constant
    // every other intent record's Name property returns) are two genuinely
    // different strings; a positional property literally named "Name" would
    // have collided with (and shadowed the meaning of) IIntent<T>.Name.
    string IIntent<ViewArrangement>.Name => "enter-single";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is { LayoutKind: LayoutKinds.Single } a && a.Members.Count == 1 && a.Members[0] == ViewName
            ? current
            : new ViewArrangement(new[] { ViewName }, LayoutKinds.Single, null, false);
}

/// <summary>
/// R2: "split-h = host left / guest right." <paramref name="Host"/> becomes
/// <c>Members[0]</c>, <paramref name="Guest"/> becomes <c>Members[1]</c> --
/// positional, not privileged (any two distinct view names are legal here;
/// there is no compiled notion of "the" host view any more, per the owner's
/// own ruling). Idempotent: re-entering the SAME (Host, Guest) PAIR, in the
/// SAME order, while already split that way, is a no-op that preserves
/// whatever DividerFraction/Follow are already there (mirrors ST-2's own
/// <c>Split</c> no-op-when-already-there reasoning verbatim) -- a genuinely
/// NEW pairing (different host, different guest, or the same pair
/// reversed) always starts a FRESH arrangement from the caller-supplied
/// defaults.
/// </summary>
public sealed record EnterSplit(string Host, string Guest, bool DefaultFollow = false, double? DefaultDividerFraction = null, string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "enter-split";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is { LayoutKind: LayoutKinds.SplitH } a && a.Members.Count == 2 && a.Members[0] == Host && a.Members[1] == Guest
            ? current
            : new ViewArrangement(new[] { Host, Guest }, LayoutKinds.SplitH, DefaultDividerFraction, DefaultFollow);
}

/// <summary>
/// R2: closes the GUEST (<c>Members[1]</c>) of a split-h pair, returning to
/// single-<c>Members[0]</c> -- the generalization of World's own
/// "split-close-atlas"/Reader's own OnRequestClose-driven close (whichever
/// view is CURRENTLY playing guest closes itself this way). A member
/// wanting to close ITSELF and cede the arrangement to its OWN partner (the
/// host's own self-close, e.g. Reader's "close the reader, keep the map")
/// is not a separate intent -- it is just <see cref="EnterSingle"/> dispatched
/// directly with the partner's name, already fully general. No-op (returns
/// <c>current</c> unchanged) when the arrangement is not currently split-h.
/// </summary>
public sealed record CloseGuest(string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "close-guest";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is { LayoutKind: LayoutKinds.SplitH } a
            ? new ViewArrangement(new[] { a.Members[0] }, LayoutKinds.Single, null, false)
            : current;
}

/// <summary>
/// R2: commits a new divider fraction on the split-h pair, preserving
/// whatever Follow is already there (the two are independent sibling
/// writers -- same "two independent writers of sibling fields" reasoning
/// ST-2's own <c>SetSplitDividerFraction</c> used). "Commits on drag END
/// only" (R4/ST-2, unchanged discipline) -- the one caller
/// (<c>CompositionSplit</c>'s own <c>SplitDivider.OnCommitted</c> wiring,
/// fix round 1 -- N-6 doc rot fix, fix round 2: this used to name the
/// retired <c>CompositionHost</c>) never dispatches this from a live drag
/// frame. No-op off a split-h
/// arrangement (same correctness-tightening ST-2's own fix round already
/// established for the predecessor intent -- no reachable call site ever
/// exercises the no-op branch, SplitDivider only exists in the DOM while
/// already split-h).
/// </summary>
public sealed record SetDivider(double Fraction, string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "set-divider";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is { LayoutKind: LayoutKinds.SplitH } a ? a with { DividerFraction = Fraction } : current;
}

/// <summary>
/// R2: flips Follow on the split-h pair, preserving whatever DividerFraction
/// is already there. Meaningful only when a BearsLocus member and a
/// BearsWindow member both sit in the pair (R5 -- gated by capability at the
/// READ sites, e.g. <c>World.razor</c>'s own <c>_follow</c>/
/// <c>FollowTextLink.Active</c>, never here: this intent stays a pure,
/// unconditional field-set so a caller that dispatches it against a pairing
/// with no BearsWindow member -- e.g. sources+reader -- still gets a
/// law-abiding, inert write; R5's own "degrade cleanly" is a READER-side
/// property, not a WRITER-side refusal). No-op off a split-h arrangement,
/// same reasoning as <see cref="SetDivider"/>.
/// </summary>
public sealed record ToggleFollow(bool Follow, string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "toggle-follow";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is { LayoutKind: LayoutKinds.SplitH } a ? a with { Follow = Follow } : current;
}
