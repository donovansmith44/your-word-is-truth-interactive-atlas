using BibleAtlas.Client.State;
using BibleAtlas.Client.Views;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// Batch VC-1 (controller ruling R2): correctness of the reshaped
/// ViewArrangement value/intents (client/State/ViewArrangement.cs) --
/// SUPERSEDES ST-2's own ViewArrangementTests.cs (the ReaderOnly/WorldOnly/
/// Split three-arm union it tested is deleted wholesale, per the owner
/// ruling "no privileged split host, no privileged guest"). Distinct from
/// the generic law tests, which prove the LAWS hold for the infrastructure
/// using synthetic atoms.
/// </summary>
public class ViewArrangementTests
{
    // ========================================================================
    // Equality -- proven, not assumed (Members compares by SEQUENCE, per
    // this type's own hand-rolled Equals -- same reasoning
    // Selection/FocusStack's own list-valued equality already established).
    // ========================================================================

    [Fact]
    public void Equality_SameMembersOrderLayoutFractionFollow_AreEqual()
    {
        var a = new ViewArrangement(new[] { "reader", "world" }, LayoutKinds.SplitH, 0.5, true);
        var b = new ViewArrangement(new[] { "reader", "world" }, LayoutKinds.SplitH, 0.5, true);

        Assert.Equal(a, b);
        Assert.Equal(a.GetHashCode(), b.GetHashCode());
    }

    [Fact]
    public void Equality_MembersOrderMatters_HostVsGuestIsPositional()
    {
        var reversed = new ViewArrangement(new[] { "world", "reader" }, LayoutKinds.SplitH, null, false);
        var forward = new ViewArrangement(new[] { "reader", "world" }, LayoutKinds.SplitH, null, false);

        Assert.NotEqual(forward, reversed);
    }

    [Fact]
    public void Equality_DifferentLayoutFractionOrFollow_AreNotEqual()
    {
        var baseline = new ViewArrangement(new[] { "reader", "world" }, LayoutKinds.SplitH, 0.5, true);

        Assert.NotEqual(baseline, baseline with { DividerFraction = 0.6 });
        Assert.NotEqual(baseline, baseline with { Follow = false });
        Assert.NotEqual(baseline, baseline with { LayoutKind = LayoutKinds.Single });
    }

    [Fact]
    public void ViewArrangement_Default_IsSingleReader()
    {
        Assert.Equal(new ViewArrangement(new[] { ViewNames.Reader }, LayoutKinds.Single, null, false), ViewArrangement.Default);
    }

    // ========================================================================
    // EnterSingle (R2).
    // ========================================================================

    [Fact]
    public void EnterSingle_Apply_FromSplitH_EntersSingleWithTheNamedMember()
    {
        var current = new ViewArrangement(new[] { "reader", "world" }, LayoutKinds.SplitH, 0.5, true);
        var result = new EnterSingle("world").Apply(current);

        Assert.Equal(new ViewArrangement(new[] { "world" }, LayoutKinds.Single, null, false), result);
    }

    [Fact]
    public void EnterSingle_Apply_WhenAlreadyThatExactSingle_ReturnsCurrentUnchanged()
    {
        var current = new ViewArrangement(new[] { "reader" }, LayoutKinds.Single, null, false);
        var result = new EnterSingle("reader").Apply(current);

        Assert.Same(current, result);
    }

    [Fact]
    public void EnterSingle_Apply_IsIdempotent()
    {
        var intent = new EnterSingle("sources");
        var once = intent.Apply(ViewArrangement.Default);
        var twice = intent.Apply(once);

        Assert.Equal(once, twice);
    }

    // ========================================================================
    // EnterSplit (R2): "host left / guest right," positional, any two
    // distinct names legal -- no privileged host.
    // ========================================================================

    [Fact]
    public void EnterSplit_Apply_FromSingle_EntersSplitHWithHostFirstGuestSecond()
    {
        var intent = new EnterSplit("sources", "reader", DefaultFollow: false, DefaultDividerFraction: 0.6);
        var result = intent.Apply(ViewArrangement.Default);

        Assert.Equal(new ViewArrangement(new[] { "sources", "reader" }, LayoutKinds.SplitH, 0.6, false), result);
    }

    [Fact]
    public void EnterSplit_Apply_AnyTwoDistinctViews_IsLegal_NoPrivilegedHost()
    {
        // The owner's own ruling, proven directly: "world" can host "sources"
        // just as validly as "reader" hosting "world" -- nothing in Apply
        // special-cases any particular name.
        var result = new EnterSplit("world", "sources").Apply(ViewArrangement.Default);

        Assert.Equal(new[] { "world", "sources" }, result.Members);
        Assert.Equal(LayoutKinds.SplitH, result.LayoutKind);
    }

    [Fact]
    public void EnterSplit_Apply_WhenAlreadySplitWithTheSamePairInTheSameOrder_ReturnsCurrentUnchanged_PreservingFractionAndFollow()
    {
        var current = new ViewArrangement(new[] { "reader", "world" }, LayoutKinds.SplitH, 0.42, true);
        var intent = new EnterSplit("reader", "world", DefaultFollow: false, DefaultDividerFraction: 0.9); // different defaults -- ignored

        var result = intent.Apply(current);

        Assert.Same(current, result);
    }

    [Fact]
    public void EnterSplit_Apply_SamePairReversed_IsANewArrangement_NotANoOp()
    {
        var current = new ViewArrangement(new[] { "reader", "world" }, LayoutKinds.SplitH, 0.42, true);
        var intent = new EnterSplit("world", "reader");

        var result = intent.Apply(current);

        Assert.Equal(new[] { "world", "reader" }, result.Members);
        Assert.Null(result.DividerFraction); // a genuinely fresh arrangement, not a preserved one
    }

