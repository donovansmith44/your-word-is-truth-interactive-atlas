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
            ("client\\Components\\CompositionSplit.razor", "ViewArrangementAtom.Changed += HandleArrangementChanged",
                "Fix round 2 (ruling 1, N-1 -- re-review, binding): the ONE owned subscription -- HandleArrangementChanged => StateHasChanged() (pure re-render, covers every host whose markup lives entirely inside ChildContent, e.g. Sources) PLUS the optional OnArrangementChanged hook invocation (the sanctioned per-host exception, e.g. Reader.razor's own SyncSplitUrl -- see CompositionSplit.razor's own header). Reader.razor and Sources.razor BOTH deleted their own former entries here this round -- this single entry replaces both."),
            ("client\\Pages\\World.razor", "LocusAtom.Changed += OnLocusChanged",
                "OnLocusChanged => StateHasChanged() only (ST-2 retired its own re-scening side effect) -- pure re-render."),
            ("client\\Pages\\Reader.razor", "LocusAtom.Changed += OnLocusChangedAsGuest",
                "Batch CORP-1 (R2): guest-mounted ONLY (SplitMode is not null -- a self-routed instance never subscribes, see OnInitializedAsync's own comment). OnLocusChangedAsGuest => fire-and-forget RefreshAsGuestAsync(), which refetches this chapter (a plain GET, no JS interop) only when the shared Locus atom genuinely moved, then StateHasChanged() -- this is the OTHER half of the split-follow-by-construction proof (Kretzmann.razor's own entry below is the writer side; this is the reader-as-guest reader side). Mirrors World.razor's own pre-existing LocusAtom.Changed subscription immediately below in shape, not in role."),
            ("client\\Pages\\Kretzmann.razor", "LocusAtom.Changed += OnLocusChanged",
                "Batch CORP-1 (R2): OnLocusChanged => clears any open popover + refetches this chapter's own commentary listing (a plain GET through the already-registered IExplorableClient, no JS interop) + StateHasChanged() -- this is what makes the split-follow-by-construction proof real (navigate the reader in split, Kretzmann's own subscription fires and refetches for the new chapter); the SAME class of local-projection-sync-plus-fetch World.razor's own OnTimeWindowChanged/OnViewArrangementChanged entries above already establish, not a new pattern."),
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
    //
    // PARKED (fix round 2, N-6, trivia -- re-review, "fix cheap or park
    // with one line"): the two regexes below only catch a bare
    // `private|public|protected|internal <Type> _name` field declaration --
    // a `static` field, a property-shaped copy
    // (`FocusStack Snapshot { get; set; }`), or an `ExplorationDescriptor[]`/
    // `ImmutableArray<ExplorationDescriptor>` copy would all evade this
    // scan (verified misses, not merely suspected). Meets ruling 6.iii's own
    // stated bar today (re-planting the exact `_focus` violation IS caught,
    // proven in the re-review) -- broadening the pattern set is left for
    // ST-4's own general, all-types sweep rather than duplicated here.
    //
    // ST-4 SWEEP (A4, controller-approved MIGRATE -- closing VC-1 fix round
    // 2's own PARKED N-6 finding, "fix cheap or park with one line": "a
    // static field, a property-shaped copy (`FocusStack Snapshot { get;
    // set; }`), or an `ExplorationDescriptor[]`/`ImmutableArray&lt;
    // ExplorationDescriptor&gt;` copy would all evade this scan (verified
    // misses, not merely suspected)." Verified by direct full-tree grep
    // this batch BEFORE writing the fix: zero real instances of any of the
    // three shapes exist today (sweep-inventory.md row A4) -- this is
    // preventive hardening of a documented, currently-dormant blind spot,
    // proven via the same planted-line technique the file's own Arrangement
    // scan below already established, not a live-bug fix.
    // ------------------------------------------------------------------
    private static readonly string[] AtomValueTypeNames = { "Locus", "TimeWindow", "ViewArrangement", "FocusStack" };
    private static readonly string AtomValueTypeAlternation = string.Join("|", AtomValueTypeNames);

    // Field-shaped (this codebase's own `_name` convention) -- ORIGINAL
    // shape, now also matching an optional `static` modifier (the first of
    // N-6's three named misses: a static field never carried `readonly`
    // alone in the old alternation's assumed position).
    private static readonly Regex AtomTypedFieldPattern = new(
        @"(?:private|public|protected|internal)\s+(?:static\s+)?(?:readonly\s+)?(?:" + AtomValueTypeAlternation + @")\??\s+(_\w+)",
        RegexOptions.Compiled);

    // Property-shaped (N-6's second named miss) -- a PascalCase name (this
    // codebase never prefixes a property with `_`) terminated by a
    // block-property `{` or an expression-bodied `=>`; never a bare `;`,
    // which would just be a second match of a field the pattern above
    // already caught.
    private static readonly Regex AtomTypedPropertyPattern = new(
        @"(?:private|public|protected|internal)\s+(?:static\s+)?(?:" + AtomValueTypeAlternation + @")\??\s+(\w+)\s*(?:\{|=>)",
        RegexOptions.Compiled);

    // Selection-shaped collection, broadened (N-6's third named miss): the
    // original only matched List/IReadOnlyList<ExplorationDescriptor> --
    // ImmutableArray<ExplorationDescriptor> and a bare
    // ExplorationDescriptor[] array are two more shapes the SAME Selection
    // atom's value type (IReadOnlyList<ExplorationDescriptor>) could
    // plausibly be copied as.
    private static readonly Regex SelectionShapedListPattern = new(
        @"(?:private|public|protected|internal)\s+(?:static\s+)?(?:readonly\s+)?(?:List|IReadOnlyList|ImmutableArray)<ExplorationDescriptor>\s+(_\w+)",
        RegexOptions.Compiled);
    private static readonly Regex SelectionShapedArrayPattern = new(
        @"(?:private|public|protected|internal)\s+(?:static\s+)?(?:readonly\s+)?ExplorationDescriptor\[\]\s+(_\w+)",
        RegexOptions.Compiled);

    [Fact]
    public void NoComponentHeldSharedState_PlantedPropertyShapedViolation_IsCaught()
    {
        // The exact shape N-6 named as a hypothetical: a property, not a
        // field, so the original `_\w+`-only regex would never see it.
        const string planted = "private FocusStack Snapshot { get; set; }";

        Assert.Matches(AtomTypedPropertyPattern, planted);
    }

    [Fact]
    public void NoComponentHeldSharedState_PlantedExpressionBodiedPropertyShapedViolation_IsCaught()
    {
        const string planted = "public ViewArrangement Current => _cached;";

        Assert.Matches(AtomTypedPropertyPattern, planted);
    }

    [Fact]
    public void NoComponentHeldSharedState_PlantedStaticFieldViolation_IsCaught()
    {
        const string planted = "private static Locus _lastKnownLocus;";

        Assert.Matches(AtomTypedFieldPattern, planted);
    }

    [Fact]
    public void NoComponentHeldSharedState_PlantedImmutableArraySelectionViolation_IsCaught()
    {
        const string planted = "private readonly ImmutableArray<ExplorationDescriptor> _selectionCopy;";

        Assert.Matches(SelectionShapedListPattern, planted);
    }

    [Fact]
    public void NoComponentHeldSharedState_PlantedArraySelectionViolation_IsCaught()
    {
        const string planted = "private ExplorationDescriptor[] _selectionCopy;";

        Assert.Matches(SelectionShapedArrayPattern, planted);
    }

    [Fact]
    public void NoComponentHeldSharedState_NoFieldIsTypedAsAMigratedAtomValueTypeOutsideTheDocumentedException()
    {
        var allowlist = new (string File, string FieldName)[]
        {
            ("client\\Components\\ExplorerPopover.razor", "_frozenSnapshot"),
            // Fix round 2 (N-4, trivia -- re-review, cheap fix): a
            // MEMOIZATION KEY, not a component-held copy of shared state in
            // the sense this rule guards against -- every read of
            // CompositionSplit's own Composition property re-checks
            // `ReferenceEquals(_lastArrangement, ViewArrangementAtom.Value)`
            // FIRST and refreshes both fields together the instant it
            // disagrees (an immutable record, so reference inequality means
            // a genuinely new atom value, never a false negative); this
            // field can never be read while stale, unlike a `_splitOpen`-
            // style copy computed once and left to drift. See
            // CompositionSplit.razor's own Composition property.
            ("client\\Components\\CompositionSplit.razor", "_lastArrangement"),
            // ST-4 sweep (A4), real hit from the newly-added property-shaped
            // pattern: `FocusValue => _frozenSnapshot;` (fix round 2, N-3) is
            // a PURE, one-line forward to the field ALREADY allowlisted two
            // rows up -- not a second, independent copy. `_frozenSnapshot`
            // is the sanctioned storage; `FocusValue` is merely its public
            // read name (every consumer in this file reads `FocusValue`,
            // never `_frozenSnapshot` directly -- see that field's own
            // `SyncSnapshot`-is-the-one-write-path header comment above).
            ("client\\Components\\ExplorerPopover.razor", "FocusValue"),
        };

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
            var matches = AtomTypedFieldPattern.Matches(text).Cast<Match>()
                .Concat(AtomTypedPropertyPattern.Matches(text).Cast<Match>())
                .Concat(SelectionShapedListPattern.Matches(text).Cast<Match>())
                .Concat(SelectionShapedArrayPattern.Matches(text).Cast<Match>());
            foreach (var m in matches)
            {
                var fieldName = m.Groups[1].Value;
                if (!allowlist.Any(e => relative.EndsWith(Normalize(e.File)) && e.FieldName == fieldName))
                {
                    violations.Add($"{relative}: {fieldName} ({m.Value.Trim()})");
                }
            }
        }

        Assert.True(violations.Count == 0,
            "Found a component-held field/property typed as a migrated atom's own value type -- render a Projection<T> instead, or add a reasoned allowlist entry here (the S-2 fix's own frozen-snapshot exception is the only one on record):\n" +
            string.Join("\n", violations));
    }

    // ------------------------------------------------------------------
    // Batch VC-1 fix round 1 (S-3, CRITICAL -- review): "R6's own no-copy
    // scan extension to arrangement state was never written ... a planted
    // `private bool _splitOpen;` -- literally the pattern R6 names -- is a
    // bool, matches nothing in [ruling 6.iii's own type-name regex], and
    // passes silently." This is the extension: it does NOT match on field
    // TYPE (a `_splitOpen`-style copy is usually a bare bool/string derived
    // FROM the atom, never typed AS the atom) -- it matches on whether a
    // component-level field or property's own INITIALIZER reads
    // `ViewArrangementAtom.Value` at all, exactly the review's own "more
    // robust" suggested fix. Proven with the PLANTED-LINE technique (the
    // ST-3 re-reviewer's own method -- feed the matcher a synthetic snippet
    // containing the exact retired violation shape and a REALISTIC
    // multi-line one, assert both are caught) rather than trusting an
    // empty real-file scan alone to mean the pattern works.
    // ------------------------------------------------------------------
    private static readonly Regex ArrangementStateInitializerPattern = new(
        @"(?:private|public|protected|internal)\s+(?:readonly\s+)?[^\n;{}]+?\s+(_?\w+)\s*(?:=>|=)\s*[^;]{0,400}?ViewArrangementAtom\.Value",
        RegexOptions.Compiled);

    // Secondary shape: a bare assignment INTO a field from a method body
    // (not the field's own declaration) -- e.g. `_splitOpen =
    // ViewArrangementAtom.Value...;` inside OnParametersSetAsync. This
    // codebase's own established convention (every private field visibly
    // uses a leading underscore, confirmed across client/Pages/client/
    // Components) makes `_\w+\s*=` a safe, low-false-positive heuristic for
    // "assigning into a FIELD," not a local variable (locals never carry
    // this app's own `_` prefix).
    private static readonly Regex ArrangementStateAssignmentPattern = new(
        @"(?<![.\w])(_\w+)\s*=\s*[^;=]{0,400}?ViewArrangementAtom\.Value",
        RegexOptions.Compiled);

    [Fact]
    public void NoComponentHeldArrangementState_PlantedRetiredShapeViolation_IsCaught()
    {
        // The EXACT shape this batch's own first-draft Reader.razor/
        // Sources.razor shipped, pre-fix-round-1 (now genuinely retired).
        const string planted = "private bool _splitOpen => SplitMode ?? (ViewArrangementAtom.Value is { LayoutKind: LayoutKinds.SplitH } a && a.Members.Count > 0 && a.Members[0] == ViewNames.Reader);";

        Assert.Matches(ArrangementStateInitializerPattern, planted);
    }

    [Fact]
    public void NoComponentHeldArrangementState_PlantedMultiLineViolation_IsCaught()
    {
        // A realistic MULTI-LINE shape (an expression body wrapped onto
        // several lines, exactly how World.razor's own legitimate, real,
        // ALLOWLISTED _follow property below is actually written) -- proves
        // the scan is not merely a single-line trick.
        const string planted = "private bool _isHost =>\n        ViewArrangementAtom.Value is { LayoutKind: LayoutKinds.SplitH } a\n        && a.Members[0] == HostName;";

        Assert.Matches(ArrangementStateInitializerPattern, planted);
    }

    [Fact]
    public void NoComponentHeldArrangementState_PlantedAssignmentViolation_IsCaught()
    {
        const string planted = "        _cachedArrangement = ViewArrangementAtom.Value;";

        Assert.Matches(ArrangementStateAssignmentPattern, planted);
    }

    [Fact]
    public void NoComponentHeldArrangementState_NoRealSiteOutsideTheSanctionedOwners()
    {
        var allowlist = new (string File, string Justification)[]
        {
            ("client\\Components\\CompositionSplit.razor",
                "The ONE sanctioned generic reader -- the Composition property's own `_lastArrangement = ViewArrangementAtom.Value` read (fix round 2, N-4: now a render-pass-memoized block-bodied getter, not a bare expression-bodied one, to avoid re-materializing IViewComposition 3-5 times per render) IS R1/R3's own job (materializing the live arrangement through the compiled IViewComposition contract), not a component-held copy of role state -- `_lastArrangement`/`_lastComposition` are a CACHE of that ONE read's own most recent result, invalidated on every genuinely new atom value (reference equality against an immutable record), never a second, independently-drifting copy."),
            ("client\\Pages\\World.razor",
                "R5's own capability-gated follow read (_follow) -- pre-existing (ST-2/ST-3/VC-1), reviewed and verified-passing in this batch's own original review ('R5 capability-based follow ... verified PASSING'). Not a role-determination copy of the retired _splitOpen shape -- Reader/Sources no longer have ANY such property after this fix round; CompositionSplit computes role ONCE, centrally."),
            ("client\\Pages\\Kretzmann.razor",
                "Batch CORPREAD-1b, THE FOLLOW-RELEASE LAW's own `Following` property -- the SAME capability-gated follow read World.razor's own `_follow` (immediately above) already establishes, applied to a second locus-bearing view for the first time (Kretzmann now genuinely needs to know whether IT is following, not merely whether split is open). Reads ViewArrangementAtom.Value.Follow directly (never re-derives the split-h-host formula RoleFormulaRederivationPattern polices -- that scan stays green here, verified) -- a live, un-cached read every call, not a component-held COPY (no field is ever assigned FROM this expression; it is computed fresh on every access, the identical live-projection shape World's own _follow uses)."),
        };

        static string Normalize(string path) => path.Replace('\\', '/');

        var violations = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
            var relative = Normalize(Path.GetRelativePath(RepoRoot(), file));
            if (relative.StartsWith("client/State/") || relative.StartsWith("client/Views/"))
            {
                continue; // where the atom and the registry-backed contract projection are legitimately DEFINED
            }

            var text = File.ReadAllText(file);
            var matched = ArrangementStateInitializerPattern.IsMatch(text) || ArrangementStateAssignmentPattern.IsMatch(text);
            if (!matched)
            {
                continue;
            }

            if (!allowlist.Any(e => relative.EndsWith(Normalize(e.File))))
            {
                violations.Add(relative);
            }
        }

        Assert.True(violations.Count == 0,
            "Found a component-held field/property reading ViewArrangementAtom.Value directly outside the sanctioned owners -- retire it in favor of CompositionSplit's own centrally-computed role (IsSplitOpen/IsHost via ctx or @ref), or add a reasoned allowlist entry here:\n" +
            string.Join("\n", violations));

        // The allowlist itself must stay accurate -- every entry must
        // actually match something real, or it is silently covering for a
        // site that no longer exists.
        foreach (var (file, _) in allowlist)
        {
            var path = Path.Combine(RepoRoot(), file.Replace('\\', Path.DirectorySeparatorChar));
            Assert.True(File.Exists(path), $"Stale allowlist entry -- file no longer exists: {file}");
            var text = File.ReadAllText(path);
            Assert.True(ArrangementStateInitializerPattern.IsMatch(text) || ArrangementStateAssignmentPattern.IsMatch(text),
                $"Stale allowlist entry -- {file} no longer matches the pattern it was allowlisted for.");
        }
    }

    // ------------------------------------------------------------------
    // ST-4 SWEEP (B2, controller-approved MIGRATE): the no-copy scan above
    // (ArrangementStateInitializerPattern/ArrangementStateAssignmentPattern)
    // is hard-coded to `ViewArrangementAtom.Value` alone. Deliverable 1b's
    // own instruction is to generalize this class of scan wherever
    // scannable, not just leave it proven for the one atom that happened to
    // have a live historical violation (Reader:520). This is the same
    // field/property-initializer + bare-assignment shape, generalized to
    // the OTHER four atoms (`LocusAtom`/`TimeWindowAtom`/`SelectionAtom`/
    // `FocusStackAtom`) -- `ViewArrangementAtom` itself is deliberately left
    // out of this pattern (the scan above already covers it; duplicating it
    // here would just double-report the same real site under two tests).
    //
    // Verified by direct full-tree grep BEFORE writing this (sweep-
    // inventory.md row B2): each of the four other atoms has exactly ONE
    // sanctioned site today --
    // `LocusAtom`: zero direct-assignment sites at all (every consumer
    //   reads through a live `Projection<Locus>`);
    // `TimeWindowAtom`: `World.razor`'s own `SyncTimeWindowProjection`
    //   switch statement (the ONE place `_from`/`_to`/`_scriptureRef` are
    //   ever assigned, per the A1 sweep row -- a `switch` STATEMENT has no
    //   `=`/`=>` binding `TimeWindowAtom.Value` to a name, so this pattern
    //   does not even match it; the real per-field assignments inside each
    //   `case` arm read `tm.From`/`tm.To`/`sm.Ref`, never `TimeWindowAtom.Value`
    //   itself a second time, so nothing here needs an allowlist entry);
    // `SelectionAtom`: `SelectionTray.razor`'s own `var items =
    //   SelectionAtom.Value;` -- a LOCAL variable (no access modifier, no
    //   leading underscore), invisible to both patterns by construction,
    //   exactly the "read live, never copy" shape this rule wants;
    // `FocusStackAtom`: `ExplorerPopover.razor`'s own ownership-handoff
    //   check (`FocusStackAtom.Value == FocusStack.Empty`, a comparison
    //   inside an `if`, not an assignment or initializer) -- also invisible
    //   to both patterns by construction.
    // No allowlist entries are needed as a result -- confirmed empirically
    // by running the real-tree assertion below, not assumed from the grep
    // alone.
    // ------------------------------------------------------------------
    private static readonly string[] OtherAtomNames = { "Locus", "TimeWindow", "Selection", "FocusStack" };
    private static readonly string OtherAtomAlternation = string.Join("|", OtherAtomNames.Select(n => n + "Atom"));

    // ST4-m1 (batch-finalp2-brief.md ticket 8; origin: batch-st4-review.md
    // Q-1, Trivia): the lazy-match window between the assignment operator
    // and the Atom.Value token was originally 400 characters -- wide
    // enough that "a sufficiently long, unrelated multi-line expression
    // containing an incidental FooAtom.Value substring somewhere within
    // 400 chars of an assignment could false-positive" (the review's own
    // words). Tightened to 120: comfortably wider than any real single-
    // statement C# field/property initializer or assignment in this
    // codebase's own style (verified: every real match this scan has ever
    // found -- the planted test literals below -- sits well under 20
    // chars from operator to Atom.Value), while shrinking the false-
    // positive surface named in Q-1 by ~3.3x. No real site's detection
    // changes (confirmed: `NoComponentHeldOtherAtomState_
    // NoRealSiteOutsideTheSanctionedOwners` below stays green, and there
    // were zero real matches at 400 chars to begin with -- see this
    // pattern's own header comment above).
    private static readonly Regex OtherAtomStateInitializerPattern = new(
        @"(?:private|public|protected|internal)\s+(?:readonly\s+)?[^\n;{}]+?\s+(_?\w+)\s*(?:=>|=)\s*[^;]{0,120}?(?:" + OtherAtomAlternation + @")\.Value",
        RegexOptions.Compiled);

    private static readonly Regex OtherAtomStateAssignmentPattern = new(
        @"(?<![.\w])(_\w+)\s*=\s*[^;=]{0,120}?(?:" + OtherAtomAlternation + @")\.Value",
        RegexOptions.Compiled);

    [Fact]
    public void NoComponentHeldOtherAtomState_PlantedLocusShapeViolation_IsCaught()
    {
        const string planted = "private string _book => LocusAtom.Value.Book;\n    private string _bookCopy = LocusAtom.Value.Book;";

        // The FIRST line (a live Projection-style read) is the CORRECT,
        // sanctioned pattern this app actually uses (Reader.razor/
        // World.razor's own `_book => _locus.Value.Book`, which reads
        // through a `Projection<Locus>`, never the bare atom, so it does
        // NOT match this pattern at all -- proven by the second line here
        // instead, a hypothetical BARE-atom copy this scan exists to catch).
        Assert.Matches(OtherAtomStateInitializerPattern, planted);
    }

    [Fact]
    public void NoComponentHeldOtherAtomState_PlantedSelectionAssignmentViolation_IsCaught()
    {
        const string planted = "        _cachedSelection = SelectionAtom.Value;";

        Assert.Matches(OtherAtomStateAssignmentPattern, planted);
    }

    [Fact]
    public void NoComponentHeldOtherAtomState_PlantedFocusStackAssignmentViolation_IsCaught()
    {
        const string planted = "        _cachedFocus = FocusStackAtom.Value;";

        Assert.Matches(OtherAtomStateAssignmentPattern, planted);
    }

    [Fact]
    public void NoComponentHeldOtherAtomState_NoRealSiteOutsideTheSanctionedOwners()
    {
        static string Normalize(string path) => path.Replace('\\', '/');

        var violations = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
            var relative = Normalize(Path.GetRelativePath(RepoRoot(), file));
            if (relative.StartsWith("client/State/") || relative.StartsWith("client/Views/"))
            {
                continue; // where the atoms themselves are legitimately DEFINED
            }

            var text = File.ReadAllText(file);
            var matched = OtherAtomStateInitializerPattern.IsMatch(text) || OtherAtomStateAssignmentPattern.IsMatch(text);
            if (matched)
            {
                violations.Add(relative);
            }
        }

        Assert.True(violations.Count == 0,
            "Found a component-held field/property reading Locus/TimeWindow/Selection/FocusStack Atom.Value directly, outside a live Projection<T> read -- render a Projection<T> (or read the atom's own .Value fresh, never copy it into a field) instead, or add a reasoned allowlist entry here (none exist yet -- see ConformanceTests.cs's own header comment on this test for why):\n" +
            string.Join("\n", violations));
    }

    // ------------------------------------------------------------------
    // Batch VC-1 fix round 2 (N-2, Important -- re-review, controller
    // ruling 2, binding): "extend the no-copy scan to catch this shape:
    // role/arrangement FORMULAS re-derived outside CompositionSplit and the
    // atom ... The wrapper's 'never re-derived' claim must be enforced, not
    // asserted." The S-3 scan above catches a component-held FIELD/PROPERTY
    // that reads the atom -- it does NOT catch a LOCAL variable inside a
    // method body computing the exact "who is the split-h host" shape by
    // hand (Reader.razor:520's own pre-fix-round-2 formula, now retired in
    // favor of the single `IsHostedBy` extension method,
    // `client/Views/ViewRegistry.cs`), which is precisely what slipped past
    // it. This pattern targets the FORMULA'S OWN SHAPE directly -- a
    // `LayoutKind`/`LayoutKinds.SplitH` comparison and a `Members[0]` index
    // read within a short window of each other, in EITHER order (a `record`
    // pattern match like `is { LayoutKind: LayoutKinds.SplitH }` puts the
    // kind check FIRST; a hand-rolled `==` check could as easily be written
    // either way) -- rather than on any particular variable name or type,
    // since the retired violation was a LOCAL (`var isSplitOpen = ...`),
    // invisible to any field-declaration-shaped regex by construction.
    // ------------------------------------------------------------------
    private static readonly Regex RoleFormulaRederivationPattern = new(
        @"(?:(?:LayoutKind|Layout\.Kind)\s*(?:==|:)\s*LayoutKinds\.SplitH|LayoutKinds\.SplitH\s*==\s*(?:LayoutKind|Layout\.Kind))[\s\S]{0,250}?Members\s*\[\s*0\s*\]" +
        @"|Members\s*\[\s*0\s*\][\s\S]{0,250}?(?:(?:LayoutKind|Layout\.Kind)\s*(?:==|:)\s*LayoutKinds\.SplitH|LayoutKinds\.SplitH\s*==\s*(?:LayoutKind|Layout\.Kind))",
        RegexOptions.Compiled);

    [Fact]
    public void RoleFormulaRederivation_PlantedRetiredReaderShapeViolation_IsCaught()
    {
        // The EXACT shape Reader.razor's own SyncSplitUrl carried before
        // this fix round (the re-review's own N-2 finding, byte-identical
        // to the review's own quote of it).
        const string planted = "var isSplitOpen = ViewArrangementAtom.Value is { LayoutKind: LayoutKinds.SplitH } a && a.Members.Count > 0 && a.Members[0] == ViewNames.Reader;";

        Assert.Matches(RoleFormulaRederivationPattern, planted);
    }

    [Fact]
    public void RoleFormulaRederivation_PlantedInlineComparisonShapeViolation_IsCaught()
    {
        // A DIFFERENT hand-rolled phrasing of the same law (plain `==`
        // comparisons, `Members[0]` written BEFORE the LayoutKind check) --
        // proves the scan is not keyed to the one exact retired string, and
        // catches the direction CompositionSplit's own pre-fix-round-2
        // IsSplitOpen was written in (`Composition.Layout.Kind == LayoutKinds.SplitH
        // && Composition.Members.Count > 0 && Composition.Members[0].Name == HostName`)
        // were it ever pasted somewhere OTHER than the sanctioned
        // `IsHostedBy` definition.
        const string planted = "return other.Members[0].Name == target && other.Layout.Kind == LayoutKinds.SplitH;";

        Assert.Matches(RoleFormulaRederivationPattern, planted);
    }

    [Fact]
    public void RoleFormulaRederivation_NoRealSiteOutsideTheSanctionedDefinition()
    {
        // The ONE sanctioned definition site: ViewCompositionExtensions.IsHostedBy
        // (client/Views/ViewRegistry.cs) -- everywhere else (CompositionSplit's
        // own IsSplitOpen, Reader.razor's own SyncSplitUrl) now CALLS that
        // one method instead of writing the formula out by hand. client/Views/
        // is the definition-site exclusion the S-3 scan above already
        // establishes (ViewArrangement/the registry-backed contract
        // projection legitimately live there) -- reused verbatim, not a new
        // carve-out invented for this test.
        static string Normalize(string path) => path.Replace('\\', '/');

        var violations = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
            var relative = Normalize(Path.GetRelativePath(RepoRoot(), file));
            if (relative.StartsWith("client/State/") || relative.StartsWith("client/Views/"))
            {
                continue;
            }

            var text = File.ReadAllText(file);
            if (RoleFormulaRederivationPattern.IsMatch(text))
            {
                violations.Add(relative);
            }
        }

        Assert.True(violations.Count == 0,
            "Found a hand-derived 'is this the split-h host' formula (a LayoutKind/SplitH check near a Members[0] read) outside the sanctioned IsHostedBy definition -- call Composition.IsHostedBy(name) (or Registry.ComposeFrom(...).IsHostedBy(name) where ctx/Composition is out of reach) instead:\n" +
            string.Join("\n", violations));
    }

    // ------------------------------------------------------------------
    // ST-4 SWEEP (D1, controller-approved MIGRATE): "view-identity checks
    // where capability checks belong" (deliverable 1d) had NO conformance
    // mechanism at all before this batch -- a genuine spec §0 conformance-
    // corollary gap ("a contract whose violation nothing can fail on is
    // documentation, not a contract, and is itself a defect"). Verified by
    // direct full-tree grep this batch (sweep-inventory.md row D1): ZERO
    // `==`/`!=` comparisons against a `ViewNames.*` constant exist anywhere
    // outside `client/Views/` today -- every real `ViewNames` reference is
    // an intent-construction argument, a hatch-by-name registry lookup, a
    // `HostName=` declaration, or a call into the sanctioned `IsHostedBy`
    // law. This test starts GREEN and stays green until a future
    // regression re-introduces a name check where R5's own capability
    // query (`Registry.CapabilitiesOf(name) & ViewCapabilities.X`) belongs
    // -- its value is the tripwire, per the controller's own ruling.
    // ------------------------------------------------------------------
    private static readonly Regex ViewIdentityComparisonPattern = new(
        @"ViewNames\.\w+\s*(?:==|!=)|(?:==|!=)\s*ViewNames\.\w+",
        RegexOptions.Compiled);

    [Fact]
    public void ViewIdentityVsCapability_PlantedEqualityComparisonViolation_IsCaught()
    {
        // The exact shape this rule guards against: gating a behavior on
        // "is this literally the Reader/World view" instead of querying
        // its DECLARED capability (BearsLocus/BearsWindow) through the
        // registry -- R5's own "never a name check" law (client/Pages/
        // World.razor's own _follow property is the real, correct example
        // this planted line deliberately does NOT match, since it uses
        // Registry.CapabilitiesOf(...), never ViewNames.*, at all).
        const string planted = "if (member == ViewNames.Reader) { EnableFollowScene(); }";

        Assert.Matches(ViewIdentityComparisonPattern, planted);
    }

    [Fact]
    public void ViewIdentityVsCapability_PlantedInequalityComparisonViolation_IsCaught()
    {
        const string planted = "var isNotWorld = ViewNames.World != memberName;";

        Assert.Matches(ViewIdentityComparisonPattern, planted);
    }

    // Strips `// ...` line comments before scanning -- a real, live false
    // positive caught while writing this test: World.razor:447's own
    // EXPLANATORY comment for R5's capability-based follow read literally
    // quotes the retired shape this rule forbids ('never a "==
    // ViewNames.Reader" ... name check'), which is prose ABOUT the rule,
    // not a violation of it. Naive (does not understand string literals
    // containing "//"), but sufficient here: no real ViewNames identity
    // comparison in this codebase sits on a line with a "//" earlier in
    // the same line, confirmed by this fix's own before/after diff on the
    // real tree (World.razor:447 was the only site this stripping changed
    // the outcome for).
    private static string StripLineComments(string text) => Regex.Replace(text, "//[^\n]*", string.Empty);

    [Fact]
    public void StripLineComments_DisclosedLimitation_ASlashSlashInsideAStringLiteralSilentlyEatsTheRestOfTheLine()
    {
        // ST4-m2 (batch-finalp2-brief.md ticket 8; origin: batch-st4-review.md
        // Q-2, Trivia): StripLineComments' own header comment candidly
        // discloses it "does not understand string literals containing
        // '//'" -- correct to accept as-is (no real site in this codebase
        // has this shape today, confirmed by the green NoRealSite scan
        // below), but a latent false-negative surface. PINNED here directly
        // (was: prose only) -- a real ViewNames identity-comparison
        // violation, sharing a line with an EARLIER string literal that
        // itself contains "//" (a URL, most plausibly), is silently
        // stripped away before ViewIdentityComparisonPattern ever sees it.
        // If this assertion ever starts FAILING (the violation survives
        // stripping), StripLineComments was improved to understand string
        // literals -- update this test's own doc comment to match, don't
        // just delete the test.
        const string planted = "var note = \"see http://example.com//path\"; var isWorld = memberName == ViewNames.World;";

        // Sanity first: the SAME violation, UNSTRIPPED, is caught -- proving
        // the pattern itself is sound; the disclosed gap is specifically in
        // the stripping step, not the identity-comparison pattern.
        Assert.Matches(ViewIdentityComparisonPattern, planted);

        var stripped = StripLineComments(planted);

        // The naive stripper treats the FIRST "//" (inside "http://", part
        // of the string literal) as a comment opener and truncates
        // EVERYTHING after it on the line -- including the real violation
        // that follows.
        Assert.DoesNotContain("ViewNames.World", stripped);
        Assert.DoesNotMatch(ViewIdentityComparisonPattern, stripped);
    }

    [Fact]
    public void ViewIdentityVsCapability_NoRealSiteOutsideTheRegistryDeclarationLayer()
    {
        static string Normalize(string path) => path.Replace('\\', '/');

        var violations = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
            var relative = Normalize(Path.GetRelativePath(RepoRoot(), file));
            if (relative.StartsWith("client/Views/"))
            {
                continue; // the registry/declaration layer itself -- ViewNames constants are legitimately DEFINED and internally referenced here (e.g. RegisteredView construction, hatch OwnerView/PartnerView/HostView wiring)
            }

            var text = StripLineComments(File.ReadAllText(file));
            if (ViewIdentityComparisonPattern.IsMatch(text))
            {
                violations.Add(relative);
            }
        }

        Assert.True(violations.Count == 0,
            "Found a view-IDENTITY comparison (== / != against a ViewNames constant) outside the registry/declaration layer -- query the view's own DECLARED capability instead (Registry.CapabilitiesOf(name) & ViewCapabilities.X, R5's own law), or add a reasoned allowlist entry here (none exist yet):\n" +
            string.Join("\n", violations));
    }

    // ------------------------------------------------------------------
    // ST-4 SWEEP EXEMPTION LEDGER (A2, D2 -- controller-ruled EXEMPT,
    // checkpoint response verbatim). Neither exemption below trips ANY
    // scan in this file today (ViewStateService's fields are never typed
    // as one of the five atom value types; MainLayout's IsWorld references
    // no ViewNames constant at all) -- there is no regex for either to be
    // "allowlisted" against in the mechanical sense. Recorded here as a
    // real, TESTABLE ledger instead of prose alone (matching this file's
    // own "an allowlist entry must match something real, or it is
    // silently covering for a site that no longer exists" discipline,
    // e.g. NoComponentHeldArrangementState_NoRealSiteOutsideTheSanctionedOwners's
    // own stale-entry check above): each assertion fails loud if its own
    // exempted site disappears or changes shape without this ledger being
    // updated to match, which is the honest substitute for a scan that has
    // nothing to match against.
    // ------------------------------------------------------------------
    [Fact]
    public void SweepExemption_A2_ViewStateServiceIsThePersistenceLayerBeneathAtoms()
    {
        // Spec §4d, verbatim: "ViewStateService remains the PERSISTENCE
        // layer beneath atoms (it persists state; atoms OWN it); URLs/deep
        // links are projections under the same agreement law." MapViewState/
        // ReaderViewState hold plain scalar fields (never typed as Locus/
        // TimeWindow/ViewArrangement/FocusStack themselves, confirmed --
        // this is WHY neither NoComponentHeldSharedState nor the two
        // no-copy scans above ever flag them) and are consumed by 3+
        // components (Reader.razor/World.razor/CompositionSplit.razor) by
        // DESIGN: write-on-commit, read-only-at-mount-to-seed-a-dispatch,
        // never a live runtime value two components race to read. This
        // test pins the exemption's own PRECONDITION (the persistence
        // classes still exist, still hold plain scalar fields, never the
        // atom value types) -- if a future edit changes that shape, this
        // fails loud rather than leaving the exemption's justification
        // silently stale.
        var text = File.ReadAllText(Path.Combine(RepoRoot(), "client", "ViewStateService.cs"));

        Assert.Contains("public sealed class MapViewState", text);
        Assert.Contains("public sealed class ReaderViewState", text);
        Assert.DoesNotMatch(AtomTypedFieldPattern, text);
        Assert.DoesNotMatch(AtomTypedPropertyPattern, text);
    }

    [Fact]
    public void SweepExemption_D2_MainLayoutIsWorldIsCosmeticHeaderTheming()
    {
        // Controller ruling (checkpoint response, verbatim): "MainLayout's
        // IsWorld URL sniff is cosmetic header theming outside the
        // composition/capability system's scope... CORP-1 (per-corpus
        // tabs, queued next) may grow per-view header theming -- if it
        // does, this exemption is the first thing that batch revisits;
        // note that in the exemption justification so the tripwire is a
        // breadcrumb, not a blessing."
        //
        // BREADCRUMB FOR CORP-1: if per-corpus tabs grow per-view header
        // theming beyond "world vs. everything else," re-examine whether
        // IsWorld should become a real registry capability query instead
        // of a raw URL-path sniff -- this exemption was ruled correct for
        // TODAY'S single reader/world distinction, not a permanent
        // blessing on route-string sniffing in general.
        var text = File.ReadAllText(Path.Combine(RepoRoot(), "client", "Layout", "MainLayout.razor"));

        Assert.Contains("Nav.ToBaseRelativePath(Nav.Uri).StartsWith(\"world\"", text);
        // Confirms the exemption's own precondition: this is a route-path
        // sniff, not a ViewNames comparison -- D1's own scan correctly
        // never flags it (no ViewNames.* token appears on this line at all).
        Assert.DoesNotContain("ViewNames.", text.Split('\n').First(l => l.Contains("IsWorld =>")));
    }

    // ------------------------------------------------------------------
    // ST4-m3 (batch-finalp2-brief.md ticket 8; origin: batch-st4-review.md
    // Q-3, Trivia): the two SweepExemption tests above only ever pinned
    // the EXEMPTED site's own shape ("did ViewStateService.cs/
    // MainLayout.razor change?") -- they would catch that, but NOT "a
    // SECOND site matching the SAME exemption rationale appeared elsewhere
    // without being added to this ledger" (they never asserted the
    // exempted site's own ABSENCE from anywhere else, the way a real
    // allowlist entry would). Each exemption below has a genuine
    // MECHANICAL fingerprint distinct from the regex each scan already
    // runs (that's WHY a controller ruling was needed instead of just
    // passing the existing scan) -- these two tests search the whole
    // client tree for that fingerprint and assert the one ruled-exempt
    // file is the ONLY site carrying it.
    // ------------------------------------------------------------------
    [Fact]
    public void SweepExemption_A2_NoOtherViewStateNamedClassExistsOutsideViewStateServiceCs()
    {
        // The A2 exemption's own mechanical fingerprint: the "*ViewState"
        // class-naming convention ViewStateService.cs already establishes
        // (MapViewState/ReaderViewState) for "a persistence-layer class
        // that mirrors atom state in plain scalar fields." A second class
        // adopting that SAME naming convention outside that one file would
        // be claiming the same exemption without ever having been ruled on.
        var classPattern = new Regex(@"(?:public|internal)\s+sealed\s+class\s+(\w*ViewState\w*)\b", RegexOptions.Compiled);
        var violations = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
            var relative = Path.GetRelativePath(RepoRoot(), file).Replace('\\', '/');
            if (relative == "client/ViewStateService.cs")
            {
                continue; // the one file spec §4d's A2 ruling was scoped to
            }

            var text = File.ReadAllText(file);
            foreach (Match m in classPattern.Matches(text))
            {
                violations.Add($"{relative}: {m.Groups[1].Value}");
            }
        }

        Assert.True(violations.Count == 0,
            "Found a *ViewState-named class OUTSIDE ViewStateService.cs -- the A2 exemption (spec §4d, \"ViewStateService remains the PERSISTENCE layer beneath atoms\") was ruled for THAT file specifically; a second site needs its own reasoned ledger entry, not a silent free ride on this one:\n" +
            string.Join("\n", violations));
    }

    [Fact]
    public void SweepExemption_D2_NoOtherRoutePathViewIdentitySniffExistsOutsideMainLayoutRazor()
    {
        // The D2 exemption's own mechanical fingerprint: a
        // Nav.ToBaseRelativePath(Nav.Uri).StartsWith(...) route-path string
        // sniff used to derive a view identity -- structurally invisible to
        // D1's own ViewIdentityComparisonPattern scan (no ViewNames.* token
        // is involved at all), which is WHY this needed a controller ruling
        // rather than simply passing that scan.
        var sniffPattern = new Regex(@"Nav\.ToBaseRelativePath\(Nav\.Uri\)\.StartsWith\(", RegexOptions.Compiled);
        var violations = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
            var relative = Path.GetRelativePath(RepoRoot(), file).Replace('\\', '/');
            if (relative == "client/Layout/MainLayout.razor")
            {
                continue; // the one file the controller's D2 ruling was scoped to
            }

            var text = File.ReadAllText(file);
            if (sniffPattern.IsMatch(text))
            {
                violations.Add(relative);
            }
        }

        Assert.True(violations.Count == 0,
            "Found a Nav.ToBaseRelativePath(Nav.Uri).StartsWith(...) route-path view-identity sniff OUTSIDE MainLayout.razor -- the D2 exemption (controller ruling, \"cosmetic header theming\") was scoped to THAT file specifically; a second site needs its own reasoned ruling, not a silent free ride:\n" +
            string.Join("\n", violations));
    }

    // ------------------------------------------------------------------
    // Batch CORPREAD-1a, DELIVERABLE 0d ("conformance where scannable: the
    // DividerFraction default constant referenced from exactly one site
    // (grep-able)"), landed alongside SPLIT-5050 (the ticket that gives the
    // constant its one real read site). ViewArrangement.InitialDividerFraction
    // is the ONE named constant the no-stored-fraction initial-width
    // computation reads (named distinctly from EnterSplit's own, pre-
    // existing DefaultDividerFraction SEED parameter -- a genuinely
    // different concern -- to avoid exactly the name collision a bare
    // grep for "DefaultDividerFraction" would have produced against every
    // EnterSplit(..., DefaultDividerFraction: null) call site; see that
    // constant's own doc comment). A second production read site would mean
    // a second place could drift from 0.5 independently -- exactly the
    // "magic number, not a constant" shape the owner order rejected.
    // ------------------------------------------------------------------
    [Fact]
    public void InitialDividerFraction_ReferencedFromExactlyOneProductionSite()
    {
        var pattern = new Regex(@"InitialDividerFraction\b", RegexOptions.Compiled);
        var sites = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
            var relative = Path.GetRelativePath(RepoRoot(), file).Replace('\\', '/');
            if (relative == "client/State/ViewArrangement.cs")
            {
                continue; // the constant's own declaration site, not a read/reference
            }

            if (pattern.IsMatch(File.ReadAllText(file)))
            {
                sites.Add(relative);
            }
        }

        Assert.True(sites.Count == 1 && sites[0] == "client/Components/CompositionSplit.razor",
            "ViewArrangement.InitialDividerFraction must be referenced from EXACTLY ONE production site (CompositionSplit.razor's own OnAfterRenderAsync, SPLIT-5050's sole initial-width computation) -- found: " +
            (sites.Count == 0 ? "(none -- the site was removed or renamed)" : string.Join(", ", sites)));
    }
}
