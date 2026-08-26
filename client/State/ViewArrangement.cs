using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.State;

/// <summary>
/// Batch ST-2 (controller ruling R4): which of the two pages is showing, and
/// how -- <see cref="AtomNames.ViewArrangement"/>'s value type. A three-arm
/// union: <see cref="ReaderOnly"/> | <see cref="WorldOnly"/> |
/// <see cref="Split"/>. Deliberately encodes "Reader.razor is the split's
/// ONLY ever host" rather than generalizing to a symmetric "either page can
/// host either" shape -- the app has never had a "World hosts an embedded
/// Reader pane" mode, and pretending the type is more general than the app
/// actually is would just invite an unreachable state back in, the exact
/// disease this whole migration exists to retire. There is no standalone
/// "Reader embeds nothing, World embeds nothing, but a split exists anyway"
/// state representable here, on purpose.
/// </summary>
public abstract record ViewArrangement
{
    /// <summary>A fresh session (or a fresh Reader.razor mount with no
    /// ?split=1) starts here -- mirrors Reader.razor's own pre-ST-2
    /// <c>_splitOpen = false</c> field default.</summary>
    public static readonly ViewArrangement Default = new ReaderOnly();
}

/// <summary>Standalone <c>/read/{book}/{chapter}</c> -- no atlas pane.</summary>
public sealed record ReaderOnly : ViewArrangement;

/// <summary>Standalone <c>/world</c> -- no reader pane.</summary>
public sealed record WorldOnly : ViewArrangement;

/// <summary>
/// Reader.razor hosting an embedded World pane. <see cref="Follow"/> is the
/// field World.razor's own <c>_follow</c> used to be (ST-1/pre-ST-2) --
/// migrated INTO this arm per R4 ("_follow migrates INTO the Split arm"),
/// since it is structurally meaningless outside a split (standalone /world
/// never had a follow chip to toggle). <see cref="DividerFraction"/> is the
/// last COMMITTED (drag-end/keyboard-nudge, never a live drag frame -- R4:
/// "DividerFraction commits on drag END only") position of the reader/atlas
/// split divider, as a fraction of the measured `.split-view` container
/// width at the moment it was committed; null until the user has ever
/// actually moved it this session (Reader.razor's own
/// <c>_splitReaderWidthPx</c> field -- UNCHANGED, still the actual render
/// source for the divider's pixel position -- supplies the visual default
/// otherwise; see that field's own comment for why this atom is a
/// write-through mirror for cross-instance observability/law-compliance,
/// not the sole memory of "the current split width" the way it is for
/// Follow/arrangement).
/// </summary>
public sealed record Split(bool Follow, double? DividerFraction) : ViewArrangement;

/// <summary>Standalone reader, no map pane. Idempotent by construction (a
/// fresh, field-less record every call).</summary>
public sealed record EnterReaderOnly(string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "enter-reader-only";

    public ViewArrangement Apply(ViewArrangement current) => new ReaderOnly();
}

/// <summary>Standalone map, no reader pane.</summary>
public sealed record EnterWorldOnly(string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "enter-world-only";

    public ViewArrangement Apply(ViewArrangement current) => new WorldOnly();
}

/// <summary>
/// Enters split. <see cref="DefaultFollow"/> is the caller-supplied starting
/// Follow value for a GENUINELY fresh entry (current is NOT already
/// <see cref="Split"/>) -- Reader.razor's own dispatch sites pass
/// <c>ViewState.Map.Follow</c> here, the SAME restoration source
/// <c>World.RestoreMapState</c> already reads for TimeWindow (ViewStateService
/// remains the PERSISTENCE layer beneath the atom, per spec §4d's own
/// header -- see ViewStateService.cs's own updated comment). If the atom is
/// ALREADY <see cref="Split"/> (a redundant re-open, or a repeat dispatch of
/// this exact intent), <see cref="Apply"/> returns <c>current</c> UNCHANGED
/// -- both for law-2 idempotence and so a redundant open never clobbers a
/// DividerFraction already sitting there.
/// </summary>
public sealed record EnterSplit(bool DefaultFollow, string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "enter-split";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is Split ? current : new Split(DefaultFollow, null);
}

/// <summary>
/// Flips the Split arm's Follow flag, preserving whatever DividerFraction is
/// already there (reads <paramref name="current"/> for exactly that --
/// R4/deliverable: the follow-toggle and the divider-drag are two
/// independent writers of sibling fields on the SAME arm, and neither should
/// need to know the other's latest value to avoid clobbering it). Applying
/// this while current is NOT already Split is defensive-only (no real call
/// site does it) -- falls back to DividerFraction: null rather than
/// throwing, matching this codebase's own "degrade gracefully" house style.
/// </summary>
public sealed record SetSplitFollow(bool Follow, string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "set-split-follow";

    public ViewArrangement Apply(ViewArrangement current) =>
        new Split(Follow, current is Split s ? s.DividerFraction : null);
}

/// <summary>
/// Commits a new DividerFraction, preserving whatever Follow is already
/// there -- the divider-drag side of the same "two independent writers of
/// sibling fields" reasoning <see cref="SetSplitFollow"/>'s own comment
/// gives. R4: "DividerFraction commits on drag END only" -- the ONE caller
/// of this intent (SplitDivider's own commit callback, wired in
/// Reader.razor) never dispatches it from a live drag frame, only once per
/// completed gesture (pointer-up or a keyboard nudge, each already a
/// complete, atomic gesture).
/// </summary>
public sealed record SetSplitDividerFraction(double Fraction, string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "set-split-divider-fraction";

    public ViewArrangement Apply(ViewArrangement current) =>
        new Split(current is Split s ? s.Follow : true, Fraction);
}
