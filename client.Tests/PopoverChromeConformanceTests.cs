using BibleAtlas.Client.Tests.State;
using System.Text.RegularExpressions;
using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Tests;

public class PopoverChromeConformanceTests
{
    private const int ConcreteExplorableNodeClasses = 10;

    private static string StripLineComments(string text) => Regex.Replace(text, "//[^\n]*", string.Empty);

    private static IEnumerable<string> ExploreNodeFiles()
    {
        var exploreDir = Path.Combine(ConformanceTests.RepoRoot(), "client", "Legacy");
        return Directory.EnumerateFiles(exploreDir, "*Node.cs", SearchOption.TopDirectoryOnly);
    }

    private static string Normalize(string path) => path.Replace('\\', '/');

    private static readonly Regex KindPattern = new(@"public string Kind\s*=>\s*""([^""]+)"";", RegexOptions.Compiled);

    private static readonly Regex ChipTestIdArgPattern = new(
        @"new Chip\([^,]*,\s*\$?""([^""]*)""",
        RegexOptions.Compiled);

    private static bool IsDeclaredChip(string kind, string chipTestId) =>
        PopoverChromeRegistry.ByKind.TryGetValue(kind, out var declared)
        && declared.Any(d => d.Matches(chipTestId));

    private static List<string> ChipTestIdsIn(string sourceText) =>
        ChipTestIdArgPattern.Matches(StripLineComments(sourceText))
            .Select(m => m.Groups[1].Value)
            .ToList();

    [Fact]
    public void EveryNodeFile_EveryChipItConstructs_IsDeclaredForItsOwnKind()
    {
        var violations = new List<string>();
        var filesChecked = 0;

        foreach (var file in ExploreNodeFiles())
        {
            var relative = Normalize(Path.GetRelativePath(ConformanceTests.RepoRoot(), file));
            var text = StripLineComments(File.ReadAllText(file));

            var kindMatch = KindPattern.Match(text);
            Assert.True(kindMatch.Success, $"{relative}: no `public string Kind => \"...\";` found -- every *Node.cs file under client/Legacy is expected to declare one (IExplorable.Kind).");
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

        Assert.Equal(ConcreteExplorableNodeClasses, filesChecked);
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
            Assert.True(realKinds.Contains(declaredKind), $"PopoverChromeRegistry declares a Kind (\"{declaredKind}\") that no real client/Legacy/*Node.cs file's own IExplorable.Kind produces -- a stale/orphaned registry row.");
        }

        Assert.Equal(realKinds.Count, PopoverChromeRegistry.ByKind.Count);
        Assert.Equal(ConcreteExplorableNodeClasses, PopoverChromeRegistry.ByKind.Count);
    }

    [Fact]
    public void PlantedUndeclaredChip_OnARealKind_FailsTheCheck()
    {
        Assert.False(IsDeclaredChip("Verse", "popover-chip-map"), "The planted chip is undeclared for \"Verse\" -- the check itself must reject it, or the real-tree scan above is vacuous.");

        Assert.False(IsDeclaredChip("Verse", "popover-chip-xrefs"));
        Assert.False(IsDeclaredChip("Event", "popover-chip-xrefs"));
    }

    [Fact]
    public void PlantedUndeclaredChip_OnAnUnknownKind_FailsTheCheck()
    {
        Assert.False(IsDeclaredChip("NotARealKind", "popover-chip-map"));
    }

    [Fact]
    public void PlantedSourceShape_AnUndeclaredChipInAExploreAsyncBody_IsCaughtByTheSameExtractionTheRealScanUses()
    {
        const string planted = """
            public string Kind => "Passage";

            public Task<IReadOnlyList<Chip>> ExploreAsync(AtlasClient api)
            {
                IReadOnlyList<Chip> list = new[]
                {
                    new Chip("About this book", "popover-chip-book", new ChipTarget.Push(new AuthorNode(book), EdgeKind.MemberOf)),
                    new Chip("Read in context", "popover-chip-context", new ChipTarget.NavigateReader(book, chapter, verse)),
                    new Chip("A planted dead chip", "popover-chip-planted-violation", new ChipTarget.NavigateWorld("ref=GEN")),
                };
                return Task.FromResult(list);
            }
            """;

        var chips = ChipTestIdsIn(planted);
        Assert.Equal(new[] { "popover-chip-book", "popover-chip-context", "popover-chip-planted-violation" }, chips);

        var kind = KindPattern.Match(StripLineComments(planted)).Groups[1].Value;
        Assert.Equal("Passage", kind);

        var undeclared = chips.Where(c => !IsDeclaredChip(kind, c)).ToList();
        Assert.Equal(new[] { "popover-chip-planted-violation" }, undeclared);
    }

    [Fact]
    public void StripLineComments_ADocCommentNamingARealChipTestId_IsNotSelfFlagged()
    {
        const string commentedOut = "    // history: this used to be `new Chip(\"x\", \"popover-chip-retired-example\", target)`.";

        Assert.Matches(ChipTestIdArgPattern, commentedOut);
        Assert.Empty(ChipTestIdsIn(commentedOut));
    }
}
