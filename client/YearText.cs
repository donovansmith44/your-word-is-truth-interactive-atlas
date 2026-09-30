namespace BibleAtlas.Client;

// Mirrors tests/ux/lib/years.ts's formatClaim -- keep the two in sync, or the TS property
// helpers and this implementation will disagree on wire text.
public static class YearText
{
    private const string EnDashSeparator = " – ";

    public static string FormatRange(int from, int to) =>
        from == to ? Format(from) : $"{Format(from)}{EnDashSeparator}{Format(to)}";

    public static string FormatClaim(int from, int to, string? note)
    {
        var range = FormatRange(from, to);
        return note is null ? range : $"c. {range}";
    }

    private static string Format(int year) => year < 0 ? $"{-year} BC" : $"AD {year}";
}
