using System.Reflection;
using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// Batch ST-1: laws 1 (single-writer) and 2 (idempotence) -- see
/// Contracts/State.cs's own header for the exact wording each test pins.
/// Uses the synthetic Counter/SetCounter/ResetCounter vocabulary
/// (Generators.cs) over the REAL <see cref="StateAtom{T}"/> -- these tests
/// outlive any one migration (Locus, TimeWindow, or whatever ST-2/ST-3 add).
/// </summary>
public class StateAtomLawTests
{
    // ------------------------------------------------------------------
    // Law 1: single-writer -- atoms mutate only via Dispatch.
    // ------------------------------------------------------------------

    [Fact]
    public void Law1_SingleWriter_ValuePropertyHasNoPublicSetter()
    {
        // Structural, not behavioral: proves by REFLECTION that Dispatch is
        // the only code path in this assembly that can ever change Value --
        // a future edit that widens the setter fails this test loudly,
        // rather than the law silently eroding.
        var property = typeof(StateAtom<>).GetProperty(nameof(StateAtom<Counter>.Value));
        Assert.NotNull(property);
        var setter = property!.SetMethod;
        Assert.True(setter is null || !setter.IsPublic,
            "StateAtom<T>.Value must have no PUBLIC setter -- Dispatch must be the only write path (law 1).");
    }

    [Fact]
    public void Law1_SingleWriter_DispatchIsTheOnlyPublicWayToChangeValue()
    {
        // Behavioral counterpart: an atom's Value only ever moves in
        // response to a Dispatch call -- never spontaneously, never via any
        // other public member (StateAtom<T>'s own public surface is Name,
        // Value, Dispatch, Changed -- none of the other three can mutate
        // Value on their own).
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var before = atom.Value;
        _ = atom.Name;
        var changed = false;
        atom.Changed += () => changed = true;

        // Merely reading Name/Value/subscribing to Changed must never itself
        // mutate anything.
        Assert.Equal(before, atom.Value);
        Assert.False(changed);

        atom.Dispatch(new SetCounter(42));
        Assert.Equal(new Counter(42), atom.Value);
        Assert.True(changed);
    }

    // ------------------------------------------------------------------
    // Law 2: idempotence -- same intent twice = once. Generated, seeded,
    // deterministic sequence of ONLY genuinely idempotent intents (see
    // Generators.cs's own header for why Increment is excluded here).
    // ------------------------------------------------------------------

    [Theory]
    [InlineData(1001)]
    [InlineData(1002)]
    [InlineData(1003)]
    public void Law2_Idempotence_RedispatchingTheSameIntentInstanceIsANoOp(int seed)
    {
        var intents = Generators.IdempotentCounterIntents(seed, 30);
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        foreach (var intent in intents)
        {
            atom.Dispatch(intent);
            var afterFirst = atom.Value;
            var changedAfterFirst = changedCount;

            atom.Dispatch(intent); // the SAME intent instance, dispatched again

            Assert.Equal(afterFirst, atom.Value); // no further movement
            Assert.Equal(changedAfterFirst, changedCount); // no second Changed event -- the observable half of law 2
        }
    }

    [Fact]
    public void Law2_Idempotence_ADispatchThatProducesTheCurrentValueRaisesNoChanged()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(7));
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        atom.Dispatch(new SetCounter(7)); // already the current value

        Assert.Equal(new Counter(7), atom.Value);
        Assert.Equal(0, changedCount);
    }

    [Fact]
    public void Law2_Idempotence_ADeltaIntentIsCorrectlyNOTIdempotent_NegativeControl()
    {
        // Negative control, disclosed in the batch report: Increment is
        // DELIBERATELY not idempotent (Generators.cs's own header) --
        // dispatching the SAME Increment instance twice must genuinely move
        // Value twice. This proves the atom isn't silently swallowing every
        // redispatch (which would make Law2's own positive tests above
        // vacuous) -- law 2 is about intents that are THEMSELVES idempotent,
        // not about Dispatch collapsing all repeats unconditionally.
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var changedCount = 0;
        atom.Changed += () => changedCount++;

        var increment = new Increment(5);
        atom.Dispatch(increment);
        atom.Dispatch(increment);

        Assert.Equal(new Counter(10), atom.Value);
        Assert.Equal(2, changedCount);
    }
}
