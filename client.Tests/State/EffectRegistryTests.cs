using System.Text.RegularExpressions;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// Batch ST-3 (R1/R5): property tests for <see cref="EffectRegistry"/> --
/// the ONE concrete <see cref="IEffectRegistry"/> implementation -- proving
/// every semantic <see cref="IStateEffect{T}"/>'s own doc comment (client/
/// Contracts/State.cs) names: one owner per effect name, latest-claim-wins,
/// release-on-dispose, reconcile-on-claim, AppliesTo gating. Uses the SAME
/// synthetic Counter/SetCounter vocabulary as StateAtomLawTests.cs
/// (Generators.cs) over a small, purpose-built <see cref="DelegateEffect{T}"/>
/// -- no new production types needed for these tests beyond what R1 already
/// ships.
/// </summary>
public class EffectRegistryTests
{
    private static DelegateEffect<Counter> MakeEffect(
        string name, StateAtom<Counter> atom, List<Counter> materializedValues, Func<Counter, bool>? appliesTo = null) =>
        new(name, atom, appliesTo ?? (_ => true), value =>
        {
            materializedValues.Add(value);
            return Task.CompletedTask;
        });

    [Fact]
    public void LatestClaimWins_TwoClaimantsOfTheSameName_OnlyTheNewestMaterializesOnAFutureChange()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var firstValues = new List<Counter>();
        var secondValues = new List<Counter>();

        var firstClaim = registry.Claim(MakeEffect("counter-effect", atom, firstValues));
        firstValues.Clear(); // discard the reconcile-on-claim call -- this test is about ONGOING changes after the second claim supersedes

        var secondClaim = registry.Claim(MakeEffect("counter-effect", atom, secondValues));
        secondValues.Clear(); // discard its own reconcile-on-claim call too, same reasoning

        atom.Dispatch(new SetCounter(7)); // a genuine change -- both claimants' Source.Changed subscriptions are still WIRED

        Assert.Empty(firstValues); // the superseded claimant's handler is permanently inert
        Assert.Single(secondValues);
        Assert.Equal(new Counter(7), secondValues[0]);

