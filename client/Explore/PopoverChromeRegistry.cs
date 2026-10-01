namespace BibleAtlas.Client.Explore;

public static class PopoverChromeRegistry
{
    public readonly record struct ChipDeclaration(string TestId, bool IsPrefix)
    {
        public static ChipDeclaration Exact(string testId) => new(testId, IsPrefix: false);
        public static ChipDeclaration Prefix(string prefix) => new(prefix, IsPrefix: true);

        public bool Matches(string actual) => IsPrefix ? actual.StartsWith(TestId, StringComparison.Ordinal) : actual == TestId;
    }

    private static ChipDeclaration[] Exact(params string[] testIds) => testIds.Select(ChipDeclaration.Exact).ToArray();

    public static readonly IReadOnlyDictionary<string, ChipDeclaration[]> ByKind = new Dictionary<string, ChipDeclaration[]>
    {
        ["Verse"] = Exact("popover-chip-book", "popover-chip-context"),
        ["Passage"] = Exact("popover-chip-book", "popover-chip-context"),
        ["Chapter"] = Exact("popover-chip-map", "popover-chip-context", "popover-chip-book"),
        ["Book"] = Exact("popover-chip-map", "popover-chip-context", "popover-chip-book"),

        ["Author"] = Exact("popover-chip-map"),
        ["Event"] = Exact("popover-chip-map"),
        ["PolityDelta"] = Exact("popover-chip-map"),

        ["Year"] = Exact("popover-chip-map"),

        ["Catechism"] = Array.Empty<ChipDeclaration>(),
        ["Person"] = Exact("popover-chip-year-born", "popover-chip-year-died", "popover-chip-year-span"),
        ["CommentaryItem"] = Array.Empty<ChipDeclaration>(),
        ["ConcordUnit"] = Array.Empty<ChipDeclaration>(),
    };
}
