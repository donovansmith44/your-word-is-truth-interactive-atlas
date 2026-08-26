using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// Batch ST-1: correctness of the CONCRETE Locus/TimeWindow/FollowTextLink
/// shapes (client/State/Locus.cs, TimeWindow.cs) -- distinct from the
/// generic law tests (StateAtomLawTests.cs, ConfluenceAgreementLawTests.cs,
/// NoEchoLawTests.cs), which prove the LAWS hold for the infrastructure
/// using synthetic atoms. These re-check the SAME idempotence/agreement
/// obligations specifically for the real seed atom this batch migrates, plus
/// the SYNC-1-relevant shapes (Ref formatting, the follow-text link).
/// </summary>
public class LocusAndTimeWindowTests
{
    [Fact]
    public void SetLocus_Apply_ProducesTheExpectedLocusRegardlessOfCurrent()
    {
        var intent = new SetLocus("EXO", 3);
        Assert.Equal(new Locus("EXO", 3), intent.Apply(Locus.Default));
        Assert.Equal(new Locus("EXO", 3), intent.Apply(new Locus("REV", 22)));
    }

    [Fact]
    public void Locus_Ref_FormatsAsBookDotChapter()
    {
        Assert.Equal("GEN.1", Locus.Default.Ref);
        Assert.Equal("JOS.6", new Locus("JOS", 6).Ref);
    }

    [Theory]
    [InlineData(5001)]
    [InlineData(5002)]
    public void LocusAtom_Idempotence_DispatchingTheSameBookChapterTwiceRaisesChangedOnce(int seed)
    {
        var sequence = Generators.LocusSequence(seed, 25);
        var atom = new StateAtom<Locus>(AtomNamesLocus, Locus.Default);
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        foreach (var (book, chapter) in sequence)
        {
            var intent = new SetLocus(book, chapter);
            atom.Dispatch(intent);
            var afterFirst = atom.Value;
            var changedAfterFirst = changedCount;

            atom.Dispatch(new SetLocus(book, chapter)); // a DIFFERENT instance, same values -- still idempotent (record equality)

            Assert.Equal(afterFirst, atom.Value);
            Assert.Equal(changedAfterFirst, changedCount);
        }
    }

    [Fact]
    public void LocusAtom_Agreement_MultipleProjectionsAgreeAfterEveryNavigation()
    {
        var atom = new StateAtom<Locus>(AtomNamesLocus, Locus.Default);
        var readerProjection = new Projection<Locus>(atom);
        var worldProjection = new Projection<Locus>(atom);

        // The SYNC-1 shape itself: a "reader picker" and a "world picker"
        // both reading from ONE atom.
        foreach (var (book, chapter) in Generators.LocusSequence(5010, 15))
        {
            atom.Dispatch(new SetLocus(book, chapter));
            Assert.Equal(readerProjection.Value, worldProjection.Value);
            Assert.Equal(new Locus(book, chapter), readerProjection.Value);
        }
    }

    [Fact]
    public void FollowTextLink_Derive_IsAPureIdentityPassThrough_Disclosed()
    {
        // Disclosed design decision (TimeWindow.cs's own header comment):
        // this batch's Derive never computes a real numeric window from a
        // Locus (that needs a network fetch, and Derive must be pure) -- it
        // passes `current` through unchanged. Pinned here so a future batch
        // that upgrades this Derive does so deliberately, not by accident.
        var locusAtom = new StateAtom<Locus>(AtomNamesLocus, Locus.Default);
        var windowAtom = new StateAtom<TimeWindow>(AtomNamesTimeWindow, TimeWindow.Default);
        var link = new FollowTextLink(locusAtom, windowAtom, () => true);

        var current = new TimeWindow(100, 200);
        var derived = link.Derive(new Locus("JOS", 6), current);

        Assert.Equal(current, derived);
        Assert.Same(current, derived); // literally the same reference -- true identity, not just equal
    }

    [Fact]
    public void FollowTextLink_Active_ReflectsTheSuppliedDelegateLive()
    {
        var locusAtom = new StateAtom<Locus>(AtomNamesLocus, Locus.Default);
        var windowAtom = new StateAtom<TimeWindow>(AtomNamesTimeWindow, TimeWindow.Default);
        var active = false;
        var link = new FollowTextLink(locusAtom, windowAtom, () => active);

        Assert.False(link.Active);
        active = true;
        Assert.True(link.Active); // re-read live, not captured at construction -- mirrors the follow chip's own mutable bool
    }

    [Fact]
    public void FollowTextLink_WiredThroughARunner_IsInert_NeverChangesValueNeverFiresChangedNeverStampsLastOrigin()
    {
        // End-to-end sanity for the ACTUAL wiring World.razor constructs, and
        // the direct test of review Adjudication 2's finding: because Derive
        // is identity, the runner's own dispatch
        // (LinkDerivedIntent(name, current)) is REJECTED by StateAtom.Dispatch's
        // equality check BEFORE it reaches the Value assignment, the
        // LastOrigin assignment, or the Changed invocation (StateAtom.cs) --
        // so this link is inert at runtime on all three fronts, not just the
        // Value one. Fix round 1 (Q-2): the ORIGINAL name/doc comment here
        // claimed LastOrigin WAS stamped -- false; Dispatch returns before
        // ever reaching that line when the intent's result already equals
        // the current value, which the identity Derive guarantees is always
        // the case here.
        var locusAtom = new StateAtom<Locus>(AtomNamesLocus, Locus.Default);
        var windowAtom = new StateAtom<TimeWindow>(AtomNamesTimeWindow, TimeWindow.Default);
        var link = new FollowTextLink(locusAtom, windowAtom, () => true);
        using var runner = new StateLinkRunner<Locus, TimeWindow>(link.Name, link, locusAtom, windowAtom);

        var windowChanged = false;
        windowAtom.Changed += () => windowChanged = true;

        locusAtom.Dispatch(new SetLocus("JOS", 6));

        Assert.Equal(TimeWindow.Default, windowAtom.Value); // identity Derive -- never actually moves
        Assert.False(windowChanged); // Changed correctly never fires (law 2's own no-op guard)
        Assert.Null(windowAtom.LastOrigin); // LastOrigin is correctly NEVER stamped -- Dispatch returns before that line
    }

    // AtomNames.Locus/AtomNames.TimeWindow (Contracts/State.cs) are the
    // canonical names -- referenced by their literal constant values here
    // (rather than a `using static` on the Contracts type) purely to keep
    // each test's own atom construction self-contained and readable.
    private const string AtomNamesLocus = BibleAtlas.Client.Contracts.AtomNames.Locus;
    private const string AtomNamesTimeWindow = BibleAtlas.Client.Contracts.AtomNames.TimeWindow;
}
