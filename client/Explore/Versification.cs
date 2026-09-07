namespace BibleAtlas.Client.Explore;

/// <summary>
/// EVT-3 fix round 2 (ACCT-COALESCE-1, review Critical N-1): the client-side
/// chapter-length lookup that makes account coalescing VERSIFICATION-AWARE.
/// Whether two verse ranges inside ONE witness account are genuinely
/// contiguous across a chapter boundary (e.g. the Sermon on the Mount's own
/// MAT.5.48 -&gt; MAT.6.1) is a fact about the canon's own chapter lengths --
/// something the wire's per-witness <c>VerseGroup.Count</c> (that WITNESS's
/// verse total in the chapter, not the chapter's) can never answer. This
/// wraps the ONE canon source the client already has: <c>GET /api/books</c>
/// (<see cref="BookTocEntry.Chapters"/>, each book's own per-chapter verse
/// counts, straight from <c>data/compiled/canon.json</c>), fetched once and
/// memo-cached for the app's lifetime by <see cref="AtlasClient.Books"/> --
/// zero new endpoints, zero new fetches on any path that hasn't already
/// paid for the TOC.
///
/// A missing book/chapter answers <c>null</c>, and every consumer treats
/// null CONSERVATIVELY (a chapter boundary whose length is unknown is NOT
/// treated as contiguous) -- under the inerrancy law, "render the honest
/// compound list" is the only acceptable degrade; "guess that it joins"
/// never is.
/// </summary>
public sealed class Versification
{
    private readonly Dictionary<string, int[]> _chapters;

    private Versification(Dictionary<string, int[]> chapters)
    {
        _chapters = chapters;
    }

    /// <summary>Builds the lookup from the books TOC. Duplicate book codes
    /// (never real -- the TOC is the compiled canon) keep the first
    /// occurrence rather than throwing on a malformed network response.</summary>
    public static Versification From(IEnumerable<BookTocEntry> books)
    {
        var byCode = new Dictionary<string, int[]>();
        foreach (var b in books)
        {
            if (!byCode.ContainsKey(b.Code))
            {
                byCode[b.Code] = b.Chapters.ToArray();
            }
        }
        return new Versification(byCode);
    }

    /// <summary>The canon's own verse count for <paramref name="book"/>'s
    /// chapter <paramref name="chapter"/> (1-based), or null when the book
    /// or chapter is unknown -- callers must treat null conservatively (see
    /// the class doc comment).</summary>
    public int? VersesIn(string book, int chapter) =>
        _chapters.TryGetValue(book, out var counts) && chapter >= 1 && chapter <= counts.Length
            ? counts[chapter - 1]
            : null;
}
