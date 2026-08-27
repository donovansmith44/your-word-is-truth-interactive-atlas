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
    // Fix round 1 (Q-1 -- IMPORTANT, review): "add a registry property test
    // that would have caught the double-materialize window (two rapid
    // re-claims -> exactly one live materialization)." Reproduces
    // World.razor's own EnableFollowScene shape directly against the
    // registry -- correct order (dispose the prior claim BEFORE dispatching
    // + re-claiming) vs. the pre-fix-round wrong order (dispatch while the
    // prior claim is still held), as a positive/negative-control pair.
    // ------------------------------------------------------------------
    [Fact]
    public void TwoRapidReClaims_DisposePriorClaimBeforeDispatchAndReClaim_ExactlyOneLiveMaterialization()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var values = new List<Counter>();
        var claim1 = registry.Claim(MakeEffect("e", atom, values));
        values.Clear();

        // CORRECT order (the fixed EnableFollowScene): dispose FIRST, then
        // dispatch (no live claim, so nothing fires), then re-claim
        // (reconciles exactly once against the fresh value).
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
        // Reproduces the WRONG order (the pre-fix-round `EnableFollowScene`
        // shape: sync/dispatch BEFORE disposing the prior claim) directly
        // against the registry -- proves the mechanism WOULD have caught
        // Q-1 had this test existed first. Disclosed negative control, not
        // a registry defect: EffectRegistry's own contract is "materialize
        // when claimed and Source changes" -- it correctly does exactly
        // that; avoiding the double fire is the CALLER's own ordering
        // responsibility (fixed in World.razor, this same round).
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var values = new List<Counter>();
        var claim1 = registry.Claim(MakeEffect("e", atom, values));
        values.Clear();

        atom.Dispatch(new SetCounter(5)); // claim1's own handler fires -- materialization #1
        claim1.Dispose();
        var claim2 = registry.Claim(MakeEffect("e", atom, values)); // reconciles -- materialization #2

        Assert.Equal(2, values.Count); // the double-materialize window, reproduced on demand
        claim2.Dispose();
    }

    // ------------------------------------------------------------------
    // Fix round 1 (S-7 / ruling 6.ii -- controller, binding): "effect-name
    // uniqueness made real -- assert at the REGISTRY level across ALL
    // claims (a runtime-registry test that fails on a planted duplicate)."
    // ------------------------------------------------------------------
    [Fact]
    public void PlantedDuplicateEffectName_TwoUnrelatedEffectsSharingOneNameSupersedeAtTheRegistryLevel()
    {
        // Two GENUINELY DIFFERENT effects (different Source atoms -- the
        // shape a real naming COLLISION takes, as opposed to the SAME
        // effect legitimately re-claimed across component instances, which
        // always shares one Source) that happen to share a Name string --
        // exactly the accident EffectNames.cs (client/State/EffectNames.cs,
        // reflected over by ConformanceTests.cs) exists to prevent at the
        // SOURCE level. This test pins what happens at the REGISTRY level
        // if it ever occurs anyway: "one owner per name" means the SECOND
        // claimant silently steals ownership from the first, regardless of
        // whether they represent the same real-world thing -- a genuine,
        // runtime, planted-duplicate scenario, not a source-text scan.
        var atomA = new StateAtom<Counter>("counter-a", new Counter(1));
        var atomB = new StateAtom<Counter>("counter-b", new Counter(2));
        var registry = new EffectRegistry();
        var valuesA = new List<Counter>();
        var valuesB = new List<Counter>();

        var claimA = registry.Claim(MakeEffect("planted-duplicate", atomA, valuesA));
        var claimB = registry.Claim(MakeEffect("planted-duplicate", atomB, valuesB)); // same name, UNRELATED source -- the collision

        valuesA.Clear();
        valuesB.Clear();
        atomA.Dispatch(new SetCounter(99)); // A's own claim was superseded by B's claim -- A's handler must NOT fire
        atomB.Dispatch(new SetCounter(100));

        Assert.Empty(valuesA); // superseded -- the collision is real and DETECTABLE via this exact test shape
        Assert.Single(valuesB);

        claimA.Dispose();
        claimB.Dispose();
    }

    // ------------------------------------------------------------------
    // Fix round 2 (N-1 -- IMPORTANT, re-review): `EffectRegistry.Release(name)`
    // -- the registry-level "release-before-direct-write" primitive that
    // replaces World.razor's own instance-local `DisableFollowScene()` call
    // at `EnterScriptureMode`'s own choke point. Proves the property that
    // matters: a direct writer can force "no one currently owns this name"
    // WITHOUT knowing or caring whether IT is the owner -- composing across
    // instances, unlike a per-instance claim handle.
    // ------------------------------------------------------------------
    [Fact]
    public void ReleaseByName_ReleasesWhoeverCurrentlyOwnsTheSlot_RegardlessOfWhichHandleWouldHaveReleasedIt()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var values = new List<Counter>();
        var claim = registry.Claim(MakeEffect("e", atom, values)); // claim held by "someone" -- the caller below never touches this handle at all
        values.Clear();

        registry.Release("e"); // registry-level release, by NAME -- not claim.Dispose()

        atom.Dispatch(new SetCounter(5));
        Assert.Empty(values); // the still-undisposed claim handle's own handler no longer fires -- released regardless of who held it

        claim.Dispose(); // safe afterward -- the token no longer matches, so this is a no-op on the (already-cleared) slot
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
        var claim2 = registry.Claim(MakeEffect("e", atom, values)); // a fresh claim after a by-name release reconciles exactly like any other claim

        Assert.Single(values);
        Assert.Equal(new Counter(7), values[0]);

        claim1.Dispose(); // stale handle from before the release -- harmless no-op
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
        // Reproduces N-1's own named reachable shape directly: caller B does
        // NOT hold the "follow-scene" claim (instance A does) but still
        // needs "a direct write materializes exactly once" to hold. Before
        // this fix, only A's own claim.Dispose() could release A's claim --
        // B had no way to prevent A's own Materialize from ALSO firing
        // alongside B's own direct effect. With Release(name), B (or ANY
        // caller) can guarantee no one is claimed before its own direct
        // write, regardless of who owned it.
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var ownerValues = new List<Counter>(); // instance A's own materialized values
        var claimA = registry.Claim(MakeEffect("follow-scene", atom, ownerValues));
        ownerValues.Clear();

        // Caller B's own direct-write choke point: release by name FIRST
        // (B never touches claimA at all), then dispatch.
        registry.Release("follow-scene");
        var callerOwnDirectPath = new List<Counter>();
        atom.Dispatch(new SetCounter(42));
        callerOwnDirectPath.Add(atom.Value);

        Assert.Empty(ownerValues); // A's own Materialize did NOT also fire -- the cross-instance double-fetch N-1 named is closed
        Assert.Single(callerOwnDirectPath); // materializes exactly once, from B's own direct path

        claimA.Dispose();
    }

    // ------------------------------------------------------------------
    // Fix round 1 (S-4 -- IMPORTANT, review): "release-before-direct-write"
    // invariant, pinned at the registry level (World.razor's own
    // EnterScriptureMode fix -- calling DisableFollowScene first -- is the
    // production application of this same fact). EffectRegistry's own
    // AppliesTo gate is origin-blind BY DESIGN (it only ever sees the atom's
    // VALUE, never which write produced it) -- both tests below pin that
    // fact honestly, as documented behavior, not a defect: the first proves
    // a direct write WHILE claimed also materializes through the registry
    // (the double-fetch hazard, if a caller does not release first); the
    // second proves a direct write AFTER release does not.
    // ------------------------------------------------------------------
    [Fact]
    public void DirectWriteWhileClaimed_AlsoMaterializesViaTheRegistry_DocumentedOriginBlindTrait()
    {
        var atom = new StateAtom<Counter>("counter", new Counter(0));
        var registry = new EffectRegistry();
        var registryValues = new List<Counter>();
        var claim = registry.Claim(MakeEffect("e", atom, registryValues));
        registryValues.Clear();

        var callerOwnDirectPath = new List<Counter>();
        atom.Dispatch(new SetCounter(9)); // a "direct, non-link-derived" write -- e.g. EnterScriptureMode's own dispatch, BEFORE this batch's own release-first fix
        callerOwnDirectPath.Add(atom.Value); // the caller's own unconditional effect (RunScriptureModeEffect's own analogue)

        Assert.Single(registryValues); // the registry ALSO fired -- the hazard S-4 names, reproduced
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

        // The fixed EnterScriptureMode shape: release BEFORE the direct write.
        claim.Dispose();
        var callerOwnDirectPath = new List<Counter>();
        atom.Dispatch(new SetCounter(9));
        callerOwnDirectPath.Add(atom.Value);

        Assert.Empty(registryValues); // no live claim -- the registry never fires
        Assert.Single(callerOwnDirectPath); // materializes exactly once, from the caller's own path
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

            // Fix round 1 (cheap hardening, review's own caveat on this
            // test): excludes by RELATIVE PATH now, not bare basename -- a
            // second file named "EffectRegistry.cs" anywhere else under
            // client/ would previously have been wrongly exempted too.
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
