using System.Reflection;
using System.Text.RegularExpressions;
using BibleAtlas.Client;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.State;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.JSInterop;

namespace BibleAtlas.Client.Tests.State;

public class ConformanceTests
{
    internal static string RepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null && !Directory.Exists(Path.Combine(dir.FullName, "client")))
        {
            dir = dir.Parent;
        }

        Assert.NotNull(dir);
        return dir!.FullName;
    }

    internal static string ClientRoot => Path.Combine(RepoRoot(), "client");

    internal static string SourceUnder(string repoRelativeRoot) =>
        string.Join("\n", SourceFilesUnder(repoRelativeRoot).Select(File.ReadAllText));

    internal static IEnumerable<string> ClientSourceFiles() => SourceFilesUnder("client");

    private static IEnumerable<string> SourceFilesUnder(string repoRelativeRoot) =>
        Directory.EnumerateFiles(Path.Combine(RepoRoot(), repoRelativeRoot), "*.*", SearchOption.AllDirectories)
            .Where(IsHandWrittenSourceFile);

    private static bool IsHandWrittenSourceFile(string path) =>
        (path.EndsWith(".cs") || path.EndsWith(".razor"))
        && !path.Contains($"{Path.DirectorySeparatorChar}obj{Path.DirectorySeparatorChar}")
        && !path.Contains($"{Path.DirectorySeparatorChar}bin{Path.DirectorySeparatorChar}");

    private static string ProgramCsText() => File.ReadAllText(Path.Combine(ClientRoot, "Program.cs"));

    private sealed class ThrowingJsRuntime : IJSInProcessRuntime
    {
        public TValue Invoke<TValue>(string identifier, params object?[]? args) =>
            throw new NotSupportedException("test double -- no real JS interop in client.Tests");

        public ValueTask<TValue> InvokeAsync<TValue>(string identifier, object?[]? args) =>
            throw new NotSupportedException("test double -- no real JS interop in client.Tests");

        public ValueTask<TValue> InvokeAsync<TValue>(string identifier, CancellationToken cancellationToken, object?[]? args) =>
            throw new NotSupportedException("test double -- no real JS interop in client.Tests");
    }

    [Fact]
    public void EffectsOnlyViaRegistry_EveryChangedSubscriptionOutsideStateInfrastructureIsExplicitlyAllowlisted()
    {
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
            ("client\\State\\PaneScope.cs", "source.Locus.Changed += Mirror",
                "D2 pane-link infrastructure (R-D2-1): the same-view host -> guest locus mirror -- dispatches the target atom's own SetLocus and nothing else (no fetch/JS interop); the sibling of StateLinkRunner's entry above."),
            ("client\\State\\PaneScope.cs", "source.TimeWindow.Changed += MirrorWindow",
                "D2 pane-link infrastructure (R-D2-1): the same-view host -> guest time-window mirror -- dispatches SetTimeWindow/SetScriptureWindow and nothing else."),
            ("client\\Components\\CompositionSplit.razor", "guestLocus.Changed += SyncSplitUrl",
                "D2: the unfollowed reader-guest's chapter is projected into `?guest=` by the SAME SyncSplitUrl the arrangement subscription above already runs -- a URL projection, not an effect."),
        }.ToArray();

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

        var stale = Enumerable.Range(0, allowlist.Length).Except(seenAllowlisted).Select(i => allowlist[i]).ToList();
        Assert.True(stale.Count == 0,
            "Stale allowlist entries (no longer match a real .Changed += site -- update ConformanceTests.cs): " +
            string.Join(", ", stale.Select(e => $"{e.File}: \"{e.ExpressionSubstring}\"")));
    }

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
        AssertSingletonWithName<Explore.ExplorationState>(provider, AtomNames.Exploration);
        AssertSingletonWithName<IReadOnlyList<Explore.ExplorationDescriptor>>(provider, AtomNames.Selection);
    }

    private static void AssertSingletonWithName<T>(IServiceProvider provider, string expectedName) where T : notnull
    {
        var first = provider.GetRequiredService<StateAtom<T>>();
        var second = provider.GetRequiredService<StateAtom<T>>();
        Assert.Same(first, second);
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

    [Fact]
    public void EffectConformance_EveryEffectNamesConstantValueIsUnique()
    {
        var values = typeof(EffectNames).GetFields(BindingFlags.Public | BindingFlags.Static)
            .Where(f => f.FieldType == typeof(string))
            .Select(f => (string)f.GetValue(null)!)
            .ToList();

        Assert.NotEmpty(values);
        Assert.Equal(values.Count, values.Distinct().Count());
    }

    [Fact]
    public void EffectConformance_EveryDelegateEffectConstructionReferencesAnEffectNamesConstant()
    {
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

    private static readonly string[] AtomValueTypeNames = { "Locus", "TimeWindow", "ViewArrangement", "FocusStack" };
    private static readonly string AtomValueTypeAlternation = string.Join("|", AtomValueTypeNames);

    private static readonly Regex AtomTypedFieldPattern = new(
        @"(?:private|public|protected|internal)\s+(?:static\s+)?(?:readonly\s+)?(?:" + AtomValueTypeAlternation + @")\??\s+(_\w+)",
        RegexOptions.Compiled);

    private static readonly Regex AtomTypedPropertyPattern = new(
        @"(?:private|public|protected|internal)\s+(?:static\s+)?(?:" + AtomValueTypeAlternation + @")\??\s+(\w+)\s*(?:\{|=>)",
        RegexOptions.Compiled);

    private static readonly Regex SelectionShapedListPattern = new(
        @"(?:private|public|protected|internal)\s+(?:static\s+)?(?:readonly\s+)?(?:List|IReadOnlyList|ImmutableArray)<ExplorationDescriptor>\s+(_\w+)",
        RegexOptions.Compiled);
    private static readonly Regex SelectionShapedArrayPattern = new(
        @"(?:private|public|protected|internal)\s+(?:static\s+)?(?:readonly\s+)?ExplorationDescriptor\[\]\s+(_\w+)",
        RegexOptions.Compiled);

    [Fact]
    public void NoComponentHeldSharedState_PlantedPropertyShapedViolation_IsCaught()
    {
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
            ("client\\Components\\CompositionSplit.razor", "_lastArrangement"),
            ("client\\Components\\ExplorerPopover.razor", "FocusValue"),
        };

        static string Normalize(string path) => path.Replace('\\', '/');

        var violations = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
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

    private static readonly Regex ArrangementStateInitializerPattern = new(
        @"(?:private|public|protected|internal)\s+(?:readonly\s+)?[^\n;{}]+?\s+(_?\w+)\s*(?:=>|=)\s*[^;]{0,400}?ViewArrangementAtom\.Value",
        RegexOptions.Compiled);

    private static readonly Regex ArrangementStateAssignmentPattern = new(
        @"(?<![.\w])(_\w+)\s*=\s*[^;=]{0,400}?ViewArrangementAtom\.Value",
        RegexOptions.Compiled);

    [Fact]
    public void NoComponentHeldArrangementState_PlantedRetiredShapeViolation_IsCaught()
    {
        const string planted = "private bool _splitOpen => SplitMode ?? (ViewArrangementAtom.Value is { LayoutKind: LayoutKinds.SplitH } a && a.Members.Count > 0 && a.Members[0] == ViewNames.Reader);";

        Assert.Matches(ArrangementStateInitializerPattern, planted);
    }

    [Fact]
    public void NoComponentHeldArrangementState_PlantedMultiLineViolation_IsCaught()
    {
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
                continue;
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

        foreach (var (file, _) in allowlist)
        {
            var path = Path.Combine(RepoRoot(), file.Replace('\\', Path.DirectorySeparatorChar));
            Assert.True(File.Exists(path), $"Stale allowlist entry -- file no longer exists: {file}");
            var text = File.ReadAllText(path);
            Assert.True(ArrangementStateInitializerPattern.IsMatch(text) || ArrangementStateAssignmentPattern.IsMatch(text),
                $"Stale allowlist entry -- {file} no longer matches the pattern it was allowlisted for.");
        }
    }

    private static readonly string[] OtherAtomNames = { "Locus", "TimeWindow", "Selection", "FocusStack" };
    private static readonly string OtherAtomAlternation = string.Join("|", OtherAtomNames.Select(n => n + "Atom"));

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
                continue;
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

    private static readonly Regex RoleFormulaRederivationPattern = new(
        @"(?:(?:LayoutKind|Layout\.Kind)\s*(?:==|:)\s*LayoutKinds\.SplitH|LayoutKinds\.SplitH\s*==\s*(?:LayoutKind|Layout\.Kind))[\s\S]{0,250}?Members\s*\[\s*0\s*\]" +
        @"|Members\s*\[\s*0\s*\][\s\S]{0,250}?(?:(?:LayoutKind|Layout\.Kind)\s*(?:==|:)\s*LayoutKinds\.SplitH|LayoutKinds\.SplitH\s*==\s*(?:LayoutKind|Layout\.Kind))",
        RegexOptions.Compiled);

    [Fact]
    public void RoleFormulaRederivation_PlantedRetiredReaderShapeViolation_IsCaught()
    {
        const string planted = "var isSplitOpen = ViewArrangementAtom.Value is { LayoutKind: LayoutKinds.SplitH } a && a.Members.Count > 0 && a.Members[0] == ViewNames.Reader;";

        Assert.Matches(RoleFormulaRederivationPattern, planted);
    }

    [Fact]
    public void RoleFormulaRederivation_PlantedInlineComparisonShapeViolation_IsCaught()
    {
        const string planted = "return other.Members[0].Name == target && other.Layout.Kind == LayoutKinds.SplitH;";

        Assert.Matches(RoleFormulaRederivationPattern, planted);
    }

    [Fact]
    public void RoleFormulaRederivation_NoRealSiteOutsideTheSanctionedDefinition()
    {
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

    private static readonly Regex ViewIdentityComparisonPattern = new(
        @"ViewNames\.\w+\s*(?:==|!=)|(?:==|!=)\s*ViewNames\.\w+",
        RegexOptions.Compiled);

    [Fact]
    public void ViewIdentityVsCapability_PlantedEqualityComparisonViolation_IsCaught()
    {
        const string planted = "if (member == ViewNames.Reader) { EnableFollowScene(); }";

        Assert.Matches(ViewIdentityComparisonPattern, planted);
    }

    [Fact]
    public void ViewIdentityVsCapability_PlantedInequalityComparisonViolation_IsCaught()
    {
        const string planted = "var isNotWorld = ViewNames.World != memberName;";

        Assert.Matches(ViewIdentityComparisonPattern, planted);
    }

    private static string StripLineComments(string text) => Regex.Replace(text, "//[^\n]*", string.Empty);

    [Fact]
    public void StripLineComments_DisclosedLimitation_ASlashSlashInsideAStringLiteralSilentlyEatsTheRestOfTheLine()
    {
        const string planted = "var note = \"see http://example.com//path\"; var isWorld = memberName == ViewNames.World;";

        Assert.Matches(ViewIdentityComparisonPattern, planted);

        var stripped = StripLineComments(planted);

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
                continue;
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

    [Fact]
    public void SweepExemption_A2_ViewStateServiceIsThePersistenceLayerBeneathAtoms()
    {
        var text = File.ReadAllText(Path.Combine(ClientRoot, "ViewStateService.cs"));

        Assert.Contains("public sealed class MapViewState", text);
        Assert.Contains("public sealed class ReaderViewState", text);
        Assert.DoesNotMatch(AtomTypedFieldPattern, text);
        Assert.DoesNotMatch(AtomTypedPropertyPattern, text);
    }

    [Fact]
    public void SweepExemption_D2_MainLayoutIsWorldIsCosmeticHeaderTheming()
    {
        var text = File.ReadAllText(Path.Combine(ClientRoot, "Layout", "MainLayout.razor"));

        Assert.Contains("Nav.ToBaseRelativePath(Nav.Uri).StartsWith(\"world\"", text);
        Assert.DoesNotContain("ViewNames.", text.Split('\n').First(l => l.Contains("IsWorld =>")));
    }

    [Fact]
    public void SweepExemption_A2_NoOtherViewStateNamedClassExistsOutsideViewStateServiceCs()
    {
        var classPattern = new Regex(@"(?:public|internal)\s+sealed\s+class\s+(\w*ViewState\w*)\b", RegexOptions.Compiled);
        var violations = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
            var relative = Path.GetRelativePath(RepoRoot(), file).Replace('\\', '/');
            if (relative == "client/ViewStateService.cs")
            {
                continue;
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
        var sniffPattern = new Regex(@"Nav\.ToBaseRelativePath\(Nav\.Uri\)\.StartsWith\(", RegexOptions.Compiled);
        var violations = new List<string>();
        foreach (var file in ClientSourceFiles())
        {
            var relative = Path.GetRelativePath(RepoRoot(), file).Replace('\\', '/');
            if (relative == "client/Layout/MainLayout.razor")
            {
                continue;
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
                continue;
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

    [Fact]
    public void OneScriptureSelectorLaw_EveryNonReaderPickerMountIsFollowGuarded()
    {
        var pages = Directory.GetFiles(Path.Combine(ClientRoot, "Pages"), "*.razor");
        var violations = new List<string>();
        foreach (var page in pages)
        {
            if (Path.GetFileName(page) == "Reader.razor") continue;
            var lines = File.ReadAllLines(page);
            for (var i = 0; i < lines.Length; i++)
            {
                if (!lines[i].Contains("<ScripturePicker", StringComparison.Ordinal)) continue;
                var window = string.Join("\n", lines.Skip(Math.Max(0, i - 6)).Take(6));
                var guarded = window.Contains("@if", StringComparison.Ordinal)
                    && (window.Contains("!(SplitMode &&", StringComparison.Ordinal)
                        || window.Contains("!(ctx.IsSplitOpen &&", StringComparison.Ordinal));
                if (!guarded)
                {
                    violations.Add($"{Path.GetFileName(page)}:{i + 1}");
                }
            }
        }

        Assert.True(violations.Count == 0, "ScripturePicker mounted without a follow guard at: " + string.Join(", ", violations));
    }
}
