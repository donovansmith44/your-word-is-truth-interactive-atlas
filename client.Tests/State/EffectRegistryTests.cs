using System.Text.RegularExpressions;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

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
        firstValues.Clear();

        var secondClaim = registry.Claim(MakeEffect("counter-effect", atom, secondValues));
        secondValues.Clear();

        atom.Dispatch(new SetCounter(7));

        Assert.Empty(firstValues);
        Assert.Single(secondValues);
        Assert.Equal(new Counter(7), secondValues[0]);

        firstClaim.Dispose();
        secondClaim.Dispose();
    }

    [Fact]
    public void LatestClaimWins_SupersessionDoesNotRequireTheOlderClaimantToDisposeFirst()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var staleValues = new List<Counter>();
        var freshValues = new List<Counter>();

        _ = registry.Claim(MakeEffect("counter-effect", atom, staleValues));
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

        Assert.Single(values);
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
        var atom = new StateAtom<Counter>("counter", new Counter(42));
        var registry = new EffectRegistry();
        var values = new List<Counter>();

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

        var secondValues = new List<Counter>();
        var secondClaim = registry.Claim(MakeEffect("counter-effect", atom, secondValues));
        Assert.Single(secondValues);
        Assert.Equal(new Counter(5), secondValues[0]);
        secondClaim.Dispose();
    }

    [Fact]
    public void AppliesToGating_ClaimTimeReconcileIsSkippedWhenAppliesToIsFalse()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(-1));
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
        values.Clear();

        atom.Dispatch(new SetCounter(-5));
        Assert.Empty(values);

        atom.Dispatch(new SetCounter(2));
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

    [Fact]
    public void TwoRapidReClaims_DisposePriorClaimBeforeDispatchAndReClaim_ExactlyOneLiveMaterialization()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var values = new List<Counter>();
        var claim1 = registry.Claim(MakeEffect("e", atom, values));
        values.Clear();

        claim1.Dispose();
        atom.Dispatch(new SetCounter(5));
        var claim2 = registry.Claim(MakeEffect("e", atom, values));

        Assert.Single(values);
        Assert.Equal(new Counter(5), values[0]);
        claim2.Dispose();
    }

    [Fact]
    public void TwoRapidReClaims_DispatchWhileThePriorClaimIsStillHeldDoubleMaterializes_NegativeControl()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var values = new List<Counter>();
        var claim1 = registry.Claim(MakeEffect("e", atom, values));
        values.Clear();

        atom.Dispatch(new SetCounter(5));
        claim1.Dispose();
        var claim2 = registry.Claim(MakeEffect("e", atom, values));

        Assert.Equal(2, values.Count);
        claim2.Dispose();
    }

    [Fact]
    public void PlantedDuplicateEffectName_TwoUnrelatedEffectsSharingOneNameSupersedeAtTheRegistryLevel()
    {
        var atomA = new StateAtom<Counter>("counter-a", new Counter(1));
        var atomB = new StateAtom<Counter>("counter-b", new Counter(2));
        var registry = new EffectRegistry();
        var valuesA = new List<Counter>();
        var valuesB = new List<Counter>();

        var claimA = registry.Claim(MakeEffect("planted-duplicate", atomA, valuesA));
        var claimB = registry.Claim(MakeEffect("planted-duplicate", atomB, valuesB));

        valuesA.Clear();
        valuesB.Clear();
        atomA.Dispatch(new SetCounter(99));
        atomB.Dispatch(new SetCounter(100));

        Assert.Empty(valuesA);
        Assert.Single(valuesB);

        claimA.Dispose();
        claimB.Dispose();
    }

    [Fact]
    public void ReleaseByName_ReleasesWhoeverCurrentlyOwnsTheSlot_RegardlessOfWhichHandleWouldHaveReleasedIt()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var values = new List<Counter>();
        var claim = registry.Claim(MakeEffect("e", atom, values));
        values.Clear();

        registry.Release("e");

        atom.Dispatch(new SetCounter(5));
        Assert.Empty(values);

        claim.Dispose();
    }

    [Fact]
    public void ReleaseByName_ThenReClaim_ReconcilesNormallyAgainstTheCurrentValue()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(7));
        var registry = new EffectRegistry();
        var values = new List<Counter>();
        var claim1 = registry.Claim(MakeEffect("e", atom, values));
        values.Clear();

        registry.Release("e");
        var claim2 = registry.Claim(MakeEffect("e", atom, values));

        Assert.Single(values);
        Assert.Equal(new Counter(7), values[0]);

        claim1.Dispose();
        claim2.Dispose();
    }

    [Fact]
    public void ReleaseByName_OnAnUnclaimedNameIsASafeNoOp()
    {
        var registry = new EffectRegistry();
        var exception = Record.Exception(() => registry.Release("never-claimed"));
        Assert.Null(exception);
    }

    [Fact]
    public void ReleaseByName_ComposesAcrossTwoDifferentCallers_ClosesTheN1CrossInstanceGap()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var ownerValues = new List<Counter>();
        var claimA = registry.Claim(MakeEffect("follow-scene", atom, ownerValues));
        ownerValues.Clear();

        registry.Release("follow-scene");
        var callerOwnDirectPath = new List<Counter>();
        atom.Dispatch(new SetCounter(42));
        callerOwnDirectPath.Add(atom.Value);

        Assert.Empty(ownerValues);
        Assert.Single(callerOwnDirectPath);

        claimA.Dispose();
    }

    [Fact]
    public void DirectWriteWhileClaimed_AlsoMaterializesViaTheRegistry_DocumentedOriginBlindTrait()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var registryValues = new List<Counter>();
        var claim = registry.Claim(MakeEffect("e", atom, registryValues));
        registryValues.Clear();

        var callerOwnDirectPath = new List<Counter>();
        atom.Dispatch(new SetCounter(9));
        callerOwnDirectPath.Add(atom.Value);

        Assert.Single(registryValues);
        Assert.Single(callerOwnDirectPath);

        claim.Dispose();
    }

    [Fact]
    public void DirectWriteAfterRelease_MaterializesOnlyViaTheCallersOwnPath_NotTheRegistry()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var registryValues = new List<Counter>();
        var claim = registry.Claim(MakeEffect("e", atom, registryValues));
        registryValues.Clear();

        claim.Dispose();
        var callerOwnDirectPath = new List<Counter>();
        atom.Dispatch(new SetCounter(9));
        callerOwnDirectPath.Add(atom.Value);

        Assert.Empty(registryValues);
        Assert.Single(callerOwnDirectPath);
    }

    [Fact]
    public void NoDirectAtomSubscriptionByEffects_MaterializeIsOnlyEverCalledFromEffectRegistry()
    {
        var clientDir = ConformanceTests.ClientRoot;
        var materializeCallers = new List<string>();
        var pattern = new Regex(@"\.Materialize\(", RegexOptions.Compiled);

        foreach (var file in ConformanceTests.ClientSourceFiles())
        {
            var relative = Path.GetRelativePath(clientDir, file).Replace(Path.DirectorySeparatorChar, '/');
            var text = File.ReadAllText(file);
            if (pattern.IsMatch(text) && relative != "State/EffectRegistry.cs")
            {
                materializeCallers.Add(file);
            }
        }

        Assert.True(materializeCallers.Count == 0,
            "IStateEffect<T>.Materialize must only ever be invoked from client/State/EffectRegistry.cs (the registry's own Changed subscription/reconcile-on-claim call) -- " +
            "found direct call(s) in: " + string.Join(", ", materializeCallers));
    }
}
