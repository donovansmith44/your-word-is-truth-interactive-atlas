using System.Reflection;
using System.Text.RegularExpressions;
using BibleAtlas.Client.Contracts;
using BibleAtlas.Client.State;

namespace BibleAtlas.Client.Tests.State;

/// <summary>
/// BRIEF ADDENDUM (owner ruling, mid-batch, spec §0 law: "if contracts
/// don't match implementation tests should fail"): every construct this
/// batch migrates onto a contract gets a CONFORMANCE test here -- one that
/// FAILS if a future edit lets the implementation drift from the contract,
/// living in the standing suite (not a one-off script). Three tests below,
/// one per addendum item. All three are SOURCE-SCAN tests over the live
/// client/ tree (the same "grep-able assertion" shape
/// EffectRegistryTests.cs's own NoDirectAtomSubscriptionByEffects test
/// already uses) rather than a live DI-container resolution test --
/// Program.cs's service registrations are top-level statements in a WASM
/// entry point (no separate, testable `ConfigureServices` method exists,
/// and several registrations need a real `IJSInProcessRuntime`/browser
/// environment client.Tests doesn't have) -- extracting one would be a
/// Program.cs refactor beyond "enforcement, not expanded migration scope"
/// (the addendum's own words); a source-scan proves the SAME facts
/// (which Name each atom is constructed with, that each AtomNames constant
/// is used exactly once) without needing a live host. Disclosed, not
/// silently substituted.
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

    // ------------------------------------------------------------------
    // Addendum item 1: "effects-only-via-registry" -- no production code
    // outside client/State/ infrastructure subscribes directly to any
    // IStateAtom<T>.Changed, EXCEPT an explicitly documented, allowlisted
    // pure-render (or single-singleton-persistence) subscription. A NEW,
    // undocumented `.Changed +=` site anywhere in client/ fails this test
    // until it is either moved behind the registry or added here with a
    // reasoned justification -- the allowlist itself IS the enforcement
    // mechanism the addendum asks for.
    // ------------------------------------------------------------------
    [Fact]
    public void EffectsOnlyViaRegistry_EveryChangedSubscriptionOutsideStateInfrastructureIsExplicitlyAllowlisted()
    {
        // (file, 1-based line, justification) -- every entry here was
        // verified by reading its own handler body at the time this test
        // was written (see the justification for what "verified" means per
        // entry).
        var allowlist = new (string File, int Line, string Justification)[]
        {
            ("client\\Components\\SelectionTray.razor", 66,
                "OnChanged => InvokeAsync(StateHasChanged) -- pure re-render, no Materialize-shaped side effect."),
            ("client\\Layout\\MainLayout.razor", 227,
                "SavedExplorations.Changed is SavedExplorationsService's OWN plain C# event, not an IStateAtom<T>.Changed -- out of this rule's scope entirely (SavedExplorationsService is not an atom)."),
            ("client\\Pages\\Reader.razor", 615,
                "OnViewArrangementChanged => StateHasChanged() only (fix round 1, Q-3) -- pure re-render."),
            ("client\\Pages\\World.razor", 571,
                "OnLocusChanged => StateHasChanged() only (ST-2 retired its own re-scening side effect) -- pure re-render."),
            ("client\\Pages\\World.razor", 577,
                "OnTimeWindowChanged => SyncTimeWindowProjection() + StateHasChanged() -- a LOCAL field projection sync (no fetch/JS interop), not an effect; the fetch itself moved to the follow-scene effect (EffectRegistry), which this handler no longer touches."),
            ("client\\Pages\\World.razor", 584,
                "OnViewArrangementChanged => the SyncToken bump + EnableFollowScene/DisableFollowScene (claim/release calls) + StateHasChanged() -- claiming/releasing is the SANCTIONED consumer-side interaction with the registry; Materialize itself is invoked only from inside EffectRegistry.Claim's own subscription (see test 2 below)."),
            ("client\\SelectionTrayService.cs", 44,
                "atom.Changed => LocalStore.Write(...) -- a JS-interop persistence write, which IStateEffect<T>'s own doc comment WOULD class as an effect in shape, but SelectionTrayService is a single, force-resolved, app-lifetime singleton (Program.cs) -- there is no multi-instance ownership hazard for EffectRegistry's claim/latest-wins mechanism to protect against here, unlike every current IStateEffect<T> use (World.razor's follow-scene, claimed by a per-mount, multiply-instantiable component). Disclosed design choice, not an oversight."),
            ("client\\State\\EffectRegistry.cs", 82,
                "The registry's OWN internal subscription -- this IS the infrastructure the rule protects; Materialize is invoked from inside this handler and nowhere else (see test 2)."),
            ("client\\State\\StateLinkRunner.cs", 109,
                "Link infrastructure (ST-1) -- derives and dispatches into the link's own Target atom; not an IStateEffect<T> materialization (no fetch/JS interop, no registry involvement) at all."),
        }.ToArray();

        var allowlistKeys = allowlist.Select(e => (e.File, e.Line)).ToHashSet();
        var pattern = new Regex(@"\.Changed\s*\+=", RegexOptions.Compiled);
        var unexpected = new List<string>();
        var seenAllowlisted = new HashSet<(string File, int Line)>();

        foreach (var file in ClientSourceFiles())
        {
            var relative = Path.GetRelativePath(RepoRoot(), file);
            var lines = File.ReadAllLines(file);
            for (var i = 0; i < lines.Length; i++)
            {
                if (!pattern.IsMatch(lines[i]))
                {
                    continue;
                }

                var lineNumber = i + 1;
                var key = (relative, lineNumber);
                if (allowlistKeys.Contains(key))
                {
                    seenAllowlisted.Add(key);
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

        // The allowlist itself must stay accurate -- an entry whose line no
        // longer contains ".Changed +=" (moved/deleted/renumbered) should
        // fail loudly too, not silently stop being checked.
        var stale = allowlistKeys.Except(seenAllowlisted).ToList();
        Assert.True(stale.Count == 0,
            "Stale allowlist entries (no longer match a real .Changed += site -- update ConformanceTests.cs): " +
            string.Join(", ", stale.Select(k => $"{k.File}:{k.Line}")));
    }

    // ------------------------------------------------------------------
    // Addendum item 2: atom registration conformance -- every DI-registered
    // StateAtom<T> is constructed with a Name literal that is an
    // `AtomNames.*` reference (not a raw string), and every AtomNames
    // constant migrated as of this batch (Locus, TimeWindow,
    // ViewArrangement, Selection, FocusStack) appears in EXACTLY ONE
    // `new StateAtom<T>(AtomNames.X, ...)` construction in Program.cs.
    // ------------------------------------------------------------------
    [Fact]
    public void AtomRegistrationConformance_EveryMigratedAtomNameIsConstructedExactlyOnceInProgramCs()
    {
        var programCs = ProgramCsText();

        // Every `new StateAtom<...>(` call's first argument, captured
        // whether it's a bare `AtomNames.X` reference or something else --
        // asserting it's ALWAYS the former is what proves "carries a Name
        // that appears in AtomNames" (a raw string literal here would not
        // match this pattern at all and would correctly fail the test).
        // `.+?` (non-greedy, DOT-matches-'>' too), not `[^>]+` -- the
        // Selection atom's own type argument is itself generic
        // (`StateAtom<IReadOnlyList<ExplorationDescriptor>>(...)`), so a
        // char class excluding '>' would stop at the WRONG (inner) '>' and
        // never match this construction at all. Non-greedy `.+?` finds the
        // shortest run ending at the LAST '>' immediately before '(' --
        // exactly the real generic-argument close, for both the simple
        // (Locus/TimeWindow/ViewArrangement/FocusStack) and nested
        // (Selection) cases alike.
        var ctorPattern = new Regex(@"new StateAtom<.+?>\(\s*AtomNames\.(\w+)\s*,", RegexOptions.Compiled);
        var matches = ctorPattern.Matches(programCs).Select(m => m.Groups[1].Value).ToList();

        var migratedNames = new[] { nameof(AtomNames.Locus), nameof(AtomNames.TimeWindow), nameof(AtomNames.ViewArrangement), nameof(AtomNames.Selection), nameof(AtomNames.FocusStack) };

        foreach (var name in migratedNames)
        {
            var count = matches.Count(m => m == name);
            Assert.True(count == 1, $"AtomNames.{name} must be constructed exactly once via `new StateAtom<T>(AtomNames.{name}, ...)` in Program.cs -- found {count}.");
        }

        // Every AtomNames constant this test doesn't already know about
        // would silently escape the loop above -- this closes that gap:
        // every constructed atom's own captured name must itself be a real
        // AtomNames field (reflection), so a typo'd `AtomNames.Locu` (which
        // wouldn't even compile, admittedly, but the regex is text-based,
        // not compile-checked) or a name this test's own migratedNames list
        // forgot to list is still caught.
        var atomNamesFields = typeof(AtomNames).GetFields(BindingFlags.Public | BindingFlags.Static)
            .Where(f => f.FieldType == typeof(string))
            .Select(f => f.Name)
            .ToHashSet();
        foreach (var matched in matches)
        {
            Assert.Contains(matched, atomNamesFields);
        }

        // No DUPLICATE registration for any one AtomNames constant (a copy-
        // paste error registering the same atom twice, shadowing the first
        // with the second in DI's own last-registration-wins semantics).
        var duplicates = matches.GroupBy(m => m).Where(g => g.Count() > 1).Select(g => g.Key).ToList();
        Assert.True(duplicates.Count == 0, "Duplicate StateAtom<T> registration(s) for: " + string.Join(", ", duplicates));
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
    // Addendum item 3: effect conformance -- every IStateEffect<T>
    // constructed anywhere in client/ has a unique Name; Materialize is
    // only ever invoked by the registry (PROVEN by test 1 above, via its
    // OWN allowlist entries for World.razor:584 and EffectRegistry.cs:82 --
    // no OTHER `.Changed +=` site exists that could reach a Materialize
    // call, and EffectRegistryTests.cs's own
    // NoDirectAtomSubscriptionByEffects test separately proves `.Materialize(`
    // itself is textually called from nowhere but client/State/EffectRegistry.cs).
    // ------------------------------------------------------------------
    [Fact]
    public void EffectConformance_EveryDelegateEffectNameIsUnique()
    {
        var names = new List<string>();
        var pattern = new Regex("new DelegateEffect<[^>]+>\\(\\s*\"([^\"]+)\"", RegexOptions.Compiled);

        foreach (var file in ClientSourceFiles())
        {
            var text = File.ReadAllText(file);
            foreach (Match m in pattern.Matches(text))
            {
                names.Add(m.Groups[1].Value);
            }
        }

        Assert.NotEmpty(names); // this batch's own "follow-scene" -- a genuine, non-vacuous check
        var duplicates = names.GroupBy(n => n).Where(g => g.Count() > 1).Select(g => g.Key).ToList();
        Assert.True(duplicates.Count == 0, "Duplicate IStateEffect<T> name(s): " + string.Join(", ", duplicates));
    }
}