    [Fact]
    public void EnterSplit_Apply_IsIdempotent()
    {
        var intent = new EnterSplit("reader", "world", DefaultFollow: true, DefaultDividerFraction: 0.5);
        var once = intent.Apply(ViewArrangement.Default);
        var twice = intent.Apply(once);

        Assert.Equal(once, twice);
    }

    // ========================================================================
    // CloseGuest (R2): closes Members[1], returning to single-Members[0].
    // ========================================================================

    [Fact]
    public void CloseGuest_Apply_FromSplitH_ReturnsSingleWithTheHostOnly()
    {
        var current = new ViewArrangement(new[] { "sources", "reader" }, LayoutKinds.SplitH, 0.5, false);
        var result = new CloseGuest().Apply(current);

        Assert.Equal(new ViewArrangement(new[] { "sources" }, LayoutKinds.Single, null, false), result);
    }

    [Fact]
    public void CloseGuest_Apply_WhenNotSplitH_IsANoOp()
    {
        var single = ViewArrangement.Default;
        Assert.Same(single, new CloseGuest().Apply(single));
    }

    // ========================================================================
    // SetDivider / ToggleFollow (R2): independent sibling writers, neither
    // clobbers the other; no-op off split-h.
    // ========================================================================

    [Fact]
    public void SetDivider_Apply_CommitsFractionAndPreservesFollow()
    {
        var current = new ViewArrangement(new[] { "reader", "world" }, LayoutKinds.SplitH, 0.3, true);
        var result = new SetDivider(0.6).Apply(current);

        Assert.Equal(new ViewArrangement(new[] { "reader", "world" }, LayoutKinds.SplitH, 0.6, true), result);
    }

    [Fact]
    public void SetDivider_Apply_WhenNotSplitH_IsANoOp()
    {
        var single = ViewArrangement.Default;
        Assert.Same(single, new SetDivider(0.6).Apply(single));
    }

    [Fact]
    public void ToggleFollow_Apply_FlipsFollowAndPreservesDividerFraction()
    {
        var current = new ViewArrangement(new[] { "reader", "world" }, LayoutKinds.SplitH, 0.3, false);
        var result = new ToggleFollow(true).Apply(current);

        Assert.Equal(new ViewArrangement(new[] { "reader", "world" }, LayoutKinds.SplitH, 0.3, true), result);
    }

    [Fact]
    public void ToggleFollow_Apply_WhenNotSplitH_IsANoOp()
    {
        var single = ViewArrangement.Default;
        Assert.Same(single, new ToggleFollow(true).Apply(single));
    }

    [Fact]
    public void SetDivider_And_ToggleFollow_AreIndependentWriters_NeitherClobbersTheOther()
    {
        var atom = new StateAtom<ViewArrangement>(BibleAtlas.Client.Contracts.AtomNames.ViewArrangement, new ViewArrangement(new[] { "reader", "world" }, LayoutKinds.SplitH, null, true));

        atom.Dispatch(new SetDivider(0.4));
        Assert.Equal(0.4, atom.Value.DividerFraction);
        Assert.True(atom.Value.Follow);

        atom.Dispatch(new ToggleFollow(false));
        Assert.Equal(0.4, atom.Value.DividerFraction); // fraction survived the follow flip
        Assert.False(atom.Value.Follow);

        atom.Dispatch(new SetDivider(0.7));
        Assert.Equal(0.7, atom.Value.DividerFraction);
        Assert.False(atom.Value.Follow); // follow survived the fraction commit
    }

    // ========================================================================
    // R2: "Persistence (ViewState demotion from ST-2) migrates losslessly --
    // a saved ReaderOnly/Split restores to the equivalent Arrangement (cold-
    // start compatibility test)." ViewArrangement itself was never
    // localStorage-persisted (in-memory atom, seeded from
    // ViewArrangement.Default -- see AppServices.AddStateAtoms); the ACTUAL
    // cross-session persistence for split state is the ?split=1 URL query
    // Reader.razor's own SplitQuery param consumes, plus ViewState.Map's own
    // Follow/DividerFraction restoration source. This test proves the
    // MAPPING every pre-VC-1 shape had onto the reshaped type is exactly
    // what R2's own worked examples specify, verbatim.
    // ========================================================================

    [Fact]
    public void ColdStart_PreVC1ReaderOnly_MapsToSingleReader()
    {
        // R2, verbatim: "ReaderOnly => Arrangement(["reader"], "single")."
        Assert.Equal(new ViewArrangement(new[] { "reader" }, "single", null, false), ViewArrangement.Default);
    }

    [Fact]
    public void ColdStart_PreVC1SplitReaderWorld_MapsToSplitHReaderWorld()
    {
        // R2, verbatim: "today's split => Arrangement(["reader","world"],
        // "split-h", ...)." -- exercised via the SAME dispatch path
        // Reader.razor's own ?split=1 consumption (OnParametersSetAsync) and
        // World's own OpenReadBesideMap hatch both use: EnterSplit(reader,
        // world, defaultFollow, defaultDividerFraction), landing on a fresh
        // atom (the cold-start case -- no prior session).
        var atom = new StateAtom<ViewArrangement>(BibleAtlas.Client.Contracts.AtomNames.ViewArrangement, ViewArrangement.Default);

        atom.Dispatch(new EnterSplit(ViewNames.Reader, ViewNames.World, DefaultFollow: true, DefaultDividerFraction: 0.55));

        Assert.Equal(new ViewArrangement(new[] { "reader", "world" }, "split-h", 0.55, true), atom.Value);
    }
}
