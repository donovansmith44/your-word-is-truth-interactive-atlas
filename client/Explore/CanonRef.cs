using System.Text.RegularExpressions;

namespace BibleAtlas.Client.Explore;

internal static class CanonRef
{
    private static readonly Regex HeadPattern = new(@"^[A-Z0-9]{3}\.\d+\.\d+", RegexOptions.Compiled);

    public static string FirstVerseOf(string target)
    {
        var m = HeadPattern.Match(target);
        return m.Success ? m.Value : target;
    }

    public static (string Book, int Chapter, int Verse) ParseVerse(string vref)
    {
        var parts = vref.Split('.');
        return (parts[0], int.Parse(parts[1]), int.Parse(parts[2]));
    }

    /// Returns null for a cross-chapter or cross-book target (e.g. "MAT.5.3-MAT.6.2");
    /// callers fall back to a first-verse-only preview in that case.
    public static (string Book, int Chapter, int FromVerse, int ToVerse)? TargetSpan(string target)
    {
        var parts = target.Split('.');
        if (parts.Length != 3)
        {
            return null;
        }

        var book = parts[0];
        if (!int.TryParse(parts[1], out var chapter))
        {
            return null;
        }

        var versePart = parts[2];
        var dash = versePart.IndexOf('-');
        if (dash < 0)
        {
            return int.TryParse(versePart, out var v) ? (book, chapter, v, v) : null;
        }

        var fromOk = int.TryParse(versePart[..dash], out var fromVerse);
        var toOk = int.TryParse(versePart[(dash + 1)..], out var toVerse);
        return fromOk && toOk ? (book, chapter, fromVerse, toVerse) : null;
    }

    public static string Humanize(string slug) =>
        string.Join(' ', slug.Split('-', StringSplitOptions.RemoveEmptyEntries)
            .Select(w => char.ToUpperInvariant(w[0]) + w[1..]));
}
