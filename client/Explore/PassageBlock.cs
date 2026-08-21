namespace BibleAtlas.Client.Explore;

/// <summary>
/// Batch F2, 6-ARCH: one already-resolved verse (ref + full KJV text) going
/// into a <see cref="PassageSourceUnit"/> -- the common currency every
/// verse-list surface (cross-references, THE SCRIPTURES, place est/dest)
/// hands to <see cref="PassageGrouping"/>/<c>PassageList.razor</c>.
/// </summary>
public sealed record PassageListVerse(string Vref, string Text);

/// <summary>
/// One INDEPENDENT source of verses to group into passage blocks -- e.g.
/// one cross-reference target's own member verses, one catechism
/// question's own verses, or a place's own item-level embedded citation.
/// Grouping (<see cref="PassageGrouping.Groups"/>) runs SEPARATELY per
/// unit and never merges across two different units, even if their own
/// verses happen to be numerically adjacent -- two different xref targets,
/// or two different catechism questions, are two different pieces of
/// context and must never silently read as one contiguous passage.
/// <see cref="Caption"/> (e.g. a catechism question's own title) renders on
/// every block this unit produces, when present.
/// </summary>
public sealed record PassageSourceUnit(IReadOnlyList<PassageListVerse> Verses, string? Caption = null);

/// <summary>
/// One renderable passage/lone-verse block -- <see cref="PassageList.razor"/>'s
/// own unit of display, truncation-counting, and per-entry mini-reader
/// expansion. <see cref="Span"/> is the block's own ref-range (or bare
/// vref for a lone verse); <see cref="Verses"/> is 1 (lone verse) or more
/// (passage) already-resolved verses, in order.
/// </summary>
public sealed record PassageBlockData(string Span, IReadOnlyList<PassageListVerse> Verses, string? Caption)
{
    public bool IsPassage => Verses.Count >= 2;
    public string FirstVref => Verses[0].Vref;
    public string LastVref => Verses[^1].Vref;
}

/// <summary>
/// Batch F2, 6-ARCH: builds the full, ordered list of renderable
/// <see cref="PassageBlockData"/> from one or more <see cref="PassageSourceUnit"/>s
/// -- the one function every verse-list-with-grouping surface in this app
/// calls (<c>PassageList.razor</c>'s own resolve step; <c>PlaceCard.razor</c>
/// calls <see cref="PassageGrouping.Groups"/> directly instead, since its
/// own reveal-by-count mechanism -- Batch D, untouched by this batch -- is a
/// different truncation model from the down-arrow cap this batch adds; see
/// that file's own comment).
/// </summary>
/// <summary>
/// Batch F2, requirement 6b: resolves FULL KJV text for an already-
/// individually-canonical, ordered list of verse refs (e.g. a place's own
/// curated <c>established</c>/<c>destroyed</c> claim verses) via the
/// existing chapter fetch + LRU cache (<see cref="AtlasClient.Chapter"/>) --
/// the SAME "fetch whole chapters, slice out what's needed" pattern
/// <c>PlaceCard.razor</c>'s own <c>LoadChaptersFor</c> already established,
/// reused here rather than re-implemented (house verse rendering: ref +
/// FULL text, never truncated/invented). A verse whose own chapter fetch
/// fails, or that doesn't turn up in the fetched chapter, is simply
/// omitted -- graceful degrade, never a placeholder or fabricated text.
/// </summary>
public static class VerseTextResolver
{
    public static async Task<List<PassageListVerse>> ResolveAsync(AtlasClient api, IReadOnlyList<string> vrefs)
    {
        var pairs = vrefs.Select(CanonRef.ParseVerse).Select(p => (p.Book, p.Chapter)).Distinct().ToList();
        var chapters = new Dictionary<(string, int), ChapterOut>();
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
            // graceful degrade -- verses below simply resolve to nothing found
        }

        var result = new List<PassageListVerse>();
        foreach (var vref in vrefs)
        {
            var (book, chapter, verse) = CanonRef.ParseVerse(vref);
            if (!chapters.TryGetValue((book, chapter), out var c))
            {
                continue;
            }
            var text = c.Verses.FirstOrDefault(v => v.Verse == verse)?.Text;
            if (text is not null)
            {
                result.Add(new PassageListVerse(vref, text));
            }
        }
        return result;
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

            var vrefs = unit.Verses.Select(v => v.Vref).ToList();
            foreach (var run in PassageGrouping.Groups(vrefs))
            {
                var members = unit.Verses.Skip(run.Start).Take(run.Length).ToList();
                var span = PassageGrouping.SpanRef(members[0].Vref, members[^1].Vref);
                blocks.Add(new PassageBlockData(span, members, unit.Caption));
            }
        }

        return blocks;
    }
}
