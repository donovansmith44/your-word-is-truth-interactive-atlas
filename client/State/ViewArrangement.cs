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
/// actually moved it this session.
///
/// Fix round 1 (S-3/Q-2, review): this is now a REAL reader, not
/// write-only state. <c>Reader.OnAfterRenderAsync</c> reads it back on
/// every split-open transition to seed <c>_splitReaderWidthPx</c> (the
/// actual render source, still the live-drag mechanism, still updated on
/// every pointermove) via a live remeasurement of `.split-view`'s own
/// width. <see cref="EnterSplit"/>'s own <c>DefaultDividerFraction</c> is
/// what lets this survive a genuine cross-remount (the arm itself cannot --
/// see that intent's own doc comment for the indirection this requires,
/// the same one <c>Follow</c> already relies on via
/// <c>ViewState.Map.Follow</c>).
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
/// Enters split. <see cref="DefaultFollow"/>/<see cref="DefaultDividerFraction"/>
/// are the caller-supplied starting values for a GENUINELY fresh entry
/// (current is NOT already <see cref="Split"/>) -- Reader.razor's own
/// dispatch sites pass <c>ViewState.Map.Follow</c>/<c>ViewState.Map.DividerFraction</c>
/// here, the SAME restoration source <c>World.RestoreMapState</c> already
/// reads for TimeWindow (ViewStateService remains the PERSISTENCE layer
/// beneath the atom, per spec §4d's own header -- see
/// <c>MapViewState.DividerFraction</c>'s own doc comment). This indirection
/// is WHY <see cref="Split.DividerFraction"/> can survive a genuine
/// cross-remount despite the arm itself having no memory across a
/// <see cref="ReaderOnly"/>/<see cref="WorldOnly"/> detour (fix round 1,
/// Adjudication C): a FRESH <see cref="Split"/> is re-seeded from
/// ViewState at the moment of creation, exactly the same mechanism
/// <see cref="Follow"/> already used before this fix round -- the atom
/// read site (<c>Reader.OnAfterRenderAsync</c>) genuinely reads the atom;
/// the atom's own root value, for a cross-remount specifically, is
/// ViewState. If the atom is ALREADY <see cref="Split"/> (a redundant
/// re-open, or a repeat dispatch of this exact intent), <see cref="Apply"/>
/// returns <c>current</c> UNCHANGED -- both for law-2 idempotence and so a
/// redundant open never clobbers a DividerFraction already sitting there.
/// </summary>
public sealed record EnterSplit(bool DefaultFollow, double? DefaultDividerFraction, string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "enter-split";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is Split ? current : new Split(DefaultFollow, DefaultDividerFraction);
}

/// <summary>
/// Flips the Split arm's Follow flag, preserving whatever DividerFraction is
/// already there (reads <paramref name="current"/> for exactly that --
/// R4/deliverable: the follow-toggle and the divider-drag are two
/// independent writers of sibling fields on the SAME arm, and neither should
/// need to know the other's latest value to avoid clobbering it).
///
/// Fix round 1 (Q-1, review): applying this while current is NOT already
/// <see cref="Split"/> is now a NO-OP (returns <c>current</c> unchanged),
/// not a fabricated <see cref="Split"/> arm -- the ORIGINAL "degrade
/// gracefully, default DividerFraction to null" behavior legalized a
/// dispatchable transition into split FROM a page not hosting one, in a
/// union whose whole point is that illegal states are unrepresentable. No
/// real call site ever exercises this branch (<c>ToggleFollow</c>/
/// <c>ApplyExternalQuery</c> are both SplitMode-only) -- the no-op is
/// purely a correctness tightening, not a behavior change for any reachable
/// path.
/// </summary>
public sealed record SetSplitFollow(bool Follow, string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "set-split-follow";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is Split s ? new Split(Follow, s.DividerFraction) : current;
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
///
/// Fix round 1 (Q-1, review): same no-op-off-arm fix as
/// <see cref="SetSplitFollow"/>'s own doc comment describes, for the same
/// reason. `SplitDivider` only exists in the DOM while `_splitOpen` (i.e.
/// the atom already reads `Split`), so this is likewise unreachable for
/// any real call site today.
/// </summary>
public sealed record SetSplitDividerFraction(double Fraction, string? Origin = null) : IIntent<ViewArrangement>
{
    public string Name => "set-split-divider-fraction";

    public ViewArrangement Apply(ViewArrangement current) =>
        current is Split s ? new Split(s.Follow, Fraction) : current;
}
