using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// Batch ST-1/ST-2: correctness of the CONCRETE Locus/TimeWindow/FollowTextLink
/// shapes (client/State/Locus.cs, TimeWindow.cs) -- distinct from the
/// generic law tests (StateAtomLawTests.cs, ConfluenceAgreementLawTests.cs,
/// NoEchoLawTests.cs), which prove the LAWS hold for the infrastructure
/// using synthetic atoms. These re-check the SAME idempotence/agreement
/// obligations specifically for the real seed atoms this batch migrates,
/// plus the SYNC-1/ST-2-relevant shapes (Ref formatting, the NOW-REAL
/// follow-text link, the TimeWindow union's own equality).
///
/// Batch ST-2 replaces the ST-1-era "FollowTextLink is inert" tests below --
/// Derive is now a real, working transform (controller ruling R2) -- with
/// tests proving the REAL behavior instead. Deliverable 2's own ask
/// ("union value-type equality tested explicitly... a hand-written Equals
/// bug here poisons every law") is covered in the TimeWindow union region.
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

    // ========================================================================
    // Batch ST-2 (R1): the TimeWindow union's own equality -- deliverable 2's
    // explicit ask ("prove it, don't assume it"). Records give structural
    // equality for free, but ONLY correctly if the union is genuinely a
    // sealed hierarchy of distinct runtime types (not, say, a base type with
    // an accidentally-shared Equals) -- these pin that it actually is.
    // ========================================================================

    [Fact]
    public void TimeMode_RecordEquality_SameFromToAreEqual()
    {
        Assert.Equal(new TimeMode(-5, 33), new TimeMode(-5, 33));
        Assert.Equal(new TimeMode(-5, 33).GetHashCode(), new TimeMode(-5, 33).GetHashCode());
    }

    [Fact]
    public void TimeMode_RecordEquality_DifferentFromOrToAreNotEqual()
    {
        Assert.NotEqual(new TimeMode(-5, 33), new TimeMode(-5, 34));
        Assert.NotEqual(new TimeMode(-5, 33), new TimeMode(-4, 33));
    }

    [Fact]
    public void ScriptureMode_RecordEquality_SameRefIsEqual()
    {
        Assert.Equal(new ScriptureMode("GEN.1"), new ScriptureMode("GEN.1"));
        Assert.Equal(new ScriptureMode("GEN.1").GetHashCode(), new ScriptureMode("GEN.1").GetHashCode());
    }

    [Fact]
    public void ScriptureMode_RecordEquality_DifferentRefIsNotEqual()
    {
        Assert.NotEqual(new ScriptureMode("GEN.1"), new ScriptureMode("GEN.2"));
    }

    [Fact]
    public void TimeWindow_UnionArms_AreNeverEqualAcrossArms_EvenAtStructuralCoincidence()
    {
        // A TimeMode and a ScriptureMode can never be equal to each other,
        // no matter what values they carry -- proving the hand-written
        // union discipline (a sealed hierarchy, not one record with
        // nullable fields for both arms) actually produces distinct
        // runtime types record equality respects, not merely distinct C#
        // syntax that happens to compile.
        TimeWindow asTimeMode = new TimeMode(1, 1);
        TimeWindow asScriptureMode = new ScriptureMode("1"); // deliberately "coincidental" content
        Assert.NotEqual(asTimeMode, asScriptureMode);
        Assert.False(asTimeMode.Equals(asScriptureMode));
        Assert.False(asScriptureMode.Equals(asTimeMode));
    }

    [Fact]
    public void TimeWindow_Default_IsATimeModeMatchingWorldRazorsOwnGospelsEraDefault()
    {
        Assert.Equal(new TimeMode(-5, 33), TimeWindow.Default);
    }

    [Fact]
    public void SetTimeWindow_Apply_ProducesTheExpectedTimeModeRegardlessOfCurrent()
    {
        var intent = new SetTimeWindow(100, 200);
        Assert.Equal(new TimeMode(100, 200), intent.Apply(TimeWindow.Default));
        Assert.Equal(new TimeMode(100, 200), intent.Apply(new ScriptureMode("GEN.1")));
    }

    [Fact]
    public void SetScriptureWindow_Apply_ProducesTheExpectedScriptureModeRegardlessOfCurrent()
    {
        var intent = new SetScriptureWindow("EXO.3");
        Assert.Equal(new ScriptureMode("EXO.3"), intent.Apply(TimeWindow.Default));
        Assert.Equal(new ScriptureMode("EXO.3"), intent.Apply(new ScriptureMode("GEN.1")));
    }

    // ========================================================================
    // Batch ST-2 (R2): FollowTextLink is now REAL -- Derive computes
    // ScriptureMode(locus.Ref), not identity. These replace ST-1's own
    // "...IsAPureIdentityPassThrough_Disclosed" / "...IsInert_..." tests,
    // which asserted the OLD (correct-for-ST-1) behavior that no longer
    // holds.
    // ========================================================================

    [Theory]
    [InlineData(6101)]
    [InlineData(6102)]
    public void FollowTextLink_Derive_OverAGeneratedLocusSequence_AlwaysProducesScriptureModeOfTheLocusRef(int seed)
    {
        var locusAtom = new StateAtom<Locus>(AtomNamesLocus, Locus.Default);
        var windowAtom = new StateAtom<TimeWindow>(AtomNamesTimeWindow, TimeWindow.Default);
        var link = new FollowTextLink(locusAtom, windowAtom, () => true);

        foreach (var (book, chapter) in Generators.LocusSequence(seed, 25))
        {
            var locus = new Locus(book, chapter);
            var derived = link.Derive(locus, windowAtom.Value);
            Assert.Equal(new ScriptureMode(locus.Ref), derived);
        }
    }

    [Theory]
    [InlineData(6201)]
    [InlineData(6202)]
    public void FollowTextLink_Derive_IsIdempotent_SameLocusTwiceProducesAnEqualScriptureMode(int seed)
    {
        var locusAtom = new StateAtom<Locus>(AtomNamesLocus, Locus.Default);
        var windowAtom = new StateAtom<TimeWindow>(AtomNamesTimeWindow, TimeWindow.Default);
        var link = new FollowTextLink(locusAtom, windowAtom, () => true);

        foreach (var (book, chapter) in Generators.LocusSequence(seed, 20))
        {
            var locus = new Locus(book, chapter);
            var first = link.Derive(locus, windowAtom.Value);
            var second = link.Derive(locus, first); // Apply(Apply(v)) == Apply(v), law 2's per-intent obligation
            Assert.Equal(first, second);
        }
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
    public void FollowTextLink_WiredThroughARunner_ActuallyDerivesAndDispatchesScriptureMode()
    {
        // End-to-end sanity for the ACTUAL wiring World.razor constructs --
        // R2's whole point: the link now DOES real work, no longer the
        // ST-1-era inert scaffolding.
        var locusAtom = new StateAtom<Locus>(AtomNamesLocus, Locus.Default);
        var windowAtom = new StateAtom<TimeWindow>(AtomNamesTimeWindow, TimeWindow.Default);
        var link = new FollowTextLink(locusAtom, windowAtom, () => true);
        using var runner = new StateLinkRunner<Locus, TimeWindow>(link.Name, link, locusAtom, windowAtom);

        var windowChanged = false;
        windowAtom.Changed += () => windowChanged = true;

        locusAtom.Dispatch(new SetLocus("JOS", 6));

        Assert.Equal(new ScriptureMode("JOS.6"), windowAtom.Value);
        Assert.True(windowChanged);
        Assert.Equal(link.Name, windowAtom.LastOrigin); // law 3's own echo tag -- the mechanism World.OnTimeWindowChanged's own gate depends on
    }

    [Fact]
    public void FollowTextLink_WiredThroughARunner_DispatchingTheSameLocusTwiceFiresWindowChangedOnce()
    {
        // Deliverable 2: "idempotence -- locus sequence ⇒ ScriptureMode refs,
        // idempotence (same locus twice ⇒ one Changed)" -- the FULL chain
        // (Locus dispatch -> derive -> TimeWindow dispatch), not just
        // Derive's own per-call idempotence (covered separately above).
        var locusAtom = new StateAtom<Locus>(AtomNamesLocus, Locus.Default);
        var windowAtom = new StateAtom<TimeWindow>(AtomNamesTimeWindow, TimeWindow.Default);
        var link = new FollowTextLink(locusAtom, windowAtom, () => true);
        using var runner = new StateLinkRunner<Locus, TimeWindow>(link.Name, link, locusAtom, windowAtom);

        var windowChangedCount = 0;
        windowAtom.Changed += () => windowChangedCount++;

        locusAtom.Dispatch(new SetLocus("JOS", 6));
        Assert.Equal(1, windowChangedCount);
        Assert.Equal(new ScriptureMode("JOS.6"), windowAtom.Value);

        locusAtom.Dispatch(new SetLocus("JOS", 6)); // a DIFFERENT intent instance, same values
        Assert.Equal(1, windowChangedCount); // still just the one -- Locus itself no-ops the second dispatch (record equality), so the link never even re-derives
    }

    [Fact]
    public void FollowTextLink_NoEcho_ALinkDerivedWriteIntoTimeWindowNeverLoopsBackIntoLocus()
    {
        // "No-echo with the real link" (deliverable 2). Locus -> TimeWindow
        // is the app's only link -- genuinely one-directional, no reverse
        // TimeWindow -> Locus runner exists anywhere. Proven here by
        // counting Locus's OWN Changed firings across a gesture that DOES
        // cause a real, non-trivial TimeWindow write: if the link (or
        // anything else) looped back, this count would exceed the ONE
        // genuine user dispatch.
        var locusAtom = new StateAtom<Locus>(AtomNamesLocus, Locus.Default);
        var windowAtom = new StateAtom<TimeWindow>(AtomNamesTimeWindow, TimeWindow.Default);
        var link = new FollowTextLink(locusAtom, windowAtom, () => true);
        using var runner = new StateLinkRunner<Locus, TimeWindow>(link.Name, link, locusAtom, windowAtom);

        var locusChangedCount = 0;
        locusAtom.Changed += () => locusChangedCount++;

        locusAtom.Dispatch(new SetLocus("JOS", 6)); // real change -> derives into windowAtom

        Assert.Equal(new ScriptureMode("JOS.6"), windowAtom.Value);
        Assert.Equal(1, locusChangedCount); // exactly the one user dispatch -- the link-derived window write never looped back
    }

    [Fact]
    public void FollowTextLink_Agreement_MultipleTimeWindowProjectionsAgreeAfterEveryFollowedLocusChange()
    {
        // Mirrors LocusAtom_Agreement above, one level downstream: while
        // following, TWO independent TimeWindow projections (e.g. a reader
        // pane's own read and a world pane's own read of the SAME atom)
        // must agree after every locus change the link derives from.
        var locusAtom = new StateAtom<Locus>(AtomNamesLocus, Locus.Default);
        var windowAtom = new StateAtom<TimeWindow>(AtomNamesTimeWindow, TimeWindow.Default);
        var link = new FollowTextLink(locusAtom, windowAtom, () => true);
        using var runner = new StateLinkRunner<Locus, TimeWindow>(link.Name, link, locusAtom, windowAtom);
        var readerWindowProjection = new Projection<TimeWindow>(windowAtom);
        var worldWindowProjection = new Projection<TimeWindow>(windowAtom);

        foreach (var (book, chapter) in Generators.LocusSequence(6301, 15))
        {
            locusAtom.Dispatch(new SetLocus(book, chapter));
            Assert.Equal(readerWindowProjection.Value, worldWindowProjection.Value);
            Assert.Equal(new ScriptureMode($"{book}.{chapter}"), readerWindowProjection.Value);
        }
    }

    [Fact]
    public void FollowTextLink_Inactive_LocusChangesNeverTouchTimeWindow()
    {
        // The Active gate itself, over the REAL (non-identity) Derive --
        // ST-1's generic StateLinkRunnerTests.cs already covers Active
        // gating structurally; this re-confirms it specifically for the
        // real production link/derive shape, not just a synthetic one.
        var locusAtom = new StateAtom<Locus>(AtomNamesLocus, Locus.Default);
        var windowAtom = new StateAtom<TimeWindow>(AtomNamesTimeWindow, TimeWindow.Default);
        var link = new FollowTextLink(locusAtom, windowAtom, () => false);
        using var runner = new StateLinkRunner<Locus, TimeWindow>(link.Name, link, locusAtom, windowAtom);

        locusAtom.Dispatch(new SetLocus("JOS", 6));

        Assert.Equal(TimeWindow.Default, windowAtom.Value);
        Assert.Null(windowAtom.LastOrigin);
    }

    // AtomNames.Locus/AtomNames.TimeWindow (Contracts/State.cs) are the
    // canonical names -- referenced by their literal constant values here
    // (rather than a `using static` on the Contracts type) purely to keep
    // each test's own atom construction self-contained and readable.
    private const string AtomNamesLocus = BibleAtlas.Client.Contracts.AtomNames.Locus;
    private const string AtomNamesTimeWindow = BibleAtlas.Client.Contracts.AtomNames.TimeWindow;
}
