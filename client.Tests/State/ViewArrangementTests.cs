using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// Batch ST-2 (controller ruling R4): correctness of the concrete
/// ViewArrangement union/intents (client/State/ViewArrangement.cs) --
/// distinct from the generic law tests, which prove the LAWS hold for the
/// infrastructure using synthetic atoms. Deliverable 2's "union value-type
/// equality tested explicitly" applies here too, alongside the union's own
/// (R4-specific) intents.
/// </summary>
public class ViewArrangementTests
{
    // ========================================================================
    // Union equality -- proven, not assumed (same reasoning as TimeWindow's
    // own union tests in LocusAndTimeWindowTests.cs).
    // ========================================================================

    [Fact]
    public void ReaderOnly_And_WorldOnly_AreEqualToThemselvesAndUnequalToEachOther()
    {
        Assert.Equal(new ReaderOnly(), new ReaderOnly());
        Assert.Equal(new WorldOnly(), new WorldOnly());
        Assert.NotEqual<ViewArrangement>(new ReaderOnly(), new WorldOnly());
    }

    [Fact]
    public void Split_RecordEquality_SameFollowAndFractionAreEqual()
    {
        Assert.Equal(new Split(true, 0.5), new Split(true, 0.5));
        Assert.Equal(new Split(true, null), new Split(true, null));
        Assert.Equal(new Split(true, 0.5).GetHashCode(), new Split(true, 0.5).GetHashCode());
    }

    [Fact]
    public void Split_RecordEquality_DifferentFollowOrFractionAreNotEqual()
    {
        Assert.NotEqual(new Split(true, 0.5), new Split(false, 0.5));
        Assert.NotEqual(new Split(true, 0.5), new Split(true, 0.6));
        Assert.NotEqual(new Split(true, 0.5), new Split(true, null));
    }

    [Fact]
    public void ViewArrangement_UnionArms_AreNeverEqualAcrossArms()
    {
        ViewArrangement readerOnly = new ReaderOnly();
        ViewArrangement worldOnly = new WorldOnly();
        ViewArrangement split = new Split(true, null);

        Assert.False(readerOnly.Equals(worldOnly));
        Assert.False(readerOnly.Equals(split));
        Assert.False(worldOnly.Equals(split));
    }

    [Fact]
    public void ViewArrangement_Default_IsReaderOnly()
    {
        Assert.Equal(new ReaderOnly(), ViewArrangement.Default);
    }

    // ========================================================================
    // EnterSplit (R4): idempotent, preserves an already-Split arm untouched.
    // ========================================================================

    [Fact]
    public void EnterSplit_Apply_FromReaderOnly_EntersSplitWithTheSuppliedDefaultFollowAndNoFraction()
    {
        var intent = new EnterSplit(DefaultFollow: true);
        Assert.Equal(new Split(true, null), intent.Apply(new ReaderOnly()));

        var intentFalse = new EnterSplit(DefaultFollow: false);
        Assert.Equal(new Split(false, null), intentFalse.Apply(new WorldOnly()));
    }

    [Fact]
    public void EnterSplit_Apply_WhenAlreadySplit_ReturnsCurrentUnchanged_PreservingAnyExistingFraction()
    {
        var current = new Split(false, 0.42);
        var intent = new EnterSplit(DefaultFollow: true); // a different DefaultFollow -- still ignored, current wins

        var result = intent.Apply(current);

        Assert.Same(current, result); // literally the same reference -- a true no-op, not just an equal value
    }

    [Fact]
    public void EnterSplit_Apply_IsIdempotent_ReapplyingProducesTheSameValue()
    {
        var intent = new EnterSplit(DefaultFollow: true);
        var once = intent.Apply(new ReaderOnly());
        var twice = intent.Apply(once);

        Assert.Equal(once, twice);
    }

    // ========================================================================
    // SetSplitFollow / SetSplitDividerFraction (R4): each preserves the
    // sibling field on the Split arm without needing to know its value.
    // ========================================================================

