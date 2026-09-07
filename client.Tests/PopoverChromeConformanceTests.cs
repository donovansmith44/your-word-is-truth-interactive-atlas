using System.Text.RegularExpressions;
using BibleAtlas.Client.Explore;

namespace BibleAtlas.Client.Tests;

/// <summary>
/// Batch UX-2 (CHROME-UNIFORMITY-1, owner: "we're not reusing the
/// structure for our frontiers"): a conformance-style scan, same discipline
/// as <c>AsyncMemoConformanceTests.cs</c>/<c>State/ViewRegistryConformanceTests.cs</c>'s
/// own established precedent (a <c>RepoRoot</c>/real-tree file scan, a
/// bounded regex, a comment-stripped source read, a shared predicate both
/// the real scan and the planted-violation proof call, a "never vacuous"
/// count pin). Reads every real <c>client/Explore/*Node.cs</c> file (the 14
/// concrete <see cref="IExplorable"/> node classes -- confirmed by the same
/// naming convention every one of them already follows), extracts the chip
/// testids each one's own <c>ExploreAsync</c> actually constructs via
/// <c>new Exploration(...)</c>, and fails loud if ANY of them is not a
/// <see cref="PopoverChromeRegistry"/>-declared chip for that node's own
/// <c>Kind</c>. A future dead branch (a chip added to one node's
/// <c>ExploreAsync</c> without registering it centrally -- the exact
/// ambiguity CHROME-1/CHROME-1b's own live-tracing had to resolve by hand,
/// one node kind at a time) is now caught here, at test time, instead.
/// </summary>
public class PopoverChromeConformanceTests
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

    // Same disclosed-limitation stripper AsyncMemoConformanceTests.cs/
    // ConformanceTests.cs already establish -- applied so this file's own
    // doc comments (which name real testids like "popover-chip-map" in
    // prose) never self-trigger the scan they document.
    private static string StripLineComments(string text) => Regex.Replace(text, "//[^\n]*", string.Empty);

    private static IEnumerable<string> ExploreNodeFiles()
    {
        var exploreDir = Path.Combine(RepoRoot(), "client", "Explore");
        return Directory.EnumerateFiles(exploreDir, "*Node.cs", SearchOption.TopDirectoryOnly);
    }

    private static string Normalize(string path) => path.Replace('\\', '/');

    private static readonly Regex KindPattern = new(@"public string Kind\s*=>\s*""([^""]+)"";", RegexOptions.Compiled);

    // Matches the SECOND positional argument of `new Exploration(label, chipTestId, target)`
    // -- an ordinary string literal ("popover-chip-book") or an interpolated
    // one ($"popover-chip-verse-{v}"); the first argument (a label string or
    // a bare identifier, never containing a comma in any real call site --
    // "About this book", "Read in context", "Show on /world", "Show on the
    // map", "Show this time on the map", or YearNode's own bare `v`) is
    // consumed by `[^,]*` so the SECOND quoted string is what this group
    // captures, not the first.
    private static readonly Regex ChipTestIdArgPattern = new(
        @"new Exploration\([^,]*,\s*\$?""([^""]*)""",
        RegexOptions.Compiled);

    /// <summary>
    /// The shared check BOTH the real-tree scan and the planted-violation
    /// proof below call -- ViewRegistryConformanceTests.cs's own Q-5
    /// discipline (a self-referential assertion that re-implements the
    /// predicate inline proves nothing about whether the REAL check would
    /// catch the same defect). A chip testid actually found in source for
    /// <paramref name="kind"/> is declared IFF <see cref="PopoverChromeRegistry.ByKind"/>
    /// has an entry for that Kind AND one of its declarations matches it
    /// (exact or prefix, per <see cref="PopoverChromeRegistry.ChipDeclaration.Matches"/>).
    /// An undeclared KIND (no row at all) makes every one of its chips
    /// undeclared too -- there is no separate "unknown kind" escape.
    /// </summary>
    private static bool IsDeclaredChip(string kind, string chipTestId) =>
        PopoverChromeRegistry.ByKind.TryGetValue(kind, out var declared)
        && declared.Any(d => d.Matches(chipTestId));

    /// <summary>
    /// Runs the SAME extraction the real-tree scan uses against an
    /// arbitrary source text -- shared by the real test and the planted-
    /// source-shape proof below, so both exercise identical extraction
    /// logic, not two hand-written copies that could quietly drift apart.
    /// </summary>
    private static List<string> ChipTestIdsIn(string sourceText) =>
        ChipTestIdArgPattern.Matches(StripLineComments(sourceText))
            .Select(m => m.Groups[1].Value)
            .ToList();

    // ------------------------------------------------------------------
    // THE REAL SCAN.
    // ------------------------------------------------------------------

    [Fact]
    public void EveryNodeFile_EveryChipItConstructs_IsDeclaredForItsOwnKind()
    {
        var violations = new List<string>();
        var filesChecked = 0;

        foreach (var file in ExploreNodeFiles())
        {
            var relative = Normalize(Path.GetRelativePath(RepoRoot(), file));
            var text = StripLineComments(File.ReadAllText(file));

            var kindMatch = KindPattern.Match(text);
            Assert.True(kindMatch.Success, $"{relative}: no `public string Kind => \"...\";` found -- every *Node.cs file under client/Explore is expected to declare one (IExplorable.Kind).");
            var kind = kindMatch.Groups[1].Value;
            filesChecked++;

            foreach (var chipTestId in ChipTestIdsIn(text))
            {
                if (!IsDeclaredChip(kind, chipTestId))
                {
                    violations.Add($"{relative} (Kind=\"{kind}\"): emits undeclared chip \"{chipTestId}\"");
                }
            }
        }

        Assert.Equal(14, filesChecked); // never vacuous -- the 14 concrete IExplorable node classes, pinned
        Assert.True(violations.Count == 0,
            "A node's own ExploreAsync constructs a popover-chip-* testid that PopoverChromeRegistry.ByKind " +
            "does not declare for that Kind -- either the chip is a genuine new affordance (declare it in " +
            "PopoverChromeRegistry.cs, the ONE declaration site) or it is a dead/leftover branch that should " +
            "not ship (CHROME-1/CHROME-1b's own investigation is exactly the shape this guards against):\n" +
            string.Join("\n", violations));
    }

    [Fact]
    public void PopoverChromeRegistry_EveryDeclaredKind_MatchesARealNodeFilesKind()
    {
        var realKinds = ExploreNodeFiles()
            .Select(f => KindPattern.Match(StripLineComments(File.ReadAllText(f))).Groups[1].Value)
            .ToHashSet();

        foreach (var declaredKind in PopoverChromeRegistry.ByKind.Keys)
        {
            Assert.True(realKinds.Contains(declaredKind), $"PopoverChromeRegistry declares a Kind (\"{declaredKind}\") that no real client/Explore/*Node.cs file's own IExplorable.Kind produces -- a stale/orphaned registry row.");
        }

        // Never vacuous, and pins the registry's own total shape: exactly
        // the 14 real Kinds, one row each, no more, no fewer.
        Assert.Equal(realKinds.Count, PopoverChromeRegistry.ByKind.Count);
        Assert.Equal(14, PopoverChromeRegistry.ByKind.Count);
    }

    // ------------------------------------------------------------------
    // PLANTED-VIOLATION PROOF (this house's own established style, e.g.
    // ViewRegistryConformanceTests.HatchConformance_PlantedBearsLocusViewWithNoToggleFollowHatch_FailsTheLawsOwnCheck,
    // AsyncMemoConformanceTests.PlantedDirectShapeViolation_IsCaught): proves
    // the CHECK ITSELF genuinely fails on a defect shaped exactly like the
    // one this ticket names, not merely that today's real tree happens to
    // pass. Both proofs below call the SAME IsDeclaredChip/ChipTestIdsIn
    // helpers the real tests above use -- a future weakening of either
    // helper breaks the planted proof too, not just the real-tree test
    // silently.
    // ------------------------------------------------------------------

    [Fact]
    public void PlantedUndeclaredChip_OnARealKind_FailsTheCheck()
    {
        // "Verse" is a real, declared Kind (popover-chip-book/-context
        // only) -- popover-chip-map was never one of its declared chips
        // (VerseNode has no map scene to bracket for a bare in-canon
        // locus; O1 retired its one-time geo-temporal chip entirely). A
        // hypothetical future edit that gave VerseNode a stray, dead map
        // chip without registering it is exactly what this must catch.
        Assert.False(IsDeclaredChip("Verse", "popover-chip-map"), "The planted chip is undeclared for \"Verse\" -- the check itself must reject it, or the real-tree scan above is vacuous.");

        // The retired popover-chip-xrefs toggle (O1, ExplorerPopover.razor's
        // own header comment: "the whole mechanism... GONE") -- a genuinely
        // historical dead chip this registry correctly never re-admits for
        // any Kind.
        Assert.False(IsDeclaredChip("Verse", "popover-chip-xrefs"));
        Assert.False(IsDeclaredChip("Event", "popover-chip-xrefs"));
    }

    [Fact]
    public void PlantedUndeclaredChip_OnAnUnknownKind_FailsTheCheck()
    {
        // No registry row at all -- every chip is undeclared, by
        // construction (IsDeclaredChip's own "no separate unknown-kind
        // escape" rule, this file's own header comment on that helper).
        Assert.False(IsDeclaredChip("NotARealKind", "popover-chip-map"));
    }

    [Fact]
    public void PlantedSourceShape_AnUndeclaredChipInAExploreAsyncBody_IsCaughtByTheSameExtractionTheRealScanUses()
    {
        // A synthetic node file's own ExploreAsync -- shaped exactly like a
        // real one (VerseNode.cs's own two-chip body), but with a THIRD,
        // undeclared chip planted in. Proves the regex extraction the real
        // scan depends on genuinely finds it (not just that IsDeclaredChip
        // rejects a testid handed to it directly, the narrower proof
        // immediately above).
        const string planted = """
            public string Kind => "Verse";

            public Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api)
            {
                IReadOnlyList<Exploration> list = new[]
                {
                    new Exploration("About this book", "popover-chip-book", new ExplorationTarget.Push(new AuthorNode(book))),
                    new Exploration("Read in context", "popover-chip-context", new ExplorationTarget.NavigateReader(book, chapter, verse)),
                    new Exploration("A planted dead chip", "popover-chip-planted-violation", new ExplorationTarget.NavigateWorld("ref=GEN")),
                };
                return Task.FromResult(list);
            }
            """;

        var chips = ChipTestIdsIn(planted);
        Assert.Equal(new[] { "popover-chip-book", "popover-chip-context", "popover-chip-planted-violation" }, chips);

        var kind = KindPattern.Match(StripLineComments(planted)).Groups[1].Value;
        Assert.Equal("Verse", kind);

        var undeclared = chips.Where(c => !IsDeclaredChip(kind, c)).ToList();
        Assert.Equal(new[] { "popover-chip-planted-violation" }, undeclared);
    }

    [Fact]
    public void StripLineComments_ADocCommentNamingARealChipTestId_IsNotSelfFlagged()
    {
        // This file's own doc comments quote real testids ("popover-chip-map",
        // etc.) as prose throughout -- proving a `new Exploration(...)`-shaped
        // line INSIDE a `//` comment is stripped before the scan ever sees
        // it (the same "explain the rule without tripping it" property
        // AsyncMemoConformanceTests.cs's own equivalent test proves).
        const string commentedOut = "    // history: this used to be `new Exploration(\"x\", \"popover-chip-retired-example\", target)`.";

        Assert.Matches(ChipTestIdArgPattern, commentedOut); // sanity: the raw text DOES match before stripping
        Assert.Empty(ChipTestIdsIn(commentedOut)); // ChipTestIdsIn strips comments first -- the real scan never sees this
    }
}
