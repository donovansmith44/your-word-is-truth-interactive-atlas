using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// Batch ST-3 (R3/R5): law + shape coverage for the FocusStack atom
/// (Visit/Back/Reset/SeedFromTrail) over the REAL <see cref="VerseNode"/>
/// (no network -- VerseNode's own Title/Kind are synchronous, constructor-
/// only). Confluence/agreement mirror ConfluenceAgreementLawTests.cs's own
/// idiom (ST-1), applied to this atom's own richer value type.
/// </summary>
public class FocusStackTests
{
    private static VerseNode V(string vref) => new(vref);

    // ------------------------------------------------------------------
    // Shape: Visit appends trail + pushes; consecutive re-visit is a no-op.
    // ------------------------------------------------------------------

    [Fact]
    public void Visit_OnAnEmptyStack_PushesAndRecordsTheTrail()
    {
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        atom.Dispatch(new Visit(V("GEN.1.1")));

        Assert.Single(atom.Value.Stack);
        Assert.Equal("GEN.1.1", atom.Value.Current!.Descriptor.Key);
        Assert.Equal(new[] { "GEN.1.1" }, atom.Value.Trail.Select(d => d.Key));
    }

    [Fact]
    public void Visit_ADifferentNode_PushesOnTopAndAppendsTheTrail()
    {
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        atom.Dispatch(new Visit(V("GEN.1.1")));
        atom.Dispatch(new Visit(V("GEN.1.2")));

        Assert.Equal(2, atom.Value.Stack.Count);
        Assert.Equal("GEN.1.2", atom.Value.Current!.Descriptor.Key);
        Assert.Equal(new[] { "GEN.1.1", "GEN.1.2" }, atom.Value.Trail.Select(d => d.Key));
    }

    [Fact]
    public void Law2_Idempotence_VisitingTheSameNodeAlreadyOnTopIsAGenuineNoOp()
    {
        // See FocusStack.cs's own Visit record doc comment: a naive
        // unconditional push would NOT be idempotent (Apply(Apply(v)) !=
        // Apply(v)) -- this is the guard that makes it so.
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        atom.Dispatch(new Visit(V("GEN.1.1")));
        var afterFirst = atom.Value;
        var changedAfterFirst = changedCount;

        atom.Dispatch(new Visit(V("GEN.1.1"))); // a DIFFERENT VerseNode instance, same descriptor -- Descriptor equality, not reference equality, is what Apply checks

        Assert.Equal(afterFirst, atom.Value);
        Assert.Equal(changedAfterFirst, changedCount);
        Assert.Single(atom.Value.Stack); // did NOT grow to 2
        Assert.Single(atom.Value.Trail);
    }

    [Fact]
    public void Visit_ReturningToAnEarlierEntryDoesNotDedupe_OnlyConsecutiveRepeatsCollapse()
    {
        // A -> B -> A (not consecutive-same) IS three real trail entries --
        // only immediately-repeated visits collapse, matching the pre-atom
        // RecordTrailVisit's own "consecutive duplicates" rule.
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        atom.Dispatch(new Visit(V("GEN.1.1")));
        atom.Dispatch(new Visit(V("GEN.1.2")));
        atom.Dispatch(new Visit(V("GEN.1.1")));

        Assert.Equal(3, atom.Value.Stack.Count);
        Assert.Equal(new[] { "GEN.1.1", "GEN.1.2", "GEN.1.1" }, atom.Value.Trail.Select(d => d.Key));
    }

    // ------------------------------------------------------------------
    // Shape: Back pops stack, TRAIL UNTOUCHED (R3, verbatim -- a disclosed
    // behavior change from the pre-ST-3 component code).
    // ------------------------------------------------------------------

    [Fact]
    public void Back_PopsTheStack_TrailUntouched()
    {
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        atom.Dispatch(new Visit(V("GEN.1.1")));
        atom.Dispatch(new Visit(V("GEN.1.2")));
        var trailBeforeBack = atom.Value.Trail;

        atom.Dispatch(new Back());

        Assert.Single(atom.Value.Stack);
        Assert.Equal("GEN.1.1", atom.Value.Current!.Descriptor.Key);
        Assert.Equal(trailBeforeBack, atom.Value.Trail); // R3, verbatim: "trail untouched"
    }

    [Fact]
    public void Law2_Idempotence_BackAtTheBottomOfTheStackIsANoOp()
    {
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        atom.Dispatch(new Visit(V("GEN.1.1")));
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        atom.Dispatch(new Back());
        atom.Dispatch(new Back());

        Assert.Equal(0, changedCount);
        Assert.Single(atom.Value.Stack);
    }

