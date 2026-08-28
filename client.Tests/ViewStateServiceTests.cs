namespace BibleAtlas.Client.Tests;

// Batch CORPREAD-2 fix round 3 (N-1, IMPORTANT -- re-review): direct,
// isolated proof of the ref-counted multiset's own bookkeeping
// (ViewStateService.MountReaderChapter/UnmountReaderChapter/
// IsReaderChapterMounted) under the EXACT sequence the re-review traced
// concretely -- split view, Following, both Reader.razor and
// Kretzmann.razor mounted on the SAME chapter (count 2); one instance
// relinquishes its own credit while the sibling's legitimate membership
// must survive, and never a second time for the same relinquished credit.
//
// DISCLOSED SCOPE: this project carries no Razor-component test harness
// (bUnit or equivalent -- see BibleAtlas.Client.Tests.csproj, plain xUnit
// over standalone classes only, the same discipline VerseLineTests.cs's
// own header already documents for VerseLine.razor's static helpers), so
// Reader.razor's/Kretzmann.razor's own PRIVATE _loadedBook/_loadedChapterNum
// fields and their LoadChapter(Async)'s own null-out (the actual N-1 fix,
// clearing both fields in the same breath as the top-of-method unmount so
// a failed fetch can never leave a stale, already-spent credit behind) are
// NOT unit-reachable. That half of the fix is verified by direct code read
// (both files' own diff, and the byte-identical placement/comment in each)
// and the full Playwright suite staying green, not by an automated test
// exercising the razor fields themselves.
public class ViewStateServiceTests
{
    [Fact]
    public void UnmountingOnceLeavesASiblingsLegitimateShareMounted()
    {
        var vs = new ViewStateService();

        // Split, Following: Reader.razor AND Kretzmann.razor both mount
        // GEN.1 on their own successful load -- count 2.
        vs.MountReaderChapter("GEN", 1);
        vs.MountReaderChapter("GEN", 1);
        Assert.True(vs.IsReaderChapterMounted("GEN", 1));

        // Kretzmann navigates away (or its fetch to a NEW chapter fails) --
        // its own top-of-method unmount relinquishes ITS credit, exactly
        // once (the N-1 fix: _loadedBook/_loadedChapterNum are nulled in
        // the same breath, so nothing later re-reads this chapter as still
        // "owned" by this instance).
        vs.UnmountReaderChapter("GEN", 1);

        // Reader's own pane is still genuinely, actively showing GEN.1 --
        // its legitimate share must survive Kretzmann's own departure.
        Assert.True(vs.IsReaderChapterMounted("GEN", 1));
    }

    [Fact]
    public void PhantomDoubleUnmountWronglyClearsASiblingsLegitimateShare()
    {
        // The FAILURE MODE N-1 named, reproduced directly against the
        // multiset: if a caller (pre-fix Reader.razor/Kretzmann.razor -- a
        // failed load left _loadedBook/_loadedChapterNum stale, pointing at
        // an already-relinquished chapter) spends the SAME credit a SECOND
        // time -- a subsequent navigation attempt's own top-of-method
        // unmount, or DisposeAsync, both reading the stale fields -- the
        // sibling's still-legitimate membership is wrongly cleared. This
        // test documents the exact bug shape the razor-level null-out
        // fixes; it does NOT claim the multiset itself was ever broken (see
        // the sibling test above for the correct, single-unmount sequence
        // the fix now guarantees at the razor call sites).
        var vs = new ViewStateService();

        vs.MountReaderChapter("GEN", 1); // Reader's own share
        vs.MountReaderChapter("GEN", 1); // Kretzmann's own share
        Assert.True(vs.IsReaderChapterMounted("GEN", 1));

        vs.UnmountReaderChapter("GEN", 1); // Kretzmann's own, legitimate, first spend
        Assert.True(vs.IsReaderChapterMounted("GEN", 1)); // Reader's share still stands

        // The bug: the SAME credit spent again (a stale _loadedBook read a
        // second time, pre-fix -- e.g. Kretzmann's own DisposeAsync firing
        // after its failed-load top-of-method unmount already relinquished
        // this chapter, without an intervening null-out).
        vs.UnmountReaderChapter("GEN", 1);
        Assert.False(vs.IsReaderChapterMounted("GEN", 1)); // wrongly cleared -- Reader is still genuinely mounted
    }

    [Fact]
    public void UnmountingAnAlreadyAbsentChapterIsASafeNoOp()
    {
        // ViewStateService.cs's own documented contract for
        // UnmountReaderChapter -- never throws, never goes negative, on a
        // chapter that was never mounted at all (e.g. a superseded
        // navigation whose own success branch never ran).
        var vs = new ViewStateService();
        vs.UnmountReaderChapter("GEN", 1);
        Assert.False(vs.IsReaderChapterMounted("GEN", 1));
    }

    [Fact]
    public void DifferentChaptersAreTrackedIndependently()
    {
        // The released-split case (Q-2's own original scenario, distinct
        // from N-1): two panes on DIFFERENT chapters must never share one
        // count, unlike the old single last-writer-wins value.
        var vs = new ViewStateService();
        vs.MountReaderChapter("GEN", 1);
        vs.MountReaderChapter("EXO", 3);

        Assert.True(vs.IsReaderChapterMounted("GEN", 1));
        Assert.True(vs.IsReaderChapterMounted("EXO", 3));

        vs.UnmountReaderChapter("GEN", 1);
        Assert.False(vs.IsReaderChapterMounted("GEN", 1));
        Assert.True(vs.IsReaderChapterMounted("EXO", 3)); // untouched by the other chapter's own unmount
    }
}
