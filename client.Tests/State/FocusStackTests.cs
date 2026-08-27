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
    // Shape: Back pops stack AND records the landed-on node in the trail
    // (Adjudication D, fix round 1 -- G2's own shipped rule, restored: "a
    // Back landing is a visit," CONTRACT.md's own EXPLORE-TRAIL-1 row,
    // verbatim). An earlier draft read R3's "trail untouched" wording
    // literally and shipped the OPPOSITE behavior -- wrong, per the
    // controller's own ruling; see FocusStack.cs's own Back record doc
    // comment for the full history.
    // ------------------------------------------------------------------

    [Fact]
    public void Back_PopsTheStack_AndRecordsTheLandedNodeInTheTrail()
    {
        var atom = new StateAtom<FocusStack>("focus-stack", FocusStack.Empty);
        atom.Dispatch(new Visit(V("GEN.1.1"))); // A
        atom.Dispatch(new Visit(V("GEN.1.2"))); // B -- trail: [A, B]

        atom.Dispatch(new Back()); // lands back on A -- trail: [A, B, A]

        Assert.Single(atom.Value.Stack);
        Assert.Equal("GEN.1.1", atom.Value.Current!.Descriptor.Key);
        Assert.Equal(new[] { "GEN.1.1", "GEN.1.2", "GEN.1.1" }, atom.Value.Trail.Select(d => d.Key));
    }

    [Fact]
    public void Back_ConsecutiveDedupeGuard_WhenTheTrailAlreadyEndsWithTheLandedDescriptor_DoesNotDoubleAppend()
    {
        // Fix round 1 (Adjudication D, item 5): the SAME consecutive-dedupe
        // guard Visit uses (never double-appending when a landed descriptor
        // already equals the trail's own last entry) must also hold on the
        // Back path. NOT reachable through an ordinary Visit/Back dispatch
        // sequence, disclosed: Visit's own idempotence guard means no two
        // ADJACENT stack entries can ever share a descriptor, which in turn
        // means an ordinary walk up/down the stack never lands on the exact
        // descriptor the trail's own last entry already names. Constructed
        // directly here instead, to prove the GUARD ITSELF is correct by
        // inspection -- the same "prove it, don't just assume it never
        // triggers" discipline this file already applies to Visit's own
        // guard, and cheap insurance against a future change (e.g. a
        // relaxed Visit dedupe rule) that WOULD make this reachable.
        var a = new Focus(new ExplorationDescriptor("Verse", "GEN.1.1", "GEN.1.1"), V("GEN.1.1"));
        var b = new Focus(new ExplorationDescriptor("Verse", "GEN.1.2", "GEN.1.2"), V("GEN.1.2"));
        var handBuilt = new FocusStack(new[] { a, b }, new[] { a.Descriptor, b.Descriptor, a.Descriptor }); // trail ALREADY ends with "GEN.1.1" -- exactly what Back is about to land on

        var result = new Back().Apply(handBuilt);

        Assert.Single(result.Stack);
        Assert.Equal(3, result.Trail.Count); // unchanged -- did NOT double-append
        Assert.Equal(handBuilt.Trail, result.Trail);
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