    // ------------------------------------------------------------------
    // Shape: Reset clears both; idempotent by construction.
    // ------------------------------------------------------------------

    [Fact]
    public void Reset_ClearsBothStackAndTrail()
    {
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        atom.Dispatch(new Visit(V("GEN.1.1")));
        atom.Dispatch(new Visit(V("GEN.1.2")));

        atom.Dispatch(new Reset());

        Assert.Equal(FocusStack.Empty, atom.Value);
        Assert.Null(atom.Value.Current);
    }

    [Fact]
    public void Law2_Idempotence_ResettingAnAlreadyEmptyAtomIsANoOp()
    {
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        atom.Dispatch(new Reset());

        Assert.Equal(0, changedCount);
    }

    // ------------------------------------------------------------------
    // Shape: SeedFromTrail (G2 decision 5, "continue") -- ordered seed,
    // last entry becomes Current, ignores whatever was there before.
    // ------------------------------------------------------------------

    [Fact]
    public void SeedFromTrail_PushesEveryNodeInOrder_LastBecomesCurrent()
    {
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        var seed = new IExplorable[] { V("GEN.1.1"), V("GEN.1.2"), V("EXO.3.14") };

        atom.Dispatch(new SeedFromTrail(seed));

        Assert.Equal(3, atom.Value.Stack.Count);
        Assert.Equal("EXO.3.14", atom.Value.Current!.Descriptor.Key);
        Assert.Equal(new[] { "GEN.1.1", "GEN.1.2", "EXO.3.14" }, atom.Value.Stack.Select(f => f.Descriptor.Key));
    }

    [Fact]
    public void SeedFromTrail_OverwritesWhateverWasThereBefore_IgnoringCurrent()
    {
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        atom.Dispatch(new Visit(V("PSA.23.1")));

        atom.Dispatch(new SeedFromTrail(new IExplorable[] { V("GEN.1.1") }));

        Assert.Single(atom.Value.Stack);
        Assert.Equal("GEN.1.1", atom.Value.Current!.Descriptor.Key);
    }

    [Fact]
    public void Law2_Idempotence_RedispatchingTheSameSeedFromTrailInstanceIsANoOp()
    {
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        var intent = new SeedFromTrail(new IExplorable[] { V("GEN.1.1"), V("GEN.1.2") });
        atom.Dispatch(intent);
        var afterFirst = atom.Value;
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        atom.Dispatch(intent);

        Assert.Equal(afterFirst, atom.Value);
        Assert.Equal(0, changedCount);
    }

    // ------------------------------------------------------------------
    // Law 4 (confluence) / Law 5 (agreement), generated sequences -- same
    // idiom as ConfluenceAgreementLawTests.cs (ST-1).
    // ------------------------------------------------------------------

    private static IIntent<FocusStack> RandomIntent(Random rng, string vref) => rng.Next(4) switch
    {
        0 or 1 => new Visit(V(vref)), // weighted toward Visit -- Back/Reset need a non-trivial stack to be interesting
        2 => new Back(),
        _ => new Reset(),
    };

    [Theory]
    [InlineData(7001)]
    [InlineData(7002)]
    [InlineData(7003)]
    public void Law4_Confluence_FinalValueIsIndependentOfSubscriberCount(int seed)
    {
        var refs = Generators.VerseRefSequence(seed, 40);
        var rng = new Random(seed + 1);
        var intents = refs.Select(r => RandomIntent(rng, r)).ToList();

        FocusStack Run(int subscriberCount)
        {
            var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
            for (var i = 0; i < subscriberCount; i++)
            {
                atom.Changed += () => { };
            }

            foreach (var intent in intents)
            {
                atom.Dispatch(intent);
            }

            return atom.Value;
        }

        var noObservers = Run(0);
        var manyObservers = Run(5);
        Assert.Equal(noObservers, manyObservers);
    }

    [Theory]
    [InlineData(7011)]
    [InlineData(7012)]
    public void Law5_Agreement_AllProjectionsAgreeAfterEveryDispatch(int seed)
    {
        var refs = Generators.VerseRefSequence(seed, 30);
        var rng = new Random(seed + 1);
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        var projections = Enumerable.Range(0, 4).Select(_ => new Projection<FocusStack>(atom)).ToList();

        foreach (var r in refs)
        {
            atom.Dispatch(RandomIntent(rng, r));
            foreach (var projection in projections)
            {
                Assert.Equal(atom.Value, projection.Value);
            }
        }
    }
}
