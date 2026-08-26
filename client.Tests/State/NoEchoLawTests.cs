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
    // Fix round 1 (S-4, IMPORTANT / Q-7, trivia): this used to be
    // `[InlineData(seed, userValue)]` with `seed` discarded at the top of the
    // body ("kept for signature symmetry") -- a dead parameter masquerading
    // as generated coverage the report itself then mis-described. Now
    // genuinely feeds Generators.MixedCounterIntents(seed, N) -- the SAME
    // generator laws 4/5 use -- through the bidirectional pair, asserting
    // the one-hop-settle invariant after EVERY generated intent, not just
    // once.
    [Theory]
    [InlineData(4001)]
    [InlineData(4002)]
    [InlineData(4003)]
    public void Law3_NoEcho_ABidirectionalPairSettlesAfterExactlyOneHop_OverAGeneratedSequence(int seed)
    {
        var a = new StateAtom<Counter>("a", new Counter(0));
        // `b` starts at a.Value+1 (1, not 0) -- establishes the "b == a+1"
        // invariant as the TRUE base case before any dispatch happens (a
        // live-caught bug in an earlier draft of this test started both at
        // 0, which makes the invariant FALSE at t=0 -- 0 != 0+1 -- and stays
        // false for however many leading generated intents happen to be
        // no-ops against a==0, e.g. a ResetCounter or SetCounter(0), since a
        // no-op dispatch never fires the link at all).
        var b = new StateAtom<Counter>("b", new Counter(1));

        // Deliberately runaway: WITHOUT the no-echo guard, one user dispatch
        // into `a` would bounce a->b->a->b->... forever (each hop +1),
        // since neither Derive is a fixed point of the other.
        var linkAb = new DelegateLink<Counter, Counter>(a, b, (src, _) => new Counter(src.Value + 1), () => true);
        var linkBa = new DelegateLink<Counter, Counter>(b, a, (src, _) => new Counter(src.Value + 1), () => true);

        using var runnerAb = new StateLinkRunner<Counter, Counter>("ab", linkAb, a, b);
        using var runnerBa = new StateLinkRunner<Counter, Counter>("ba", linkBa, b, a);

        var intents = Generators.MixedCounterIntents(seed, 40);
        foreach (var intent in intents)
        {
            a.Dispatch(intent); // Origin null -- a genuine user gesture into `a`

            // Holds after EVERY dispatch, whether or not this particular
            // intent actually changed `a` (a no-op dispatch leaves both
            // sides exactly where they already were, so the invariant
            // trivially still holds -- true by induction from the seeded
            // base case above): `b` always sits exactly one hop ahead of
            // `a`'s own current value, never bounced further by an echo.
            Assert.Equal(a.Value.Value + 1, b.Value.Value);
        }
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
