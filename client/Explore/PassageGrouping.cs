namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch F2, 6-ARCH ("it should use the same underlying data structure as
/// the hover menu everywhere else... reuse the bits that we have!"): the
/// passage-grouping algorithm Batch D's <c>PlaceCard.razor</c> introduced
/// (maximal runs of numerically-consecutive same-book/chapter verses become
/// ONE passage; a run of one is a lone verse), EXTRACTED here so every
/// verse-list surface in the popover platform (cross-references, THE
/// SCRIPTURES, place est/dest supporting verses -- <c>PassageList.razor</c>)
/// reuses the EXACT same algorithm rather than a second, parallel
/// implementation. <c>PlaceCard.razor</c> itself now calls
/// <see cref="Groups"/> too (see that file's own comment) -- this is the
/// ONE place the "what counts as a passage" rule lives.
/// </summary>
public static class PassageGrouping
{
    /// A maximal run of consecutive same-book/chapter verses within an
    /// ordered verse-ref list, identified by its start index into that
    /// list. Length &gt;= 2 is a "passage" (renders as one flowing block);
    /// length == 1 is a lone verse.
    public sealed record VerseRun(int Start, int Length)
    {
        public bool IsPassage => Length >= 2;
    }

    /// Splits `verses` into maximal runs of numerically-consecutive same-
    /// book/chapter verses. Adjacency is checked purely by LIST POSITION,
    /// never a fresh sort -- two verses that are numerically back-to-back
    /// but arrived from non-adjacent sources (two different xref targets,
    /// two different catechism questions, ...) never silently merge into
    /// one passage UNLESS the caller's own list already placed them
    /// adjacently (see `PassageList.razor`'s own doc comment for how each
    /// consumer decides what one "source unit" is -- grouping never crosses
    /// a source-unit boundary there, by construction: each unit is grouped
    /// with its own, separate call to this method).
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

    /// The single ref label for a passage/lone-verse block (span form, e.g.
    /// `GEN.12.1-4`, or a bare `GEN.12.1` for a lone verse). `first`/`last`
    /// are always same book+chapter by construction (`Groups` only ever
    /// runs within one book+chapter at a time).
    public static string SpanRef(string firstVref, string lastVref)
    {
        var first = CanonRef.ParseVerse(firstVref);
        if (firstVref == lastVref)
        {
            return firstVref;
        }
        var last = CanonRef.ParseVerse(lastVref);
        return $"{first.Book}.{first.Chapter}.{first.Verse}-{last.Verse}";
    }
}
