namespace BibleAtlas.Client.Exploring;

public static class PassageGrouping
{
    public sealed record VerseRun(int Start, int Length)
    {
        public bool IsPassage => Length >= 2;
    }

    public static List<VerseRun> Groups(IReadOnlyList<string> verses)
    {
        var groups = new List<VerseRun>();
        var i = 0;
        while (i < verses.Count)
        {
            var start = i;
            var (book, chapter, num) = CanonRef.ParseVerse(verses[i]);
            i++;
            while (i < verses.Count)
            {
                var next = CanonRef.ParseVerse(verses[i]);
                if (next.Book != book || next.Chapter != chapter || next.Verse != num + 1)
                {
                    break;
                }

                num = next.Verse;
                i++;
            }

            groups.Add(new VerseRun(start, i - start));
        }

        return groups;
    }

    public static string SpanRef(string firstVref, string lastVref)
    {
        var first = CanonRef.ParseVerse(firstVref);
        if (firstVref == lastVref)
        {
            return firstVref;
        }
        var last = CanonRef.ParseVerse(lastVref);
        return first.Chapter == last.Chapter
            ? $"{first.Book}.{first.Chapter}.{first.Verse}-{last.Verse}"
            : $"{first.Book}.{first.Chapter}.{first.Verse}-{last.Chapter}.{last.Verse}";
    }
}
