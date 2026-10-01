using System.Text.RegularExpressions;
using BibleAtlas.Client.Contract;

namespace BibleAtlas.Client.Explore;

internal static class CanonRef
{
    private static readonly Regex HeadPattern = new(@"^[A-Z0-9]{3}\.\d+\.\d+", RegexOptions.Compiled);

    public static string FirstVerseOf(string target)
    {
        var m = HeadPattern.Match(target);
        return m.Success ? m.Value : target;
    }

    public static string VerseOf(BibleRef verse) => $"{verse.Book.WireName()}.{verse.Chapter}.{verse.Verse}";

    public static string SpanOf(TextSpan span) => PassageGrouping.SpanRef(VerseOf(FirstVerseOf(span)), VerseOf(LastVerseOf(span)));

    public static BibleRef BibleRefOf(string vref)
    {
        var (book, chapter, verse) = ParseVerse(vref);
        return new BibleRef(WireNames.Parse<BookId>(book), chapter, verse);
    }

    public static bool Covers(TextSpan span, BibleRef verse) =>
        verse.Book == FirstVerseOf(span).Book
        && (verse.Chapter, verse.Verse).CompareTo((FirstVerseOf(span).Chapter, FirstVerseOf(span).Verse)) >= 0
        && (verse.Chapter, verse.Verse).CompareTo((LastVerseOf(span).Chapter, LastVerseOf(span).Verse)) <= 0;

    public static BibleRef FirstVerseOf(TextSpan span) => (BibleRef)span.From.Unit;

    public static BibleRef LastVerseOf(TextSpan span) => (BibleRef)span.To.Unit;

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
}
