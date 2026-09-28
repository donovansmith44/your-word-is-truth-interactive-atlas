using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

// Places/Persons/WordsOfChrist: null means no source data to check; empty means checked, none found.
public sealed record PassageListVerse(string Vref, string Text, int? GroupCount = null, IReadOnlyList<PlaceRef>? Places = null, IReadOnlyList<PersonRef>? Persons = null, IReadOnlyList<WordsOfChristSpan>? WordsOfChrist = null);

public sealed record PassageSourceUnit(IReadOnlyList<PassageListVerse> Verses, string? Caption = null, bool CoalesceAcrossChapters = false, Versification? Canon = null);

public sealed record PassageBlockData(string Span, IReadOnlyList<PassageListVerse> Verses, string? Caption, int TruncatedBy = 0, string? FirstRangeEndVref = null, string? ExploreSref = null, int ExploreVerseCount = 0)
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
        var chapters = new Dictionary<(string, int), Chapter>();
        try
        {
            var fetched = await Task.WhenAll(pairs.Select(p => api.Chapter(p.Book, p.Chapter)));
            foreach (var (pair, chapter) in pairs.Zip(fetched))
            {
                chapters[pair] = chapter;
            }
        }
        catch (Exception)
        {
        }

        var result = new List<PassageListVerse>();
        foreach (var vref in vrefs)
        {
            var (book, chapter, verse) = CanonRef.ParseVerse(vref);
            if (!chapters.TryGetValue((book, chapter), out var c))
            {
                continue;
            }
            var cv = c.Verses.FirstOrDefault(v => v.Verse1 == verse);
            if (cv is not null)
            {
                result.Add(new PassageListVerse(vref, cv.Text, Places: cv.Places, Persons: cv.Persons, WordsOfChrist: cv.WordsOfChrist));
            }
        }
        return result;
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
        foreach (var unit in units)
        {
            if (unit.Verses.Count == 0)
            {
                continue;
            }

            if (unit.CoalesceAcrossChapters)
            {
                blocks.Add(BuildCoalescedBlock(unit));
                continue;
            }

            var vrefs = unit.Verses.Select(v => v.Vref).ToList();
            var unitBlocks = new List<PassageBlockData>();
            foreach (var run in PassageGrouping.Groups(vrefs))
            {
                var members = unit.Verses.Skip(run.Start).Take(run.Length).ToList();
                var span = PassageGrouping.SpanRef(members[0].Vref, members[^1].Vref);
                unitBlocks.Add(new PassageBlockData(span, members, unit.Caption));
            }

            var highestVerseForGroup = new Dictionary<(string Book, int Chapter), (int Verse, int TrueCount)>();
            foreach (var v in unit.Verses)
            {
                if (v.GroupCount is not int trueCount)
                {
                    continue;
                }
                var vid = CanonRef.ParseVerse(v.Vref);
                var key = (vid.Book, vid.Chapter);
                if (!highestVerseForGroup.TryGetValue(key, out var existing) || vid.Verse > existing.Verse)
                {
                    highestVerseForGroup[key] = (vid.Verse, trueCount);
                }
            }

            foreach (var block in unitBlocks)
            {
                var last = CanonRef.ParseVerse(block.LastVref);
                if (highestVerseForGroup.TryGetValue((last.Book, last.Chapter), out var top) && last.Verse == top.Verse)
                {
                    var truncatedBy = top.TrueCount - block.Verses.Count;
                    blocks.Add(truncatedBy > 0 ? block with { TruncatedBy = truncatedBy } : block);
                }
                else
                {
                    blocks.Add(block);
                }
            }
        }

        return blocks;
    }

    public static List<PassageListVerse> FlattenWitness(EventWitness witness) =>
        witness.VerseGroups.SelectMany(g => g.Verses.Select(v => new PassageListVerse(v, "", g.Count))).ToList();

    // FirstVerse/LastVerse are true extents (cap-corrected), not necessarily delivered verses.
    private sealed record AccountRange(string Book, int FirstChapter, int FirstVerse, int LastChapter, int LastVerse)
    {
        public string FirstVref => $"{Book}.{FirstChapter}.{FirstVerse}";
        public string LastVref => $"{Book}.{LastChapter}.{LastVerse}";
    }

    private sealed record TrueRun(string Book, int Chapter, int FirstVerse, int LastVerse, int Delivered);

    // Assumes unit.Verses is in ascending (book, chapter, verse) order; on unsorted input the
    // wire-cap remainder below can extend the wrong run.
    public static PassageBlockData BuildCoalescedBlock(PassageSourceUnit unit)
    {
        var runs = TrueRunsOf(unit.Verses, unit.Canon);
        var ranges = StitchRuns(runs, unit.Canon);
        var span = AccountSpan(ranges);

        var firstRun = runs[0];
        var exploreSref = PassageGrouping.SpanRef(
            $"{firstRun.Book}.{firstRun.Chapter}.{firstRun.FirstVerse}",
            $"{firstRun.Book}.{firstRun.Chapter}.{firstRun.LastVerse}");

        var byGroup = new Dictionary<(string Book, int Chapter), (int Delivered, int? TrueCount)>();
        foreach (var v in unit.Verses)
        {
            var (book, chapter, _) = CanonRef.ParseVerse(v.Vref);
            var existing = byGroup.GetValueOrDefault((book, chapter), (0, null));
            byGroup[(book, chapter)] = (existing.Delivered + 1, v.GroupCount ?? existing.TrueCount);
        }
        var truncatedBy = 0;
        foreach (var (_, entry) in byGroup)
        {
            if (entry.TrueCount is int trueCount && trueCount > entry.Delivered)
            {
                truncatedBy += trueCount - entry.Delivered;
            }
        }

        return new PassageBlockData(span, unit.Verses, unit.Caption, truncatedBy, ranges[0].LastVref, exploreSref, firstRun.Delivered);
    }

    private static List<TrueRun> TrueRunsOf(IReadOnlyList<PassageListVerse> verses, Versification? canon)
    {
        var runs = new List<TrueRun>();
        var i = 0;
        while (i < verses.Count)
        {
            var (book, chapter, _) = CanonRef.ParseVerse(verses[i].Vref);
            int? trueCount = null;
            var nums = new List<int>();
            while (i < verses.Count)
            {
                var v = CanonRef.ParseVerse(verses[i].Vref);
                if (v.Book != book || v.Chapter != chapter)
                {
                    break;
                }
                nums.Add(v.Verse);
                trueCount = verses[i].GroupCount ?? trueCount;
                i++;
            }

            var segmentRuns = new List<(int First, int Last, int Delivered)>();
            var (runStart, runEnd, runCount) = (nums[0], nums[0], 1);
            for (var k = 1; k < nums.Count; k++)
            {
                if (nums[k] == runEnd + 1)
                {
                    runEnd = nums[k];
                    runCount++;
                    continue;
                }
                segmentRuns.Add((runStart, runEnd, runCount));
                (runStart, runEnd, runCount) = (nums[k], nums[k], 1);
            }
            segmentRuns.Add((runStart, runEnd, runCount));

            // The server's cap keeps the lowest-numbered verses of a group, so any undelivered
            // remainder is assumed to continue after the segment's last delivered verse, clamped
            // to the chapter's real length when known (never below the last delivered verse).
            var undelivered = Math.Max(0, (trueCount ?? nums.Count) - nums.Count);
            if (undelivered > 0)
            {
                var last = segmentRuns[^1];
                var extendedEnd = last.Last + undelivered;
                if (canon?.VersesIn(book, chapter) is int chapterLen && extendedEnd > chapterLen)
                {
                    extendedEnd = Math.Max(chapterLen, last.Last);
                }
                segmentRuns[^1] = (last.First, extendedEnd, last.Delivered);
            }

            runs.AddRange(segmentRuns.Select(r => new TrueRun(book, chapter, r.First, r.Last, r.Delivered)));
        }
        return runs;
    }

    private static List<AccountRange> StitchRuns(List<TrueRun> runs, Versification? canon)
    {
        var ranges = new List<AccountRange>();
        AccountRange? current = null;
        foreach (var run in runs)
        {
            if (current is not null && run.Book == current.Book)
            {
                var sameChapterAdjacent = run.Chapter == current.LastChapter && run.FirstVerse == current.LastVerse + 1;
                var boundaryAdjacent = run.Chapter == current.LastChapter + 1 && run.FirstVerse == 1
                    && canon?.VersesIn(run.Book, current.LastChapter) == current.LastVerse;
                if (sameChapterAdjacent || boundaryAdjacent)
                {
                    current = current with { LastChapter = run.Chapter, LastVerse = run.LastVerse };
                    continue;
                }
            }
            if (current is not null)
            {
                ranges.Add(current);
            }
            current = new AccountRange(run.Book, run.Chapter, run.FirstVerse, run.Chapter, run.LastVerse);
        }
        if (current is not null)
        {
            ranges.Add(current);
        }
        return ranges;
    }

    private static string AccountSpan(List<AccountRange> ranges)
    {
        var sb = new System.Text.StringBuilder();
        AccountRange? prev = null;
        foreach (var r in ranges)
        {
            if (prev is null)
            {
                sb.Append(PassageGrouping.SpanRef(r.FirstVref, r.LastVref));
            }
            else
            {
                sb.Append(", ");
                if (r.Book != prev.Book)
                {
                    sb.Append(PassageGrouping.SpanRef(r.FirstVref, r.LastVref));
                }
                else if (r.FirstChapter != r.LastChapter)
                {
                    sb.Append($"{r.FirstChapter}.{r.FirstVerse}-{r.LastChapter}.{r.LastVerse}");
                }
                else if (r.FirstChapter == prev.LastChapter)
                {
                    sb.Append(r.FirstVerse == r.LastVerse ? $"{r.FirstVerse}" : $"{r.FirstVerse}-{r.LastVerse}");
                }
                else
                {
                    sb.Append(r.FirstVerse == r.LastVerse ? $"{r.FirstChapter}.{r.FirstVerse}" : $"{r.FirstChapter}.{r.FirstVerse}-{r.LastVerse}");
                }
            }
            prev = r;
        }
        return sb.ToString();
    }
}
