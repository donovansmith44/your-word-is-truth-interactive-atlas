using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// Batch ST-1: laws 4 (confluence) and 5 (agreement). Generated, seeded,
/// deterministic sequences (Generators.cs) over the REAL StateAtom{T}/
/// Projection{T}.
/// </summary>
public class ConfluenceAgreementLawTests
{
    // ------------------------------------------------------------------
    // Law 4: confluence -- same intent sequence => same final state
    // regardless of render timing. This atom layer is synchronous (no
    // async gap inside Dispatch), so "render timing" independence means:
    // the number/timing of Changed SUBSCRIBERS (a render's own stand-in)
    // must never affect the final Value a sequence produces.
    // ------------------------------------------------------------------

    [Theory]
    [InlineData(2001)]
    [InlineData(2002)]
    [InlineData(2003)]
    public void Law4_Confluence_FinalValueIsIndependentOfSubscriberCount(int seed)
    {
        var intents = Generators.MixedCounterIntents(seed, 50);

        Counter Run(int subscriberCount)
        {
            var atom = new StateAtom<Counter>("counter", new Counter(0));
            for (var i = 0; i < subscriberCount; i++)
            {
                atom.Changed += () => { }; // an inert observer -- a render's own stand-in
            }

            foreach (var intent in intents)
            {
                atom.Dispatch(intent);
            }

            return atom.Value;
        }

        var noObservers = Run(0);
        var oneObserver = Run(1);
        var manyObservers = Run(7);

        Assert.Equal(noObservers, oneObserver);
        Assert.Equal(noObservers, manyObservers);
    }

    [Theory]
    [InlineData(2011)]
    [InlineData(2012)]
    public void Law4_Confluence_FinalValueMatchesAPureFoldOverTheSameSequence(int seed)
    {
        // Proves Dispatch has no HIDDEN state beyond what each intent's own
        // Apply computes -- the atom's final Value is exactly the left fold
        // of Apply over the initial value, nothing more.
        var intents = Generators.MixedCounterIntents(seed, 50);
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        foreach (var intent in intents)
        {
            atom.Dispatch(intent);
        }

        var expected = intents.Aggregate(new Counter(0), (acc, intent) => intent.Apply(acc));
        Assert.Equal(expected, atom.Value);
    }

    [Theory]
    [InlineData(2021)]
    public void Law4_Confluence_InterleavingDispatchesFromTwoLogicalCallersProducesTheSameResultAsOneOrdering(int seed)
    {
        // "Render timing" independence stress case: the SAME sequence,
        // dispatched by what LOOK like two different callers racing each
        // other (odd/even indices interleaved), still lands at the SAME
        // final value as a straight-through single-caller dispatch -- there
        // is no per-caller/per-render private buffering anywhere in Dispatch.
        var intents = Generators.MixedCounterIntents(seed, 40);

        var straight = new StateAtom<Counter>("counter", new Counter(0));
        foreach (var intent in intents)
        {
            straight.Dispatch(intent);
        }

        var interleaved = new StateAtom<Counter>("counter", new Counter(0));
        var evens = intents.Where((_, i) => i % 2 == 0).ToList();
        var odds = intents.Where((_, i) => i % 2 == 1).ToList();
        // Dispatched in the SAME overall order as `intents` (interleaved
        // reconstruction), just routed through two separate local loops --
        // the point is that Dispatch's own correctness doesn't depend on
        // which loop/caller happens to invoke it, only on ORDER.
        int e = 0, o = 0;
        for (var i = 0; i < intents.Count; i++)
        {
            if (i % 2 == 0)
            {
                interleaved.Dispatch(evens[e++]);
            }
            else
            {
                interleaved.Dispatch(odds[o++]);
            }
        }

        Assert.Equal(straight.Value, interleaved.Value);
    }

    // Fix round 1 (Q-3, IMPORTANT): the timing variation that can ACTUALLY
    // break this layer in Blazor is RE-ENTRANT dispatch from inside a
    // Changed handler -- and production does exactly this shape
    // (World.OnLocusChanged -> StateHasChanged -> a child's own
    // OnParametersSet, one of which could dispatch). StateAtom.Dispatch
    // happens to get this right because Value is committed BEFORE
    // Changed?.Invoke() fires (StateAtom.cs) -- a re-entrant dispatch reads
    // a consistent, already-current value, and the OUTER frame has nothing
    // left to do after invoking Changed, so it can never clobber the
    // re-entrant write. Nothing in the earlier tests above exercises this;
    // a future refactor that moved the Invoke above the assignment would
    // pass every one of them while breaking this.
    [Fact]
    public void Law4_Confluence_ReEntrantDispatchFromInsideAChangedHandlerIsHandledConsistently()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var reentered = false;
        atom.Changed += () =>
        {
            if (reentered)
            {
                return; // this test's own re-entry guard, not part of StateAtom itself
            }

            reentered = true;
            // Dispatched from INSIDE the outer Dispatch's own Changed
            // invocation -- exactly the production shape (OnLocusChanged ->
            // StateHasChanged -> a child dispatching).
            atom.Dispatch(new Increment(1));
        };

        atom.Dispatch(new SetCounter(10));

        // Expected: the fold of BOTH intents, in the order they were
        // actually dispatched (SetCounter(10), then the re-entrant
        // Increment(1)) -- proving the re-entrant write neither gets lost
        // nor gets clobbered by the outer frame resuming afterward.
        var expected = new Increment(1).Apply(new SetCounter(10).Apply(new Counter(0)));
        Assert.Equal(expected, atom.Value);
    }

    // ------------------------------------------------------------------
    // Law 5: agreement -- all projections of one atom are equal ALWAYS.
    // ------------------------------------------------------------------

    [Theory]
    [InlineData(3001)]
    [InlineData(3002)]
    [InlineData(3003)]
    public void Law5_Agreement_AllProjectionsAgreeAfterEveryDispatch(int seed)
    {
        var intents = Generators.MixedCounterIntents(seed, 40);
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var projections = Enumerable.Range(0, 5).Select(_ => new Projection<Counter>(atom)).ToList();

        foreach (var intent in intents)
        {
            atom.Dispatch(intent);

            foreach (var projection in projections)
            {
                Assert.Equal(atom.Value, projection.Value);
            }

            for (var i = 1; i < projections.Count; i++)
            {
                Assert.Equal(projections[0].Value, projections[i].Value);
            }
        }
    }

    [Fact]
    public void Law5_Agreement_AProjectionConstructedAfterPriorDispatchesStillAgrees()
    {
        // A projection created MID-session (not from the atom's very first
        // value) must still read the atom's CURRENT value, never a stale
        // snapshot from its own construction time -- Projection<T> stores
        // no value of its own (Value is a pure forward), so this holds by
        // construction; still exercised as a real assertion per the law's
        // own binding-test-obligation status.
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        atom.Dispatch(new SetCounter(5));
        atom.Dispatch(new Increment(3));

        var lateProjection = new Projection<Counter>(atom);
        Assert.Equal(atom.Value, lateProjection.Value);

        atom.Dispatch(new Increment(1));
        Assert.Equal(atom.Value, lateProjection.Value);
    }
}