        firstClaim.Dispose();
        secondClaim.Dispose();
    }

    [Fact]
    public void LatestClaimWins_SupersessionDoesNotRequireTheOlderClaimantToDisposeFirst()
    {
        // The exact hazard ST-2's own review (Adjudication A) named: a stale
        // component's own Dispose may never run, or may run arbitrarily
        // late, because of Blazor's undocumented dispose-vs-mount ordering.
        // Supersession must hold even when the OLDER claim is NEVER disposed
        // at all within the scope of this test.
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var staleValues = new List<Counter>();
        var freshValues = new List<Counter>();

        _ = registry.Claim(MakeEffect("counter-effect", atom, staleValues)); // never disposed -- deliberate
        staleValues.Clear();
        var fresh = registry.Claim(MakeEffect("counter-effect", atom, freshValues));
        freshValues.Clear();

        atom.Dispatch(new SetCounter(3));
        atom.Dispatch(new SetCounter(9));

        Assert.Empty(staleValues);
        Assert.Equal(new[] { new Counter(3), new Counter(9) }, freshValues);

        fresh.Dispose();
    }

    [Fact]
    public void ReleaseOnDispose_ADisposedOwnersEffectNeverRunsAgain()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var values = new List<Counter>();

        var claim = registry.Claim(MakeEffect("counter-effect", atom, values));
        values.Clear();

        atom.Dispatch(new SetCounter(1));
        Assert.Single(values);

        claim.Dispose();
        atom.Dispatch(new SetCounter(2));
        atom.Dispatch(new SetCounter(3));

        Assert.Single(values); // still just the one, pre-dispose materialization
    }

    [Fact]
    public void ReleaseOnDispose_IsIdempotent_ASecondDisposeIsAHarmlessNoOp()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var values = new List<Counter>();
        var claim = registry.Claim(MakeEffect("counter-effect", atom, values));

        claim.Dispose();
        var exception = Record.Exception(() => claim.Dispose());

        Assert.Null(exception);
    }

    [Fact]
    public void ReconcileOnClaim_AnAlreadyConvergedAtomStillMaterializesOnceAtClaimTime()
    {
        // Law 2 (idempotence): dispatching a value the atom ALREADY holds
        // raises no Changed at all. A brand-new claim must still see the
        // CURRENT value once -- this is the exact mechanism that replaces
        // ST-2's own hand-rolled TriggerFollowSync fix for the blank-atlas-
        // pane regression (S-1).
        var atom = new StateAtom<Counter>("counter", new Counter(42));
        var registry = new EffectRegistry();
        var values = new List<Counter>();

        // No Dispatch happens between atom construction and Claim -- the
        // atom is already sitting at its "converged" value, matching a
        // DI-singleton atom a fresh component mounts against.
        var claim = registry.Claim(MakeEffect("counter-effect", atom, values));

        Assert.Single(values);
        Assert.Equal(new Counter(42), values[0]);
        Assert.NotNull(claim.ReconcileTask);

        claim.Dispose();
    }

    [Fact]
    public void ReconcileOnClaim_RunsAgainForEachNewClaim_EvenWhenTheValueNeverChangedBetweenThem()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(5));
        var registry = new EffectRegistry();

        var firstValues = new List<Counter>();
        var firstClaim = registry.Claim(MakeEffect("counter-effect", atom, firstValues));
        Assert.Single(firstValues);
        firstClaim.Dispose();

        // A second claim, later, with the atom's value UNCHANGED in between
        // (mirrors: close a split pane at ref X, reopen at the SAME ref X --
        // S-1's own exact repro).
        var secondValues = new List<Counter>();
        var secondClaim = registry.Claim(MakeEffect("counter-effect", atom, secondValues));
        Assert.Single(secondValues);
        Assert.Equal(new Counter(5), secondValues[0]);
        secondClaim.Dispose();
    }

    [Fact]
    public void AppliesToGating_ClaimTimeReconcileIsSkippedWhenAppliesToIsFalse()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(-1)); // negative -- AppliesTo below only accepts non-negative
        var registry = new EffectRegistry();
        var values = new List<Counter>();

        var claim = registry.Claim(MakeEffect("counter-effect", atom, values, v => v.Value >= 0));

        Assert.Empty(values);
        Assert.Null(claim.ReconcileTask);

        claim.Dispose();
    }

    [Fact]
    public void AppliesToGating_OngoingChangesThatDoNotApplyNeverMaterialize()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var values = new List<Counter>();

        var claim = registry.Claim(MakeEffect("counter-effect", atom, values, v => v.Value >= 0));
        values.Clear(); // reconcile-on-claim (AppliesTo(0) is true) -- not this test's own subject

        atom.Dispatch(new SetCounter(-5)); // does not apply
        Assert.Empty(values);

        atom.Dispatch(new SetCounter(2)); // applies
        Assert.Single(values);
        Assert.Equal(new Counter(2), values[0]);

        claim.Dispose();
    }

    [Fact]
    public void DifferentEffectNames_ClaimIndependently_NeitherSupersedesTheOther()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var aValues = new List<Counter>();
        var bValues = new List<Counter>();

        var claimA = registry.Claim(MakeEffect("effect-a", atom, aValues));
        var claimB = registry.Claim(MakeEffect("effect-b", atom, bValues));
        aValues.Clear();
        bValues.Clear();

        atom.Dispatch(new SetCounter(1));

        Assert.Single(aValues);
        Assert.Single(bValues);

        claimA.Dispose();
        claimB.Dispose();
    }

    // ------------------------------------------------------------------
    // "no direct atom subscription by effects" (R5: "grep-able assertion is
    // fine") -- a source-text scan proving the ONE place any Source.Changed
    // subscription that calls IStateEffect<T>.Materialize lives is
    // EffectRegistry.cs itself. Every OTHER client/State/*.cs and
    // client/Pages/*.razor/client/Components/*.razor file must not call
    // `.Materialize(` at all -- if a future edit adds a second, direct
    // subscription, this test goes red.
    // ------------------------------------------------------------------
    [Fact]
    public void NoDirectAtomSubscriptionByEffects_MaterializeIsOnlyEverCalledFromEffectRegistry()
    {
        var repoRoot = FindRepoRoot();
        var clientDir = Path.Combine(repoRoot, "client");
        Assert.True(Directory.Exists(clientDir), $"expected a client/ directory under {repoRoot}");

        var materializeCallers = new List<string>();
        var pattern = new Regex(@"\.Materialize\(", RegexOptions.Compiled);

        foreach (var file in Directory.EnumerateFiles(clientDir, "*.cs", SearchOption.AllDirectories)
                     .Concat(Directory.EnumerateFiles(clientDir, "*.razor", SearchOption.AllDirectories)))
        {
            if (file.Contains($"{Path.DirectorySeparatorChar}obj{Path.DirectorySeparatorChar}") ||
                file.Contains($"{Path.DirectorySeparatorChar}bin{Path.DirectorySeparatorChar}"))
            {
                continue;
            }

            var text = File.ReadAllText(file);
            if (pattern.IsMatch(text) && Path.GetFileName(file) != "EffectRegistry.cs")
            {
                materializeCallers.Add(file);
            }
        }

        Assert.True(materializeCallers.Count == 0,
            "IStateEffect<T>.Materialize must only ever be invoked from client/State/EffectRegistry.cs (the registry's own Changed subscription/reconcile-on-claim call) -- " +
            "found direct call(s) in: " + string.Join(", ", materializeCallers));
    }

    private static string FindRepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null && !Directory.Exists(Path.Combine(dir.FullName, "client")))
        {
            dir = dir.Parent;
        }

        Assert.NotNull(dir);
        return dir!.FullName;
    }
}
