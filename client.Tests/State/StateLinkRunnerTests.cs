using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// Batch ST-1: non-law sanity coverage for <see cref="StateLinkRunner{A,B}"/>
/// itself (Active gating, SyncNow, constructor validation, unsubscribe on
/// Dispose) -- the imperative half the no-echo law tests
/// (NoEchoLawTests.cs) build on top of but don't individually cover.
/// </summary>
public class StateLinkRunnerTests
{
    [Fact]
    public void WhileActive_SourceChangeDispatchesTheDerivedValueIntoTarget_TaggedWithTheGivenName()
    {
        var source = new StateAtom<Counter>("source", new Counter(0));
        var target = new StateAtom<Counter>("target", new Counter(100));
        var link = new DelegateLink<Counter, Counter>(source, target, (s, _) => new Counter(s.Value * 2), () => true);
        using var runner = new StateLinkRunner<Counter, Counter>("double-link", link, source, target);

        source.Dispatch(new SetCounter(21));

        Assert.Equal(new Counter(42), target.Value);
        Assert.Equal("double-link", target.LastOrigin); // law 3's own echo tag
    }

    [Fact]
    public void WhileInactive_SourceChangeNeverTouchesTarget()
    {
        var source = new StateAtom<Counter>("source", new Counter(0));
        var target = new StateAtom<Counter>("target", new Counter(100));
        var link = new DelegateLink<Counter, Counter>(source, target, (s, _) => new Counter(s.Value * 2), () => false);
        using var runner = new StateLinkRunner<Counter, Counter>("inactive-link", link, source, target);

        source.Dispatch(new SetCounter(21));

        Assert.Equal(new Counter(100), target.Value); // untouched
        Assert.Null(target.LastOrigin);
    }

    [Fact]
    public void SyncNow_DispatchesEvenWithoutASourceChange_WhenActive()
    {
        // The "Active flipped false->true" case (e.g. the follow chip
        // clicked back on) -- a bare Changed subscription can't cover this,
        // since Source never actually changed; SyncNow forces one pass.
        var source = new StateAtom<Counter>("source", new Counter(7));
        var target = new StateAtom<Counter>("target", new Counter(0));
        var link = new DelegateLink<Counter, Counter>(source, target, (s, _) => new Counter(s.Value * 10), () => true);
        using var runner = new StateLinkRunner<Counter, Counter>("sync-now-link", link, source, target);

        runner.SyncNow();

        Assert.Equal(new Counter(70), target.Value);
    }

    [Fact]
    public void SyncNow_IsANoOpWhenInactive()
    {
        var source = new StateAtom<Counter>("source", new Counter(7));
        var target = new StateAtom<Counter>("target", new Counter(0));
        var link = new DelegateLink<Counter, Counter>(source, target, (s, _) => new Counter(s.Value * 10), () => false);
        using var runner = new StateLinkRunner<Counter, Counter>("sync-now-inactive", link, source, target);

        runner.SyncNow();

        Assert.Equal(new Counter(0), target.Value);
    }

    [Fact]
    public void Constructor_ThrowsWhenSourceAtomDoesNotMatchTheLinksOwnSource()
    {
        var linkSource = new StateAtom<Counter>("link-source", new Counter(0));
        var wrongSource = new StateAtom<Counter>("wrong-source", new Counter(0));
        var target = new StateAtom<Counter>("target", new Counter(0));
        var link = new DelegateLink<Counter, Counter>(linkSource, target, (s, _) => s, () => true);

        Assert.Throws<ArgumentException>(() => new StateLinkRunner<Counter, Counter>("bad", link, wrongSource, target));
    }

    [Fact]
    public void Constructor_ThrowsWhenTargetAtomDoesNotMatchTheLinksOwnTarget()
    {
        var source = new StateAtom<Counter>("source", new Counter(0));
        var linkTarget = new StateAtom<Counter>("link-target", new Counter(0));
        var wrongTarget = new StateAtom<Counter>("wrong-target", new Counter(0));
        var link = new DelegateLink<Counter, Counter>(source, linkTarget, (s, _) => s, () => true);

        Assert.Throws<ArgumentException>(() => new StateLinkRunner<Counter, Counter>("bad", link, source, wrongTarget));
    }

    [Fact]
    public void Dispose_UnsubscribesFromSourceChanged_NoFurtherDerivation()
    {
        var source = new StateAtom<Counter>("source", new Counter(0));
        var target = new StateAtom<Counter>("target", new Counter(0));
        var link = new DelegateLink<Counter, Counter>(source, target, (s, _) => new Counter(s.Value + 1), () => true);
        var runner = new StateLinkRunner<Counter, Counter>("disposable", link, source, target);

        source.Dispatch(new SetCounter(1));
        Assert.Equal(new Counter(2), target.Value);

        runner.Dispose();
        source.Dispatch(new SetCounter(50));

        Assert.Equal(new Counter(2), target.Value); // unchanged -- the runner no longer reacts
    }
}
