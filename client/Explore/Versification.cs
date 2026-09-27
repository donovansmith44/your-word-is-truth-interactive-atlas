namespace BibleAtlas.Client.Explore;

// A missing book/chapter answers null, and every consumer must treat null conservatively:
// a chapter boundary whose length is unknown is never treated as contiguous -- render the
// honest compound list rather than guess that it joins.
public sealed class Versification
{
    private readonly Dictionary<string, int[]> _chapters;

    private Versification(Dictionary<string, int[]> chapters)
    {
        _chapters = chapters;
    }

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

    public int? VersesIn(string book, int chapter) =>
        _chapters.TryGetValue(book, out var counts) && chapter >= 1 && chapter <= counts.Length
            ? counts[chapter - 1]
            : null;
}
