namespace BibleAtlas.Client.Exploring;

public sealed record ConcordDocEntry(int Part, string Title);

public static class ConcordToc
{
    public static readonly IReadOnlyList<ConcordDocEntry> Documents = new[]
    {
        new ConcordDocEntry(1, "Preface to the Book of Concord"),
        new ConcordDocEntry(2, "The Three Ecumenical Creeds"),
        new ConcordDocEntry(3, "The Augsburg Confession"),
        new ConcordDocEntry(4, "Apology of the Augsburg Confession"),
        new ConcordDocEntry(5, "The Smalcald Articles"),
        new ConcordDocEntry(6, "Treatise on the Power and Primacy of the Pope"),
        new ConcordDocEntry(7, "The Small Catechism"),
        new ConcordDocEntry(8, "The Large Catechism"),
        new ConcordDocEntry(9, "Formula of Concord: Epitome"),
        new ConcordDocEntry(10, "Formula of Concord: Solid Declaration"),
    };

    // Every document restarts its own article/paragraph numbering at 1.1.
    public static string StartRef(int part) => $"BoC {part}.1.1";

    public static string TitleOf(int part) => Documents.FirstOrDefault(d => d.Part == part)?.Title ?? $"Part {part}";

    // Only Part 7 (The Small Catechism) has real graph-level explorable content;
    // Part 2 ("The Three Ecumenical Creeds") carries none despite the similar name --
    // the Catechism's own Creed article lives inside Part 7.
    private static readonly IReadOnlySet<int> ExplorableParts = new HashSet<int> { 7 };

    public static bool IsExplorablePart(int part) => ExplorableParts.Contains(part);
}
