using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.Explore;
using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// Batch ST-3 (R2/R5): law + shape coverage for the Selection atom
/// (Toggle/Remove/Clear) -- an <c>IReadOnlyList{ExplorationDescriptor}</c>
/// value, using <see cref="SequenceEqualityComparer{T}"/> (see that type's
/// own header for why the default comparer would be wrong here). Toggle's
/// own deliberate NON-idempotence (per-intent sense) is tested as a NEGATIVE
/// control, same "Increment" precedent StateAtomLawTests.cs already
/// established -- the POSITIVE property this atom actually promises is
/// "idempotent TOGGLE-PAIRS" (R5's own phrasing): two toggles of the SAME
/// descriptor return to the ORIGINAL list.
/// </summary>
public class SelectionTests
{
    private static StateAtom<IReadOnlyList<ExplorationDescriptor>> NewAtom(IReadOnlyList<ExplorationDescriptor>? initial = null) =>
        new("selection", initial ?? Selection.Empty, SequenceEqualityComparer<ExplorationDescriptor>.Instance);

    private static ExplorationDescriptor D(string key) => new("Place", key, key);

    // ------------------------------------------------------------------
    // SequenceEqualityComparer: proves the atom's OWN law 2 depends on it
    // (a naive default comparer would treat two content-equal-but-distinct
    // List instances as different, breaking idempotence for every intent
    // here, since every Apply below returns a FRESH list).
    // ------------------------------------------------------------------

    [Fact]
    public void Law2_Idempotence_ADispatchThatReproducesTheCurrentListRaisesNoChanged()
    {
        // ClearSelection on an already-empty atom is the simplest case: two
        // SEPARATE Array.Empty<T> "lists" (well, the same static instance
        // here, but Toggle/Remove below build genuinely fresh List<T>
        // instances) must compare equal for law 2 to hold at all.
        var atom = NewAtom();
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        atom.Dispatch(new ClearSelection());

        Assert.Equal(0, changedCount);
    }

    [Fact]
    public void SequenceEqualityComparer_TwoDistinctListInstancesWithTheSameContentAreEqual()
    {
        var a = new List<ExplorationDescriptor> { D("x"), D("y") };
        var b = new List<ExplorationDescriptor> { D("x"), D("y") };
        Assert.NotSame(a, b);
        Assert.True(SequenceEqualityComparer<ExplorationDescriptor>.Instance.Equals(a, b));
    }

    [Fact]
    public void SequenceEqualityComparer_OrderMatters_SelectionIsAnOrderedList()
    {
        var a = new List<ExplorationDescriptor> { D("x"), D("y") };
        var b = new List<ExplorationDescriptor> { D("y"), D("x") };
        Assert.False(SequenceEqualityComparer<ExplorationDescriptor>.Instance.Equals(a, b));
    }

    // ------------------------------------------------------------------
    // Toggle: not-selected -> selected -> not-selected. Idempotent
    // TOGGLE-PAIRS (R5's own phrasing), NOT literal per-intent idempotence
    // (negative control, mirroring StateAtomLawTests.cs's own Increment).
    // ------------------------------------------------------------------

    [Fact]
    public void Toggle_NotYetSelected_AddsIt()
    {
        var atom = NewAtom();
        atom.Dispatch(new ToggleSelection(D("place-1")));
        Assert.Equal(new[] { D("place-1") }, atom.Value);
    }

    [Fact]
    public void Toggle_AlreadySelected_RemovesIt()
    {
        var atom = NewAtom(new List<ExplorationDescriptor> { D("place-1") });
        atom.Dispatch(new ToggleSelection(D("place-1")));
        Assert.Empty(atom.Value);
    }

    [Fact]
    public void IdempotentTogglePairs_TwoTogglesOfTheSameDescriptorReturnToTheOriginalList()
    {
        var original = new List<ExplorationDescriptor> { D("existing") };
        var atom = NewAtom(original);

        atom.Dispatch(new ToggleSelection(D("place-1")));
        atom.Dispatch(new ToggleSelection(D("place-1")));

        Assert.Equal(original, atom.Value);
    }

    [Theory]
    [InlineData(8001)]
    [InlineData(8002)]
    [InlineData(8003)]
    public void IdempotentTogglePairs_HoldsOverAGeneratedSequenceOfPairedToggles(int seed)
    {
        var rng = new Random(seed);
        var pool = Enumerable.Range(0, 6).Select(i => D($"item-{i}")).ToList();
        var atom = NewAtom();
        var before = atom.Value;

        for (var i = 0; i < 25; i++)
        {
            var d = pool[rng.Next(pool.Count)];
            atom.Dispatch(new ToggleSelection(d));
            atom.Dispatch(new ToggleSelection(d)); // immediately paired -- must cancel

            Assert.Equal(before, atom.Value); // every pair returns to whatever preceded it
            before = atom.Value;
        }
    }

