using BibleAtlas.Client.Contract;
using BibleAtlas.Client.Exploring;

namespace BibleAtlas.Client.Legacy;

public sealed record PassageListVerse(string Vref, string Text, int? GroupCount = null, IReadOnlyList<Anchor>? Anchors = null, IReadOnlyList<WordsOfChristSpan>? WordsOfChrist = null)
{
    public static PassageListVerse Of(TextUnit unit) => new(unit.Ref, unit.Body.Text, Anchors: unit.Body.Anchors, WordsOfChrist: unit.Body.WordsOfChrist);
}

public record PassageSourceUnit(IReadOnlyList<PassageListVerse> Verses, string? Caption = null);

public sealed record AccountSourceUnit(IReadOnlyList<PassageListVerse> Verses, EventAccount Account) : PassageSourceUnit(Verses);

public sealed record PassageBlockData(string Span, IReadOnlyList<PassageListVerse> Verses, string? Caption, string? FirstRangeEndVref = null, string? ExploreSref = null, int ExploreVerseCount = 0)
{
    public bool IsPassage => Verses.Count >= 2;
    public string FirstVref => Verses[0].Vref;
    public string LastVref => Verses[^1].Vref;
}

public static class VerseTextResolver
{
    public static async Task<List<PassageListVerse>> ResolveAsync(AtlasClient api, IReadOnlyList<string> vrefs)
    {
        var pairs = vrefs.Select(CanonRef.ParseVerse).Select(p => (p.Book, p.Chapter)).Distinct().ToList();
        var chapters = new Dictionary<(string, int), ChapterText>();
        var fetched = await Task.WhenAll(pairs.Select(p => api.ChapterText(p.Book, p.Chapter)));
        foreach (var (pair, chapter) in pairs.Zip(fetched))
        {
            chapters[pair] = chapter;
        }

        var result = new List<PassageListVerse>();
        foreach (var vref in vrefs)
        {
            var (book, chapter, verse) = CanonRef.ParseVerse(vref);
            if (chapters.TryGetValue((book, chapter), out var c))
            {
                result.AddRange(c.Between(verse, verse).Select(PassageListVerse.Of));
            }
        }
        return result;
    }

    public static async Task<List<PassageListVerse>> ResolveSpansAsync(AtlasClient api, IReadOnlyList<TextSpan> spans)
    {
        var chapters = spans.SelectMany(ChaptersOf).Distinct().ToList();
        Dictionary<(BookId Book, int Chapter), ChapterText> text;
        var fetched = await Task.WhenAll(chapters.Select(c => api.ChapterText(c.Book.WireName(), c.Chapter)));
        text = chapters.Zip(fetched).ToDictionary(pair => pair.First, pair => pair.Second);

        return spans
            .SelectMany(span => ChaptersOf(span).SelectMany(c => text[c].Units.Where(unit => CanonRef.Covers(span, (BibleRef)unit.Body.Locus))))
            .Select(PassageListVerse.Of)
            .ToList();
    }

    private static IEnumerable<(BookId Book, int Chapter)> ChaptersOf(TextSpan span)
    {
        var (first, last) = (CanonRef.FirstVerseOf(span), CanonRef.LastVerseOf(span));
        return Enumerable.Range(first.Chapter, last.Chapter - first.Chapter + 1).Select(chapter => (first.Book, chapter));
    }

    public static async Task<List<PassageListVerse>> ResolveGroupsAsync(AtlasClient api, IReadOnlyList<VerseGroup> groups)
    {
        var countByVref = new Dictionary<string, int>();
        foreach (var g in groups)
        {
            foreach (var v in g.Verses)
            {
                countByVref[v] = g.Count;
            }
        }
        var resolved = await ResolveAsync(api, groups.SelectMany(g => g.Verses).ToList());
        return resolved.Select(v => v with { GroupCount = countByVref.TryGetValue(v.Vref, out var c) ? c : null }).ToList();
    }
}

public static class PassageBlockBuilder
{
    public static List<PassageBlockData> Build(IReadOnlyList<PassageSourceUnit> units)
    {
        var blocks = new List<PassageBlockData>();
        foreach (var unit in units.Where(unit => unit.Verses.Count > 0))
        {
            if (unit is AccountSourceUnit account)
            {
                blocks.Add(AccountBlock(account));
                continue;
            }

            var vrefs = unit.Verses.Select(v => v.Vref).ToList();
            foreach (var run in PassageGrouping.Groups(vrefs))
            {
                var members = unit.Verses.Skip(run.Start).Take(run.Length).ToList();
                blocks.Add(new PassageBlockData(PassageGrouping.SpanRef(members[0].Vref, members[^1].Vref), members, unit.Caption));
            }
        }
        return blocks;
    }

    private static PassageBlockData AccountBlock(AccountSourceUnit unit)
    {
        var firstRun = PassageGrouping.Groups(unit.Verses.Select(v => v.Vref).ToList())[0];
        var exploreSref = PassageGrouping.SpanRef(unit.Verses[0].Vref, unit.Verses[firstRun.Length - 1].Vref);
        return new PassageBlockData(unit.Account.Reference, unit.Verses, null, unit.Account.FirstRunEnd, exploreSref, firstRun.Length);
    }
}
