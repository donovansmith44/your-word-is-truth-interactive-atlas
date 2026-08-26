using BibleAtlas.Client.Contracts;

namespace BibleAtlas.Client.Tests.State;

// ============================================================================
// Batch ST-1: synthetic atom/intent vocabulary + hand-rolled, seeded
// generators for the five-laws property tests in this directory. These
// exercise the GENERIC atom/projection/link layer (client/State/*.cs) with
// throwaway types -- not Locus/TimeWindow -- so the laws are proven for the
// infrastructure itself, independent of any one migration (LocusAndTimeWindowTests.cs,
// alongside these, separately re-checks the concrete Locus/SetLocus shapes).
//
// House law (batch-st1-brief.md): "never wall-clock random; no new test
// dependency without disclosure -- prefer none." DISCLOSED: no new package
// was added. System.Random seeded with a fixed, checked-in literal is
// deterministic and reproducible across runs/machines by construction (.NET's
// Random(int seed) algorithm is a stable, documented PRNG, not wall-clock
// based) -- exactly "seeded, deterministic, hand-rolled," with zero new
// dependency surface.
// ============================================================================

/// <summary>Counter -- the synthetic atom VALUE type every law test below
/// shares.</summary>
public sealed record Counter(int Value);

/// <summary>Absolute-set intent -- genuinely idempotent (Apply ignores
/// `current` entirely), used wherever a test needs an intent that honors
/// each intent's own law-2 obligation (see IIntent{T}'s own doc comment).</summary>
public sealed record SetCounter(int Value, string? Origin = null) : IIntent<Counter>
{
    public string Name => "set-counter";
    public Counter Apply(Counter current) => new(Value);
}

/// <summary>Absolute-set intent (a fixed zero) -- same idempotence shape as
/// <see cref="SetCounter"/>, kept separate so generated sequences exercise
/// more than one intent SHAPE (mirroring "small records per intent shape,"
/// the brief's own wording for the real Locus/TimeWindow intents).</summary>
public sealed record ResetCounter(string? Origin = null) : IIntent<Counter>
{
    public string Name => "reset-counter";
    public Counter Apply(Counter current) => new(0);
}

/// <summary>Delta intent -- DELIBERATELY NOT idempotent (Apply(Apply(v)) !=
/// Apply(v) whenever Amount != 0): used only in confluence/agreement/no-echo
/// tests, which don't require per-intent idempotence, NEVER in the
/// idempotence-law test itself (redispatching the same Increment instance
/// twice is correctly NOT a no-op -- that would be testing a law violation,
/// not the law).</summary>
public sealed record Increment(int Amount, string? Origin = null) : IIntent<Counter>
{
    public string Name => "increment";
    public Counter Apply(Counter current) => current with { Value = current.Value + Amount };
}

/// <summary>A pure-delegate <see cref="IStateLink{A,B}"/> for tests that need
/// a link shape the production code doesn't otherwise have a reason to
/// define (e.g. a deliberately-oscillation-prone pair for the no-echo law).
/// Production links (<see cref="BibleAtlas.Client.State.FollowTextLink"/>)
/// stay their own named, documented types -- this exists ONLY so law tests
/// can construct arbitrary Derive/Active shapes without a proliferation of
/// tiny one-off link classes.</summary>
internal sealed class DelegateLink<A, B> : IStateLink<A, B>
{
    private readonly Func<A, B, B> _derive;
    private readonly Func<bool> _active;

    public DelegateLink(IStateAtom<A> source, IStateAtom<B> target, Func<A, B, B> derive, Func<bool> active)
    {
        Source = source;
        Target = target;
        _derive = derive;
        _active = active;
    }

    public IStateAtom<A> Source { get; }
    public IStateAtom<B> Target { get; }
    public B Derive(A source, B current) => _derive(source, current);
    public bool Active => _active();
}

internal static class Generators
{
    /// <summary>Only genuinely idempotent intents (SetCounter/ResetCounter)
    /// -- for the idempotence-law test.</summary>
    public static IReadOnlyList<IIntent<Counter>> IdempotentCounterIntents(int seed, int count)
    {
        var rng = new Random(seed);
        var list = new List<IIntent<Counter>>(count);
        for (var i = 0; i < count; i++)
        {
            list.Add(rng.Next(2) == 0 ? new SetCounter(rng.Next(-100, 100)) : new ResetCounter());
        }

        return list;
    }

    /// <summary>A richer mix, including the non-idempotent Increment -- for
    /// confluence/agreement, where per-intent idempotence isn't required.</summary>
    public static IReadOnlyList<IIntent<Counter>> MixedCounterIntents(int seed, int count)
    {
        var rng = new Random(seed);
        var list = new List<IIntent<Counter>>(count);
        for (var i = 0; i < count; i++)
        {
            list.Add(rng.Next(3) switch
            {
                0 => new SetCounter(rng.Next(-100, 100)),
                1 => new ResetCounter(),
                _ => new Increment(rng.Next(-20, 20)),
            });
        }

        return list;
    }

    /// <summary>Seeded (book, chapter) pairs -- for LocusAndTimeWindowTests.cs's own
    /// generated sequences over the REAL SetLocus/Locus shapes.</summary>
    public static IReadOnlyList<(string Book, int Chapter)> LocusSequence(int seed, int count)
    {
        var books = new[] { "GEN", "EXO", "LEV", "PSA", "MAT", "REV", "JOS" };
        var rng = new Random(seed);
        var list = new List<(string, int)>(count);
        for (var i = 0; i < count; i++)
        {
            list.Add((books[rng.Next(books.Length)], rng.Next(1, 50)));
        }

        return list;
    }
}