    [Fact]
    public void NegativeControl_ToggleIsNotIndividuallyIdempotent_RedispatchingTheSameInstanceTwiceFlipsTwice()
    {
        // Same shape as StateAtomLawTests.cs's own Increment negative
        // control -- proves the atom isn't silently swallowing every
        // redispatch (which would make the pair-cancellation test above
        // vacuous): the SAME ToggleSelection instance, dispatched twice,
        // genuinely fires Changed twice.
        var atom = NewAtom();
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        var toggle = new ToggleSelection(D("place-1"));
        atom.Dispatch(toggle);
        atom.Dispatch(toggle);

        Assert.Equal(2, changedCount);
        Assert.Empty(atom.Value); // added then removed
    }

    [Fact]
    public void Toggle_IdentityIsKindAndKey_NeverTitle()
    {
        // R2/SelectionTrayService's own pre-atom rule, verbatim: identity is
        // Kind+Key, never Title (cached display text).
        var atom = NewAtom(new List<ExplorationDescriptor> { new("Place", "p1", "Old Name") });
        atom.Dispatch(new ToggleSelection(new ExplorationDescriptor("Place", "p1", "New Name")));
        Assert.Empty(atom.Value); // matched and removed despite the differing Title
    }

    // ------------------------------------------------------------------
    // Remove / Clear: genuinely idempotent (standard law 2 sense).
    // ------------------------------------------------------------------

    [Fact]
    public void Remove_AnAbsentDescriptor_IsAStructuralNoOp()
    {
        var atom = NewAtom(new List<ExplorationDescriptor> { D("a") });
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        atom.Dispatch(new RemoveSelection(D("not-there")));

        Assert.Equal(0, changedCount);
        Assert.Equal(new[] { D("a") }, atom.Value);
    }

    [Fact]
    public void Law2_Idempotence_RedispatchingTheSameRemoveInstanceIsANoOp()
    {
        var atom = NewAtom(new List<ExplorationDescriptor> { D("a"), D("b") });
        var remove = new RemoveSelection(D("a"));
        atom.Dispatch(remove);
        var afterFirst = atom.Value;
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        atom.Dispatch(remove);

        Assert.Equal(afterFirst, atom.Value);
        Assert.Equal(0, changedCount);
    }

    [Fact]
    public void Law2_Idempotence_ClearIsIdempotent()
    {
        var atom = NewAtom(new List<ExplorationDescriptor> { D("a") });
        atom.Dispatch(new ClearSelection());
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        atom.Dispatch(new ClearSelection());

        Assert.Equal(0, changedCount);
        Assert.Empty(atom.Value);
    }

    // ------------------------------------------------------------------
    // Law 5 (agreement) / Law 4 (confluence), generated -- same idiom as
    // ConfluenceAgreementLawTests.cs (ST-1).
    // ------------------------------------------------------------------

    [Theory]
    [InlineData(8101)]
    [InlineData(8102)]
    public void Law5_Agreement_AllProjectionsAgreeAfterEveryDispatch(int seed)
    {
        var rng = new Random(seed);
        var pool = Enumerable.Range(0, 5).Select(i => D($"item-{i}")).ToList();
        var atom = NewAtom();
        var projections = Enumerable.Range(0, 4).Select(_ => new Projection<IReadOnlyList<ExplorationDescriptor>>(atom)).ToList();

        for (var i = 0; i < 30; i++)
        {
            var d = pool[rng.Next(pool.Count)];
            IIntent<IReadOnlyList<ExplorationDescriptor>> intent = rng.Next(3) switch
            {
                0 => new ToggleSelection(d),
                1 => new RemoveSelection(d),
                _ => new ClearSelection(),
            };
            atom.Dispatch(intent);

            foreach (var projection in projections)
            {
                Assert.Equal(atom.Value, projection.Value);
            }
        }
    }

    [Theory]
    [InlineData(8111)]
    public void Law4_Confluence_FinalValueMatchesAPureFoldOverTheSameSequence(int seed)
    {
        var rng = new Random(seed);
        var pool = Enumerable.Range(0, 5).Select(i => D($"item-{i}")).ToList();
        var intents = Enumerable.Range(0, 40).Select(_ =>
        {
            var d = pool[rng.Next(pool.Count)];
            return (IIntent<IReadOnlyList<ExplorationDescriptor>>)(rng.Next(3) switch
            {
                0 => new ToggleSelection(d),
                1 => new RemoveSelection(d),
                _ => new ClearSelection(),
            });
        }).ToList();

        var atom = NewAtom();
        foreach (var intent in intents)
        {
            atom.Dispatch(intent);
        }

        var expected = intents.Aggregate(Selection.Empty, (acc, intent) => intent.Apply(acc));
        Assert.Equal(expected, atom.Value, SequenceEqualityComparer<ExplorationDescriptor>.Instance);
    }
}
