using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// Batch ST-1: law 3 (no-echo) -- "link-derived updates carry origin, never
/// re-derive their source (bidirectional links cannot oscillate)." The
/// meatiest of the five: constructs a genuinely oscillation-PRONE
/// bidirectional link pair (each Derive adds 1 -- neither is a fixed point
/// of the other) over two synthetic Counter atoms, wires both directions
/// live via <see cref="StateLinkRunner{A,B}"/>, and proves the guard
/// actually stops the cascade after one hop. See StateLinkRunner.cs's own
/// header comment for the full mechanism/trace this test is pinning.
/// </summary>
public class NoEchoLawTests
{
    [Theory]
    [InlineData(4001, 1)]
    [InlineData(4002, 500)]
    [InlineData(4003, -37)]
    public void Law3_NoEcho_ABidirectionalPairSettlesAfterExactlyOneHop(int seed, int userValue)
    {
        _ = seed; // kept for signature symmetry with the other seeded law tests; this case is deterministic in userValue alone
        var a = new StateAtom<Counter>("a", new Counter(0));
        var b = new StateAtom<Counter>("b", new Counter(0));

        // Deliberately runaway: WITHOUT the no-echo guard, one user dispatch
        // into `a` would bounce a->b->a->b->... forever (each hop +1),
        // since neither Derive is a fixed point of the other.
        var linkAb = new DelegateLink<Counter, Counter>(a, b, (src, _) => new Counter(src.Value + 1), () => true);
        var linkBa = new DelegateLink<Counter, Counter>(b, a, (src, _) => new Counter(src.Value + 1), () => true);

        var aChanges = 0;
        var bChanges = 0;
        a.Changed += () => aChanges++;
        b.Changed += () => bChanges++;

        using var runnerAb = new StateLinkRunner<Counter, Counter>("ab", linkAb, a, b);
        using var runnerBa = new StateLinkRunner<Counter, Counter>("ba", linkBa, b, a);

        a.Dispatch(new SetCounter(userValue)); // Origin null -- a genuine user gesture

        Assert.Equal(userValue, a.Value.Value); // `a` was never bounced back by the echo
        Assert.Equal(userValue + 1, b.Value.Value); // exactly ONE hop of derivation into `b`
        Assert.Equal(1, aChanges); // only the user's own dispatch ever changed `a`
        Assert.Equal(1, bChanges); // only the one link-derived hop ever changed `b`
    }

    [Fact]
    public void Law3_NoEcho_AUserDispatchIntoEitherSideOfThePairStillOnlyPropagatesOneHop()
    {
        // Symmetric case: a user gesture landing on `b` instead of `a`
        // propagates exactly one hop the OTHER direction, proving the guard
        // isn't accidentally asymmetric between the two link instances.
        var a = new StateAtom<Counter>("a", new Counter(0));
        var b = new StateAtom<Counter>("b", new Counter(0));
        var linkAb = new DelegateLink<Counter, Counter>(a, b, (src, _) => new Counter(src.Value + 1), () => true);
        var linkBa = new DelegateLink<Counter, Counter>(b, a, (src, _) => new Counter(src.Value + 1), () => true);

        using var runnerAb = new StateLinkRunner<Counter, Counter>("ab", linkAb, a, b);
        using var runnerBa = new StateLinkRunner<Counter, Counter>("ba", linkBa, b, a);

        b.Dispatch(new SetCounter(10));

        Assert.Equal(10, b.Value.Value);
        Assert.Equal(11, a.Value.Value);
    }

    [Fact]
    public void Law3_NoEcho_ChainOfSeveralUserDispatchesEachSettlesIndependently()
    {
        // A SEQUENCE of user gestures (not just one) -- each must settle to
        // its own one-hop propagation, never compounding across gestures
        // (LastOrigin is reset to null the moment a genuinely NEW
        // user-originated Dispatch lands, since that dispatch's own Origin
        // is null).
        var a = new StateAtom<Counter>("a", new Counter(0));
        var b = new StateAtom<Counter>("b", new Counter(0));
        var linkAb = new DelegateLink<Counter, Counter>(a, b, (src, _) => new Counter(src.Value + 1), () => true);
        var linkBa = new DelegateLink<Counter, Counter>(b, a, (src, _) => new Counter(src.Value + 1), () => true);

        using var runnerAb = new StateLinkRunner<Counter, Counter>("ab", linkAb, a, b);
        using var runnerBa = new StateLinkRunner<Counter, Counter>("ba", linkBa, b, a);

        foreach (var value in new[] { 1, 20, 300, -5 })
        {
            a.Dispatch(new SetCounter(value));
            Assert.Equal(value, a.Value.Value);
            Assert.Equal(value + 1, b.Value.Value);
        }
    }

    [Fact]
    public void Law3_NoEcho_ALinkThatIsInactiveNeverParticipatesInTheEcho()
    {
        // An inactive reverse link can't echo at all (Active gates BEFORE
        // the no-echo check even runs) -- the forward link alone still
        // derives normally.
        var a = new StateAtom<Counter>("a", new Counter(0));
        var b = new StateAtom<Counter>("b", new Counter(0));
        var linkAb = new DelegateLink<Counter, Counter>(a, b, (src, _) => new Counter(src.Value + 1), () => true);
        var linkBaActive = false;
        var linkBa = new DelegateLink<Counter, Counter>(b, a, (src, _) => new Counter(src.Value + 1), () => linkBaActive);

        using var runnerAb = new StateLinkRunner<Counter, Counter>("ab", linkAb, a, b);
        using var runnerBa = new StateLinkRunner<Counter, Counter>("ba", linkBa, b, a);

        a.Dispatch(new SetCounter(5));
        Assert.Equal(5, a.Value.Value);
        Assert.Equal(6, b.Value.Value);
    }
}