    [Fact]
    public void SetSplitFollow_Apply_FlipsFollowAndPreservesDividerFraction()
    {
        var current = new Split(false, 0.3);
        var intent = new SetSplitFollow(true);

        Assert.Equal(new Split(true, 0.3), intent.Apply(current));
    }

    [Fact]
    public void SetSplitFollow_Apply_WhenNotAlreadySplit_DegradesGracefullyWithNoFraction()
    {
        var intent = new SetSplitFollow(true);
        Assert.Equal(new Split(true, null), intent.Apply(new ReaderOnly()));
    }

    [Fact]
    public void SetSplitDividerFraction_Apply_CommitsFractionAndPreservesFollow()
    {
        var current = new Split(true, 0.3);
        var intent = new SetSplitDividerFraction(0.6);

        Assert.Equal(new Split(true, 0.6), intent.Apply(current));
    }

    [Fact]
    public void SetSplitDividerFraction_Apply_WhenNotAlreadySplit_DegradesGracefullyWithFollowTrue()
    {
        var intent = new SetSplitDividerFraction(0.6);
        Assert.Equal(new Split(true, 0.6), intent.Apply(new WorldOnly()));
    }

    [Fact]
    public void SetSplitFollow_And_SetSplitDividerFraction_AreIndependentWriters_NeitherClobbersTheOther()
    {
        // The exact scenario both intents' own doc comments describe: a
        // follow-toggle and a divider-drag, dispatched in either order,
        // never need to know each other's latest value to avoid clobbering
        // it.
        var atom = new StateAtom<ViewArrangement>(AtomNamesViewArrangement, new Split(true, null));

        atom.Dispatch(new SetSplitDividerFraction(0.4));
        Assert.Equal(new Split(true, 0.4), atom.Value);

        atom.Dispatch(new SetSplitFollow(false));
        Assert.Equal(new Split(false, 0.4), atom.Value); // fraction survived the follow flip

        atom.Dispatch(new SetSplitDividerFraction(0.7));
        Assert.Equal(new Split(false, 0.7), atom.Value); // follow survived the fraction commit
    }

    // ========================================================================
    // Idempotence (law 2) over StateAtom.Dispatch, and agreement (law 5)
    // over multiple projections -- the SAME obligations LocusAndTimeWindowTests.cs
    // proves for Locus/TimeWindow, here for the real ViewArrangement atom.
    // ========================================================================

    [Fact]
    public void ViewArrangementAtom_Idempotence_RedispatchingEnterReaderOnlyIsANoOpTheSecondTime()
    {
        var atom = new StateAtom<ViewArrangement>(AtomNamesViewArrangement, new Split(true, 0.5));
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        atom.Dispatch(new EnterReaderOnly());
        Assert.Equal(1, changedCount);

        atom.Dispatch(new EnterReaderOnly()); // a DIFFERENT intent instance, same shape
        Assert.Equal(1, changedCount); // no second Changed
    }

    [Fact]
    public void ViewArrangementAtom_Agreement_MultipleProjectionsAgreeAfterEveryDispatch()
    {
        var atom = new StateAtom<ViewArrangement>(AtomNamesViewArrangement, ViewArrangement.Default);
        var readerProjection = new Projection<ViewArrangement>(atom);
        var worldProjection = new Projection<ViewArrangement>(atom);

        atom.Dispatch(new EnterSplit(true));
        Assert.Equal(readerProjection.Value, worldProjection.Value);

        atom.Dispatch(new SetSplitDividerFraction(0.55));
        Assert.Equal(readerProjection.Value, worldProjection.Value);
        Assert.Equal(new Split(true, 0.55), readerProjection.Value);

        atom.Dispatch(new SetSplitFollow(false));
        Assert.Equal(readerProjection.Value, worldProjection.Value);

        atom.Dispatch(new EnterReaderOnly());
        Assert.Equal(readerProjection.Value, worldProjection.Value);
        Assert.Equal(new ReaderOnly(), readerProjection.Value);
    }

    private const string AtomNamesViewArrangement = BibleAtlas.Client.Contracts.AtomNames.ViewArrangement;
}
