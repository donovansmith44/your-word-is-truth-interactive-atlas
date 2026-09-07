namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch UX-2 (CHROME-UNIFORMITY-1, the structural answer to the owner's
/// "we're not reusing the structure for our frontiers"): ONE declaration
/// site naming every <c>popover-chip-*</c> testid a given <see cref="IExplorable.Kind"/>
/// is allowed to ever emit from its own <see cref="IExplorable.ExploreAsync"/>.
/// <c>client.Tests/PopoverChromeConformanceTests.cs</c> scans every real
/// <c>client/Explore/*.cs</c> node file, extracts the actual chip testids
/// each one's own <c>ExploreAsync</c> constructs, and fails loud if any Kind
/// emits a chip this registry does NOT declare for it -- a future dead
/// branch (a stray, hand-added chip nobody wired a real handler for, or
/// left over from a retired feature -- CHROME-1's own "the dead button"
/// investigation is exactly the shape this guards against) is caught at
/// test time, not left for a future owner live-preview to notice by hand.
///
/// Every entry below is transcribed DIRECTLY from a live read of the
/// current source (this batch's own chrome-per-kind diff, see
/// batch-ux2-report.md) -- not inferred or guessed. "chrome" here is
/// SCOPED to the per-Kind-VARYING chip vocabulary specifically (the part
/// CHROME-1/1b's own investigation turned on) -- <c>popover-breadcrumb-
/// back</c>/<c>popover-save-exploration</c>/<c>popover-close</c> are
/// UNIVERSAL, rendered identically by <c>ExplorerPopover.razor</c>
/// regardless of the current node's Kind (conditional on stack depth /
/// storage availability, never on Kind), so they carry no per-Kind
/// declaration here -- declaring them once per Kind would be 14 identical
/// rows, ceremony over signal.
///
/// A Kind entry's set may be EMPTY (Catechism/Person/CommentaryItem/
/// ConcordUnit correctly offer zero chips today -- "conditional presence
/// extends to affordances too," CONTRACT.md's own CATECH-1 wording) --
/// empty is a valid, deliberate declaration, not an omission.
/// </summary>
public static class PopoverChromeRegistry
{
    /// <summary>
    /// A single declared chip: either an EXACT testid (most chips -- a
    /// fixed vocabulary member, e.g. <c>popover-chip-map</c>) or a PREFIX
    /// (<see cref="IsPrefix"/> true -- <c>YearNode</c>'s own
    /// <c>popover-chip-verse-{VREF}</c> family, one real testid per curated
    /// supporting verse, a per-instance suffix no fixed string can name).
    /// </summary>
    public readonly record struct ChipDeclaration(string TestId, bool IsPrefix)
    {
        public static ChipDeclaration Exact(string testId) => new(testId, IsPrefix: false);
        public static ChipDeclaration Prefix(string prefix) => new(prefix, IsPrefix: true);

        /// True when <paramref name="actual"/> (a real testid found in the
        /// source scan) satisfies this declaration.
        public bool Matches(string actual) => IsPrefix ? actual.StartsWith(TestId, StringComparison.Ordinal) : actual == TestId;
    }

    private static ChipDeclaration[] Exact(params string[] testIds) => testIds.Select(ChipDeclaration.Exact).ToArray();

    /// <summary>
    /// Kind -&gt; the closed set of chips that Kind's own <c>ExploreAsync</c>
    /// may emit. Every one of the 14 concrete <see cref="IExplorable"/>
    /// classes in <c>client/Explore/*.cs</c> (grep-confirmed -- the same
    /// count <c>ExplorationDescriptor.Capture</c>'s own header names, plus
    /// the two later-added Batch CORP-1 kinds, CommentaryItem/ConcordUnit)
    /// has exactly one row -- <c>PopoverChromeConformanceTests</c>'s own
    /// "every Kind has a declaration, every declaration matches a real
    /// Kind" test pins this 1:1 mapping, not just the per-row content.
    /// </summary>
    public static readonly IReadOnlyDictionary<string, ChipDeclaration[]> ByKind = new Dictionary<string, ChipDeclaration[]>
    {
        // Reading-hierarchy kinds -- "About this book"/"Read in context"
        // (Book/Chapter carry the map chip too; Verse/Passage do not --
        // there is no map scene to bracket for a bare in-canon locus, only
        // once a book/chapter's own real-world writing window is known).
        ["Verse"] = Exact("popover-chip-book", "popover-chip-context"),
        ["Passage"] = Exact("popover-chip-book", "popover-chip-context"),
        ["Chapter"] = Exact("popover-chip-map", "popover-chip-context", "popover-chip-book"),
        ["Book"] = Exact("popover-chip-map", "popover-chip-context", "popover-chip-book"),

        // Frontier kinds reached by leaving the base text -- each offers
        // AT MOST the one "Show on /world"/"Show on the map" chip,
        // conditional on the underlying data actually carrying a
        // when/where to bracket (AuthorNode: WritePlace+WriteFrom/To;
        // EventNode: When; PolityDeltaNode: unconditional, always has a
        // window by construction) -- never MORE than one chip each.
        ["Author"] = Exact("popover-chip-map"),
        ["Place"] = Exact("popover-chip-map"),
        ["TimeAndPlace"] = Exact("popover-chip-map"),
        ["Event"] = Exact("popover-chip-map"),
        ["PolityDelta"] = Exact("popover-chip-map"),

        // YearNode: the one Kind with a genuinely PER-INSTANCE chip family
        // (one popover-chip-verse-{VREF} per curated supporting verse,
        // DATE-1's own "supporting verses first" ordering) ALONGSIDE the
        // same map chip every other frontier above offers.
        ["Year"] = new[] { ChipDeclaration.Prefix("popover-chip-verse-"), ChipDeclaration.Exact("popover-chip-map") },

        // Zero-chip kinds -- no geography/authorship of their own to
        // target ("a CATECHISM node offers NO chips at all," CONTRACT.md
        // verbatim) -- an EMPTY array is the deliberate declaration, not a
        // missing one.
        ["Catechism"] = Array.Empty<ChipDeclaration>(),
        ["Person"] = Array.Empty<ChipDeclaration>(),
        ["CommentaryItem"] = Array.Empty<ChipDeclaration>(),
        ["ConcordUnit"] = Array.Empty<ChipDeclaration>(),
    };
}
