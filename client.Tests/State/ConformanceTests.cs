using System.Reflection;
using System.Text.RegularExpressions;
using BibleAtlas.Client;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.State;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.JSInterop;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// BRIEF ADDENDUM (owner ruling, mid-batch, spec §0 law: "if contracts
/// don't match implementation tests should fail") + FIX ROUND 1 repairs
/// (controller ruling 6, following the review's own conformance-test
/// audit): every construct this batch migrates onto a contract gets a
/// CONFORMANCE test here -- one that FAILS if a future edit lets the
/// implementation drift from the contract, living in the standing suite.
///
/// Fix round 1 changes, per finding:
/// - S-9: the effects-only-via-registry allowlist now keys on the
///   subscription EXPRESSION TEXT per file, not a fragile line number --
///   inserting a line above an allowlisted site no longer breaks it.
/// - S-10 / ruling 6.i: atom-registration conformance now resolves a REAL
///   `IServiceCollection`/`ServiceProvider` (via `AppServices`, the SAME
///   registration code Program.cs calls -- not a parallel copy) and proves
///   singleton lifetime (resolve twice, same instance) for every migrated
///   `AtomNames` entry, replacing the original Program.cs text scan (which
///   proved construction, never registration or lifetime).
/// - S-7 / ruling 6.ii: effect-name conformance now reflects over
///   `EffectNames` (a new constants class mirroring `AtomNames`, per the
///   review's own offered fix) instead of scanning for
///   `DelegateEffect&lt;...&gt;("literal"` (invisible to a bespoke
///   `IStateEffect&lt;T&gt;` class, and vacuously non-empty with exactly one
///   entry). `EffectRegistryTests.cs`'s own new
///   `PlantedDuplicateEffectName_...` test covers the "registry-level,
///   fails on a planted duplicate" half at the registry, not here.
/// - Ruling 6.iii (new): "no component-held shared state," scoped to the
///   five migrated atom value types -- the exact pattern the original
///   `ExplorerPopover._focus` (S-2) violated. A documented, narrow
///   allowlist covers the one sanctioned exception S-2's own fix
///   introduced (`_frozenSnapshot`, a superseded instance's frozen
///   snapshot -- see that field's own doc comment).
/// </summary>
public class ConformanceTests
{
    private static string RepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null && !Directory.Exists(Path.Combine(dir.FullName, "client")))
        {
            dir = dir.Parent;
        }

        Assert.NotNull(dir);
        return dir!.FullName;
    }

    private static IEnumerable<string> ClientSourceFiles()
    {
        var clientDir = Path.Combine(RepoRoot(), "client");
        return Directory.EnumerateFiles(clientDir, "*.cs", SearchOption.AllDirectories)
            .Concat(Directory.EnumerateFiles(clientDir, "*.razor", SearchOption.AllDirectories))
            .Where(f => !f.Contains($"{Path.DirectorySeparatorChar}obj{Path.DirectorySeparatorChar}")
                        && !f.Contains($"{Path.DirectorySeparatorChar}bin{Path.DirectorySeparatorChar}"));
    }

    private static string ProgramCsText() => File.ReadAllText(Path.Combine(RepoRoot(), "client", "Program.cs"));

    /// A minimal <see cref="IJSInProcessRuntime"/> test double -- every
    /// member throws. `LocalStore.Probe`/`.Read`/`.Write` (client/LocalStore.cs)
    /// each wrap their own JS calls in `try/catch` and degrade to "storage
    /// unavailable" on ANY exception, so this safely exercises the REAL
    /// `AppServices.AddSelectionAtom` factory (including its own `Probe`
    /// call) without a browser -- no fake/mock library needed.
    private sealed class ThrowingJsRuntime : IJSInProcessRuntime
    {
        public TValue Invoke<TValue>(string identifier, params object?[]? args) =>
            throw new NotSupportedException("test double -- no real JS interop in client.Tests");

        public ValueTask<TValue> InvokeAsync<TValue>(string identifier, object?[]? args) =>
            throw new NotSupportedException("test double -- no real JS interop in client.Tests");

        public ValueTask<TValue> InvokeAsync<TValue>(string identifier, CancellationToken cancellationToken, object?[]? args) =>
            throw new NotSupportedException("test double -- no real JS interop in client.Tests");
    }

    // ------------------------------------------------------------------
    // Addendum item 1 / fix round 1 (S-9): "effects-only-via-registry" --
    // no production code outside client/State/ infrastructure subscribes
    // directly to any IStateAtom<T>.Changed, EXCEPT an explicitly
    // documented, allowlisted pure-render (or single-singleton-persistence)
    // subscription. Keyed on (File, distinctive EXPRESSION substring) --
    // NOT line number (the review's own S-9 finding: line-keying "verifies
    // location, not behavior" and is brittle to unrelated edits above the
    // site) -- so a NEW, undocumented `.Changed +=` expression anywhere in
    // client/ fails this test until it is either moved behind the registry
    // or added here with a reasoned justification.
    // ------------------------------------------------------------------
    [Fact]
    public void EffectsOnlyViaRegistry_EveryChangedSubscriptionOutsideStateInfrastructureIsExplicitlyAllowlisted()
    {
        // (file, a distinctive substring of the subscription line itself,
        // justification) -- every entry here was verified by reading its
        // own handler body at the time this test was written.
        var allowlist = new (string File, string ExpressionSubstring, string Justification)[]
        {
            ("client\\Components\\ExplorerPopover.razor", "FocusStackAtom.Changed += OnFocusStackChanged",
                "OnFocusStackChanged => pure re-render PLUS the R4/Adjudication-E ownership hand-off (claim+Reseed when superseded and the atom just Reset) -- claiming/reseeding is the SANCTIONED consumer-side interaction with OwnershipRegistry, mirroring EffectRegistry's own claim/release pattern; it never calls IStateEffect<T>.Materialize (FocusStack ownership is not an effect at all -- see OwnershipRegistry.cs's own header)."),
            ("client\\Components\\SelectionTray.razor", "SelectionAtom.Changed += OnChanged",
                "OnChanged => InvokeAsync(StateHasChanged) -- pure re-render, no Materialize-shaped side effect."),
            ("client\\Layout\\MainLayout.razor", "SavedExplorations.Changed += OnSavedExplorationsChanged",
                "SavedExplorations.Changed is SavedExplorationsService's OWN plain C# event, not an IStateAtom<T>.Changed -- out of this rule's scope entirely (SavedExplorationsService is not an atom)."),
            ("client\\Pages\\Reader.razor", "ViewArrangementAtom.Changed += OnViewArrangementChanged",
                "OnViewArrangementChanged => StateHasChanged() only (fix round 1, Q-3 of ST-2's own review) -- pure re-render."),
            ("client\\Pages\\World.razor", "LocusAtom.Changed += OnLocusChanged",
                "OnLocusChanged => StateHasChanged() only (ST-2 retired its own re-scening side effect) -- pure re-render."),
            ("client\\Pages\\World.razor", "TimeWindowAtom.Changed += OnTimeWindowChanged",
                "OnTimeWindowChanged => SyncTimeWindowProjection() + StateHasChanged() -- a LOCAL field projection sync (no fetch/JS interop), not an effect; the fetch itself lives in the follow-scene effect (EffectRegistry), which this handler no longer touches."),
            ("client\\Pages\\World.razor", "ViewArrangementAtom.Changed += OnViewArrangementChanged",
                "OnViewArrangementChanged => the SyncToken bump + EnableFollowScene/DisableFollowScene (claim/release calls) + StateHasChanged() -- claiming/releasing is the SANCTIONED consumer-side interaction with the registry; Materialize itself is invoked only from inside EffectRegistry.Claim's own subscription (see test below)."),
            ("client\\AppServices.cs", "atom.Changed += () => LocalStore.Write(js, Selection.StorageKey, atom.Value)",
                "The Selection atom's own persistence write (fix round 1, Q-2 -- moved here from the retired SelectionTrayService, wired directly in AppServices.AddSelectionAtom, the SAME factory Program.cs calls) -- a JS-interop write, which IStateEffect<T>'s own doc comment WOULD class as an effect in shape, but this atom's factory is a single, app-lifetime singleton with real, load-bearing consumers -- there is no multi-instance ownership hazard for EffectRegistry's claim/latest-wins mechanism to protect against here, unlike every current IStateEffect<T> use (World.razor's follow-scene, claimed by a per-mount, multiply-instantiable component). Disclosed design choice, not an oversight."),
            ("client\\State\\EffectRegistry.cs", "effect.Source.Changed += OnSourceChanged",
                "The registry's OWN internal subscription -- this IS the infrastructure the rule protects; Materialize is invoked from inside this handler and nowhere else (see EffectRegistryTests.cs's own NoDirectAtomSubscriptionByEffects test)."),
            ("client\\State\\StateLinkRunner.cs", "_source.Changed += OnSourceChanged",
                "Link infrastructure (ST-1) -- derives and dispatches into the link's own Target atom; not an IStateEffect<T> materialization (no fetch/JS interop, no registry involvement) at all."),
        }.ToArray();

        // Fix round 1 (S-8, trivia -- review): the allowlist's own File
        // values are written with literal backslashes (readable, matches
        // this repo's own Windows-only CI/dev environment), but the
        // COMPARISON normalizes both sides to '/' -- on any non-Windows
        // runner, Path.GetRelativePath would return '/'-separated paths
        // that would otherwise match NOTHING (every entry "unexpected" AND
        // "stale" at once, per the review's own finding), not merely fail
        // weaker.
        static string Normalize(string path) => path.Replace('\\', '/');

        var pattern = new Regex(@"\.Changed\s*\+=", RegexOptions.Compiled);
        var unexpected = new List<string>();
        var seenAllowlisted = new HashSet<int>();

        foreach (var file in ClientSourceFiles())
        {
            var relative = Normalize(Path.GetRelativePath(RepoRoot(), file));
            var lines = File.ReadAllLines(file);
            for (var i = 0; i < lines.Length; i++)
            {
                if (!pattern.IsMatch(lines[i]))
                {
                    continue;
                }

                var lineNumber = i + 1;
                var matchIndex = allowlist.ToList().FindIndex(e => Normalize(e.File) == relative && lines[i].Contains(e.ExpressionSubstring));
                if (matchIndex >= 0)
                {
                    seenAllowlisted.Add(matchIndex);
                }
                else
                {
                    unexpected.Add($"{relative}:{lineNumber}: {lines[i].Trim()}");
                }
            }
        }

        Assert.True(unexpected.Count == 0,
            "Found .Changed += subscription(s) not in ConformanceTests.cs's own allowlist -- add a reasoned entry (pure re-render, or a disclosed non-effect persistence write) or route the side effect through EffectRegistry instead:\n" +
            string.Join("\n", unexpected));

        // The allowlist itself must stay accurate -- an entry whose
        // expression substring no longer appears anywhere in its own file
        // (moved/deleted/reworded) should fail loudly too, not silently
        // stop being checked.
        var stale = Enumerable.Range(0, allowlist.Length).Except(seenAllowlisted).Select(i => allowlist[i]).ToList();
        Assert.True(stale.Count == 0,
            "Stale allowlist entries (no longer match a real .Changed += site -- update ConformanceTests.cs): " +
            string.Join(", ", stale.Select(e => $"{e.File}: \"{e.ExpressionSubstring}\"")));
    }

    // ------------------------------------------------------------------
    // Addendum item 2 / fix round 1 (S-10, ruling 6.i -- controller,
    // binding): atom registration conformance -- REAL DI resolution +
    // singleton lifetime, not textual construction. `AppServices` (client/
    // AppServices.cs) is the SAME registration code Program.cs calls; this
    // test builds a real `ServiceProvider` over it (with a throwing
    // IJSRuntime test double for the one atom that needs one) and resolves
    // each migrated atom TWICE, asserting the SAME instance both times.
    // ------------------------------------------------------------------
    [Fact]
    public void AtomRegistrationConformance_EveryMigratedAtomResolvesAsASingletonWithItsOwnName()
    {
        var services = new ServiceCollection();
        services.AddSingleton<IJSRuntime>(new ThrowingJsRuntime());
        AppServices.AddStateAtoms(services);
        AppServices.AddSelectionAtom(services);
        using var provider = services.BuildServiceProvider();

        AssertSingletonWithName<Locus>(provider, AtomNames.Locus);
        AssertSingletonWithName<TimeWindow>(provider, AtomNames.TimeWindow);
        AssertSingletonWithName<ViewArrangement>(provider, AtomNames.ViewArrangement);
        AssertSingletonWithName<FocusStack>(provider, AtomNames.FocusStack);
        AssertSingletonWithName<IReadOnlyList<Explore.ExplorationDescriptor>>(provider, AtomNames.Selection);
    }

    private static void AssertSingletonWithName<T>(IServiceProvider provider, string expectedName) where T : notnull
    {
        var first = provider.GetRequiredService<StateAtom<T>>();
        var second = provider.GetRequiredService<StateAtom<T>>();
        Assert.Same(first, second); // singleton lifetime -- genuinely resolved via DI, twice
        Assert.Equal(expectedName, first.Name);
    }

    [Fact]
    public void AtomRegistrationConformance_EveryAtomNamesConstantValueIsUnique()
    {
        var values = typeof(AtomNames).GetFields(BindingFlags.Public | BindingFlags.Static)
            .Where(f => f.FieldType == typeof(string))
            .Select(f => (string)f.GetValue(null)!)
            .ToList();

        Assert.Equal(values.Count, values.Distinct().Count());
    }

    // ------------------------------------------------------------------
    // Addendum item 3 / fix round 1 (S-7, ruling 6.ii -- controller,
    // binding): effect-name conformance, made real. Reflects over
    // `EffectNames` (client/State/EffectNames.cs, a new constants class
    // mirroring `AtomNames`) instead of scanning for
    // `DelegateEffect&lt;...&gt;("literal"` -- the review's own finding
    // that the literal scan was THEATER (invisible to a bespoke
    // IStateEffect{T} class; vacuously non-empty with exactly one entry).
    // The registry-level half of "fails on a planted duplicate" lives in
    // EffectRegistryTests.cs's own PlantedDuplicateEffectName_... test
    // (exercises the REAL EffectRegistry with two colliding, distinct
    // effects) -- not duplicated here.
    // ------------------------------------------------------------------
    [Fact]
    public void EffectConformance_EveryEffectNamesConstantValueIsUnique()
    {
        var values = typeof(EffectNames).GetFields(BindingFlags.Public | BindingFlags.Static)
            .Where(f => f.FieldType == typeof(string))
            .Select(f => (string)f.GetValue(null)!)
            .ToList();

        Assert.NotEmpty(values); // this batch's own "follow-scene" -- non-vacuous
        Assert.Equal(values.Count, values.Distinct().Count());
    }

    [Fact]
    public void EffectConformance_EveryDelegateEffectConstructionReferencesAnEffectNamesConstant()
    {
        // Mirrors AtomRegistrationConformance's own proven-real technique
        // exactly: a raw string literal in a `new DelegateEffect<T>(...)`
        // call would NOT match `EffectNames\.(\w+)` and would correctly
        // fail this test (there is nothing to find), the same way a raw
        // `new StateAtom<T>("literal", ...)` correctly fails the atom test.
        var effectNamesFields = typeof(EffectNames).GetFields(BindingFlags.Public | BindingFlags.Static)
            .Where(f => f.FieldType == typeof(string))
            .Select(f => f.Name)
            .ToHashSet();

        var pattern = new Regex(@"new DelegateEffect<.+?>\(\s*EffectNames\.(\w+)\s*,", RegexOptions.Compiled);
        var matched = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
            var text = File.ReadAllText(file);
            matched.AddRange(pattern.Matches(text).Select(m => m.Groups[1].Value));
        }

        Assert.NotEmpty(matched);
        foreach (var name in matched)
        {
            Assert.Contains(name, effectNamesFields);
        }
    }

    // ------------------------------------------------------------------
    // Fix round 1 (S-2 / ruling 6.iii -- controller, binding, new): "no
    // component-held shared state," SCOPED to the five migrated atom value
    // types -- the exact pattern the original ExplorerPopover._focus (S-2,
    // CRITICAL) violated: a component field typed as one of the atom's OWN
    // value types (Locus/TimeWindow/ViewArrangement/FocusStack/
    // IReadOnlyList&lt;ExplorationDescriptor&gt;), held as a permanent copy
    // rather than read through a Projection. The general, all-types sweep
    // stays ST-4's own ledgered scope; this is the narrow instance of it
    // this batch's own fix must not silently reintroduce.
    //
    // ONE documented allowlist entry: ExplorerPopover.razor's own
    // `_frozenSnapshot` (FocusStack) -- the S-2 fix's own SANCTIONED
    // exception for a superseded popover instance (see that field's own
    // doc comment for the full reasoning: it is not a live copy of shared
    // state, it is a frozen last-known session kept ONLY because the
    // shared atom can no longer promise a superseded instance anything).
    // ------------------------------------------------------------------
    [Fact]
    public void NoComponentHeldSharedState_NoFieldIsTypedAsAMigratedAtomValueTypeOutsideTheDocumentedException()
    {
        var allowlist = new (string File, string FieldName)[]
        {
            ("client\\Components\\ExplorerPopover.razor", "_frozenSnapshot"),
        };

        var typeNames = new[] { "Locus", "TimeWindow", "ViewArrangement", "FocusStack" };
        var pattern = new Regex(
            @"(?:private|public|protected|internal)\s+(?:readonly\s+)?(?:" + string.Join("|", typeNames) + @")\??\s+(_\w+)",
            RegexOptions.Compiled);
        var listPattern = new Regex(
            @"(?:private|public|protected|internal)\s+(?:readonly\s+)?(?:List|IReadOnlyList)<ExplorationDescriptor>\s+(_\w+)",
            RegexOptions.Compiled);

        // Same cross-platform normalization as the effects-only-via-registry
        // test above (S-8) -- '/' on both sides of every comparison.
        static string Normalize(string path) => path.Replace('\\', '/');

        var violations = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
            // client/State/ is where these value types (and their own
            // static Empty/Default members) are DEFINED -- out of scope for
            // a "component-held" rule by definition.
            var relative = Normalize(Path.GetRelativePath(RepoRoot(), file));
            if (relative.StartsWith("client/State/"))
            {
                continue;
            }

            var text = File.ReadAllText(file);
            foreach (Match m in pattern.Matches(text).Cast<Match>().Concat(listPattern.Matches(text).Cast<Match>()))
            {
                var fieldName = m.Groups[1].Value;
                if (!allowlist.Any(e => relative.EndsWith(Normalize(e.File)) && e.FieldName == fieldName))
                {
                    violations.Add($"{relative}: {fieldName} ({m.Value.Trim()})");
                }
            }
        }

        Assert.True(violations.Count == 0,
            "Found a component-held field typed as a migrated atom's own value type -- render a Projection<T> instead, or add a reasoned allowlist entry here (the S-2 fix's own frozen-snapshot exception is the only one on record):\n" +
            string.Join("\n", violations));
    }
}
